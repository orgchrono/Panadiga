#![deny(unsafe_code)]

//! Pure functional parsers, BCU fault detectors, and fail-closed access barriers for `ndb>`.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::pico::bench_decoders::{evaluate_live_rail_gate, parse_status_output_report};
use crate::pico::rsp_codec::policy::{
    AM34_CS0_APERTURE_END, AM34_CS0_APERTURE_START, is_am34_lethal_mmio_address,
};

/// Memory access intent validated by [`validate_ndb_access`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NdbAccessKind {
    Read,
    Write,
}

/// Decoded `ndb> dd <va> <len>` 32-bit word dump (LE memory order preserved via `.to_le_bytes()`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NdbDdResult {
    pub base_va: u32,
    pub words: Vec<u32>,
    pub bytes: Vec<u8>,
}

/// Parsed `eb` / `ew` volatile mutation echo (`ADDR: OLD -> NEW`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NdbWriteEcho {
    pub addr: u32,
    pub old_value: u32,
    pub new_value: u32,
}

/// Detected AM34 BCU bus error & PiE-OS `id[..]` task dump from `ndb>` output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NdbBcuFault {
    pub bcberr_hex: String,
    pub fault_address: u32,
    pub task_dump_count: usize,
}

/// Byte-level difference produced by hot-diffing pre/post SDRAM or MMIO snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolatileDiffEntry {
    pub addr: u32,
    pub before: u8,
    pub after: u8,
}

/// Structured deterministic summary replayed from an archived or live `ndb>` / `COM10` trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NdbLogReplaySummary {
    pub source_name: String,
    pub dd_reads: Vec<NdbDdResult>,
    pub write_echoes: Vec<NdbWriteEcho>,
    pub bcu_fault: Option<NdbBcuFault>,
    pub standby_gate_tripped: bool,
    pub standby_gate_reason: Option<String>,
    pub main3060np_osd_caveat: Option<String>,
    pub cleaned_preview: String,
}

/// Known BCU bus fault site (`0xD4004000..0xD400400F`, observed triggering `BCBERR: 00002102`).
pub const NDB_BCU_FAULT_SITE_BASE: u32 = 0xD400_4000;
pub const NDB_BCU_FAULT_SITE_LEN: u32 = 0x10;

/// Strips ANSI CSI color/control sequences (`\x1b[31m`, `\x1b[m`, etc.) and normalizes CRLF.
#[must_use]
pub fn strip_ndb_ansi_prompt(raw: &str) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(normalized.len());
    let mut chars = normalized.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn spans_overlap(addr: u32, len: usize, lo: u64, hi_excl: u64) -> bool {
    let start = u64::from(addr);
    let end_excl = start.saturating_add(len.max(1) as u64);
    start < hi_excl && end_excl > lo
}

/// Fail-closed Silicon Explorer address barrier for live `ndb>` inspection & volatile SDRAM/MMIO testing.
pub fn validate_ndb_access(addr: u32, len: usize, kind: NdbAccessKind) -> Result<()> {
    if len == 0 {
        bail!("Zero-length ndb> access at 0x{addr:08X} rejected");
    }
    if is_am34_lethal_mmio_address(addr, len) || spans_overlap(addr, len, 0xDC0F_0000, 0xDC0F_1000)
    {
        bail!(
            "BLOCKED [LETHAL_MMIO]: {kind:?} at 0x{addr:08X} (+0x{len:X}) touches fatal AM34 register (0xC000100C / 0xDC0F0000..0xDC0F0FFF)"
        );
    }
    let bcu_lo = u64::from(NDB_BCU_FAULT_SITE_BASE);
    let bcu_hi = bcu_lo + u64::from(NDB_BCU_FAULT_SITE_LEN);
    if spans_overlap(addr, len, bcu_lo, bcu_hi) {
        bail!(
            "BLOCKED [BCU_FAULT_GUARD]: {kind:?} at 0x{addr:08X} overlaps guarded BCU fault site 0xD4004000..0xD400400F"
        );
    }
    if kind == NdbAccessKind::Write
        && spans_overlap(addr, len, AM34_CS0_APERTURE_START, AM34_CS0_APERTURE_END)
    {
        bail!(
            "BLOCKED [E05_NOR_READONLY]: volatile ndb> write at 0x{addr:08X} inside CS0 NOR Flash aperture (0x80000000..0x87FFFFFF) is strictly prohibited"
        );
    }
    Ok(())
}

