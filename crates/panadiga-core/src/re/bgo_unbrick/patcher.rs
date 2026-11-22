//! Pure deterministic `PROG061` BGO inspection, payload decoding, and in-place patching.

use anyhow::{Result, bail};

use super::{
    OFF_BGO_INIT_FAIL_HOOK, ORIG_SUM_BEQ, PATCH_A2CD_AUTOFORMAT, PATCH_SUM_BRA, PROG061_BASE_VA,
    PROG061_EXPECTED_SIZE, Prog061BgoInspection, Prog061BgoPatchMode, Prog061BgoSiteStatus,
    SPEC_A2BC_BRA_MONITOR, SPEC_A2CD_AUTOFORMAT, SPEC_A2CD_UART_MONITOR, SPEC_BGO_SUM,
    SPEC_DEVINFO_CHECK,
};
use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
use crate::nvram::calculate_hashes;
use crate::re::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use crate::re::prog061_uart_monitor::{
    OFF_UART_MONITOR_SLACK, PATCH_A2CD_UART_MONITOR, PROG061_UART_MONITOR_VA,
    build_prog061_uart_monitor_payload,
};

fn site_status_from_spec(
    data: &[u8],
    spec: &PatchSpec<'_>,
    patched_label: &'static str,
) -> Result<Prog061BgoSiteStatus> {
    let cls = classify_patch_site(data, spec, patched_label)?;
    Ok(Prog061BgoSiteStatus {
        va_hex: format!("0x{:08X}", cls.va),
        file_offset_hex: format!("0x{:06X}", cls.file_offset),
        state: cls.state.to_string(),
        bytes_hex: hex::encode(cls.actual_bytes),
    })
}

fn active_bgo_sum_spec(data: &[u8]) -> Result<(PatchSpec<'static>, super::DetectedBgoGeometry)> {
    let geometry = super::detect_prog_bgo_geometry(data)?;
    let spec = if geometry.is_retargeted {
        PatchSpec {
            name: "OFF_BGO_SUM_BEQ_SSOT",
            description: "SSOT-retargeted BGO sector sum check (relocated nfrom_drv.o) -> bra +10",
            va: PROG061_BASE_VA + geometry.bgo_sum_beq_offset as u32,
            base_va: PROG061_BASE_VA,
            expect: &ORIG_SUM_BEQ,
            replace: &PATCH_SUM_BRA,
            required_len: Some(PROG061_EXPECTED_SIZE),
            input_sha256: None,
        }
    } else {
        SPEC_BGO_SUM
    };
    Ok((spec, geometry))
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
    let (bgo_sum_spec, geometry) = active_bgo_sum_spec(data)?;
    let bgo_sum_site = site_status_from_spec(data, &bgo_sum_spec, "BYPASSED_BRA")?;
    let devinfo_site = site_status_from_spec(data, &SPEC_DEVINFO_CHECK, "BYPASSED_BRA")?;
    let bgo_call_cls = classify_patch_site(data, &SPEC_A2BC_BRA_MONITOR, "BRA_TO_MONITOR")?;
    let mut bgo_init_hook_site =
        site_status_from_spec(data, &SPEC_A2CD_AUTOFORMAT, "AUTO_FORMAT_HOOK")?;
    if bgo_init_hook_site.state == "UNKNOWN_OR_MISMATCH" {
        let mon_cls = classify_patch_site(data, &SPEC_A2CD_UART_MONITOR, "UART_MONITOR_HOOK")?;
        if mon_cls.state == "UART_MONITOR_HOOK" {
            bgo_init_hook_site.state = if bgo_call_cls.state == "BRA_TO_MONITOR" {
                "UART_MONITOR_HOOK"
            } else {
                "UART_MONITOR_HOOK_CONDITIONAL_STALE"
            }
            .to_string();
        }
    }

    let detected_mode = match (
        bgo_sum_site.state.as_str(),
        devinfo_site.state.as_str(),
        bgo_init_hook_site.state.as_str(),
        bgo_call_cls.state,
    ) {
        ("FACTORY", "FACTORY", "FACTORY", "FACTORY") => "FACTORY_ORIGINAL",
        ("BYPASSED_BRA", "BYPASSED_BRA", "FACTORY", "FACTORY") => "BYPASS_CHECKS_ONLY",
        ("BYPASSED_BRA", "BYPASSED_BRA", "AUTO_FORMAT_HOOK", "FACTORY") => "AUTO_FORMAT_BGO",
        ("BYPASSED_BRA", "BYPASSED_BRA", "UART_MONITOR_HOOK", "BRA_TO_MONITOR") => {
            "UART_MONITOR_PANAMON1"
        }
        _ => "CUSTOM_OR_PARTIAL",
    };

    Ok(Prog061BgoInspection {
        raw_size: data.len(),
        raw_sha256,
        detected_mode: detected_mode.to_string(),
        bgo_target_geometry: geometry.geometry_id,
        bgo_sum_site,
        devinfo_site,
        bgo_init_hook_site,
    })
}

