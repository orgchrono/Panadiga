//! `MAIN` partition splitter (`fmaX` command line + `SWUP_ADDON`) with `RomFS` & `CramFS` verification.

use anyhow::{Context, Result, bail};
use flate2::Crc;

use super::super::codec::{extract_c_ascii, read_u32_be, read_u32_le};
use super::types::{CramFsHeaderInfo, MainPartitionSlice, PartitionFsKind, RomFsHeaderInfo};
use crate::constants::{
    PANA_CRAMFS_FLAG_FSID_VERSION_2, PANA_CRAMFS_MAGIC, PANA_CRAMFS_SIGNATURE, PANA_ROMFS_MAGIC,
};

const CRAMFS_MAGIC: u32 = PANA_CRAMFS_MAGIC;
const CRAMFS_SIGNATURE: &[u8; 16] = PANA_CRAMFS_SIGNATURE;
const CRAMFS_FLAG_FSID_VERSION_2: u32 = PANA_CRAMFS_FLAG_FSID_VERSION_2;
const ROMFS_MAGIC: &[u8; 8] = PANA_ROMFS_MAGIC;

/// Computes `RomFS` negated big-endian 32-bit word sum (`ChecksumAFFS` from `pana-fw/utils/sum.go`).
pub fn checksum_affs(data: &[u8]) -> Result<u32> {
    if !data.len().is_multiple_of(4) {
        bail!(
            "checksum_affs requires 4-byte aligned slice, got {} B",
            data.len()
        );
    }
    let (chunks, _) = data.as_chunks::<4>();
    let sum = chunks
        .iter()
        .fold(0u32, |acc, &c| acc.wrapping_add(u32::from_be_bytes(c)));
    Ok(sum.wrapping_neg())
}

/// Validates a Linux `RomFS` superblock (`-rom1fs-` + `checksum_affs` over the first `min(size, 512)` bytes).
pub fn verify_romfs_header(data: &[u8]) -> Result<RomFsHeaderInfo> {
    if data.len() < 32 {
        bail!("RomFS header too short ({} B < 32)", data.len());
    }
    if &data[0..8] != ROMFS_MAGIC {
        bail!("Invalid RomFS magic");
    }
    let size = read_u32_be(data, 8);
    let checksum_be = read_u32_be(data, 12);
    let vol_end = data[16..]
        .iter()
        .position(|&b| b == 0)
        .context("Missing NUL-terminated RomFS volume name")?;
    let volume_name = extract_c_ascii(&data[16..16 + vol_end]);

    let sum_len = (size as usize).min(512).min(data.len()) & !3;
    if sum_len < 16 {
        bail!("RomFS image too small for checksum validation");
    }
    let mut sum_buf = data[..sum_len].to_vec();
    sum_buf[12..16].fill(0);
    let computed_checksum_be = checksum_affs(&sum_buf)?;

    Ok(RomFsHeaderInfo {
        size,
        checksum_be,
        computed_checksum_be,
        sum_valid: computed_checksum_be == checksum_be,
        volume_name,
    })
}

/// Validates a Linux `CramFS` superblock (`0x28CD3D45` / `"Compressed ROMFS"` + IEEE CRC32 when FSID v2 is set).
pub fn verify_cramfs_header(data: &[u8]) -> Result<CramFsHeaderInfo> {
    if data.len() < 64 {
        bail!("CramFS header too short ({} B < 64)", data.len());
    }
    if read_u32_le(data, 0) != CRAMFS_MAGIC || &data[16..32] != CRAMFS_SIGNATURE {
        bail!("Invalid CramFS magic or signature");
    }
    let size = read_u32_le(data, 4);
    let flags = read_u32_le(data, 8);
    let fsid_crc = read_u32_le(data, 32);
    let name = extract_c_ascii(&data[48..64]);

    let (computed_crc, crc_valid) = if (flags & CRAMFS_FLAG_FSID_VERSION_2) != 0 {
        let img_len = size as usize;
        if img_len < 64 || img_len > data.len() {
            (None, false)
        } else {
            let mut crc = Crc::new();
            crc.update(&data[..32]);
            crc.update(&[0u8; 4]);
            crc.update(&data[36..img_len]);
            let sum = crc.sum();
            (Some(sum), sum == fsid_crc)
        }
    } else {
        (None, true)
    };

    Ok(CramFsHeaderInfo {
        size,
        flags,
        fsid_crc,
        computed_crc,
        crc_valid,
        name,
    })
}

fn parse_fma_size(raw_val: &str) -> Option<usize> {
    let token = raw_val.split(':').next()?.trim();
    if let Some(num) = token.strip_suffix('k').or_else(|| token.strip_suffix('K')) {
        num.parse::<usize>().ok().map(|v| v.saturating_mul(1024))
    } else if let Some(num) = token.strip_suffix('m').or_else(|| token.strip_suffix('M')) {
        num.parse::<usize>()
            .ok()
            .map(|v| v.saturating_mul(1024 * 1024))
    } else {
        None
    }
}

fn classify_partition_slice(data: &[u8]) -> PartitionFsKind {
    if let Ok(info) = verify_romfs_header(data) {
        return PartitionFsKind::RomFs(info);
    }
    if let Ok(info) = verify_cramfs_header(data) {
        return PartitionFsKind::CramFs(info);
    }
    PartitionFsKind::Raw
}