/// Parses one `ndb> dd` output line (`570DF348: F2CF20F0 F5E8FEF8 ...`), decoding LE words accurately.
pub fn parse_ndb_dd_output(raw: &str) -> Result<NdbDdResult> {
    let cleaned = strip_ndb_ansi_prompt(raw);
    for row in crate::pico::cdc::parse_pico_dump_lines(&cleaned) {
        if row.words.is_empty() {
            continue;
        }
        let base_va = u32::from_str_radix(row.address.trim_start_matches("0x"), 16)
            .with_context(|| format!("Invalid hex base address in ndb> dd row: {}", row.address))?;
        let words: Vec<u32> = row
            .words
            .iter()
            .filter_map(|w| u32::from_str_radix(w.trim_start_matches("0x"), 16).ok())
            .collect();
        if words.is_empty() {
            continue;
        }
        return Ok(NdbDdResult {
            base_va,
            words,
            bytes: row.raw_bytes,
        });
    }
    bail!("No valid ndb> dd word row found")
}

/// Parses one `ndb> eb`/`ew` mutation confirmation line (`8800002C: C0F0 -> 40F0`).
pub fn parse_ndb_write_echo(raw: &str) -> Result<NdbWriteEcho> {
    let cleaned = strip_ndb_ansi_prompt(raw);
    for line in cleaned.lines() {
        let trimmed = line.trim();
        let Some((addr_part, vals_part)) = trimmed.split_once(':') else {
            continue;
        };
        let Some((old_part, new_part)) = vals_part.split_once("->") else {
            continue;
        };
        let addr_clean = addr_part
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        if addr_clean.is_empty() || !addr_clean.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let addr = u32::from_str_radix(addr_clean, 16)?;
        let old_tok = old_part
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let new_tok = new_part
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        if old_tok.is_empty() || new_tok.is_empty() {
            continue;
        }
        let old_value = u32::from_str_radix(old_tok, 16)
            .with_context(|| format!("Invalid pre-write hex value in '{trimmed}'"))?;
        let new_value = u32::from_str_radix(new_tok, 16)
            .with_context(|| format!("Invalid post-write hex value in '{trimmed}'"))?;
        return Ok(NdbWriteEcho {
            addr,
            old_value,
            new_value,
        });
    }
    bail!("No valid ndb> write echo (`ADDR: OLD -> NEW`) found")
}

/// Detects AM34 BCU bus error (`Bus error infomation` / `BCBERR:` + `Address:` + `id[..]` task dump).
#[must_use]
pub fn detect_ndb_bcu_fault(raw: &str) -> Option<NdbBcuFault> {
    let cleaned = strip_ndb_ansi_prompt(raw);
    let mut bcberr_hex: Option<String> = None;
    let mut fault_address: Option<u32> = None;
    let mut task_dump_count = 0usize;
    let mut saw_banner = cleaned.contains("Bus error infomation")
        || cleaned.contains("Bus error information")
        || cleaned.contains("BCBERR:");

    for line in cleaned.lines() {
        let trimmed = line.trim();
        if let Some(after) = trimmed.strip_prefix("BCBERR:") {
            saw_banner = true;
            if let Some(tok) = after.split_whitespace().next() {
                bcberr_hex = Some(tok.trim_start_matches("0x").to_ascii_uppercase());
            }
        } else if let Some(after) = trimmed.strip_prefix("Address:")
            && let Some(tok) = after.split_whitespace().next()
        {
            let clean = tok.trim_start_matches("0x").trim_start_matches("0X");
            if let Ok(addr) = u32::from_str_radix(clean, 16) {
                fault_address = Some(addr);
            }
        } else if let Some(prefix_idx) = trimmed.find(":Bus error inf") {
            let clean = trimmed[..prefix_idx].trim();
            if let Ok(addr) = u32::from_str_radix(clean, 16) {
                fault_address = fault_address.or(Some(addr));
                saw_banner = true;
            }
        }
        if trimmed.starts_with("id[") && trimmed.contains("stat[") {
            task_dump_count += 1;
        }
    }

    if !saw_banner && bcberr_hex.is_none() {
        return None;
    }

    Some(NdbBcuFault {
        bcberr_hex: bcberr_hex.unwrap_or_else(|| "UNKNOWN".to_string()),
        fault_address: fault_address.unwrap_or(0),
        task_dump_count,
    })
}

