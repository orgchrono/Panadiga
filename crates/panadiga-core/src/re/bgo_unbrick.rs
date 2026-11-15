//! Panasonic `DMR-EH55` `PROG061` / `DIGADIAG.FRM` BGO Unbrick & Check-Bypass Patcher
//!
//! Targets the three empirically verified MN103S sites in `eh55_frm_PROG061_BOOT.bin`
//! (`262,144` bytes, base VA `0x50400000`) to recover from `BGO driver init failed[1].` (`U99`):
//!
//! 1. `0x504230F0` (file offset `0x000230F0`, `bgo_sum_23058`): `beq +10` (`0xC8 0x0A`) ->
//!    `bra +10` (`0xCA 0x0A`), bypassing broken `N(N+1)/2` BGO sector sum checks.
//! 2. `0x50405097` (file offset `0x00005097`, `DeviceInfo` read guard): `beq +21` (`0xC8 0x15`) ->
//!    `bra +21` (`0xCA 0x15`), bypassing missing `"INFO"` failure on erased/corrupted NOR.
//! 3. `0x5040A2CD..0x5040A2E0` (file offset `0x0000A2CD`, 20-byte failure branch in
//!    `bgo_init_A294`): optionally replaced in `AutoFormatBgo` mode with a direct `call32` to
//!    the orphan factory formatter `nfrom_format` (`0x50401F3C`, 0 static callers), continuing to
//!    `Control information init` (`0x5040A2E1`) on `d0 == 0` or returning `-1` cleanly otherwise.

mod io;
mod patcher;

#[cfg(test)]
mod tests;

pub use io::{
    ensure_parent_dir, patch_prog061_or_build_digadiag, patch_prog061_or_build_digadiag_with_policy,
};
pub use patcher::{
    decode_prog061_payload, inspect_prog061_bgo_sites, inspect_prog061_container_bytes,
    patch_prog061_bgo_in_place, patch_prog061_bgo_in_place_with_policy,
};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::patch_spec::PatchSpec;
use super::prog061_uart_monitor::PATCH_A2CD_UART_MONITOR;

/// Virtual base address of `eh55_frm_PROG061_BOOT.bin` (`SDDIAG` loads `DIGADIAG.FRM` here).
pub const PROG061_BASE_VA: u32 = 0x5040_0000;
/// Exact unpacked byte size of `eh55_frm_PROG061_BOOT.bin`.
pub const PROG061_EXPECTED_SIZE: usize = 0x4_0000;
/// Orphan factory NFROM/BGO formatter function in `PROG061` (`0` static callers).
pub const PROG061_NFROM_FORMAT_VA: u32 = 0x5040_1F3C;

/// `0x504230F0`: `beq +10` (`C8 0A`) -> `bra +10` (`CA 0A`) in `bgo_sum_23058`.
pub const OFF_BGO_SUM_BEQ: usize = 0x0002_30F0;
/// `0x50405097`: `beq +21` (`C8 15`) -> `bra +21` (`CA 15`) after `DeviceInfo` read.
pub const OFF_DEVINFO_CHECK_BEQ: usize = 0x0000_5097;
/// `0x5040A2CD`: 20-byte `BGO driver init failed[%u]` failure branch inside `bgo_init_A294`.
pub const OFF_BGO_INIT_FAIL_HOOK: usize = 0x0000_A2CD;

pub const ORIG_SUM_BEQ: [u8; 2] = [0xC8, 0x0A];
pub const PATCH_SUM_BRA: [u8; 2] = [0xCA, 0x0A];

pub const ORIG_DEVINFO_BEQ: [u8; 2] = [0xC8, 0x15];
pub const PATCH_DEVINFO_BRA: [u8; 2] = [0xCA, 0x15];

pub const ORIG_A2CD_HOOK: [u8; 20] = [
    0xFC, 0xCC, 0xB0, 0x3E, 0x43, 0x50, 0xF8, 0x07, 0xFC, 0xDD, 0xF7, 0xAA, 0xFF, 0xFF, 0x00, 0x00,
    0x80, 0xFF, 0xCA, 0x32,
];

/// `call32 0x50401F3C` (`dd 6f 7c ff ff 00 00`); `cmp 0, d0` (`a0 00`);
/// `beq +11 -> 0x5040A2E1` (`c8 0b`); `mov -1, d0` (`80 ff`);
/// `bra +0x37 -> 0x5040A311` (`ca 37`); `nop` x5 (`cb * 5`).
pub const PATCH_A2CD_AUTOFORMAT: [u8; 20] = [
    0xDD, 0x6F, 0x7C, 0xFF, 0xFF, 0x00, 0x00, 0xA0, 0x00, 0xC8, 0x0B, 0x80, 0xFF, 0xCA, 0x37, 0xCB,
    0xCB, 0xCB, 0xCB, 0xCB,
];

