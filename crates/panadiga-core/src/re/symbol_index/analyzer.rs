#![deny(unsafe_code)]

//! Deterministic MN103S binary analyzer building [`FirmwareSymbolIndex`] from raw bytes.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};

use super::types::{
    FirmwareSymbolIndex, FunctionSymbolEntry, SymbolAbiDetail, SymbolAnnotationOverlay,
    SymbolSideEffect,
};
use crate::constants::{
    AM34_DAVIS_IDE_BASE, AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE,
    AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_BASE, AM34_NVRAM_UNCACHED_BASE, SPANSION_NVRAM_SIZE_BYTES,
};
use crate::nvram::calculate_hashes;
use crate::re::mn10300::{decode_call_frames, find_fc_immediate_loads};
use crate::re::sig_locator::{discover_cfg_functions, pattern_cfg::find_return_end_off};

/// Parses a hex virtual address (`0x50409DF2`, `5040_9DF2`, etc.) into a `u32`.
pub fn parse_va_hex(raw: &str) -> Result<u32> {
    let clean = raw
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .replace('_', "");
    if clean.is_empty() {
        bail!("Empty virtual address hex string");
    }
    u32::from_str_radix(&clean, 16)
        .map_err(|e| anyhow::anyhow!("Invalid hex virtual address '{raw}': {e}"))
}

/// Formats a virtual address in canonical uppercase 8-digit hex (`0x50409DF2`).
#[must_use]
pub fn format_va_hex(va: u32) -> String {
    format!("0x{va:08X}")
}

fn is_nor_or_mmio_addr(val: u32) -> bool {
    let cached_nor = AM34_FLASH_BASE..AM34_FLASH_BASE.saturating_add(AM34_FLASH_SIZE);
    let uncached_nor =
        AM34_FLASH_UNCACHED_BASE..AM34_FLASH_UNCACHED_BASE.saturating_add(AM34_FLASH_SIZE);
    let davis_win = AM34_DAVIS_IDE_BASE..AM34_DAVIS_IDE_BASE.saturating_add(0x0000_1000);
    cached_nor.contains(&val)
        || uncached_nor.contains(&val)
        || davis_win.contains(&val)
        || (0x8800_0000..0x8800_1000).contains(&val)
        || (0xC000_0000..0xE000_0000).contains(&val)
}

fn classify_mmio_and_strings(
    mmio_vas: &[u32],
    strings: &[String],
) -> (SymbolSideEffect, Vec<String>) {
    let bgo_cached_lo = AM34_FLASH_BASE + AM34_FLASH_CRITICAL_IPL_SIZE;
    let bgo_uncached_lo = AM34_FLASH_UNCACHED_BASE + AM34_FLASH_CRITICAL_IPL_SIZE;
    let nvram_hi = AM34_NVRAM_BASE + SPANSION_NVRAM_SIZE_BYTES as u32;
    let nvram_uncached_hi = AM34_NVRAM_UNCACHED_BASE + SPANSION_NVRAM_SIZE_BYTES as u32;

    let mut effect = SymbolSideEffect::ReadOnly;
    let mut reasons = Vec::new();

    for &va in mmio_vas {
        if (bgo_cached_lo..AM34_NVRAM_BASE).contains(&va)
            || (bgo_uncached_lo..AM34_NVRAM_UNCACHED_BASE).contains(&va)
        {
            effect = SymbolSideEffect::WritesNorBgo;
            reasons.push(format!("loads Upper NOR/BGO address {}", format_va_hex(va)));
        } else if ((AM34_NVRAM_BASE..nvram_hi).contains(&va)
            || (AM34_NVRAM_UNCACHED_BASE..nvram_uncached_hi).contains(&va))
            && effect != SymbolSideEffect::WritesNorBgo
        {
            effect = SymbolSideEffect::WritesNvram;
            reasons.push(format!("loads NVRAM EEMUL address {}", format_va_hex(va)));
        } else if (AM34_DAVIS_IDE_BASE..AM34_DAVIS_IDE_BASE + 0x1000).contains(&va)
            && !effect.is_destructive()
        {
            effect = SymbolSideEffect::WritesHdd;
            reasons.push(format!("loads DAVIS IDE address {}", format_va_hex(va)));
        } else if (0xC000_0000..0xE000_0000).contains(&va) && effect == SymbolSideEffect::ReadOnly {
            effect = SymbolSideEffect::WritesMmio;
            reasons.push(format!("loads MMIO register {}", format_va_hex(va)));
        }
    }

    for s in strings {
        let low = s.to_ascii_lowercase();
        if (low.contains("bgo driver") || low.contains("nfrom")) && !effect.is_destructive() {
            effect = SymbolSideEffect::WritesNorBgo;
            reasons.push(format!("references BGO string {:?}", s.trim()));
        }
    }
    (effect, reasons)
}

