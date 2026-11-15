//! Matsushita MN103S / AM34 instruction cross-reference and immediate-load helpers.
//!
//! Encodes the empirically verified MN10300 / MN103S PC-relative rule:
//! relative displacement is computed from the virtual address of the current
//! instruction (`target_va = instruction_va + signed_disp`), not the next instruction.

use std::collections::HashSet;

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

/// Decoded MN103S relative branch or call instruction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstructionRef {
    pub va: u32,
    pub opcode: u8,
    pub mnemonic: String,
    pub displacement: i32,
    pub target_va: u32,
}

/// Decoded 6-byte `0xFC <sub_opcode> <u32>` extended immediate or pointer load.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImmediateLoadRef {
    pub va: u32,
    pub sub_opcode: u8,
    pub value: u32,
    pub c_string: Option<String>,
}

/// Decoded MN103S `call16`/`call32` caller-encoded callee stack frame metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallFrameRef {
    pub va: u32,
    pub target_va: u32,
    pub is_call32: bool,
    pub regs_mask: u8,
    pub frame_bytes: u8,
    pub saved_groups: Vec<String>,
}

/// Decoded MN103S hardware zero-overhead loop (`SETLB` .. `Lcc`/`LRA`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareLoopRef {
    pub setlb_va: u32,
    pub loop_body_va: u32,
    pub lcc_va: u32,
    pub opcode: u8,
    pub mnemonic: String,
    pub body_len: usize,
}

const REGMASK_ORDER: [&str; 8] = [
    "d2", "d3", "a2", "a3", "other", "exreg0", "exreg1", "exother",
];

const CC_SUFFIXES: [&str; 10] = ["lt", "gt", "ge", "le", "cs", "hi", "cc", "ls", "eq", "ne"];

/// Decodes an 8-bit MN103S `CALL`/`MOVM`/`RET` register save mask into active group names.
pub fn decode_regmask_names(regs_mask: u8) -> Vec<&'static str> {
    REGMASK_ORDER
        .iter()
        .enumerate()
        .filter_map(|(idx, &name)| ((regs_mask & (0x80 >> idx)) != 0).then_some(name))
        .collect()
}

fn checked_rel_target(
    buf_len: usize,
    image_base: u32,
    off: usize,
    insn_len: usize,
    disp: i32,
    max_abs_disp: Option<u32>,
) -> Option<(u32, u32)> {
    if off + insn_len > buf_len || disp == 0 {
        return None;
    }
    if let Some(limit) = max_abs_disp
        && disp.unsigned_abs() > limit
    {
        return None;
    }
    let base = image_base as i64;
    let va = base + off as i64;
    let target = va + disp as i64;
    (target >= base && target < base + buf_len as i64).then_some((va as u32, target as u32))
}

/// Maps condition low nibble `0..=9` to its MN103S suffix (`lt`..`ne`).
pub fn condition_suffix(cc_nibble: u8) -> Option<&'static str> {
    CC_SUFFIXES.get(cc_nibble as usize).copied()
}

fn va_window_bounds(
    buf_len: usize,
    image_base: u32,
    va_start: Option<u32>,
    va_end: Option<u32>,
) -> (usize, usize) {
    let base_u64 = image_base as u64;
    let lo = va_start
        .map(|va| (va as u64).saturating_sub(base_u64) as usize)
        .unwrap_or(0)
        .min(buf_len);
    let hi = va_end
        .map(|va| (va as u64).saturating_sub(base_u64) as usize)
        .unwrap_or(buf_len)
        .min(buf_len);
    (lo, hi)
}

fn rel_branch_spec(opcode: u8) -> Option<(&'static str, usize)> {
    match opcode {
        0xCA => Some(("bra8", 2)),
        0xCC => Some(("jmp16", 3)),
        0xCD => Some(("call16", 3)),
        0xDC => Some(("jmp32", 5)),
        0xDD => Some(("call32", 5)),
        _ => None,
    }
}

fn read_signed_disp(slice: &[u8], opcode: u8) -> i32 {
    match opcode {
        0xCA => (slice[1] as i8) as i32,
        0xCC | 0xCD => LittleEndian::read_i16(&slice[1..3]) as i32,
        0xDC | 0xDD => LittleEndian::read_i32(&slice[1..5]),
        _ => 0,
    }
}

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

fn scan_rel_instructions<F>(
    buf: &[u8],
    image_base: u32,
    va_start: Option<u32>,
    va_end: Option<u32>,
    max_abs_disp: Option<u32>,
    matcher: F,
) -> Vec<InstructionRef>
where
    F: Fn(&[u8], usize, u8) -> Option<(String, usize, i32)>,
{
    let (lo, hi) = va_window_bounds(buf.len(), image_base, va_start, va_end);
    (lo..hi)
        .filter_map(|off| {
            let op = buf[off];
            let (mnemonic, insn_len, disp) = matcher(buf, off, op)?;
            let (va, target_va) =
                checked_rel_target(buf.len(), image_base, off, insn_len, disp, max_abs_disp)?;
            Some(InstructionRef {
                va,
                opcode: op,
                mnemonic,
                displacement: disp,
                target_va,
            })
        })
        .collect()
}

