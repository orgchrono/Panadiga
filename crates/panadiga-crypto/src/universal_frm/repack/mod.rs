#![deny(unsafe_code)]

//! Unified `.FRM` Repacker, Version/Flag Patcher, and `MAIN` `fmaX` Partition Rebuilder.

mod options;
mod pack_bluray;
mod pack_legacy;

use anyhow::{Context, Result, bail};

use super::decode::unpack_universal_frm;
use super::types::{UniversalFrmContainer, UniversalFrmFormat, UniversalFrmSubContainer};
use crate::aes128::FrmKeyring;
use crate::pana_fw_go::{reassemble_main_partitions, wrap_aes_frm};
use options::{find_patch_for_module, resolve_output_formats};
use pack_bluray::pack_bluray_subcontainer;
use pack_legacy::pack_legacy_subcontainer;

pub use options::{FrmRepackOptions, FrmTargetMode, RepackCompression, UniversalFrmRepackReport};

fn mutate_subcontainer(
    sub: &mut UniversalFrmSubContainer,
    opts: &FrmRepackOptions,
    seen_mod_patches: &mut [bool],
    modules_modified: &mut Vec<String>,
    partitions_patched: &mut Vec<String>,
) -> Result<Vec<bool>> {
    let mut mod_flags = vec![false; sub.modules.len()];
    for (idx, m) in sub.modules.iter_mut().enumerate() {
        let up = m.name.trim().to_ascii_uppercase();
        let is_main = up.starts_with("MAIN") || up.starts_with("SMAI");
        if opts.patch_force && is_main {
            m.target_flag = 1;
        }
        if opts.patch_format_hdd && up.starts_with("PROG") {
            m.target_flag = 1;
        }
        if let Some(delta) = opts.patch_version_offset
            && is_main
        {
            m.version_u16 = m.version_u16.wrapping_add(delta as u16);
        }
        if let Some((p_idx, replacement)) = find_patch_for_module(m, &opts.patch_modules) {
            seen_mod_patches[p_idx] = true;
            m.payload = replacement.to_vec();
            mod_flags[idx] = true;
            modules_modified.push(m.name.trim().to_string());
        }
        if is_main && !opts.patch_partitions.is_empty() {
            m.payload = reassemble_main_partitions(&m.payload, &opts.patch_partitions)
                .with_context(|| format!("Patching MAIN partitions inside '{}'", m.name))?;
            mod_flags[idx] = true;
            if !modules_modified.iter().any(|x| x == m.name.trim()) {
                modules_modified.push(m.name.trim().to_string());
            }
            for (part_name, _) in &opts.patch_partitions {
                if !partitions_patched.contains(part_name) {
                    partitions_patched.push(part_name.clone());
                }
            }
        }
    }
    Ok(mod_flags)
}

/// Unpacks, patches, repacks, and optionally roundtrip-verifies any Legacy or Blu-ray `.FRM` image.
pub fn repack_universal_frm(
    raw: &[u8],
    keyring: &FrmKeyring,
    opts: &FrmRepackOptions,
) -> Result<(Vec<u8>, UniversalFrmRepackReport)> {
    let mut container: UniversalFrmContainer = unpack_universal_frm(raw, keyring)?;
    let source_format = container.format;
    let (output_format, legacy_mode) = resolve_output_formats(source_format, opts);

    let mut seen_mod_patches = vec![false; opts.patch_modules.len()];
    let mut modules_modified = Vec::new();
    let mut partitions_patched = Vec::new();
    let mut sub_buffers = Vec::with_capacity(container.sub_containers.len());

    for sub in &mut container.sub_containers {
        let mod_flags = mutate_subcontainer(
            sub,
            opts,
            &mut seen_mod_patches,
            &mut modules_modified,
            &mut partitions_patched,
        )?;
        let encoded = if let Some(lm) = legacy_mode {
            pack_legacy_subcontainer(sub, lm, opts, &mod_flags)?
        } else {
            pack_bluray_subcontainer(sub, opts, &mod_flags)?
        };
        sub_buffers.push(encoded);
    }

    for (idx, seen) in seen_mod_patches.into_iter().enumerate() {
        if !seen {
            bail!(
                "Requested --patch-module '{}' not found in container",
                opts.patch_modules[idx].0
            );
        }
    }

    let final_bytes = if output_format == UniversalFrmFormat::BlurayAes {
        let pair = container
            .aes_key_pair
            .as_ref()
            .or_else(|| keyring.aes_pairs.first())
            .context("No AES key pair available for BlurayAes repack")?;
        wrap_aes_frm(&sub_buffers, pair)?
    } else if sub_buffers.len() == 1 {
        sub_buffers.into_iter().next().unwrap_or_default()
    } else {
        bail!(
            "Multi-subfile ({}) source requires BlurayAes output or single-subfile input",
            sub_buffers.len()
        );
    };

    if opts.verify {
        let checked = unpack_universal_frm(&final_bytes, keyring)
            .context("Post-repack roundtrip verification failed")?;
        for sub in &checked.sub_containers {
            for m in &sub.modules {
                if !m.toc_sum_valid {
                    bail!(
                        "Post-repack TOC checksum invalid on module '{}' (sub {})",
                        m.name,
                        sub.sub_index
                    );
                }
                if !m.data_sum_valid && legacy_mode.is_none() {
                    bail!(
                        "Post-repack Adler-32 data checksum invalid on module '{}' (sub {})",
                        m.name,
                        sub.sub_index
                    );
                }
            }
        }
    }

    let report = UniversalFrmRepackReport {
        source_format,
        output_format,
        output_bytes: final_bytes.len(),
        sub_containers: container.sub_containers.len(),
        modules_modified,
        partitions_patched,
        verified: opts.verify,
    };
    Ok((final_bytes, report))
}
