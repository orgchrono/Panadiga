#![deny(unsafe_code)]

//! Read-only, Windows-safe `RomFS` and `CramFS` filesystem extractor for Panasonic `MAIN` `fmaX` partitions.
//!
//! Avoids OS symlink / device-node creation (recording them safely in the structured report)
//! and sanitizes relative paths against `..` traversal and reserved NTFS names/characters.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};
use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::super::codec::{extract_c_ascii, read_u16_le, read_u32_be, read_u32_le};
use crate::constants::{PANA_CRAMFS_MAGIC, PANA_CRAMFS_SIGNATURE, PANA_ROMFS_MAGIC};

/// Classification of an extracted filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractedFsEntryKind {
    RegularFile,
    Directory,
    Symlink,
    SpecialNode,
}

/// Decoded ELF header architectural metadata discovered on a regular file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElfMetadata {
    pub class_bits: u8,
    pub endian: String,
    pub e_machine: u16,
    pub arch_label: String,
    pub entry_va_hex: String,
}

/// Structured record for a single file/directory/symlink/node inside `RomFS` or `CramFS`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedFsEntry {
    pub rel_path: String,
    pub kind: ExtractedFsEntryKind,
    pub size_bytes: usize,
    pub symlink_target: Option<String>,
    pub sha256: Option<String>,
    pub elf: Option<ElfMetadata>,
    pub written_path: Option<String>,
}

/// Summary report emitted by [`extract_romfs_to_dir`], [`extract_cramfs_to_dir`], and [`extract_partition_fs`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsExtractReport {
    pub fs_kind: String,
    pub volume_name: String,
    pub total_entries: usize,
    pub regular_files: usize,
    pub symlinks_skipped_on_disk: usize,
    pub special_nodes_skipped_on_disk: usize,
    pub elf_count: usize,
    pub entries: Vec<ExtractedFsEntry>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn is_windows_reserved_stem(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or("");
    let upper = base.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.len() == 4
            && upper.as_bytes()[3].is_ascii_digit()
            && upper.as_bytes()[3] != b'0')
}

/// Sanitizes an internal archive path to prevent traversal and illegal Windows NTFS components.
pub fn sanitize_fs_rel_path(raw: &str) -> Result<String> {
    if raw.contains('\0') {
        bail!("Filesystem path contains interior NUL byte");
    }
    let mut clean_parts: Vec<String> = Vec::new();
    for part in raw.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            bail!("Unsafe path traversal '..' in filesystem entry '{raw}'");
        }
        let mut sanitized: String = part
            .chars()
            .map(|c| {
                if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        while sanitized.ends_with(['.', ' ']) {
            sanitized.pop();
        }
        if sanitized.is_empty() {
            sanitized = "_".to_string();
        }
        if is_windows_reserved_stem(&sanitized) {
            sanitized = format!("_{sanitized}");
        }
        clean_parts.push(sanitized);
    }
    if clean_parts.is_empty() {
        bail!("Empty sanitized path for '{raw}'");
    }
    Ok(clean_parts.join("/"))
}

fn elf_machine_label(e_machine: u16) -> String {
    match e_machine {
        0x59 | 0x5A => "mn10300".to_string(),
        0x28 => "arm".to_string(),
        0xB7 => "aarch64".to_string(),
        0x02 | 0x12 | 0x2B => "sparc".to_string(),
        0x08 | 0x0A => "mips".to_string(),
        0x03 => "x86".to_string(),
        0x3E => "x86_64".to_string(),
        0x14 | 0x15 => "powerpc".to_string(),
        0x2A => "sh".to_string(),
        other => format!("machine_0x{other:02X}"),
    }
}