fn clear_monitor_slack(buf: &mut [u8], monitor_payload: &[u8], slack_end: usize) {
    if &buf[OFF_UART_MONITOR_SLACK..slack_end] == monitor_payload {
        buf[OFF_UART_MONITOR_SLACK..slack_end].fill(0xFF);
    }
}

fn apply_bgo_bypass_sites(
    raw_prog061: &mut [u8],
    bgo_call_action: PatchApplyAction,
    hook_spec: &PatchSpec<'_>,
    hook_action: PatchApplyAction,
) -> Result<()> {
    let (bgo_sum_spec, _) = active_bgo_sum_spec(raw_prog061)?;
    apply_patch_specs_in_place(
        raw_prog061,
        &[
            (&bgo_sum_spec, PatchApplyAction::ApplyReplace),
            (&SPEC_DEVINFO_CHECK, PatchApplyAction::ApplyReplace),
            (&SPEC_A2BC_BRA_MONITOR, bgo_call_action),
            (hook_spec, hook_action),
        ],
    )
}

/// Applies the requested BGO recovery patch with fail-closed policy enforcement on `AutoFormatBgo`.
pub fn patch_prog061_bgo_in_place_with_policy(
    raw_prog061: &mut [u8],
    mode: Prog061BgoPatchMode,
    allow_unsafe_auto_format: bool,
) -> Result<()> {
    if mode == Prog061BgoPatchMode::AutoFormatBgo && !allow_unsafe_auto_format {
        bail!(
            "AutoFormatBgo (nfrom_format @ 0x50401F3C) is blocked fail-closed: it probes/erases CS1 window 0x84200000..0x843FFFFF which aliases Spansion S29JL064H (IC5001) +0x200000..+0x3FFFFF inside read-only IPL NOR AND invokes RAM pointers 0x504F8464/68/6C before setting 0x504802B0==1. Use 'bypass' or 'uart_monitor', or pass explicit unsafe override."
        );
    }

    if raw_prog061.len() != PROG061_EXPECTED_SIZE {
        bail!(
            "Expected {PROG061_EXPECTED_SIZE}-byte PROG061 binary, got {} bytes",
            raw_prog061.len()
        );
    }

    let monitor_payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA)?;
    let slack_end = OFF_UART_MONITOR_SLACK + monitor_payload.len();
    let active_hook_spec = if raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20]
        == PATCH_A2CD_UART_MONITOR
    {
        &SPEC_A2CD_UART_MONITOR
    } else {
        &SPEC_A2CD_AUTOFORMAT
    };

    match mode {
        Prog061BgoPatchMode::BypassChecksOnly => {
            apply_bgo_bypass_sites(
                raw_prog061,
                PatchApplyAction::RestoreExpect,
                active_hook_spec,
                PatchApplyAction::RestoreExpect,
            )?;
            clear_monitor_slack(raw_prog061, &monitor_payload, slack_end);
        }
        Prog061BgoPatchMode::AutoFormatBgo => {
            // Only reachable when caller explicitly set allow_unsafe_auto_format = true.
            if active_hook_spec.replace == PATCH_A2CD_UART_MONITOR {
                apply_patch_specs_in_place(
                    raw_prog061,
                    &[(active_hook_spec, PatchApplyAction::RestoreExpect)],
                )?;
            }
            apply_bgo_bypass_sites(
                raw_prog061,
                PatchApplyAction::RestoreExpect,
                &SPEC_A2CD_AUTOFORMAT,
                PatchApplyAction::ApplyReplace,
            )?;
            clear_monitor_slack(raw_prog061, &monitor_payload, slack_end);
        }
        Prog061BgoPatchMode::UartMonitor => {
            let slack = &raw_prog061[OFF_UART_MONITOR_SLACK..slack_end];
            if slack != monitor_payload.as_slice()
                && !slack.iter().all(|&b| b == 0xFF)
                && !slack.iter().all(|&b| b == 0x00)
            {
                bail!("PROG061 slack 0x{OFF_UART_MONITOR_SLACK:06X} is not erased (expected 0xFF)");
            }
            if active_hook_spec.replace == PATCH_A2CD_AUTOFORMAT
                && &raw_prog061[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20]
                    == PATCH_A2CD_AUTOFORMAT.as_slice()
            {
                apply_patch_specs_in_place(
                    raw_prog061,
                    &[(active_hook_spec, PatchApplyAction::RestoreExpect)],
                )?;
            }
            apply_bgo_bypass_sites(
                raw_prog061,
                PatchApplyAction::ApplyReplace,
                &SPEC_A2CD_UART_MONITOR,
                PatchApplyAction::ApplyReplace,
            )?;
            raw_prog061[OFF_UART_MONITOR_SLACK..slack_end].copy_from_slice(&monitor_payload);
        }
        Prog061BgoPatchMode::FactoryOriginal => {
            let (bgo_sum_spec, _) = active_bgo_sum_spec(raw_prog061)?;
            apply_patch_specs_in_place(
                raw_prog061,
                &[
                    (&bgo_sum_spec, PatchApplyAction::RestoreExpect),
                    (&SPEC_DEVINFO_CHECK, PatchApplyAction::RestoreExpect),
                    (&SPEC_A2BC_BRA_MONITOR, PatchApplyAction::RestoreExpect),
                    (active_hook_spec, PatchApplyAction::RestoreExpect),
                ],
            )?;
            clear_monitor_slack(raw_prog061, &monitor_payload, slack_end);
        }
    }
    Ok(())
}

