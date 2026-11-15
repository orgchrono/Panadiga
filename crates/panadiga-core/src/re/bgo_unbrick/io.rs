//! Filesystem I/O shell for patching `PROG061` and building `DIGADIAG.FRM`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::{
    PROG061_BASE_VA, Prog061BgoPatchMode, Prog061BgoPatchReport, decode_prog061_payload,
    inspect_prog061_bgo_sites, patch_prog061_bgo_in_place_with_policy,
};
use crate::constants::PANA_DIGADIAG_FRM_SIZE;
use crate::crypto::build_digadiag_frm;
use crate::nvram::{calculate_hashes, resolve_input_path};

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

/// Applies `mode` using explicit unsafe-override policy and optionally wraps as 128 KiB `DIGADIAG.FRM`.
pub fn patch_prog061_or_build_digadiag_with_policy(
    input_path: &Path,
    output_path: &Path,
    mode: Prog061BgoPatchMode,
    wrap_as_digadiag: bool,
    allow_unsafe_auto_format: bool,
) -> Result<Prog061BgoPatchReport> {
    let resolved_in = resolve_input_path(input_path)?;
    let raw = fs::read(&resolved_in)
        .with_context(|| format!("Reading PROG061/DIGADIAG input {}", resolved_in.display()))?;
    let (_, original_sha256) = calculate_hashes(&raw);

    let (container_tag, mut payload) = decode_prog061_payload(&raw)?;
    let container_format = container_tag.to_string();

    let inspection_before = inspect_prog061_bgo_sites(&payload)?;
    patch_prog061_bgo_in_place_with_policy(&mut payload, mode, allow_unsafe_auto_format)?;
    let inspection_after = inspect_prog061_bgo_sites(&payload)?;

    let out_bytes = if wrap_as_digadiag {
        let (frm, _) = build_digadiag_frm(
            &payload,
            PROG061_BASE_VA,
            "FVU ",
            "PROG061",
            4,
            PANA_DIGADIAG_FRM_SIZE,
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

/// Fail-closed entry point matching historical signature (`allow_unsafe_auto_format = false`).
pub fn patch_prog061_or_build_digadiag(
    input_path: &Path,
    output_path: &Path,
    mode: Prog061BgoPatchMode,
    wrap_as_digadiag: bool,
) -> Result<Prog061BgoPatchReport> {
    patch_prog061_or_build_digadiag_with_policy(
        input_path,
        output_path,
        mode,
        wrap_as_digadiag,
        false,
    )
}
