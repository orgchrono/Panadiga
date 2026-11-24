#![deny(unsafe_code)]

//! Handlers for `panadiga profiles` (`list`, `show`, `validate`, `authorize`, `export`, `export-schema`, `promote`, `completeness`, `discover-nor`).

mod discovery;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result, bail};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::log_println;
use panadiga_core::ssot::{
    Capability, ClaimAxis, EvidenceTier, HardwareProfileRegistry, PromoteClaimRequest,
    RuntimeFingerprint, export_schemas_to_dir, promote_model_claim,
};

use self::discovery::{build_completeness_report, discover_nor_dump};
use super::json_io::emit_json_output;
use super::nvram::build_lab_override;
use crate::cli::args::{ProfilesActions, ProfilesArgs};

fn open_registry(overlay_dir: Option<&std::path::Path>) -> Result<HardwareProfileRegistry> {
    HardwareProfileRegistry::load_with_overlay(overlay_dir)
}

fn parse_capability(raw: &str) -> Result<Capability> {
    serde_json::from_value(serde_json::Value::String(raw.trim().to_ascii_lowercase()))
        .with_context(|| format!("Unknown capability '{raw}'"))
}

fn parse_claim_axis(raw: &str) -> Result<ClaimAxis> {
    serde_json::from_value(serde_json::Value::String(raw.trim().to_ascii_lowercase()))
        .with_context(|| format!("Unknown claim axis '{raw}'"))
}

fn parse_evidence_tier(raw: &str) -> Result<EvidenceTier> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "T0" => Ok(EvidenceTier::CommunityUnverified),
        "T1" => Ok(EvidenceTier::ServiceManualOnly),
        "T2" => Ok(EvidenceTier::OfficialFrmVerified),
        "T3" => Ok(EvidenceTier::PhysicalDumpVerified),
        "T4" => Ok(EvidenceTier::BenchVerified),
        _ => serde_json::from_value(serde_json::Value::String(raw.trim().to_ascii_lowercase()))
            .with_context(|| format!("Unknown evidence tier '{raw}'")),
    }
}

fn parse_jedec_cycles(raw: Option<&str>) -> Result<Vec<u32>> {
    let Some(csv) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };
    csv.split(',').map(|p| parse_u32_radix(p.trim())).collect()
}

