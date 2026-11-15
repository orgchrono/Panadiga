//! MN103S call-frame, hardware loop (`SETLB`), and string/pointer xref extractors.

use std::collections::HashSet;

use byteorder::{ByteOrder, LittleEndian};

use super::{
    CallFrameRef, HardwareLoopRef, ImmediateLoadRef, InstructionRef, checked_rel_target,
    condition_suffix, decode_branch_call_refs, decode_regmask_names, read_signed_disp,
    va_window_bounds,
};

fn extract_printable_c_string(buf: &[u8], offset: usize, max_len: usize) -> Option<String> {
    if offset >= buf.len() {
        return None;
    }
    let search_end = buf.len().min(offset.saturating_add(max_len));
    let end = buf[offset..search_end]
        .iter()
        .position(|&b| b == 0)
        .map(|rel| offset + rel)
        .unwrap_or(search_end);
    let raw = &buf[offset..end];
    if raw.is_empty() {
        return None;
    }
    if let Ok(utf8) = std::str::from_utf8(raw)
        && utf8
            .chars()
            .all(|ch| (' '..='\u{007E}').contains(&ch) || matches!(ch, '\r' | '\n' | '\t'))
    {
        return Some(utf8.to_string());
    }
    None
}

/// Decodes full 5-byte `0xCD` (`call16`) and 7-byte `0xDD` (`call32`) caller frame specs.
pub fn decode_call_frames(
    buf: &[u8],
    image_base: u32,
    va_start: Option<u32>,
    va_end: Option<u32>,
    max_abs_disp: u32,
) -> Vec<CallFrameRef> {
    let (lo, hi) = va_window_bounds(buf.len(), image_base, va_start, va_end);
    let mut frames = Vec::new();
    for off in lo..hi {
        let op = buf[off];
        let (is_call32, insn_len) = match op {
            0xCD => (false, 5usize),
            0xDD => (true, 7usize),
            _ => continue,
        };
        if off + insn_len > buf.len() {
            continue;
        }
        let disp = read_signed_disp(&buf[off..], op);
        let Some((va, target_va)) = checked_rel_target(
            buf.len(),
            image_base,
            off,
            insn_len,
            disp,
            Some(max_abs_disp),
        ) else {
            continue;
        };
        let regs_mask = buf[off + insn_len - 2];
        let frame_bytes = buf[off + insn_len - 1];
        frames.push(CallFrameRef {
            va,
            target_va,
            is_call32,
            regs_mask,
            frame_bytes,
            saved_groups: decode_regmask_names(regs_mask)
                .into_iter()
                .map(str::to_string)
                .collect(),
        });
    }
    frames
}

/// Pairs `0xDB` (`SETLB`) markers with trailing `Lcc` (`0xD0..=0xD9`) or `LRA` (`0xDA`) terminators.
pub fn decode_setlb_loops(
    buf: &[u8],
    image_base: u32,
    va_start: Option<u32>,
    va_end: Option<u32>,
    max_body_bytes: usize,
) -> Vec<HardwareLoopRef> {
    let base_u64 = image_base as u64;
    let (lo, hi) = va_window_bounds(buf.len(), image_base, va_start, va_end);
    let mut loops = Vec::new();
    for off in lo..hi {
        if buf[off] != 0xDB || off + 5 > buf.len() {
            continue;
        }
        let body_start = off + 1;
        let scan_end = (off + 1 + max_body_bytes).min(buf.len());
        let search_from = (body_start + 4).min(scan_end);
        for (cur, &term) in buf.iter().enumerate().take(scan_end).skip(search_from) {
            let mnemonic = if term == 0xDA {
                "lra".to_string()
            } else if (0xD0..=0xD9).contains(&term) {
                let Some(suffix) = condition_suffix(term & 0x0F) else {
                    continue;
                };
                format!("l{suffix}")
            } else {
                continue;
            };
            loops.push(HardwareLoopRef {
                setlb_va: (base_u64 + off as u64) as u32,
                loop_body_va: (base_u64 + body_start as u64) as u32,
                lcc_va: (base_u64 + cur as u64) as u32,
                opcode: term,
                mnemonic,
                body_len: cur - body_start,
            });
            break;
        }
    }
    loops
}

/// Returns every MN103S branch or call referencing any virtual address in `targets`.
pub fn find_callers_to(buf: &[u8], image_base: u32, targets: &[u32]) -> Vec<InstructionRef> {
    let target_set: HashSet<u32> = targets.iter().copied().collect();
    decode_branch_call_refs(buf, image_base, None, None, 0x0020_0000)
        .into_iter()
        .filter(|r| target_set.contains(&r.target_va))
        .collect()
}

/// Extracts 6-byte `0xFC <sub_opcode> <u32>` immediate or pointer loads inside `[va_start, va_end]`.
pub fn find_fc_immediate_loads(
    buf: &[u8],
    image_base: u32,
    va_start: u32,
    va_end: u32,
) -> Vec<ImmediateLoadRef> {
    if buf.len() < 6 || va_end < va_start {
        return Vec::new();
    }
    let base_u64 = image_base as u64;
    let upper_va = base_u64 + buf.len() as u64;
    let lo = ((va_start as u64).saturating_sub(base_u64) as usize).min(buf.len() - 6);
    let hi = ((va_end as u64).saturating_sub(base_u64) as usize).min(buf.len() - 6);

    let mut out = Vec::new();
    for off in lo..=hi {
        if buf[off] != 0xFC {
            continue;
        }
        let sub_opcode = buf[off + 1];
        let val = LittleEndian::read_u32(&buf[off + 2..off + 6]);
        let val_u64 = val as u64;
        let c_string = if val_u64 >= base_u64 && val_u64 < upper_va {
            extract_printable_c_string(buf, (val_u64 - base_u64) as usize, 80)
        } else {
            None
        };
        out.push(ImmediateLoadRef {
            va: (base_u64 + off as u64) as u32,
            sub_opcode,
            value: val,
            c_string,
        });
    }
    out
}

/// Finds every 32-bit little-endian literal pointer occurrence to `target_va` inside `buf`.
pub fn find_pointer_xrefs(buf: &[u8], image_base: u32, target_va: u32) -> Vec<u32> {
    let needle = target_va.to_le_bytes();
    buf.windows(4)
        .enumerate()
        .filter_map(|(idx, win)| {
            if win == needle {
                Some(image_base.wrapping_add(idx as u32))
            } else {
                None
            }
        })
        .collect()
}