/// Splits an unpacked `MAIN` kernel/rootfs image by parsing `fmaX=` kernel cmdline args (with `SWUP_ADDON` support).
pub fn split_main_partitions(buf: &[u8]) -> Result<Vec<MainPartitionSlice>> {
    let (cmd_start, main_limit, addon_offset) = if buf.len() >= 16 && buf.starts_with(b"SWUP_ADDON")
    {
        let off = read_u32_le(buf, 12) as usize;
        if off < 16 || off > buf.len() {
            bail!(
                "Invalid SWUP_ADDON offset 0x{off:X} for {} B buffer",
                buf.len()
            );
        }
        (16usize, off, Some(off))
    } else {
        (0usize, buf.len(), None)
    };

    let nul_rel = buf[cmd_start..main_limit]
        .iter()
        .position(|&b| b == 0)
        .context("Missing NUL terminator in MAIN partition cmdline")?;
    let cmdline = String::from_utf8_lossy(&buf[cmd_start..cmd_start + nul_rel]);

    let mut fma_defs: Vec<(String, usize)> = Vec::new();
    let mut root_name: Option<String> = None;
    for token in cmdline.split(['\n', '\r', ' ', '\t']) {
        let arg = token.trim();
        if let Some(dev) = arg.strip_prefix("root=/dev/") {
            root_name = Some(dev.trim().to_string());
        } else if let Some((k, v)) = arg.split_once('=')
            && k.starts_with("fma")
            && let Some(sz) = parse_fma_size(v)
        {
            fma_defs.push((k.to_string(), sz));
        }
    }

    let root = root_name.context("Missing root=/dev/fmaX entry in MAIN cmdline")?;
    let root_idx = fma_defs
        .iter()
        .position(|(n, _)| n == &root)
        .with_context(|| format!("Root partition '{root}' missing from fma definitions"))?;

    let mut out = Vec::new();
    let mut offset = 0usize;
    for (name, sz) in fma_defs.iter().skip(root_idx.saturating_sub(1)) {
        if offset >= main_limit {
            break;
        }
        if *sz == 0 {
            continue;
        }
        let end = offset.saturating_add(*sz).min(main_limit);
        let data = buf[offset..end].to_vec();
        let fs_kind = classify_partition_slice(&data);
        out.push(MainPartitionSlice {
            name: name.clone(),
            offset,
            fs_kind,
            data,
        });
        offset = end;
    }

    if let Some(addon_off) = addon_offset {
        out.push(MainPartitionSlice {
            name: "SWUP_ADDON".to_string(),
            offset: addon_off,
            fs_kind: PartitionFsKind::SwupAddon,
            data: buf[addon_off..].to_vec(),
        });
    }

    Ok(out)
}

fn validate_replacement_fs(part_name: &str, data: &[u8]) -> Result<()> {
    if data.starts_with(ROMFS_MAGIC) {
        let info = verify_romfs_header(data)
            .with_context(|| format!("Invalid RomFS replacement for '{part_name}'"))?;
        if !info.sum_valid {
            bail!(
                "RomFS checksum mismatch for '{part_name}': header=0x{:08X} computed=0x{:08X}",
                info.checksum_be,
                info.computed_checksum_be
            );
        }
    } else if data.len() >= 64
        && read_u32_le(data, 0) == CRAMFS_MAGIC
        && &data[16..32] == CRAMFS_SIGNATURE
    {
        let info = verify_cramfs_header(data)
            .with_context(|| format!("Invalid CramFS replacement for '{part_name}'"))?;
        if !info.crc_valid {
            bail!(
                "CramFS FSID CRC32 mismatch for '{part_name}': header=0x{:08X} computed={:?}",
                info.fsid_crc,
                info.computed_crc
            );
        }
    }
    Ok(())
}

/// Reassembles a `MAIN` image by replacing selected `fmaX` or `SWUP_ADDON` partitions with validation.
pub fn reassemble_main_partitions(
    original_main: &[u8],
    replacements: &[(String, Vec<u8>)],
) -> Result<Vec<u8>> {
    let slices = split_main_partitions(original_main)?;
    let mut out = original_main.to_vec();

    for (raw_name, rep_bytes) in replacements {
        let target_name = raw_name.trim();
        let Some(slice) = slices
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(target_name))
        else {
            let available: Vec<&str> = slices.iter().map(|s| s.name.as_str()).collect();
            bail!(
                "Partition '{target_name}' not found in MAIN image (available: {})",
                available.join(", ")
            );
        };

        if target_name.eq_ignore_ascii_case("SWUP_ADDON") {
            out.truncate(slice.offset);
            out.extend_from_slice(rep_bytes);
            if out.len() >= 16 && out.starts_with(b"SWUP_ADDON") {
                out[12..16].copy_from_slice(&u32::try_from(slice.offset)?.to_le_bytes());
            }
            continue;
        }

        validate_replacement_fs(target_name, rep_bytes)?;
        let cap = slice.data.len();
        if rep_bytes.len() > cap {
            bail!(
                "Replacement for '{}' ({} B) exceeds partition capacity ({cap} B)",
                slice.name,
                rep_bytes.len()
            );
        }
        let end = slice.offset + cap;
        if end > out.len() {
            out.resize(end, 0);
        }
        out[slice.offset..slice.offset + rep_bytes.len()].copy_from_slice(rep_bytes);
        out[slice.offset + rep_bytes.len()..end].fill(0);
    }

    Ok(out)
}