/// Dispatches `panadiga profiles` subcommands.
pub fn handle_profiles(args: ProfilesArgs) -> Result<()> {
    match args.action {
        ProfilesActions::List { overlay_dir, json } => {
            let reg = open_registry(overlay_dir.as_deref())?;
            let list = reg.list_models();
            for p in &list {
                let soc_lbl = p
                    .soc
                    .as_ref()
                    .map(|s| s.part_number.as_str())
                    .or_else(|| p.soc_core.as_ref().map(|c| c.isa.as_str()))
                    .unwrap_or("UNKNOWN");
                let flash_lbl = p.flash_chip.as_ref().map_or("UNKNOWN".to_string(), |f| {
                    format!("{} ({:#X} B)", f.part_number, f.total_bytes)
                });
                let nvram_lbl = p.nvram_uncached_va_hex.as_deref().unwrap_or("NONE/UNKNOWN");
                let media = p
                    .media_formats
                    .iter()
                    .map(|m| m.id.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                log_println!(
                    "{:<12} | {:<18} | {:<28} | NVRAM={} | Media={}",
                    p.manifest.model_id,
                    soc_lbl,
                    flash_lbl,
                    nvram_lbl,
                    media
                );
            }
            emit_json_output(&list, json.as_deref())?;
        }
        ProfilesActions::Show {
            model,
            overlay_dir,
            json,
        } => {
            let reg = open_registry(overlay_dir.as_deref())?;
            let r = reg.resolve_model(&model)?;
            log_println!(
                "{} -> soc={} | core={} ({} mV) | flash={} ({}) | nvram_uncached={} | ipl_ro={} | caps={:?}",
                r.manifest.model_id,
                r.soc.as_ref().map_or("UNKNOWN", |s| s.part_number.as_str()),
                r.soc_core.as_ref().map_or("UNKNOWN", |c| c.isa.as_str()),
                r.soc_core.as_ref().map_or(0, |c| c.logic_voltage_mv),
                r.flash_chip
                    .as_ref()
                    .map_or("UNKNOWN", |f| f.part_number.as_str()),
                r.derived_sector_geometry.as_deref().unwrap_or("UNKNOWN"),
                r.nvram_uncached_va_hex.as_deref().unwrap_or("NONE/UNKNOWN"),
                r.ipl_protected_range_hex.as_deref().unwrap_or("UNKNOWN"),
                r.effective_capabilities
            );
            emit_json_output(&r, json.as_deref())?;
        }
        ProfilesActions::Validate { overlay_dir, json } => {
            let reg = open_registry(overlay_dir.as_deref())?;
            let report = reg.validate()?;
            log_println!(
                "[OK] Relational hardware profiles valid={} (models={}, soc_cores={}, socs={}, boards={}, flash={}, layouts={}, checks={})",
                report.valid,
                report.model_count,
                report.soc_core_count,
                report.soc_count,
                report.board_count,
                report.flash_chip_count,
                report.nor_layout_count,
                report.safety_checks_passed
            );
            emit_json_output(&report, json.as_deref())?;
            if !report.valid {
                bail!("Registry validation failed: {}", report.errors.join("; "));
            }
        }
        ProfilesActions::Authorize {
            model,
            capability,
            overlay_dir,
            dump_size,
            jedec_mfg,
            jedec_dev,
            wp_deasserted,
            has_eemul_f1,
            frm_sha256,
            loader_banner,
            loader_profile,
            lab_override,
            json,
        } => {
            let reg = open_registry(overlay_dir.as_deref())?;
            let cap = parse_capability(&capability)?;
            let ov = build_lab_override(&lab_override)?;
            let fp = RuntimeFingerprint {
                dump_size_bytes: dump_size,
                jedec_mfg_id: jedec_mfg.as_deref().map(parse_u32_radix).transpose()?,
                jedec_device_cycles: parse_jedec_cycles(jedec_dev.as_deref())?,
                wp_pin_deasserted: wp_deasserted,
                has_eemul_f1_marker: has_eemul_f1,
                frm_sha256,
                loader_banner,
                requested_loader_profile: loader_profile,
            };
            match reg.authorize(&model, cap, &fp, ov.as_ref())? {
                Ok(auth) => {
                    log_println!(
                        "[ALLOWED] model={} capability={:?} mode={:?} tier={}",
                        auth.model_id,
                        auth.capability,
                        auth.mode,
                        auth.effective_tier.short_tag()
                    );
                    emit_json_output(&auth, json.as_deref())?;
                }
                Err(denial) => {
                    emit_json_output(&denial, json.as_deref())?;
                    bail!(
                        "[DENIED] model={} capability={:?} reason={} (lab_override_eligible={})",
                        denial.model_id,
                        denial.capability,
                        denial.reason,
                        denial.lab_override_eligible
                    );
                }
            }
        }
        ProfilesActions::Export {
            format,
            out,
            overlay_dir,
        } => {
            let reg = open_registry(overlay_dir.as_deref())?;
            match format.trim().to_ascii_lowercase().as_str() {
                "bundle" => emit_json_output(reg.bundle(), Some(out.as_path()))?,
                "json" => emit_json_output(&reg.list_models(), Some(out.as_path()))?,
                other => bail!("Unsupported export format '{other}' (expected 'bundle' or 'json')"),
            }
        }
        ProfilesActions::ExportSchema { out_dir } => {
            let count = export_schemas_to_dir(&out_dir)?;
            log_println!(
                "[OK] Exported {} JSON Schema Draft 2020-12 files to {}",
                count,
                out_dir.display()
            );
        }
        ProfilesActions::Promote {
            model,
            axis,
            tier,
            source_kind,
            source_ref,
            sha256,
            fixture,
            bench_log,
            notes,
            profiles_dir,
            workspace_root,
        } => {
            let parsed_axis = parse_claim_axis(&axis)?;
            let parsed_tier = parse_evidence_tier(&tier)?;
            let inferred_log = if bench_log.is_none() && source_kind.as_deref() == Some("bench_log")
            {
                source_ref.as_deref()
            } else {
                bench_log.as_deref()
            };
            let fixture_rel = fixture
                .as_deref()
                .or(source_ref.as_deref())
                .context("promote requires --fixture <PATH> (or --source-ref <PATH>)")?;

            let req = PromoteClaimRequest {
                workspace_root: &workspace_root,
                profiles_dir: &profiles_dir,
                model_id: &model,
                axis: parsed_axis,
                new_tier: parsed_tier,
                evidence_sha256: &sha256,
                fixture_rel_path: fixture_rel,
                bench_log_rel_path: inferred_log,
                notes: notes.as_deref(),
            };
            let updated = promote_model_claim(&req)?;
            log_println!(
                "[OK] Promoted {} {:?} to {} (fixture={})",
                updated.model_id,
                parsed_axis,
                parsed_tier.short_tag(),
                updated.fixture_ref.as_deref().unwrap_or("-")
            );
        }
        ProfilesActions::Completeness {
            overlay_dir,
            min_tier,
            json,
        } => {
            let reg = open_registry(overlay_dir.as_deref())?;
            let min_parsed = min_tier.as_deref().map(parse_evidence_tier).transpose()?;
            let report = build_completeness_report(reg.bundle(), min_parsed);
            if json.as_deref() != Some(std::path::Path::new("-")) {
                log_println!(
                    "SSOT Completeness: {} total models ({} matching filter {:?}), tiers={:?}, incomplete={}",
                    report.total_models,
                    report.filtered_models_count,
                    report.min_tier_filter,
                    report.models_by_max_tier,
                    report.incomplete_models.len()
                );
            }
            emit_json_output(&report, json.as_deref())?;
        }
        ProfilesActions::DiscoverNor {
            input,
            model_id,
            out_dir,
            json,
        } => {
            let report = discover_nor_dump(&input, &model_id, out_dir.as_deref())?;
            if json.as_deref() != Some(std::path::Path::new("-")) {
                log_println!(
                    "NOR Intake ({}): {} B SHA={} | Loader={:?} | EXTRHEAD={} | BGOsectors={} | EEMUL_F1={:?}",
                    report.model_id,
                    report.size_bytes,
                    &report.sha256[..16],
                    report.loader_banner,
                    report.extrhead_entries.len(),
                    report.bgo_sector_count_64k,
                    report.eemul_f1_offsets_hex
                );
            }
            emit_json_output(&report, json.as_deref())?;
        }
        ProfilesActions::ReviewProposal {
            proposal,
            apply,
            profiles_dir,
            workspace_root,
            ledger,
            json,
        } => {
            let report = panadiga_core::ssot::review_manual_proposal(
                &workspace_root,
                &profiles_dir,
                &proposal,
                apply,
                ledger.as_deref(),
            )?;
            if json.as_deref() != Some(std::path::Path::new("-")) {
                log_println!(
                    "Proposal {} ({}): valid={} sha256_verified={} ledger_synced={} promoted={}",
                    report.proposal_id,
                    report.model_id,
                    report.valid,
                    report.sha256_verified,
                    report.ledger_synced,
                    report.promoted
                );
            }
            emit_json_output(&report, json.as_deref())?;
            if apply && !report.valid {
                anyhow::bail!(
                    "Refusing to apply invalid manual proposal '{}': {}",
                    report.proposal_id,
                    report.issues.join("; ")
                );
            }
        }
    }
    Ok(())
}