/// Computes exact byte-level diffs between two [`NdbDdResult`] snapshots sharing `base_va`.
pub fn diff_dd_snapshots(
    before: &NdbDdResult,
    after: &NdbDdResult,
) -> Result<Vec<VolatileDiffEntry>> {
    if before.base_va != after.base_va {
        bail!(
            "Cannot diff snapshots with different base addresses (0x{:08X} != 0x{:08X})",
            before.base_va,
            after.base_va
        );
    }
    let common = before.bytes.len().min(after.bytes.len());
    let mut changes = Vec::new();
    for i in 0..common {
        if before.bytes[i] != after.bytes[i] {
            changes.push(VolatileDiffEntry {
                addr: before.base_va + (i as u32),
                before: before.bytes[i],
                after: after.bytes[i],
            });
        }
    }
    Ok(changes)
}

/// Deterministically replays a raw COM10 / `ndb>` evidence log into a structured telemetry report.
pub fn replay_ndb_evidence_log(raw: &str, source_name: &str) -> Result<NdbLogReplaySummary> {
    let cleaned = strip_ndb_ansi_prompt(raw);
    let mut dd_reads = Vec::new();
    let mut write_echoes = Vec::new();

    for line in cleaned.lines() {
        if let Ok(dd) = parse_ndb_dd_output(line) {
            dd_reads.push(dd);
        }
        if let Ok(echo) = parse_ndb_write_echo(line) {
            write_echoes.push(echo);
        }
    }

    let bcu_fault = detect_ndb_bcu_fault(&cleaned);

    let mut standby_gate_tripped = false;
    let mut standby_gate_reason = None;
    if let Some(pos) = raw.rfind("VDD:") {
        let report = parse_status_output_report(&raw[pos..]);
        if let Err(err) = evaluate_live_rail_gate(&report) {
            standby_gate_tripped = true;
            standby_gate_reason = Some(err.to_string());
        }
    }

    let touches_main3060np_patch_site = dd_reads.iter().any(|r| r.base_va == 0x570D_F348)
        || write_echoes
            .iter()
            .any(|e| (0x570D_F348..=0x570D_F34F).contains(&e.addr));
    let main3060np_osd_caveat = if raw.contains("/HDDEX/UDA_M6930_H_JPN_V006")
        || raw.contains("MAIN3060NP")
        || touches_main3060np_patch_site
    {
        Some(
            "MAIN3060NP build limitation: OSD menus still lack Western/Portuguese glyphs because /HDDEX/UDA_M6930_H_JPN_V006 is absent on Brazilian HDDs."
                .to_string(),
        )
    } else {
        None
    };

    let preview: String = cleaned.lines().take(40).collect::<Vec<_>>().join("\n");
    Ok(NdbLogReplaySummary {
        source_name: source_name.to_string(),
        dd_reads,
        write_echoes,
        bcu_fault,
        standby_gate_tripped,
        standby_gate_reason,
        main3060np_osd_caveat,
        cleaned_preview: preview,
    })
}
