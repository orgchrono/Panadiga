#![deny(unsafe_code)]

//! SSOT Stage-2 inspection and fail-closed CoW/in-place binary patching.

use anyhow::{Result, bail};

use super::super::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use super::super::sata_bridge_fix::{
    SATA_BRIDGE_STATE_WAIT_1000MS, inspect_prog_sata_bridge_reset_site_for_profile,
    inspect_prog061_sata_bridge_reset_site,
};
use super::super::ssot_patches::stage2_boot_spec_for_loader;
use super::payload::assemble_stage2_payload_for_spec;
use super::{
    PROG_SLACK_FILL_BYTE, PROG061_STAGE2_STATE_ENABLED, PROG061_STAGE2_STATE_FACTORY,
    PROG061_STAGE2_STATE_MISMATCH, Prog061Stage2BootStatus, Stage2BootTrainSpec,
    compute_stage2_hook_bytes,
};

fn build_hook_spec<'a>(spec: &'a Stage2BootTrainSpec, hook_patch: &'a [u8; 7]) -> PatchSpec<'a> {
    PatchSpec {
        name: "stage2_hdd_boot_hook",
        description: "Pre-FVU Stage-2 SATA HDD boot hook",
        va: spec.base_va + spec.hook_offset as u32,
        base_va: spec.base_va,
        expect: &spec.hook_expect,
        replace: hook_patch,
        required_len: Some(spec.expected_size),
        input_sha256: None,
    }
}

/// Inspects an updater RAM image against a `Stage2BootTrainSpec` (`fail-closed` bounds).
pub fn inspect_stage2_boot_site_for_spec(
    buf: &[u8],
    spec: &Stage2BootTrainSpec,
) -> Result<Prog061Stage2BootStatus> {
    let payload = assemble_stage2_payload_for_spec(spec)?;
    let hook_patch = compute_stage2_hook_bytes(spec);
    let hook_spec = build_hook_spec(spec, &hook_patch);
    let cls = classify_patch_site(buf, &hook_spec, PROG061_STAGE2_STATE_ENABLED)?;
    let end = spec.payload_offset + payload.len();
    let tail_slice = buf.get(spec.payload_offset..end).unwrap_or_default();
    let payload_present = tail_slice == payload.as_slice();
    let state = match cls.state {
        "FACTORY" if !payload_present => PROG061_STAGE2_STATE_FACTORY,
        PROG061_STAGE2_STATE_ENABLED if payload_present => PROG061_STAGE2_STATE_ENABLED,
        _ => PROG061_STAGE2_STATE_MISMATCH,
    };
    Ok(Prog061Stage2BootStatus {
        hook_va_hex: format!("0x{:08X}", cls.va),
        payload_va_hex: format!(
            "0x{:08X}",
            spec.base_va.wrapping_add(spec.payload_offset as u32)
        ),
        payload_size_bytes: payload.len(),
        state: state.to_string(),
        payload_present,
        hook_bytes_hex: hex::encode(cls.actual_bytes),
    })
}

/// Resolves a verified [`Stage2BootTrainSpec`] from SSOT `profiles/firmware_trains/*.json`.
pub fn stage2_spec_for_loader_profile(loader_profile_key: &str) -> Result<Stage2BootTrainSpec> {
    stage2_boot_spec_for_loader(loader_profile_key).ok_or_else(|| {
        anyhow::anyhow!(
            "No SSOT stage2_boot_spec defined for loader profile '{}'",
            loader_profile_key
        )
    })
}

/// Inspects `PROG061` (`Loader ver0.61`) for factory vs Stage-2 HDD boot hook via SSOT.
pub fn inspect_prog061_stage2_boot_site(buf: &[u8]) -> Result<Prog061Stage2BootStatus> {
    inspect_stage2_boot_for_loader_profile(buf, "ver061")
}

/// Inspects an updater RAM binary against the SSOT Stage-2 spec for `loader_profile_key`.
pub fn inspect_stage2_boot_for_loader_profile(
    buf: &[u8],
    loader_profile_key: &str,
) -> Result<Prog061Stage2BootStatus> {
    let spec = stage2_spec_for_loader_profile(loader_profile_key)?;
    inspect_stage2_boot_site_for_spec(buf, &spec)
}

