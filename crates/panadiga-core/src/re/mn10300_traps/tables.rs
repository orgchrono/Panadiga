//! Trap 1 & Trap 7 pointer/CLI/ATAPI/CP932 dispatch table parsers.

use super::{AtapiCdbDispatchSlot, DrvCliEntry, DrvMechaErrorRecord};
use crate::crypto::codec::{extract_c_ascii, read_u16_le, read_u32_le};

/// Trap 1 + Guarded Write helper: classifies a DRV CLI command according to `PASS` auth state.
pub fn classify_drv_cli_command(name: &str, flags: u32, pass_unlocked: bool) -> &'static str {
    let upper = name.trim().to_ascii_uppercase();
    if upper == "PASS" {
        return "UNLOCK_AUTH";
    }
    let is_write = (flags & 0x38) != 0
        || matches!(
            upper.as_str(),
            "WB" | "WW" | "WRITE" | "WTLONG" | "ERASE" | "FORMAT" | "EEP" | "MEMSET"
        );
    if is_write {
        return if pass_unlocked {
            "UNLOCKED_WRITE"
        } else {
            "WRITE_BLOCKED"
        };
    }
    if matches!(
        upper.as_str(),
        "VER" | "HELP" | "RD" | "RB" | "RW" | "READ" | "STATUS" | "SENSE" | "DUMP" | "SLOG"
    ) {
        return "READ_ONLY";
    }
    "DIAGNOSTIC"
}

/// Trap 1: parses `<8sIII>` 20-byte DRV CLI dispatch entries up to `max_entries` or NUL terminator.
pub fn parse_drv_cli_table(
    image: &[u8],
    table_offset: usize,
    image_base: u32,
    max_entries: usize,
    pass_unlocked: bool,
) -> Vec<DrvCliEntry> {
    let mut entries = Vec::new();
    for idx in 0..max_entries {
        let off = table_offset.saturating_add(idx * 20);
        if off + 20 > image.len() {
            break;
        }
        let name_bytes = &image[off..off + 8];
        if name_bytes[0] == 0 {
            break;
        }
        let name = extract_c_ascii(name_bytes);
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_graphic() || b == b' ') {
            break;
        }
        let handler_va = read_u32_le(image, off + 8);
        let flags = read_u32_le(image, off + 12);
        let arg_mask = read_u32_le(image, off + 16);
        let classification = classify_drv_cli_command(&name, flags, pass_unlocked).to_string();
        entries.push(DrvCliEntry {
            index: idx,
            entry_va: image_base.wrapping_add(off as u32),
            name,
            handler_va,
            flags,
            arg_mask,
            classification,
        });
    }
    entries
}

/// Trap 1: parses ATAPI CDB[0] handler dispatch pointers (`u32` per opcode).
pub fn parse_atapi_cdb_dispatch(
    image: &[u8],
    table_offset: usize,
    image_base: u32,
    count: usize,
    stub_va: u32,
) -> Vec<AtapiCdbDispatchSlot> {
    let mut out = Vec::new();
    for idx in 0..count.min(256) {
        let off = table_offset.saturating_add(idx * 4);
        if off + 4 > image.len() {
            break;
        }
        let handler_va = read_u32_le(image, off);
        let cdb0 = idx as u8;
        let is_active = handler_va != 0 && handler_va != stub_va;
        let role = match cdb0 {
            0x00 => "TEST UNIT READY",
            0x03 => "REQUEST SENSE",
            0x12 => "INQUIRY",
            0x1B => "START STOP UNIT",
            0x1E => "PREVENT ALLOW MEDIUM REMOVAL",
            0x28 => "READ(10)",
            0x2A => "WRITE(10)",
            0x3B => "WRITE BUFFER (Microcode/RAM Payload)",
            0x3C => "READ BUFFER (Memory/Log Readback)",
            0x43 => "READ TOC/PMA/ATIP",
            0xED => "MATSUSHITA VENDOR WRITE/DIAG (FUN_400209f3)",
            0xFD => "MATSUSHITA VENDOR 0xFD (FUN_40020147)",
            0xFE => "MATSUSHITA VENDOR 0xFE (MMODE/SLOG/PASS)",
            _ if is_active => "ACTIVE_HANDLER",
            _ => "UNIMPLEMENTED_STUB",
        }
        .to_string();
        out.push(AtapiCdbDispatchSlot {
            cdb0,
            table_entry_va: image_base.wrapping_add(off as u32),
            handler_va,
            is_active,
            role,
        });
    }
    out
}

/// Trap 1: scans aligned LE32 pointer arrays targeting `[code_lo, code_hi)` excluding known stubs.
pub fn scan_code_pointer_tables(
    data: &[u8],
    base_va: u32,
    code_lo: u32,
    code_hi: u32,
    min_run: usize,
    stub_vas: &[u32],
) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut run: Vec<(u32, u32)> = Vec::new();
    for (idx, chunk) in data.as_chunks::<4>().0.iter().enumerate() {
        let off = (idx * 4) as u32;
        let val = u32::from_le_bytes(*chunk);
        if val >= code_lo && val < code_hi {
            run.push((base_va.wrapping_add(off), val));
        } else {
            if run.len() >= min_run {
                out.extend(
                    run.iter()
                        .copied()
                        .filter(|(_, tgt)| !stub_vas.contains(tgt)),
                );
            }
            run.clear();
        }
    }
    if run.len() >= min_run {
        out.extend(
            run.iter()
                .copied()
                .filter(|(_, tgt)| !stub_vas.contains(tgt)),
        );
    }
    out
}

/// Trap 7: parses 52-byte (`0x34`) `<u16 code><50B text>` DRV error dictionary slices safely in pure Rust.
pub fn parse_drv_mecha_error_table(
    data: &[u8],
    start_offset: usize,
    end_offset: usize,
) -> Vec<DrvMechaErrorRecord> {
    const STRIDE: usize = 0x34;
    let bound = end_offset.min(data.len());
    let mut records = Vec::new();
    let mut off = start_offset;
    while off + STRIDE <= bound {
        let code = read_u16_le(data, off);
        if code == 0 || code == 0xFFFF {
            off += STRIDE;
            continue;
        }
        let body = &data[off + 2..off + STRIDE];
        let nul = body.iter().position(|&b| b == 0).unwrap_or(body.len());
        let raw_slice = &body[..nul];
        let ascii_preview: String = raw_slice
            .iter()
            .map(|&b| {
                if (0x20..=0x7E).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        records.push(DrvMechaErrorRecord {
            offset: off,
            code,
            code_hex: format!("0x{code:04X}"),
            raw_len: nul,
            ascii_preview,
        });
        off += STRIDE;
    }
    records
}
