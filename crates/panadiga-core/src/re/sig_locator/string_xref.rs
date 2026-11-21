#![deny(unsafe_code)]

//! Autonomous MN103S function discovery from ASCII string xrefs (`0xFC` loads + pointer tables).

use std::collections::BTreeSet;

use super::pattern_cfg::detect_prologue_at;
use super::{LocatedFunctionMatch, StringXrefHit};
use crate::re::mn10300::{find_fc_immediate_loads, find_pointer_xrefs};

fn find_ascii_needle_vas(buf: &[u8], image_base: u32, needle: &str) -> Vec<u32> {
    let raw = needle.as_bytes();
    if raw.is_empty() || buf.len() < raw.len() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    let mut pos = 0usize;
    while pos + raw.len() <= buf.len() {
        if &buf[pos..pos + raw.len()] == raw {
            let mut start = pos;
            while start > 0 && buf[start - 1].is_ascii_graphic() {
                start -= 1;
            }
            let va = image_base.wrapping_add(start as u32);
            if !hits.contains(&va) {
                hits.push(va);
            }
            if start != pos {
                let exact_va = image_base.wrapping_add(pos as u32);
                if !hits.contains(&exact_va) {
                    hits.push(exact_va);
                }
            }
            pos += raw.len();
        } else {
            pos += 1;
        }
    }
    hits
}

fn find_enclosing_prologue(
    buf: &[u8],
    image_base: u32,
    xref_va: u32,
    max_back: usize,
) -> (u32, String) {
    let xref_off = xref_va.wrapping_sub(image_base) as usize;
    if xref_off >= buf.len() {
        return (xref_va, "LEAF_OR_UNKNOWN".to_string());
    }
    let min_off = xref_off.saturating_sub(max_back);
    let mut scan = xref_off;
    while scan >= min_off {
        if let Some((kind, _)) = detect_prologue_at(buf, scan) {
            if kind.starts_with("ADD_IMM")
                && scan >= min_off + 2
                && let Some(("MOVM_SP", 2)) = detect_prologue_at(buf, scan - 2)
            {
                return (
                    image_base.wrapping_add((scan - 2) as u32),
                    "MOVM_SP".to_string(),
                );
            }
            return (image_base.wrapping_add(scan as u32), kind.to_string());
        }
        if scan == 0 {
            break;
        }
        scan -= 1;
    }
    (xref_va, "LEAF_OR_UNKNOWN".to_string())
}

/// Collects raw string cross-references (both `0xFC` immediate loads and 32-bit pointer tables).
#[must_use]
pub fn find_string_xref_hits(buf: &[u8], image_base: u32, needles: &[&str]) -> Vec<StringXrefHit> {
    if buf.is_empty() || needles.is_empty() {
        return Vec::new();
    }
    let end_va = image_base.wrapping_add(buf.len() as u32);
    let fc_loads = find_fc_immediate_loads(buf, image_base, image_base, end_va);
    let mut hits = Vec::new();

    for &needle in needles {
        let vas = find_ascii_needle_vas(buf, image_base, needle);
        let va_set: BTreeSet<u32> = vas.iter().copied().collect();
        for fc in &fc_loads {
            let text_hit = fc.c_string.as_deref().is_some_and(|s| s.contains(needle));
            if va_set.contains(&fc.value) || text_hit {
                hits.push(StringXrefHit {
                    needle: needle.to_string(),
                    string_va: fc.value,
                    xref_va: fc.va,
                    xref_kind: "fc_immediate".to_string(),
                });
            }
        }
        for &sva in &vas {
            for ptr_va in find_pointer_xrefs(buf, image_base, sva) {
                hits.push(StringXrefHit {
                    needle: needle.to_string(),
                    string_va: sva,
                    xref_va: ptr_va,
                    xref_kind: "pointer_table".to_string(),
                });
            }
        }
    }
    hits
}

/// Locates enclosing MN103S functions referencing any string needle via `0xFC` loads or pointer tables.
#[must_use]
pub fn locate_functions_by_string(
    buf: &[u8],
    image_base: u32,
    needles: &[&str],
    max_prologue_back: usize,
) -> Vec<LocatedFunctionMatch> {
    let xrefs = find_string_xref_hits(buf, image_base, needles);
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for hit in xrefs {
        let (func_va, prologue_kind) =
            find_enclosing_prologue(buf, image_base, hit.xref_va, max_prologue_back);
        if seen.insert((func_va, hit.needle.clone())) {
            out.push(LocatedFunctionMatch {
                func_va,
                needle: hit.needle,
                string_va: hit.string_va,
                xref_va: hit.xref_va,
                prologue_kind,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::re::mn10300_emit::{emit_add_sp_imm8, emit_mov_imm32, emit_rets};

    #[test]
    fn test_locate_function_by_string_xref_and_prologue() {
        let base = 0x5040_0000u32;
        let mut buf = vec![0xCB; 0x80];
        let str_off = 0x50usize;
        buf[str_off..str_off + 11].copy_from_slice(b"BGO failed\0");

        let mut func = Vec::new();
        func.extend_from_slice(&[0xCF, 0x04]); // MOVM [d2], (SP)
        emit_add_sp_imm8(&mut func, -8);
        let _ = emit_mov_imm32(&mut func, 0xDC, base + str_off as u32); // MOV imm32, A0
        emit_add_sp_imm8(&mut func, 8);
        emit_rets(&mut func);
        buf[0x10..0x10 + func.len()].copy_from_slice(&func);

        let found = locate_functions_by_string(&buf, base, &["BGO failed"], 0x40);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].func_va, base + 0x10);
        assert_eq!(found[0].string_va, base + str_off as u32);
    }
}