/// Applies or removes the Stage-2 HDD boot hook atomically for a train spec (`fail-closed`).
pub fn apply_stage2_boot_in_place_for_spec(
    buf: &mut [u8],
    spec: &Stage2BootTrainSpec,
    enable: bool,
) -> Result<()> {
    let payload = assemble_stage2_payload_for_spec(spec)?;
    let end = spec.payload_offset + payload.len();
    if buf.len() != spec.expected_size {
        bail!(
            "Stage-2 boot patch for '{}' requires {} B binary, got {} B",
            spec.train_id,
            spec.expected_size,
            buf.len()
        );
    }
    if enable && spec.require_reset_fix_1000ms {
        let reset_status = crate::ssot::registry::HardwareProfileRegistry::builtin()
            .bundle()
            .firmware_trains
            .get(spec.train_id)
            .map(|t| t.loader_profile_key.as_str())
            .and_then(|key| inspect_prog_sata_bridge_reset_site_for_profile(buf, key).ok());
        let reset_status = match reset_status {
            Some(st) => st,
            None => inspect_prog061_sata_bridge_reset_site(buf)?,
        };
        if reset_status.state != SATA_BRIDGE_STATE_WAIT_1000MS {
            bail!(
                "Refusing to enable Stage-2 HDD boot before SataBridgeResetFix ({} state={})",
                reset_status.va_hex,
                reset_status.state
            );
        }
    }

    let hook_patch = compute_stage2_hook_bytes(spec);
    let hook_spec = build_hook_spec(spec, &hook_patch);
    let hook_cls = classify_patch_site(buf, &hook_spec, PROG061_STAGE2_STATE_ENABLED)?;
    let tail_slice = &buf[spec.payload_offset..end];

    if enable {
        if hook_cls.state == PROG061_STAGE2_STATE_ENABLED && tail_slice == payload.as_slice() {
            return Ok(());
        }
        if hook_cls.state != "FACTORY" {
            bail!(
                "'{}' hook site 0x{:08X} unexpected state '{}' ({})",
                spec.train_id,
                hook_spec.va,
                hook_cls.state,
                hex::encode(&hook_cls.actual_bytes)
            );
        }
        if !tail_slice.iter().all(|&b| b == PROG_SLACK_FILL_BYTE) {
            bail!(
                "'{}' slack 0x{:05X}..0x{:05X} is not erased (expected 0x{:02X})",
                spec.train_id,
                spec.payload_offset,
                end,
                PROG_SLACK_FILL_BYTE
            );
        }
        buf[spec.payload_offset..end].copy_from_slice(&payload);
        apply_patch_specs_in_place(buf, &[(&hook_spec, PatchApplyAction::ApplyReplace)])?;
    } else {
        if hook_cls.state == "FACTORY" && tail_slice.iter().all(|&b| b == PROG_SLACK_FILL_BYTE) {
            return Ok(());
        }
        if hook_cls.state != PROG061_STAGE2_STATE_ENABLED || tail_slice != payload.as_slice() {
            bail!("Refusing to revert non-matching Stage-2 hook/payload");
        }
        apply_patch_specs_in_place(buf, &[(&hook_spec, PatchApplyAction::RestoreExpect)])?;
        buf[spec.payload_offset..end].fill(PROG_SLACK_FILL_BYTE);
    }
    Ok(())
}

/// Applies or removes the `PROG061` (`Loader ver0.61`) Stage-2 HDD boot hook atomically via SSOT.
pub fn apply_prog061_stage2_boot_in_place(buf: &mut [u8], enable: bool) -> Result<()> {
    apply_stage2_boot_for_loader_profile_in_place(buf, "ver061", enable)
}

/// Applies or removes the SSOT Stage-2 HDD boot hook for `loader_profile_key` atomically.
pub fn apply_stage2_boot_for_loader_profile_in_place(
    buf: &mut [u8],
    loader_profile_key: &str,
    enable: bool,
) -> Result<()> {
    let spec = stage2_spec_for_loader_profile(loader_profile_key)?;
    apply_stage2_boot_in_place_for_spec(buf, &spec, enable)
}
