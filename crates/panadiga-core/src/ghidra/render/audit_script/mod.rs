//! Renders the automated Ghidra stage, BGO, and AV passthrough audit script.

mod header_and_detect;
mod runner;
mod scanners;

use crate::re::bgo_unbrick::{
    OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
    PROG061_BASE_VA,
};
use crate::re::main_passthrough::{
    MAIN_BASE_VA, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
};

pub(super) struct AuditSiteVas {
    pub prog_sum: u32,
    pub prog_devinfo: u32,
    pub prog_a2bc: u32,
    pub prog_a2cd: u32,
    pub main_fpc_ctrl: u32,
    pub main_odd_branch: u32,
    pub main_odd_status: u32,
}

fn collect_audit_site_vas() -> AuditSiteVas {
    AuditSiteVas {
        prog_sum: PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
        prog_devinfo: PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
        prog_a2bc: PROG061_BASE_VA + OFF_BGO_INIT_CALL as u32,
        prog_a2cd: PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
        main_fpc_ctrl: MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
        main_odd_branch: MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
        main_odd_status: MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
    }
}

/// Generates an automated Ghidra Jython 2.7 / Python-3 stage detection, patch-verification,
/// MMIO-constant & XREF audit script (`ghidra_auto_unbrick_audit.py`).
pub fn generate_ghidra_auto_unbrick_audit() -> String {
    let vas = collect_audit_site_vas();
    format!(
        "{}\n{}\n{}",
        header_and_detect::render_header_and_classifiers(&vas),
        scanners::PYTHON_SCANNERS,
        runner::render_audit_runner(&vas),
    )
}
