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

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::prog061_uart_monitor::{
    OFF_UART_MONITOR_SLACK, PATCH_A2CD_UART_MONITOR, PROG061_UART_MONITOR_VA,
    build_prog061_uart_monitor_payload,
};
use crate::crypto::{ExtrheadCryptMode, build_digadiag_frm, unpack_extrhead};
use crate::nvram::{calculate_hashes, resolve_input_path};

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

fn classify_site(
    data: &[u8],
    offset: usize,
    len: usize,
    factory: &[u8],
    patched: &[u8],
    patched_label: &str,
) -> Result<Prog061BgoSiteStatus> {
    if offset + len > data.len() {
        bail!(
            "PROG061 binary too small for site 0x{offset:05X} (need {}, got {})",
            offset + len,
            data.len()
        );
    }
    let slice = &data[offset..offset + len];
    let state = if slice == factory {
        "FACTORY"
    } else if slice == patched {
        patched_label
    } else {
        "UNKNOWN_OR_MISMATCH"
    };
    Ok(Prog061BgoSiteStatus {
        va_hex: format!("0x{:08X}", PROG061_BASE_VA.wrapping_add(offset as u32)),
        file_offset_hex: format!("0x{offset:06X}"),
        state: state.to_string(),
        bytes_hex: hex::encode(slice),
    })
}

fn ensure_known_site(slice: &[u8], factory: &[u8], patched: &[u8], label: &str) -> Result<()> {
    if slice != factory && slice != patched {
        bail!(
            "Unexpected bytes at PROG061 {label}: got {} (expected factory {} or patched {})",
            hex::encode(slice),
            hex::encode(factory),
            hex::encode(patched),
        );
    }
    Ok(())
}

/// Inspects the three BGO unbrick/bypass sites in an unpacked `PROG061` image (`262,144` bytes).
pub fn inspect_prog061_bgo_sites(data: &[u8]) -> Result<Prog061BgoInspection> {
    if data.len() != PROG061_EXPECTED_SIZE {
        bail!(
            "Expected {PROG061_EXPECTED_SIZE}-byte PROG061 binary, got {} bytes",
            data.len()
        );
    }
    let (_, raw_sha256) = calculate_hashes(data);
    let bgo_sum_site = classify_site(
        data,
        OFF_BGO_SUM_BEQ,
        2,
        &ORIG_SUM_BEQ,
        &PATCH_SUM_BRA,
        "BYPASSED_BRA",
    )?;
    let devinfo_site = classify_site(
        data,
        OFF_DEVINFO_CHECK_BEQ,
        2,
        &ORIG_DEVINFO_BEQ,
        &PATCH_DEVINFO_BRA,
        "BYPASSED_BRA",
    )?;
    let mut bgo_init_hook_site = classify_site(
        data,
        OFF_BGO_INIT_FAIL_HOOK,
        20,
        &ORIG_A2CD_HOOK,
        &PATCH_A2CD_AUTOFORMAT,
        "AUTO_FORMAT_HOOK",
    )?;
    if bgo_init_hook_site.state == "UNKNOWN_OR_MISMATCH"
        && data[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20] == PATCH_A2CD_UART_MONITOR
    {
        bgo_init_hook_site.state = "UART_MONITOR_HOOK".to_string();
    }

    let detected_mode = match (
        bgo_sum_site.state.as_str(),
        devinfo_site.state.as_str(),
        bgo_init_hook_site.state.as_str(),
    ) {
        ("FACTORY", "FACTORY", "FACTORY") => "FACTORY_ORIGINAL",
        ("BYPASSED_BRA", "BYPASSED_BRA", "FACTORY") => "BYPASS_CHECKS_ONLY",
        ("BYPASSED_BRA", "BYPASSED_BRA", "AUTO_FORMAT_HOOK") => "AUTO_FORMAT_BGO",
        ("BYPASSED_BRA", "BYPASSED_BRA", "UART_MONITOR_HOOK") => "UART_MONITOR_PANAMON1",
        _ => "CUSTOM_OR_PARTIAL",
    };

    Ok(Prog061BgoInspection {
        raw_size: data.len(),
        raw_sha256,
        detected_mode: detected_mode.to_string(),
        bgo_sum_site,
        devinfo_site,
        bgo_init_hook_site,
    })
}