fn infer_abi_and_ret(
    slice: &[u8],
    incoming_saved: &BTreeSet<String>,
    max_frame: Option<u8>,
) -> (String, SymbolAbiDetail, String) {
    let mut params = BTreeSet::new();
    let mut i = 0usize;
    while i + 2 < slice.len() {
        // Common MN103S SP-relative stack argument reads (`mov (d8, SP), Dm/Am`: 0x58..=0x5F disp, or 0xF8 0x20..0x2F disp)
        let is_sp_load = matches!(slice[i], 0x58..=0x5F)
            || (slice[i] == 0xF8 && matches!(slice[i + 1], 0x20..=0x2F | 0xA0..=0xAF));
        if is_sp_load {
            let disp = if slice[i] == 0xF8 {
                slice[i + 2]
            } else {
                slice[i + 1]
            };
            if (0x08..=0x24).contains(&disp) && disp.is_multiple_of(4) {
                params.insert(format!("(0x{disp:02X},SP)"));
            }
        }
        i += 1;
    }
    let has_d0 = slice
        .iter()
        .any(|&b| matches!(b, 0x80 | 0x90 | 0xA0 | 0xC0));
    let has_d1 = slice
        .iter()
        .any(|&b| matches!(b, 0x85 | 0x95 | 0xA5 | 0xC1));
    if !params.is_empty() || has_d0 || has_d1 {
        let mut ordered = Vec::new();
        if has_d0 || !params.is_empty() {
            ordered.push("D0".to_string());
        }
        if has_d1 || !params.is_empty() {
            ordered.push("D1".to_string());
        }
        ordered.extend(params);
        let abi = ordered.join("/");
        let ret = if slice
            .windows(2)
            .any(|w| matches!(w, [0xDF, _] | [0xF0, 0xFC]))
        {
            "D0 (u32)".to_string()
        } else {
            "void".to_string()
        };
        return (
            abi,
            SymbolAbiDetail {
                params: ordered,
                saved_regs: incoming_saved.iter().cloned().collect(),
                frame_bytes: max_frame,
            },
            ret,
        );
    }
    (
        "void".to_string(),
        SymbolAbiDetail {
            params: Vec::new(),
            saved_regs: incoming_saved.iter().cloned().collect(),
            frame_bytes: max_frame,
        },
        "void".to_string(),
    )
}

fn enclosing_entry(sorted_entries: &[u32], va: u32) -> Option<u32> {
    let idx = sorted_entries.partition_point(|&e| e <= va);
    idx.checked_sub(1).map(|pos| sorted_entries[pos])
}