pub(super) const SPEC_BGO_SUM: PatchSpec<'static> = PatchSpec {
    name: "OFF_BGO_SUM_BEQ",
    description: "bgo_sum_23058 N(N+1)/2 BGO sector sum check -> bra +10",
    va: PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
    base_va: PROG061_BASE_VA,
    expect: &ORIG_SUM_BEQ,
    replace: &PATCH_SUM_BRA,
    required_len: Some(PROG061_EXPECTED_SIZE),
    input_sha256: None,
};

pub(super) const SPEC_DEVINFO_CHECK: PatchSpec<'static> = PatchSpec {
    name: "OFF_DEVINFO_CHECK_BEQ",
    description: "DeviceInfo missing INFO check -> bra +21",
    va: PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
    base_va: PROG061_BASE_VA,
    expect: &ORIG_DEVINFO_BEQ,
    replace: &PATCH_DEVINFO_BRA,
    required_len: Some(PROG061_EXPECTED_SIZE),
    input_sha256: None,
};

pub(super) const SPEC_A2CD_AUTOFORMAT: PatchSpec<'static> = PatchSpec {
    name: "OFF_BGO_INIT_FAIL_HOOK",
    description: "bgo_init_A294 failure branch -> call32 nfrom_format (0x50401F3C)",
    va: PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
    base_va: PROG061_BASE_VA,
    expect: &ORIG_A2CD_HOOK,
    replace: &PATCH_A2CD_AUTOFORMAT,
    required_len: Some(PROG061_EXPECTED_SIZE),
    input_sha256: None,
};

pub(super) const SPEC_A2CD_UART_MONITOR: PatchSpec<'static> = PatchSpec {
    name: "OFF_BGO_INIT_FAIL_HOOK",
    description: "bgo_init_A294 failure branch -> PANAMON1 UART0 monitor @ 0x50439494",
    va: PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
    base_va: PROG061_BASE_VA,
    expect: &ORIG_A2CD_HOOK,
    replace: &PATCH_A2CD_UART_MONITOR,
    required_len: Some(PROG061_EXPECTED_SIZE),
    input_sha256: None,
};

/// Supported BGO recovery strategies for `PROG061` / `DIGADIAG.FRM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prog061BgoPatchMode {
    /// Bypass `bgo_sum_23058` (`0x504230F0`) and `DeviceInfo` (`0x50405097`) checks (non-destructive).
    BypassChecksOnly,
    /// Bypass checks AND wire `0x5040A2CD` to invoke orphan `nfrom_format` (`0x50401F3C`) on error.
    AutoFormatBgo,
    /// Bypass checks AND embed interactive `PANAMON1` UART0 monitor at `0x50439494` (`B`/`F`/`D`/`Q`).
    UartMonitor,
    /// Restore verbatim Panasonic factory bytes at all 3 sites.
    FactoryOriginal,
}

impl std::str::FromStr for Prog061BgoPatchMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bypass" | "bypass_checks" | "checks_only" => Ok(Self::BypassChecksOnly),
            "auto_format" | "autoformat" | "format" => Ok(Self::AutoFormatBgo),
            "uart_monitor" | "monitor" | "panamon1" => Ok(Self::UartMonitor),
            "factory" | "factory_original" | "restore" => Ok(Self::FactoryOriginal),
            other => bail!(
                "Unsupported BGO patch mode '{other}': expected bypass, auto_format, uart_monitor, or factory"
            ),
        }
    }
}

/// State of a verified patch site in `PROG061`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prog061BgoSiteStatus {
    pub va_hex: String,
    pub file_offset_hex: String,
    pub state: String,
    pub bytes_hex: String,
}

/// Complete inspection report of `PROG061` BGO unbrick & bypass sites.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prog061BgoInspection {
    pub raw_size: usize,
    pub raw_sha256: String,
    pub detected_mode: String,
    pub bgo_sum_site: Prog061BgoSiteStatus,
    pub devinfo_site: Prog061BgoSiteStatus,
    pub bgo_init_hook_site: Prog061BgoSiteStatus,
}

/// Summary produced after patching `PROG061` or building `DIGADIAG.FRM`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prog061BgoPatchReport {
    pub input_path: String,
    pub output_path: String,
    pub container_format: String,
    pub mode: String,
    pub digadiag_wrapped: bool,
    pub output_size: usize,
    pub original_sha256: String,
    pub output_sha256: String,
    pub inspection_before: Prog061BgoInspection,
    pub inspection_after: Prog061BgoInspection,
}
