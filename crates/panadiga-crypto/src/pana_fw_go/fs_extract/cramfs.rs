#![deny(unsafe_code)]

//! Read-only Windows-safe Linux `Compressed ROMFS` (`CramFS`) extractor.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};
use flate2::read::ZlibDecoder;

use super::super::super::codec::{extract_c_ascii, read_u32_le};
use super::types::{
    ExtractedFsEntryKind, FsExtractReport, new_extract_report, push_non_regular_entry,
    push_regular_entry, sanitize_fs_rel_path,
};
use crate::constants::{PANA_CRAMFS_MAGIC, PANA_CRAMFS_SIGNATURE};

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