/// Applies the requested BGO recovery patch directly to an unpacked `262,144`-byte `PROG061` buffer.
pub fn patch_prog061_bgo_in_place(raw_prog061: &mut [u8], mode: Prog061BgoPatchMode) -> Result<()> {
    if raw_prog061.len() != PROG061_EXPECTED_SIZE {
        bail!(
            "Expected {PROG061_EXPECTED_SIZE}-byte PROG061 binary, got {} bytes",
            raw_prog061.len()
        );
    }

    ensure_known_site(
        &raw_prog061[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2],
        &ORIG_SUM_BEQ,
        &PATCH_SUM_BRA,
        "OFF_BGO_SUM_BEQ",
    )?;
    ensure_known_site(
        &raw_prog061[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2],
        &ORIG_DEVINFO_BEQ,
        &PATCH_DEVINFO_BRA,
        "OFF_DEVINFO_CHECK_BEQ",
    )?;
    if raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20] != PATCH_A2CD_UART_MONITOR {
        ensure_known_site(
            &raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20],
            &ORIG_A2CD_HOOK,
            &PATCH_A2CD_AUTOFORMAT,
            "OFF_BGO_INIT_FAIL_HOOK",
        )?;
    }

    let monitor_payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA)?;
    let slack_end = OFF_UART_MONITOR_SLACK + monitor_payload.len();
    let clear_slack_if_installed = |buf: &mut [u8]| {
        if &buf[OFF_UART_MONITOR_SLACK..slack_end] == monitor_payload.as_slice() {
            buf[OFF_UART_MONITOR_SLACK..slack_end].fill(0xFF);
        }
    };

    match mode {
        Prog061BgoPatchMode::BypassChecksOnly => {
            raw_prog061[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&PATCH_SUM_BRA);
            raw_prog061[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2]
                .copy_from_slice(&PATCH_DEVINFO_BRA);
            raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20]
                .copy_from_slice(&ORIG_A2CD_HOOK);
            clear_slack_if_installed(raw_prog061);
        }
        Prog061BgoPatchMode::AutoFormatBgo => {
            raw_prog061[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&PATCH_SUM_BRA);
            raw_prog061[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2]
                .copy_from_slice(&PATCH_DEVINFO_BRA);
            raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20]
                .copy_from_slice(&PATCH_A2CD_AUTOFORMAT);
            clear_slack_if_installed(raw_prog061);
        }
        Prog061BgoPatchMode::UartMonitor => {
            let slack = &raw_prog061[OFF_UART_MONITOR_SLACK..slack_end];
            if slack != monitor_payload.as_slice()
                && !slack.iter().all(|&b| b == 0xFF)
                && !slack.iter().all(|&b| b == 0x00)
            {
                bail!("PROG061 slack 0x{OFF_UART_MONITOR_SLACK:06X} is not erased (expected 0xFF)");
            }
            raw_prog061[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&PATCH_SUM_BRA);
            raw_prog061[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2]
                .copy_from_slice(&PATCH_DEVINFO_BRA);
            raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20]
                .copy_from_slice(&PATCH_A2CD_UART_MONITOR);
            raw_prog061[OFF_UART_MONITOR_SLACK..slack_end].copy_from_slice(&monitor_payload);
        }
        Prog061BgoPatchMode::FactoryOriginal => {
            raw_prog061[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
            raw_prog061[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2]
                .copy_from_slice(&ORIG_DEVINFO_BEQ);
            raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20]
                .copy_from_slice(&ORIG_A2CD_HOOK);
            clear_slack_if_installed(raw_prog061);
        }
    }
    Ok(())
}

/// Ensures `path.parent()` exists before writing an output binary artifact.
pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Creating output directory {}", parent.display()))?;
    }
    Ok(())
}