/// Extracts ELF class, endianness, machine architecture, and entry VA if `data` starts with `\x7fELF`.
pub fn parse_elf_metadata(data: &[u8]) -> Option<ElfMetadata> {
    if data.len() < 28 || &data[0..4] != b"\x7fELF" {
        return None;
    }
    let class_bits = match data[4] {
        1 => 32u8,
        2 => 64u8,
        _ => return None,
    };
    let is_le = match data[5] {
        1 => true,
        2 => false,
        _ => return None,
    };
    let e_machine = if is_le {
        read_u16_le(data, 18)
    } else {
        u16::from_be_bytes([data[18], data[19]])
    };
    let entry_va: u64 = match (class_bits, is_le) {
        (32, true) => u64::from(read_u32_le(data, 24)),
        (32, false) => u64::from(u32::from_be_bytes(data[24..28].try_into().ok()?)),
        (64, true) if data.len() >= 32 => u64::from_le_bytes(data[24..32].try_into().ok()?),
        (64, false) if data.len() >= 32 => u64::from_be_bytes(data[24..32].try_into().ok()?),
        _ => 0,
    };
    Some(ElfMetadata {
        class_bits,
        endian: if is_le { "little" } else { "big" }.to_string(),
        e_machine,
        arch_label: elf_machine_label(e_machine),
        entry_va_hex: format!("0x{entry_va:08X}"),
    })
}

fn push_regular_entry(
    out_dir: Option<&Path>,
    elf_only: bool,
    rel_path: String,
    payload: Option<&[u8]>,
    declared_size: usize,
    report: &mut FsExtractReport,
) -> Result<()> {
    report.regular_files += 1;
    let elf = payload.and_then(parse_elf_metadata);
    if elf.is_some() {
        report.elf_count += 1;
    }
    let sha = payload.map(sha256_hex);
    let written_path = match (out_dir, payload) {
        (Some(root), Some(bytes)) if !elf_only || elf.is_some() => {
            let mut dest = root.to_path_buf();
            for comp in rel_path.split('/') {
                dest.push(comp);
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Creating parent dir {}", parent.display()))?;
            }
            std::fs::write(&dest, bytes)
                .with_context(|| format!("Writing extracted file {}", dest.display()))?;
            Some(dest.to_string_lossy().into_owned())
        }
        _ => None,
    };
    report.entries.push(ExtractedFsEntry {
        rel_path,
        kind: ExtractedFsEntryKind::RegularFile,
        size_bytes: declared_size,
        symlink_target: None,
        sha256: sha,
        elf,
        written_path,
    });
    Ok(())
}

fn push_non_regular_entry(
    rel_path: String,
    kind: ExtractedFsEntryKind,
    size_bytes: usize,
    symlink_target: Option<String>,
    report: &mut FsExtractReport,
) {
    match kind {
        ExtractedFsEntryKind::Symlink => report.symlinks_skipped_on_disk += 1,
        ExtractedFsEntryKind::SpecialNode => report.special_nodes_skipped_on_disk += 1,
        ExtractedFsEntryKind::Directory | ExtractedFsEntryKind::RegularFile => {}
    }
    report.entries.push(ExtractedFsEntry {
        rel_path,
        kind,
        size_bytes,
        symlink_target,
        sha256: None,
        elf: None,
        written_path: None,
    });
}

fn new_extract_report(fs_kind: &str, volume_name: String) -> FsExtractReport {
    FsExtractReport {
        fs_kind: fs_kind.to_string(),
        volume_name,
        total_entries: 0,
        regular_files: 0,
        symlinks_skipped_on_disk: 0,
        special_nodes_skipped_on_disk: 0,
        elf_count: 0,
        entries: Vec::new(),
    }
}