/// Fail-closed wrapper around `patch_prog061_bgo_in_place_with_policy(..., false)`.
pub fn patch_prog061_bgo_in_place(raw_prog061: &mut [u8], mode: Prog061BgoPatchMode) -> Result<()> {
    patch_prog061_bgo_in_place_with_policy(raw_prog061, mode, false)
}

/// Decodes raw `PROG061` binary bytes or a valid plaintext/Feistel `EXTRHEAD` container into `(format, payload)`.
pub fn decode_prog061_payload(raw: &[u8]) -> Result<(&'static str, Vec<u8>)> {
    if raw.starts_with(b"EXTRHEAD") {
        let unpacked = unpack_extrhead(raw, ExtrheadCryptMode::Plain)?;
        if unpacked.decompressed.len() != PROG061_EXPECTED_SIZE
            || unpacked.header.dst_addr != PROG061_BASE_VA
            || unpacked.decompressed.starts_with(b"EXTRHEAD")
        {
            bail!(
                "Invalid EXTRHEAD PROG061 payload: got {} B at dst=0x{:08X} (expected {PROG061_EXPECTED_SIZE} B at 0x{PROG061_BASE_VA:08X})",
                unpacked.decompressed.len(),
                unpacked.header.dst_addr
            );
        }
        return Ok(("EXTRHEAD_CONTAINER", unpacked.decompressed));
    }
    if raw.len() == PROG061_EXPECTED_SIZE {
        return Ok(("RAW_PROG061", raw.to_vec()));
    }
    if raw.len() == PANA_DIGADIAG_FRM_SIZE {
        bail!(
            "Raw 128 KiB ({PANA_DIGADIAG_FRM_SIZE} B) buffer without 'EXTRHEAD' header rejected (unbootable DIGADIAG.FRM / wrong container)"
        );
    }
    if raw.len() > 0x40 {
        let unpacked = unpack_extrhead(raw, ExtrheadCryptMode::Auto)?;
        if unpacked.decompressed.len() != PROG061_EXPECTED_SIZE {
            bail!(
                "Unpacked container length ({} B) does not match PROG061 ({PROG061_EXPECTED_SIZE} B)",
                unpacked.decompressed.len()
            );
        }
        return Ok(("EXTRHEAD_CONTAINER", unpacked.decompressed));
    }
    bail!(
        "Unrecognized PROG061 input size ({} bytes); expected raw {PROG061_EXPECTED_SIZE}B PROG061 or EXTRHEAD",
        raw.len()
    );
}

/// Inspects an in-memory raw `PROG061` binary or `EXTRHEAD` (`DIGADIAG.FRM`) container.
pub fn inspect_prog061_container_bytes(raw: &[u8]) -> Result<Prog061BgoInspection> {
    let (_, payload) = decode_prog061_payload(raw)?;
    inspect_prog061_bgo_sites(&payload)
}
