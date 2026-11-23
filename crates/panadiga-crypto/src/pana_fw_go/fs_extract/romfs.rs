#![deny(unsafe_code)]

//! Read-only Windows-safe Linux `-rom1fs-` extractor.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::super::super::codec::{extract_c_ascii, read_u32_be};
use super::types::{
    ExtractedFsEntryKind, FsExtractReport, new_extract_report, push_non_regular_entry,
    push_regular_entry, sanitize_fs_rel_path,
};
use crate::constants::PANA_ROMFS_MAGIC;

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