/// Extracts a Linux `-rom1fs-` partition read-only and safely (skipping symlinks/special nodes on disk).
pub fn extract_romfs_to_dir(
    data: &[u8],
    out_dir: Option<&Path>,
    elf_only: bool,
) -> Result<FsExtractReport> {
    if data.len() < 32 || &data[0..8] != PANA_ROMFS_MAGIC {
        bail!("Invalid RomFS superblock");
    }
    let img_limit = (read_u32_be(data, 8) as usize).min(data.len());
    let vol_end = data[16..img_limit]
        .iter()
        .position(|&b| b == 0)
        .map(|rel| 16 + rel)
        .context("Missing NUL terminator on RomFS volume name")?;
    let volume_name = extract_c_ascii(&data[16..vol_end]);
    let root_start = (vol_end + 16) & !15;

    let mut report = new_extract_report("RomFS", volume_name);
    let mut stack: Vec<(usize, String)> = vec![(root_start, String::new())];
    let mut visited_dirs: BTreeSet<usize> = BTreeSet::new();

    while let Some((first_off, prefix)) = stack.pop() {
        let mut cur = first_off;
        while cur != 0 && cur + 16 <= img_limit {
            if !visited_dirs.insert(cur) {
                break;
            }
            let next_fh = read_u32_be(data, cur);
            let spec_info = read_u32_be(data, cur + 4) as usize;
            let fsize = read_u32_be(data, cur + 8) as usize;
            let ftype = next_fh & 0x7;
            let next_off = (next_fh & !0xF) as usize;

            let Some(name_rel_end) = data[cur + 16..img_limit].iter().position(|&b| b == 0) else {
                break;
            };
            let name_end = cur + 16 + name_rel_end;
            let raw_name = extract_c_ascii(&data[cur + 16..name_end]);
            let data_start = ((name_end - cur) + 16) & !15;
            let payload_off = cur + data_start;

            if raw_name != "." && raw_name != ".." && !raw_name.is_empty() {
                let raw_rel = if prefix.is_empty() {
                    raw_name
                } else {
                    format!("{prefix}/{raw_name}")
                };
                let rel_path = sanitize_fs_rel_path(&raw_rel)?;
                match ftype {
                    1 => {
                        push_non_regular_entry(
                            rel_path.clone(),
                            ExtractedFsEntryKind::Directory,
                            0,
                            None,
                            &mut report,
                        );
                        if spec_info != 0 && spec_info < img_limit {
                            stack.push((spec_info, rel_path));
                        }
                    }
                    2 => {
                        let end = payload_off.saturating_add(fsize).min(img_limit);
                        let slice = &data[payload_off.min(img_limit)..end];
                        let effective = if elf_only && !slice.starts_with(b"\x7fELF") {
                            None
                        } else {
                            Some(slice)
                        };
                        push_regular_entry(
                            out_dir,
                            elf_only,
                            rel_path,
                            effective,
                            fsize,
                            &mut report,
                        )?;
                    }
                    3 => {
                        let end = payload_off.saturating_add(fsize).min(img_limit);
                        let target = extract_c_ascii(&data[payload_off.min(img_limit)..end]);
                        push_non_regular_entry(
                            rel_path,
                            ExtractedFsEntryKind::Symlink,
                            fsize,
                            Some(target),
                            &mut report,
                        );
                    }
                    _ => push_non_regular_entry(
                        rel_path,
                        ExtractedFsEntryKind::SpecialNode,
                        fsize,
                        None,
                        &mut report,
                    ),
                }
            }
            cur = next_off;
        }
    }

    report.total_entries = report.entries.len();
    Ok(report)
}

fn decompress_cramfs_payload(
    data: &[u8],
    offset: usize,
    fsize: usize,
    check_elf_only: bool,
) -> Result<Option<Vec<u8>>> {
    if fsize == 0 || offset == 0 {
        return Ok(if check_elf_only {
            None
        } else {
            Some(Vec::new())
        });
    }
    if check_elf_only && fsize < 28 {
        return Ok(None);
    }
    let nblks = fsize.div_ceil(4096);
    let table_end = offset
        .checked_add(nblks * 4)
        .filter(|&e| e <= data.len())
        .context("CramFS block table exceeds partition buffer")?;

    let mut out = Vec::with_capacity(if check_elf_only { 4096 } else { fsize });
    let mut prev = table_end;

    for blk_idx in 0..nblks {
        let blk_end = read_u32_le(data, offset + blk_idx * 4) as usize;
        if blk_end < prev || blk_end > data.len() {
            bail!("Invalid CramFS block bounds 0x{prev:X}..0x{blk_end:X}");
        }
        let target_blk_len = (fsize - out.len()).min(4096);
        if blk_end == prev {
            out.resize(out.len() + target_blk_len, 0);
        } else {
            let mut dec = ZlibDecoder::new(&data[prev..blk_end]);
            let mut blk = Vec::with_capacity(target_blk_len);
            dec.read_to_end(&mut blk)
                .with_context(|| format!("Decompressing CramFS block {blk_idx} at 0x{prev:X}"))?;
            out.extend_from_slice(&blk);
        }
        if check_elf_only && blk_idx == 0 {
            if !out.starts_with(b"\x7fELF") {
                return Ok(None);
            }
            out.reserve(fsize.saturating_sub(out.len()));
        }
        prev = blk_end;
    }

    out.truncate(fsize);
    Ok(Some(out))
}

