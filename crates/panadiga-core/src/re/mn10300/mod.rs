//! Matsushita MN103S / AM34 instruction cross-reference and immediate-load helpers.
//!
//! Encodes the empirically verified MN10300 / MN103S PC-relative rule:
//! relative displacement is computed from the virtual address of the current
//! instruction (`target_va = instruction_va + signed_disp`), not the next instruction.

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;
mod xrefs;

pub use xrefs::{
    decode_call_frames, decode_setlb_loops, find_callers_to, find_fc_immediate_loads,
    find_pointer_xrefs,
};

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

pub(super) fn checked_rel_target(
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

pub(super) fn va_window_bounds(
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

pub(super) fn read_signed_disp(slice: &[u8], opcode: u8) -> i32 {
    match opcode {
        0xCA => (slice[1] as i8) as i32,
        0xCC | 0xCD => LittleEndian::read_i16(&slice[1..3]) as i32,
        0xDC | 0xDD => LittleEndian::read_i32(&slice[1..5]),
        _ => 0,
    }
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