/// Scans a binary window for MN103S relative branches/calls targeting `[image_base, image_end)`.
pub fn decode_branch_call_refs(
    buf: &[u8],
    image_base: u32,
    va_start: Option<u32>,
    va_end: Option<u32>,
    max_abs_disp: u32,
) -> Vec<InstructionRef> {
    scan_rel_instructions(
        buf,
        image_base,
        va_start,
        va_end,
        Some(max_abs_disp),
        |b, off, op| {
            let (mnemonic, insn_len) = rel_branch_spec(op)?;
            (off + insn_len <= b.len()).then(|| {
                (
                    mnemonic.to_string(),
                    insn_len,
                    read_signed_disp(&b[off..], op),
                )
            })
        },
    )
}

/// Decodes 2-byte conditional branches `Bcc` (`0xC0..=0xC9`) inside `[va_start, va_end)`.
pub fn decode_bcc_refs(
    buf: &[u8],
    image_base: u32,
    va_start: Option<u32>,
    va_end: Option<u32>,
) -> Vec<InstructionRef> {
    scan_rel_instructions(buf, image_base, va_start, va_end, None, |b, off, op| {
        let suffix = (0xC0..=0xC9)
            .contains(&op)
            .then(|| condition_suffix(op & 0x0F))
            .flatten()?;
        (off + 2 <= b.len()).then(|| (format!("b{suffix}"), 2, (b[off + 1] as i8) as i32))
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mn103s_call32_and_fc_string_xrefs() {
        let base = 0x5040_0000u32;
        let mut buf = vec![0xFFu8; 0x80];
        // At 0x5040_0040: call32 targeting 0x5040_0010 (disp = -0x30)
        buf[0x40] = 0xDD;
        buf[0x41..0x45].copy_from_slice(&(-0x30i32).to_le_bytes());
        // At 0x5040_0020: FC CC <ptr 0x5040_0060> -> "BGO OK\n"
        buf[0x20] = 0xFC;
        buf[0x21] = 0xCC;
        buf[0x22..0x26].copy_from_slice(&(base + 0x60).to_le_bytes());
        buf[0x60..0x68].copy_from_slice(b"BGO OK\n\0");

        let callers = find_callers_to(&buf, base, &[0x5040_0010]);
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].va, 0x5040_0040);
        assert_eq!(callers[0].mnemonic, "call32");

        let loads = find_fc_immediate_loads(&buf, base, 0x5040_0018, 0x5040_0030);
        assert_eq!(loads.len(), 1);
        assert_eq!(loads[0].c_string.as_deref(), Some("BGO OK\n"));

        let ptrs = find_pointer_xrefs(&buf, base, base + 0x60);
        assert_eq!(ptrs, vec![0x5040_0022]);
    }

    #[test]
    fn test_call_frames_bcc_and_setlb_loops() {
        let base = 0x5700_0000u32;
        let mut buf = vec![0xCBu8; 0x80];
        // call16 at +0x40 -> +0x10 (disp = -0x30), regs=0x98 (d2,a3,other), frame=0x24
        buf[0x40] = 0xCD;
        buf[0x41..0x43].copy_from_slice(&(-0x30i16).to_le_bytes());
        buf[0x43] = 0x98;
        buf[0x44] = 0x24;
        // beq +0x08 at +0x50 -> +0x58
        buf[0x50] = 0xC8;
        buf[0x51] = 0x08;
        // SETLB at +0x60, 4 bytes body (NOPs), LNE at +0x65
        buf[0x60] = 0xDB;
        buf[0x65] = 0xD9;

        let frames = decode_call_frames(&buf, base, None, None, 0x1000);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].target_va, base + 0x10);
        assert_eq!(frames[0].frame_bytes, 0x24);
        assert_eq!(frames[0].saved_groups, vec!["d2", "a3", "other"]);

        let bccs = decode_bcc_refs(&buf, base, Some(base + 0x4E), Some(base + 0x56));
        assert_eq!(bccs.len(), 1);
        assert_eq!(bccs[0].mnemonic, "beq");
        assert_eq!(bccs[0].target_va, base + 0x58);

        let loops = decode_setlb_loops(&buf, base, None, None, 32);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].setlb_va, base + 0x60);
        assert_eq!(loops[0].loop_body_va, base + 0x61);
        assert_eq!(loops[0].lcc_va, base + 0x65);
        assert_eq!(loops[0].mnemonic, "lne");
    }
}