/// Disassembles an unpacked MN103S binary stage deterministically into [`FirmwareSymbolIndex`].
#[must_use]
pub fn build_symbol_index(
    data: &[u8],
    firmware_train_id: &str,
    module: &str,
    base_va: u32,
    overlays: &[SymbolAnnotationOverlay],
) -> FirmwareSymbolIndex {
    let (_, sha256) = calculate_hashes(data);
    let end_va = base_va.saturating_add(data.len() as u32);
    let cfg_funcs = discover_cfg_functions(data, base_va, 0x4000);
    let call_frames = decode_call_frames(data, base_va, None, None, 0x40_0000);
    let fc_loads = find_fc_immediate_loads(data, base_va, base_va, end_va);

    let mut overlay_map: BTreeMap<u32, &SymbolAnnotationOverlay> = BTreeMap::new();
    let mut verified_spans: Vec<(u32, u32)> = Vec::new();
    for ov in overlays {
        if let Ok(va) = parse_va_hex(&ov.va)
            && va >= base_va
            && va < end_va
        {
            overlay_map.insert(va, ov);
            if let Some(sz) = ov.size_bytes
                && sz > 0
            {
                verified_spans.push((va, va.saturating_add(sz).min(end_va)));
            }
        }
    }

    let in_verified_span = |va: u32| -> bool {
        !overlay_map.contains_key(&va) && verified_spans.iter().any(|&(lo, hi)| va > lo && va < hi)
    };
    let mut entry_set: BTreeSet<u32> = overlay_map.keys().copied().collect();
    for c in &cfg_funcs {
        if !in_verified_span(c.entry_va) {
            entry_set.insert(c.entry_va);
        }
    }
    for cf in &call_frames {
        if cf.target_va >= base_va && cf.target_va < end_va && !in_verified_span(cf.target_va) {
            entry_set.insert(cf.target_va);
        }
    }

    let entries: Vec<u32> = entry_set.into_iter().collect();
    let cfg_end_map: BTreeMap<u32, u32> =
        cfg_funcs.iter().map(|c| (c.entry_va, c.end_va)).collect();

    let mut bounds: BTreeMap<u32, u32> = BTreeMap::new();
    for (idx, &entry_va) in entries.iter().enumerate() {
        let next_va = entries.get(idx + 1).copied().unwrap_or(end_va);
        let candidate_end = overlay_map
            .get(&entry_va)
            .and_then(|ov| ov.size_bytes)
            .filter(|&sz| sz > 0)
            .map(|sz| entry_va.saturating_add(sz))
            .unwrap_or_else(|| {
                cfg_end_map.get(&entry_va).copied().unwrap_or_else(|| {
                    let entry_off = entry_va.saturating_sub(base_va) as usize;
                    let off_end = find_return_end_off(data, entry_off, 0x2000);
                    base_va.saturating_add(off_end as u32)
                })
            });
        let bounded_end = candidate_end
            .min(next_va)
            .max(entry_va.saturating_add(2))
            .min(end_va);
        bounds.insert(entry_va, bounded_end);
    }

    let mut callers_by_target: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut callees_by_caller: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut saved_regs_by_target: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut frame_bytes_by_target: BTreeMap<u32, u8> = BTreeMap::new();

    for cf in &call_frames {
        if let Some(caller_entry) = enclosing_entry(&entries, cf.va) {
            let caller_end = bounds.get(&caller_entry).copied().unwrap_or(end_va);
            if cf.va < caller_end {
                callers_by_target
                    .entry(cf.target_va)
                    .or_default()
                    .insert(caller_entry);
                callees_by_caller
                    .entry(caller_entry)
                    .or_default()
                    .insert(cf.target_va);
            }
        }
        let reg_entry = saved_regs_by_target.entry(cf.target_va).or_default();
        for r in &cf.saved_groups {
            reg_entry.insert(r.clone());
        }
        frame_bytes_by_target
            .entry(cf.target_va)
            .and_modify(|cur| *cur = (*cur).max(cf.frame_bytes))
            .or_insert(cf.frame_bytes);
    }

    let mut str_xrefs_by_func: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut mmio_xrefs_by_func: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for fc in &fc_loads {
        let Some(owner) = enclosing_entry(&entries, fc.va) else {
            continue;
        };
        if fc.va >= bounds.get(&owner).copied().unwrap_or(end_va) {
            continue;
        }
        if let Some(text) = &fc.c_string
            && text.len() >= 3
            && text.chars().any(|c| c.is_ascii_alphanumeric())
        {
            str_xrefs_by_func
                .entry(owner)
                .or_default()
                .insert(text.clone());
        }
        if is_nor_or_mmio_addr(fc.value) {
            mmio_xrefs_by_func
                .entry(owner)
                .or_default()
                .insert(fc.value);
        }
    }

    let mut functions: Vec<FunctionSymbolEntry> = entries
        .iter()
        .map(|&entry_va| {
            let entry_off = entry_va.saturating_sub(base_va) as usize;
            let func_end = bounds.get(&entry_va).copied().unwrap_or(end_va);
            let end_off = func_end.saturating_sub(base_va) as usize;
            let slice = &data[entry_off..end_off.min(data.len())];

            let strings: Vec<String> = str_xrefs_by_func
                .get(&entry_va)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            let mmio_raw: Vec<u32> = mmio_xrefs_by_func
                .get(&entry_va)
                .map(|s| s.iter().copied().collect())
                .unwrap_or_default();
            let mmio_xrefs: Vec<String> = mmio_raw.iter().copied().map(format_va_hex).collect();

            let (mut side_effects, mut reasons) = classify_mmio_and_strings(&mmio_raw, &strings);
            let saved = saved_regs_by_target
                .get(&entry_va)
                .cloned()
                .unwrap_or_default();
            let (mut abi, abi_detail, mut ret) =
                infer_abi_and_ret(slice, &saved, frame_bytes_by_target.get(&entry_va).copied());

            let mut name = format!("FUN_{entry_va:08x}");
            let mut evidence_tier = "static_cfg_inferred".to_string();
            let mut notes = None;
            if let Some(ov) = overlay_map.get(&entry_va) {
                if let Some(n) = &ov.name {
                    name = n.clone();
                }
                if let Some(a) = &ov.abi {
                    abi = a.clone();
                }
                if let Some(r) = &ov.ret {
                    ret = r.clone();
                }
                if let Some(se) = ov.side_effects {
                    side_effects = se;
                    reasons.push(format!("curated verified annotation: {}", se.as_str()));
                }
                if let Some(tier) = &ov.evidence_tier {
                    evidence_tier = tier.clone();
                }
                notes = ov.notes.clone();
            }

            FunctionSymbolEntry {
                va: format_va_hex(entry_va),
                end_va: format_va_hex(func_end),
                name,
                abi,
                abi_detail,
                ret,
                callers: callers_by_target
                    .get(&entry_va)
                    .map(|s| s.iter().copied().map(format_va_hex).collect())
                    .unwrap_or_default(),
                callees: callees_by_caller
                    .get(&entry_va)
                    .map(|s| s.iter().copied().map(format_va_hex).collect())
                    .unwrap_or_default(),
                string_xrefs: strings,
                mmio_xrefs,
                side_effects,
                side_effect_reasons: reasons,
                evidence_tier,
                notes,
            }
        })
        .collect();

    propagate_transitive_side_effects(&mut functions);
    let destructive_count = functions
        .iter()
        .filter(|f| f.side_effects.is_destructive())
        .count();

    FirmwareSymbolIndex {
        schema_version: 1,
        firmware_train_id: firmware_train_id.to_string(),
        module: module.to_string(),
        sha256,
        base_va: format_va_hex(base_va),
        size_bytes: data.len(),
        function_count: functions.len(),
        destructive_function_count: destructive_count,
        functions,
    }
}

fn propagate_transitive_side_effects(functions: &mut [FunctionSymbolEntry]) {
    let idx_by_va: BTreeMap<String, usize> = functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.va.clone(), i))
        .collect();

    for _ in 0..8 {
        let mut changed = false;
        let snapshot: Vec<(String, String, SymbolSideEffect)> = functions
            .iter()
            .map(|f| (f.va.clone(), f.name.clone(), f.side_effects))
            .collect();
        for func in functions.iter_mut() {
            if func.side_effects.is_destructive() {
                continue;
            }
            for callee_va in &func.callees {
                let Some(&cidx) = idx_by_va.get(callee_va) else {
                    continue;
                };
                let (ref c_va, ref c_name, c_eff) = snapshot[cidx];
                if c_eff.is_destructive() {
                    func.side_effects = c_eff;
                    func.side_effect_reasons.push(format!(
                        "transitively calls {c_va} ({c_name}) [{}]",
                        c_eff.as_str()
                    ));
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
}
