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

/// Scans a binary window for MN103S relative branches/calls targeting `[image_base, image_end)`.
pub fn decode_branch_call_refs(
    buf: &[u8],
    image_base: u32,
    va_start: Option<u32>,
    va_end: Option<u32>,
    max_abs_disp: u32,
) -> Vec<InstructionRef> {
    let base_u64 = image_base as u64;
    let upper_va = base_u64 + buf.len() as u64;
    let lo = va_start
        .map(|va| (va as u64).saturating_sub(base_u64) as usize)
        .unwrap_or(0)
        .min(buf.len());
    let hi = va_end
        .map(|va| (va as u64).saturating_sub(base_u64) as usize)
        .unwrap_or(buf.len())
        .min(buf.len());

    let mut refs = Vec::new();
    for off in lo..hi {
        let op = buf[off];
        let Some((mnemonic, insn_len)) = rel_branch_spec(op) else {
            continue;
        };
        if off + insn_len > buf.len() {
            continue;
        }
        let disp = read_signed_disp(&buf[off..], op);
        if disp == 0 || disp.unsigned_abs() > max_abs_disp {
            continue;
        }
        let va = base_u64 + off as u64;
        let target = (va as i64) + (disp as i64);
        if target >= (base_u64 as i64) && target < (upper_va as i64) {
            refs.push(InstructionRef {
                va: va as u32,
                opcode: op,
                mnemonic: mnemonic.to_string(),
                displacement: disp,
                target_va: target as u32,
            });
        }
    }
    refs
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
}