/// Extracts a Linux `CramFS` v1/v2 partition read-only and safely (skipping symlinks/special nodes on disk).
pub fn extract_cramfs_to_dir(
    data: &[u8],
    out_dir: Option<&Path>,
    elf_only: bool,
) -> Result<FsExtractReport> {
    if data.len() < 76
        || read_u32_le(data, 0) != PANA_CRAMFS_MAGIC
        || &data[16..32] != PANA_CRAMFS_SIGNATURE
    {
        bail!("Invalid CramFS superblock");
    }
    let volume_name = extract_c_ascii(&data[48..64]);
    let w1 = read_u32_le(data, 68);
    let w2 = read_u32_le(data, 72);
    let root_size = (w1 & 0x00FF_FFFF) as usize;
    let root_off = ((w2 >> 6) << 2) as usize;

    let mut report = new_extract_report("CramFS", volume_name);
    if root_off == 0 || root_size == 0 {
        return Ok(report);
    }

    let mut stack: Vec<(usize, usize, String, usize)> = vec![(
        root_off,
        root_off.saturating_add(root_size).min(data.len()),
        String::new(),
        0,
    )];
    let mut visited_dirs: BTreeSet<usize> = BTreeSet::new();

    while let Some((dir_off, dir_end, prefix, depth)) = stack.pop() {
        if depth > 32 || !visited_dirs.insert(dir_off) {
            continue;
        }
        let mut pos = dir_off;
        while pos + 12 <= dir_end && pos + 12 <= data.len() {
            let iw0 = read_u32_le(data, pos);
            let iw1 = read_u32_le(data, pos + 4);
            let iw2 = read_u32_le(data, pos + 8);
            let mode = (iw0 & 0xFFFF) as u16;
            let fsize = (iw1 & 0x00FF_FFFF) as usize;
            let namelen = ((iw2 & 0x3F) << 2) as usize;
            let offset = ((iw2 >> 6) << 2) as usize;

            let name_end = pos + 12 + namelen;
            if name_end > data.len() {
                break;
            }
            let raw_name = extract_c_ascii(&data[pos + 12..name_end]);
            pos = name_end;

            if raw_name.is_empty() || raw_name == "." || raw_name == ".." {
                continue;
            }
            let raw_rel = if prefix.is_empty() {
                raw_name
            } else {
                format!("{prefix}/{raw_name}")
            };
            let rel_path = sanitize_fs_rel_path(&raw_rel)?;
            let ftype = (mode >> 12) & 0xF;

            match ftype {
                0x4 => {
                    push_non_regular_entry(
                        rel_path.clone(),
                        ExtractedFsEntryKind::Directory,
                        0,
                        None,
                        &mut report,
                    );
                    if offset != 0 && fsize != 0 && offset < data.len() {
                        let sub_end = offset.saturating_add(fsize).min(data.len());
                        stack.push((offset, sub_end, rel_path, depth + 1));
                    }
                }
                0x8 => {
                    let payload = decompress_cramfs_payload(data, offset, fsize, elf_only)?;
                    push_regular_entry(
                        out_dir,
                        elf_only,
                        rel_path,
                        payload.as_deref(),
                        fsize,
                        &mut report,
                    )?;
                }
                0xA => {
                    let target = decompress_cramfs_payload(data, offset, fsize, false)?
                        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                        .unwrap_or_default();
                    push_non_regular_entry(
                        rel_path,
                        ExtractedFsEntryKind::Symlink,
                        fsize,
                        Some(target),
                        &mut report,
                    );
                }
                _ => push_non_regular_entry(
                    rel_path,
                    ExtractedFsEntryKind::SpecialNode,
                    fsize,
                    None,
                    &mut report,
                ),
            }
        }
    }

    report.total_entries = report.entries.len();
    Ok(report)
}

