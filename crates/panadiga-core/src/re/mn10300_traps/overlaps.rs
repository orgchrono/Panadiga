//! Trap 4, Trap 5, Trap 7 false-constant filtering, and AM33 missing Sleigh opcode detectors.

use super::{ExtendedOpcodeHit, InstructionSpan, MidInstructionOverlap};
use crate::crypto::codec::read_u32_le;
use crate::re::mn10300::{decode_branch_call_refs, decode_setlb_loops};

const AM33_REG_NAMES: [&str; 16] = [
    "R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "A0", "A1", "A2", "A3", "D0", "D1", "D2", "D3",
];

fn branch_insn_len(opcode: u8) -> u32 {
    match opcode {
        0xDD => 7,
        0xCD | 0xDC => 5,
        0xCC => 3,
        _ => 2,
    }
}

/// Builds sorted instruction byte spans `[start_va, end_va)` from decoded MN103S flow & SETLB opcodes.
pub fn build_instruction_spans(data: &[u8], base_va: u32) -> Vec<InstructionSpan> {
    let mut spans: Vec<InstructionSpan> =
        decode_branch_call_refs(data, base_va, None, None, 0x0100_0000)
            .into_iter()
            .map(|r| InstructionSpan {
                start_va: r.va,
                end_va: r.va.wrapping_add(branch_insn_len(r.opcode)),
                mnemonic: r.mnemonic,
            })
            .collect();
    for lp in decode_setlb_loops(data, base_va, None, None, 64) {
        spans.push(InstructionSpan {
            start_va: lp.setlb_va,
            end_va: lp.setlb_va.wrapping_add(1),
            mnemonic: "setlb".to_string(),
        });
        spans.push(InstructionSpan {
            start_va: lp.lcc_va,
            end_va: lp.lcc_va.wrapping_add(1),
            mnemonic: lp.mnemonic,
        });
    }
    spans.sort_by_key(|s| s.start_va);
    spans
}

/// Trap 4: detects branch/call targets falling inside `[start_va + 1, end_va)` (`BRA 0x5040ea2f` -> `??`).
pub fn detect_mid_instruction_overlaps(
    data: &[u8],
    base_va: u32,
    extra_spans: &[InstructionSpan],
) -> Vec<MidInstructionOverlap> {
    let mut spans = build_instruction_spans(data, base_va);
    spans.extend_from_slice(extra_spans);
    spans.sort_by_key(|s| s.start_va);

    let mut out = Vec::new();
    for br in decode_branch_call_refs(data, base_va, None, None, 0x0100_0000) {
        if let Some(sp) = spans
            .iter()
            .find(|s| br.target_va > s.start_va && br.target_va < s.end_va)
        {
            out.push(MidInstructionOverlap {
                source_va: br.va,
                target_va: br.target_va,
                containing_start_va: sp.start_va,
                containing_end_va: sp.end_va,
                mnemonic: br.mnemonic,
            });
        }
    }
    out
}

/// Trap 5: checks whether a virtual address hits Spansion CS1 Flash CFI / JEDEC command cycles.
pub fn is_flash_cfi_command_va(va: u32) -> bool {
    let low = va & 0x0FFF;
    let in_nor =
        (0x8400_0000..0x847F_0000).contains(&va) || (0x8000_0000..0x807F_0000).contains(&va);
    in_nor && matches!(low, 0x000 | 0x554 | 0x555 | 0xAAA | 0xAAB)
}

/// Trap 5: scans binary buffer for LE32 literal hits matching Spansion CS1 Flash CFI ports.
pub fn scan_cfi_flash_constant_hits(data: &[u8], base_va: u32) -> Vec<u32> {
    let mut hits = Vec::new();
    for off in 0..data.len().saturating_sub(3) {
        let val = read_u32_le(data, off);
        if is_flash_cfi_command_va(val) {
            hits.push(base_va.wrapping_add(off as u32));
        }
    }
    hits
}

/// Trap 7: partitions candidate constant addresses into genuine references vs mid-instruction false hits.
pub fn filter_mid_instruction_constant_hits(
    candidate_vas: &[u32],
    spans: &[InstructionSpan],
) -> (Vec<u32>, Vec<u32>) {
    let mut genuine = Vec::new();
    let mut false_hits = Vec::new();
    for &va in candidate_vas {
        if spans.iter().any(|sp| va > sp.start_va && va < sp.end_va) {
            false_hits.push(va);
        } else {
            genuine.push(va);
        }
    }
    (genuine, false_hits)
}

/// Trap 8 / AM33 Sleigh gap: decodes missing `0xF9 0xEA <Rm0|Rn2>` -> `MOVHU (Rm0+), Rn2`.
pub fn scan_am33_f9_ea_postinc_loads(data: &[u8], base_va: u32) -> Vec<ExtendedOpcodeHit> {
    let mut hits = Vec::new();
    for off in 0..data.len().saturating_sub(2) {
        if data[off] == 0xF9 && data[off + 1] == 0xEA {
            let reg = data[off + 2];
            let rm0 = AM33_REG_NAMES[(reg & 0x0F) as usize];
            let rn2 = AM33_REG_NAMES[((reg >> 4) & 0x0F) as usize];
            hits.push(ExtendedOpcodeHit {
                va: base_va.wrapping_add(off as u32),
                bytes_hex: format!("F9 EA {reg:02X}"),
                mnemonic: "MOVHU".to_string(),
                operands: format!("({rm0}+),{rn2}"),
            });
        }
    }
    hits
}

/// Trap 8 / AM33 Sleigh gap: locates `0xF9 0x3A 0x78 0xFF 0xF0 0xFC` / pre-`RETS` `0xFF` barrier sites.
pub fn scan_am33_ff_pre_rets_sites(data: &[u8], base_va: u32) -> Vec<u32> {
    let mut sites = Vec::new();
    for (idx, w) in data.windows(6).enumerate() {
        if w[0] == 0xF9 && w[1] == 0x3A && w[3] == 0xFF && w[4] == 0xF0 && w[5] == 0xFC {
            sites.push(base_va.wrapping_add((idx + 3) as u32));
        }
    }
    sites
}
