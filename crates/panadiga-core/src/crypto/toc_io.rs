#![deny(unsafe_code)]

//! Thin imperative I/O shell for `.FRM` Table of Contents analysis and CLI reporting.

use std::path::Path;

use anyhow::{Context, Result};

use super::codebook::open_resolved_frm;
use super::toc::{analyze_frm_toc_report, format_toc_dump_lines};
use super::types::FrmTocDumpReport;

/// Reads an `.FRM` container safely and returns its parsed TOC report without side effects.
pub fn read_and_analyze_frm_toc(frm_path: &Path) -> Result<FrmTocDumpReport> {
    let (resolved_frm, frm_bytes) = open_resolved_frm(frm_path)?;
    analyze_frm_toc_report(&frm_bytes, &resolved_frm.to_string_lossy())
}

/// Dumps and dissects the TOC and module layout from `PANA_DVD.FRM` or `PANA__SD.FRM`.
pub fn dump_toc(frm_path: &Path, json_path: Option<&Path>) -> Result<FrmTocDumpReport> {
    let report = read_and_analyze_frm_toc(frm_path)?;
    for line in format_toc_dump_lines(&report) {
        log_println!("{line}");
    }
    if let Some(jpath) = json_path {
        let json_data = serde_json::to_string_pretty(&report)
            .context("Failed serializing TOC dump report to JSON")?;
        std::fs::write(jpath, json_data)
            .with_context(|| format!("Failed writing JSON TOC dump to {}", jpath.display()))?;
        log_println!("\n[+] Relatório JSON exportado: {}", jpath.display());
    }
    Ok(report)
}
