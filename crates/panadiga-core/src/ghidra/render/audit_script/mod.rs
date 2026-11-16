//! Renders the automated Ghidra stage, BGO, and AV passthrough audit script.

mod header_and_detect;
mod runner;
mod scanners;

/// Generates an automated Ghidra Jython 2.7 / Python-3 stage detection, patch-verification,
/// MMIO-constant & XREF audit script (`ghidra_auto_unbrick_audit.py`).
pub fn generate_ghidra_auto_unbrick_audit() -> String {
    format!(
        "{}\n{}\n{}",
        header_and_detect::render_header_and_classifiers(),
        scanners::PYTHON_SCANNERS,
        runner::render_audit_runner(),
    )
}
