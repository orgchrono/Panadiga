#![deny(unsafe_code)]

//! MN103S prologue detection and CFG feature summarization.

use super::CfgFunctionCandidate;
use crate::re::mn10300::{decode_bcc_refs, decode_call_frames, decode_setlb_loops};
use crate::re::mn10300_traps::estimate_mn103_insn_len;

/// Detects a standard MN103S function prologue at `buf[off..]` and returns `(kind, byte_len)`.
#[must_use]
pub fn detect_prologue_at(buf: &[u8], off: usize) -> Option<(&'static str, usize)> {
    let rem = buf.len().saturating_sub(off);
    if rem >= 2 && buf[off] == 0xCF && buf[off + 1] != 0x00 {
        return Some(("MOVM_SP", 2));
    }
    if rem >= 3 && buf[off] == 0xF8 && buf[off + 1] == 0xFE && (buf[off + 2] as i8) < 0 {
        return Some(("ADD_IMM8_SP", 3));
    }
    if rem >= 4 && buf[off] == 0xFA && buf[off + 1] == 0xFE {
        let disp = i16::from_le_bytes([buf[off + 2], buf[off + 3]]);
        if disp < 0 {
            return Some(("ADD_IMM16_SP", 4));
        }
    }
    if rem >= 6 && buf[off] == 0xFC && buf[off + 1] == 0xFE {
        let disp = i32::from_le_bytes([buf[off + 2], buf[off + 3], buf[off + 4], buf[off + 5]]);
        if disp < 0 {
            return Some(("ADD_IMM32_SP", 6));
        }
    }
    None
}

fn is_preceded_by_boundary(buf: &[u8], off: usize) -> bool {
    if off == 0 {
        return true;
    }
    if matches!(buf[off - 1], 0x00 | 0xFF | 0xCB) {
        return true;
    }
    if off >= 2 && buf[off - 2] == 0xF0 && buf[off - 1] == 0xFC {
        return true;
    }
    off >= 3 && matches!(buf[off - 3], 0xDF | 0xDE)
}

/// Scans forward from `start_off` to find the end offset (inclusive of `RET`/`RETS`) within `max_len`.
#[must_use]
pub fn find_return_end_off(buf: &[u8], start_off: usize, max_len: usize) -> usize {
    let limit = buf.len().min(start_off.saturating_add(max_len));
    let mut off = start_off;
    while off < limit {
        if buf[off] == 0xDF && off + 3 <= limit {
            return off + 3;
        }
        if buf[off] == 0xF0 && off + 2 <= limit && buf[off + 1] == 0xFC {
            return off + 2;
        }
        let step = estimate_mn103_insn_len(buf[off]).max(1);
        off = off.saturating_add(step);
    }
    limit
}

/// Summarizes control-flow metrics for a candidate MN103S function starting at `entry_off`.
#[must_use]
pub fn summarize_function_cfg(
    buf: &[u8],
    image_base: u32,
    entry_off: usize,
    max_func_len: usize,
    prologue_kind: &str,
) -> CfgFunctionCandidate {
    let end_off = find_return_end_off(buf, entry_off, max_func_len);
    let entry_va = image_base.wrapping_add(entry_off as u32);
    let end_va = image_base.wrapping_add(end_off as u32);
    let branches = decode_bcc_refs(buf, image_base, Some(entry_va), Some(end_va));
    let calls = decode_call_frames(buf, image_base, Some(entry_va), Some(end_va), 0x10_0000);
    let loops = decode_setlb_loops(buf, image_base, Some(entry_va), Some(end_va), 128);
    CfgFunctionCandidate {
        entry_va,
        end_va,
        prologue_kind: prologue_kind.to_string(),
        branch_count: branches.len(),
        call_count: calls.len(),
        hardware_loop_count: loops.len(),
    }
}

/// Autonomously discovers MN103S functions via boundary-aligned prologues and summarizes their CFG.
#[must_use]
pub fn discover_cfg_functions(
    buf: &[u8],
    image_base: u32,
    max_func_len: usize,
) -> Vec<CfgFunctionCandidate> {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off < buf.len() {
        if is_preceded_by_boundary(buf, off)
            && let Some((kind, _)) = detect_prologue_at(buf, off)
        {
            let cand = summarize_function_cfg(buf, image_base, off, max_func_len, kind);
            let next_off = (cand.end_va.wrapping_sub(image_base) as usize).max(off + 2);
            out.push(cand);
            off = next_off;
            continue;
        }
        off += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_cfg_functions_prologue_and_ret_boundary() {
        let mut code = vec![0xCB; 4]; // padding
        code.extend_from_slice(&[0xCF, 0x0C, 0xF8, 0xFE, 0xF8]); // MOVM [d2,d3],(SP); ADD -8,SP
        code.extend_from_slice(&[0xA0, 0x00, 0xC8, 0x04, 0xCD, 0xFB, 0xFF, 0x00, 0x00]); // CMP; BEQ +4; CALL16 -5
        code.extend_from_slice(&[0xF8, 0xFE, 0x08, 0xDF, 0x0C, 0x00]); // ADD 8,SP; RET
        let funcs = discover_cfg_functions(&code, 0x4400_0000, 0x200);
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].entry_va, 0x4400_0004);
        assert_eq!(funcs[0].prologue_kind, "MOVM_SP");
        assert!(funcs[0].branch_count >= 1);
        assert!(funcs[0].call_count >= 1);
    }
}