/// Loads a raw `PROG061` binary or `EXTRHEAD` container, applies `mode`, and optionally wraps
/// the patched binary into a bootable plaintext `DIGADIAG.FRM` targeting `0x50400000`.
pub fn patch_prog061_or_build_digadiag(
    input_path: &Path,
    output_path: &Path,
    mode: Prog061BgoPatchMode,
    wrap_as_digadiag: bool,
) -> Result<Prog061BgoPatchReport> {
    let resolved_in = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved_in)
        .with_context(|| format!("Reading PROG061/DIGADIAG input {}", resolved_in.display()))?;
    let (_, original_sha256) = calculate_hashes(&raw);

    let (container_format, mut payload) = if raw.starts_with(b"EXTRHEAD") {
        let unpacked = unpack_extrhead(&raw, ExtrheadCryptMode::Plain)?;
        ("EXTRHEAD_CONTAINER".to_string(), unpacked.decompressed)
    } else if raw.len() == PROG061_EXPECTED_SIZE {
        ("RAW_PROG061".to_string(), raw)
    } else if raw.len() > 0x40 {
        // Fallback: attempt encrypted/auto EXTRHEAD extraction
        let unpacked = unpack_extrhead(&raw, ExtrheadCryptMode::Auto)?;
        ("EXTRHEAD_CONTAINER".to_string(), unpacked.decompressed)
    } else {
        bail!(
            "Unrecognized PROG061 input size ({} bytes); expected raw 262144B PROG061 or EXTRHEAD",
            raw.len()
        );
    };

    let inspection_before = inspect_prog061_bgo_sites(&payload)?;
    patch_prog061_bgo_in_place(&mut payload, mode)?;
    let inspection_after = inspect_prog061_bgo_sites(&payload)?;

    let out_bytes = if wrap_as_digadiag {
        // BOOT.BIN SDDIAG reads exact 128 KiB (0x20000) of DIGADIAG.FRM from SDSC FAT16;
        // real PROG061 (262,144 B) compresses to ~117.5 KiB with zlib-9, fitting inside 0x20000.
        let container_budget = 0x2_0000;
        let (frm, _) = build_digadiag_frm(
            &payload,
            PROG061_BASE_VA,
            "FVU ",
            "PROG061",
            4,
            container_budget,
        )?;
        frm
    } else {
        payload
    };

    ensure_parent_dir(output_path)?;
    fs::write(output_path, &out_bytes)
        .with_context(|| format!("Writing output {}", output_path.display()))?;

    let (_, output_sha256) = calculate_hashes(&out_bytes);
    let mode_label = match mode {
        Prog061BgoPatchMode::BypassChecksOnly => "bypass",
        Prog061BgoPatchMode::AutoFormatBgo => "auto_format",
        Prog061BgoPatchMode::UartMonitor => "uart_monitor",
        Prog061BgoPatchMode::FactoryOriginal => "factory",
    };

    Ok(Prog061BgoPatchReport {
        input_path: resolved_in.to_string_lossy().into_owned(),
        output_path: output_path.to_string_lossy().into_owned(),
        container_format,
        mode: mode_label.to_string(),
        digadiag_wrapped: wrap_as_digadiag,
        output_size: out_bytes.len(),
        original_sha256,
        output_sha256,
        inspection_before,
        inspection_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_factory_prog061() -> Vec<u8> {
        let mut buf = vec![0u8; PROG061_EXPECTED_SIZE];
        buf[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
        buf[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
        buf[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);
        buf
    }

    #[test]
    fn test_prog061_bgo_patch_modes_and_digadiag_roundtrip() {
        let factory = synthetic_factory_prog061();
        let insp = inspect_prog061_bgo_sites(&factory).unwrap();
        assert_eq!(insp.detected_mode, "FACTORY_ORIGINAL");

        let mut patched = factory.clone();
        patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::BypassChecksOnly).unwrap();
        assert_eq!(
            inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
            "BYPASS_CHECKS_ONLY"
        );

        patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::UartMonitor).unwrap();
        assert_eq!(
            inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
            "UART_MONITOR_PANAMON1"
        );

        patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::FactoryOriginal).unwrap();
        assert_eq!(
            inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
            "FACTORY_ORIGINAL"
        );

        patch_prog061_bgo_in_place(&mut patched, Prog061BgoPatchMode::AutoFormatBgo).unwrap();
        assert_eq!(
            inspect_prog061_bgo_sites(&patched).unwrap().detected_mode,
            "AUTO_FORMAT_BGO"
        );

        let dir = std::env::temp_dir().join("panadiga_bgo_unbrick_test");
        let _ = fs::create_dir_all(&dir);
        let raw_in = dir.join("prog061_raw.bin");
        let diag_out = dir.join("DIGADIAG.FRM");
        fs::write(&raw_in, &factory).unwrap();

        let report = patch_prog061_or_build_digadiag(
            &raw_in,
            &diag_out,
            Prog061BgoPatchMode::AutoFormatBgo,
            true,
        )
        .unwrap();
        assert!(report.digadiag_wrapped);
        assert_eq!(report.output_size, 0x2_0000);
        assert_eq!(report.inspection_before.detected_mode, "FACTORY_ORIGINAL");
        assert_eq!(report.inspection_after.detected_mode, "AUTO_FORMAT_BGO");

        let unpacked =
            unpack_extrhead(&fs::read(&diag_out).unwrap(), ExtrheadCryptMode::Plain).unwrap();
        assert_eq!(unpacked.header.dst_addr, PROG061_BASE_VA);
        assert_eq!(
            inspect_prog061_bgo_sites(&unpacked.decompressed)
                .unwrap()
                .detected_mode,
            "AUTO_FORMAT_BGO"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_prog061_real_binary_if_present() {
        let local = Path::new(r"C:\ghidra\binaries\unpacked\eh55_frm_PROG061_BOOT.bin");
        if !local.is_file() {
            return;
        }
        let mut bytes = fs::read(local).unwrap();
        let insp = inspect_prog061_bgo_sites(&bytes).unwrap();
        assert_eq!(insp.detected_mode, "FACTORY_ORIGINAL");

        patch_prog061_bgo_in_place(&mut bytes, Prog061BgoPatchMode::UartMonitor).unwrap();
        let (frm, _) =
            build_digadiag_frm(&bytes, PROG061_BASE_VA, "DIAG", "PANAMON1", 4, 0x2_0000).unwrap();
        assert_eq!(frm.len(), 0x2_0000);
        let unpacked = unpack_extrhead(&frm, ExtrheadCryptMode::Plain).unwrap();
        assert_eq!(
            inspect_prog061_bgo_sites(&unpacked.decompressed)
                .unwrap()
                .detected_mode,
            "UART_MONITOR_PANAMON1"
        );
    }
}