/// Auto-detects `RomFS` or `CramFS` in `data` and runs read-only Windows-safe extraction.
pub fn extract_partition_fs(
    data: &[u8],
    out_dir: Option<&Path>,
    elf_only: bool,
) -> Result<FsExtractReport> {
    if data.len() >= 32 && &data[0..8] == PANA_ROMFS_MAGIC {
        extract_romfs_to_dir(data, out_dir, elf_only)
    } else if data.len() >= 76
        && read_u32_le(data, 0) == PANA_CRAMFS_MAGIC
        && &data[16..32] == PANA_CRAMFS_SIGNATURE
    {
        extract_cramfs_to_dir(data, out_dir, elf_only)
    } else {
        bail!("Unsupported or non-filesystem partition slice (expected RomFS or CramFS)")
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    use super::*;
    use crate::crypto::pana_fw_go::checksum_affs;

    fn make_synthetic_mn103_elf() -> Vec<u8> {
        let mut elf = vec![0u8; 64];
        elf[0..6].copy_from_slice(b"\x7fELF\x01\x01");
        elf[16..18].copy_from_slice(&2u16.to_le_bytes());
        elf[18..20].copy_from_slice(&0x59u16.to_le_bytes());
        elf[24..28].copy_from_slice(&0x0040_1234u32.to_le_bytes());
        elf
    }

    fn build_rich_romfs() -> Vec<u8> {
        fn pad16_str(s: &[u8]) -> Vec<u8> {
            let mut out = s.to_vec();
            out.push(0);
            while !out.len().is_multiple_of(16) {
                out.push(0);
            }
            out
        }

        let elf = make_synthetic_mn103_elf();
        let sym_target = b"/bin/busybox";

        let mut img = vec![0u8; 16];
        img[0..8].copy_from_slice(PANA_ROMFS_MAGIC);
        img.extend_from_slice(&pad16_str(b"panatest"));

        let hdr0 = img.len();
        img.extend_from_slice(&[0u8; 16]);
        img.extend_from_slice(&pad16_str(b"app.elf"));
        let data0_off = img.len();
        let mut data0 = elf.clone();
        while !data0.len().is_multiple_of(16) {
            data0.push(0);
        }
        img.extend_from_slice(&data0);

        let hdr1 = img.len();
        img.extend_from_slice(&[0u8; 16]);
        img.extend_from_slice(&pad16_str(b"sh"));
        let mut sdata = sym_target.to_vec();
        while !sdata.len().is_multiple_of(16) {
            sdata.push(0);
        }
        img.extend_from_slice(&sdata);

        let hdr2 = img.len();
        img.extend_from_slice(&[0u8; 16]);
        img.extend_from_slice(&pad16_str(b"README.txt"));
        img.extend_from_slice(&pad16_str(b"hello_panadiga"));

        let total_sz = img.len() as u32;
        img[8..12].copy_from_slice(&total_sz.to_be_bytes());
        img[hdr0..hdr0 + 4].copy_from_slice(&((hdr1 as u32) | 2).to_be_bytes());
        img[hdr0 + 8..hdr0 + 12].copy_from_slice(&(elf.len() as u32).to_be_bytes());
        let _ = data0_off;
        img[hdr1..hdr1 + 4].copy_from_slice(&((hdr2 as u32) | 3).to_be_bytes());
        img[hdr1 + 8..hdr1 + 12].copy_from_slice(&(sym_target.len() as u32).to_be_bytes());
        img[hdr2..hdr2 + 4].copy_from_slice(&2u32.to_be_bytes());
        img[hdr2 + 8..hdr2 + 12].copy_from_slice(&(14u32).to_be_bytes());

        let sum_len = img.len().min(512) & !3;
        let csum = checksum_affs(&img[..sum_len]).unwrap();
        img[12..16].copy_from_slice(&csum.to_be_bytes());
        img
    }

    fn build_rich_cramfs() -> Vec<u8> {
        let mut img = vec![0u8; 76];
        img[0..4].copy_from_slice(&PANA_CRAMFS_MAGIC.to_le_bytes());
        img[4..8].copy_from_slice(&65536u32.to_le_bytes()); // deliberate dummy v1 size!
        img[16..32].copy_from_slice(PANA_CRAMFS_SIGNATURE);
        img[48..58].copy_from_slice(b"Compressed");

        // Root inode at 64: dir pointing to child list right at 76
        img[64..66].copy_from_slice(&0o040755u16.to_le_bytes());
        let root_children_off = img.len();
        let elf = make_synthetic_mn103_elf();

        let mut enc_elf = ZlibEncoder::new(Vec::new(), Compression::default());
        enc_elf.write_all(&elf).unwrap();
        let z_elf = enc_elf.finish().unwrap();

        // 1 child inode inside root directory: name = "bin\0" (4B) -> 16 bytes
        let elf_inode_off = root_children_off;
        img.resize(root_children_off + 16, 0);
        let blk_table_off = img.len();
        let blk_end = blk_table_off + 4 + z_elf.len();
        img.extend_from_slice(&(blk_end as u32).to_le_bytes());
        img.extend_from_slice(&z_elf);

        let w0 = 0o100755u32;
        let w1 = elf.len() as u32;
        let w2 = 1u32 | (((blk_table_off >> 2) as u32) << 6); // namelen/4 = 1 (4 bytes "a\0\0\0")
        img[elf_inode_off..elf_inode_off + 4].copy_from_slice(&w0.to_le_bytes());
        img[elf_inode_off + 4..elf_inode_off + 8].copy_from_slice(&w1.to_le_bytes());
        img[elf_inode_off + 8..elf_inode_off + 12].copy_from_slice(&w2.to_le_bytes());
        img[elf_inode_off + 12..elf_inode_off + 16].copy_from_slice(b"a.so");

        // Update root inode size = 16, offset = root_children_off >> 2
        img[68..72].copy_from_slice(&16u32.to_le_bytes());
        img[72..76].copy_from_slice(&(((root_children_off >> 2) as u32) << 6).to_le_bytes());
        img
    }

    #[test]
    fn sanitize_path_rejects_traversal_and_normalizes_ntfs_hazards() {
        assert!(sanitize_fs_rel_path("../etc/passwd").is_err());
        assert!(sanitize_fs_rel_path("bin/\0bad").is_err());
        assert_eq!(
            sanitize_fs_rel_path("/bin/CON.txt").unwrap(),
            "bin/_CON.txt"
        );
        assert_eq!(sanitize_fs_rel_path("dir/a:b*c. ").unwrap(), "dir/a_b_c");
    }

    #[test]
    fn extract_romfs_and_cramfs_handle_elfs_and_symlinks_safely_on_windows() {
        let dir = std::env::temp_dir().join(format!("panadiga_fs_extract_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let romfs = build_rich_romfs();
        let rom_rep = extract_partition_fs(&romfs, Some(&dir.join("romfs_elf")), true).unwrap();
        assert_eq!(rom_rep.fs_kind, "RomFS");
        assert_eq!(rom_rep.regular_files, 2);
        assert_eq!(rom_rep.symlinks_skipped_on_disk, 1);
        assert_eq!(rom_rep.elf_count, 1);
        assert!(dir.join("romfs_elf").join("app.elf").is_file());
        assert!(!dir.join("romfs_elf").join("README.txt").exists());
        assert!(!dir.join("romfs_elf").join("sh").exists());

        let cramfs = build_rich_cramfs();
        let cram_rep = extract_partition_fs(&cramfs, Some(&dir.join("cramfs")), false).unwrap();
        assert_eq!(cram_rep.fs_kind, "CramFS");
        assert_eq!(cram_rep.elf_count, 1);
        let ent = cram_rep.entries.first().unwrap();
        assert_eq!(ent.elf.as_ref().unwrap().arch_label, "mn10300");
        assert!(dir.join("cramfs").join("a.so").is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_partition_fs_real_bs850_partitions_if_present() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../firmware_and_images/dmr-xw_bd_series/bs850_bs750_xs350eg/PANA_DVD.FRM"
        ));
        if !path.is_file() {
            return;
        }
        let raw = std::fs::read(path).unwrap();
        let keyring = crate::crypto::FrmKeyring::builtin();
        let unpacked = crate::crypto::unpack_universal_frm(&raw, &keyring).unwrap();
        let main = unpacked.sub_containers[0]
            .modules
            .iter()
            .find(|m| m.name.starts_with("MAIN"))
            .unwrap();
        let parts = crate::crypto::split_main_partitions(&main.payload).unwrap();
        let mut total_elfs = 0usize;
        for part in &parts {
            if matches!(
                part.fs_kind,
                crate::crypto::PartitionFsKind::RomFs(_)
                    | crate::crypto::PartitionFsKind::CramFs(_)
            ) {
                let rep = extract_partition_fs(&part.data, None, true).unwrap();
                assert!(rep.regular_files > 0);
                total_elfs += rep.elf_count;
            }
        }
        assert!(
            total_elfs >= 200,
            "Expected >200 ELFs across bs850 fma5/fma6/fma7, got {total_elfs}"
        );
    }
}
