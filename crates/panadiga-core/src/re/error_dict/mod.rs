//! Declarative Panasonic DIGA & Matshita R6-MV4 Firmware Error Dictionary (`431` DRV + Boot/VFD SSOT).

pub mod types;

#[cfg(test)]
mod tests;

use std::sync::OnceLock;

pub use types::{
    FirmwareCorrelationMatch, FirmwareCorrelationReport, FirmwareErrorDomain, FirmwareErrorEntry,
    FirmwareErrorSeverity,
};

const FIRMWARE_ERROR_DICT_JSON: &str = include_str!("firmware_error_dict.json");
static PARSED_DICT: OnceLock<Vec<FirmwareErrorEntry>> = OnceLock::new();

/// Returns all `447` embedded Panasonic / Matshita firmware error dictionary records.
#[must_use]
pub fn all_firmware_errors() -> &'static [FirmwareErrorEntry] {
    PARSED_DICT
        .get_or_init(|| {
            serde_json::from_str::<Vec<FirmwareErrorEntry>>(FIRMWARE_ERROR_DICT_JSON)
                .expect("firmware_error_dict.json must contain valid FirmwareErrorEntry records")
        })
        .as_slice()
}

fn normalized_hex_variants(query: &str) -> Vec<String> {
    let up = query.trim().to_ascii_uppercase();
    let mut out = vec![up.clone()];
    let stripped = up.trim_start_matches("0X");
    if stripped.len() == 8
        && stripped.starts_with("8004")
        && let Ok(val) = u32::from_str_radix(stripped, 16)
    {
        let lo16 = (val & 0xFFFF) as u16;
        if lo16 == 0x0904 {
            out.push("0x83A7".to_string());
        }
        out.push(format!("0x{lo16:04X}"));
    }
    if stripped.len() == 4 && stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        out.push(format!("0X{stripped}"));
    }
    out
}

/// Looks up matching dictionary records by code (`0x83A7`, `U99`, `-91006`, `80045300`) or keyword.
#[must_use]
pub fn lookup_firmware_errors(query: &str) -> Vec<&'static FirmwareErrorEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let variants = normalized_hex_variants(trimmed);
    let q_up = trimmed.to_ascii_uppercase();
    let mut exact = Vec::new();
    let mut fuzzy = Vec::new();

    for entry in all_firmware_errors() {
        let code_up = entry.code.to_ascii_uppercase();
        let is_exact = variants.contains(&code_up)
            || entry
                .aliases
                .iter()
                .any(|a| variants.contains(&a.to_ascii_uppercase()));
        if is_exact {
            exact.push(entry);
        } else if entry.title.to_ascii_uppercase().contains(&q_up)
            || entry.translation.to_ascii_uppercase().contains(&q_up)
            || entry.native_text.to_ascii_uppercase().contains(&q_up)
        {
            fuzzy.push(entry);
        }
    }
    exact.extend(fuzzy);
    exact
}

fn push_unique_match(
    out: &mut Vec<FirmwareCorrelationMatch>,
    entry: &FirmwareErrorEntry,
    evidence_source: &str,
) {
    if out.iter().any(|m| m.code == entry.code) {
        return;
    }
    out.push(FirmwareCorrelationMatch {
        code: entry.code.clone(),
        domain: entry.domain,
        severity: entry.severity,
        title: entry.title.clone(),
        address: entry.address.clone(),
        native_text: entry.native_text.clone(),
        translation: entry.translation.clone(),
        evidence_source: evidence_source.to_string(),
        recommended_actions: entry.actions.clone(),
    });
}

fn match_token_against_core(
    out: &mut Vec<FirmwareCorrelationMatch>,
    haystack_up: &str,
    evidence_source: &str,
) {
    for entry in all_firmware_errors() {
        if entry.domain == FirmwareErrorDomain::DrvMecha && entry.code == "0x0000" {
            continue;
        }
        let hit = if entry.domain == FirmwareErrorDomain::DrvMecha {
            entry
                .aliases
                .iter()
                .any(|a| a.starts_with("0x") && haystack_up.contains(&a.to_ascii_uppercase()))
        } else {
            entry
                .aliases
                .iter()
                .any(|a| haystack_up.contains(&a.to_ascii_uppercase()))
        };
        if hit {
            push_unique_match(out, entry, evidence_source);
        }
    }
}

/// Correlates captured Pico UART text, VFD front-panel observations, and electrical rail telemetry
/// against all embedded firmware error signatures.
#[must_use]
pub fn correlate_firmware_occurrences(
    uart_capture: &str,
    vfd_observation: Option<&str>,
    vdd_volts: Option<f32>,
    nsw5v_edges: Option<u32>,
    rxd_edges: Option<u32>,
) -> FirmwareCorrelationReport {
    let mut matches = Vec::new();

    if !uart_capture.trim().is_empty() {
        match_token_against_core(
            &mut matches,
            &uart_capture.to_ascii_uppercase(),
            "P7501 UART RX capture",
        );
    }
    if let Some(vfd) = vfd_observation
        && !vfd.trim().is_empty()
    {
        match_token_against_core(&mut matches, &vfd.to_ascii_uppercase(), "Front panel VFD");
    }

    // Derive additional actionable recommendations from combined electrical/UART state
    let mut actions = Vec::new();
    for m in &matches {
        for act in &m.recommended_actions {
            if !actions.contains(act) {
                actions.push(act.clone());
            }
        }
    }

    if matches!(rxd_edges, Some(0)) && nsw5v_edges.unwrap_or(0) > 0 {
        actions.push(
            "NSW5V toggled during wakeup while GP2 (UARTP2M_T Pin 6) had 0 edges: Digital PCB powered on briefly, \
             indicating pre-UART freeze (power rail droop/PFAIL[L], SA0..SA3 IPL lockup, SDRAM fault, or swapped Pin 3/Pin 6)."
                .to_string(),
        );
    } else if matches!(vdd_volts, Some(v) if v <= 0.20) && matches.is_empty() {
        actions.push(
            "Target currently in Standby P-OFF (VDD <= 0.20 V): trigger wakeup ([POWER] or PWRTRACE/BOOTTRACE loop) before judging UART silence."
                .to_string(),
        );
    }

    let summary = if matches.is_empty() {
        "No matching firmware error dictionary signatures found in captured evidence.".to_string()
    } else {
        let codes: Vec<&str> = matches.iter().map(|m| m.code.as_str()).collect();
        format!(
            "Matched {} firmware signature(s) across {} catalog entries: {}",
            matches.len(),
            all_firmware_errors().len(),
            codes.join(", ")
        )
    };

    FirmwareCorrelationReport {
        dictionary_total_entries: all_firmware_errors().len(),
        matches: matches.into_iter().collect(),
        prioritized_actions: actions,
        summary,
    }
}
