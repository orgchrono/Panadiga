#![deny(unsafe_code)]

//! Structured report types, NTFS-safe path sanitization, and ELF inspection helpers.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::super::super::codec::{read_u16_le, read_u32_le};

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

pub(super) fn push_regular_entry(
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

pub(super) fn push_non_regular_entry(
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

pub(super) fn new_extract_report(fs_kind: &str, volume_name: String) -> FsExtractReport {
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
