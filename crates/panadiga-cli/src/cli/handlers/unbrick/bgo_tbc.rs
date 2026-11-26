#![deny(unsafe_code)]

//! Handlers for SD hardening/verification, TBC appliance planning, and BGO NOR audit/repair.

use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use panadiga_core::constants::{DEFAULT_BAUD_RATE, SD_STATUS_INVALID};
use panadiga_core::pico::panamon1::{Panamon1Client, Panamon1Transport, SerialPanamon1Transport};
use panadiga_core::re::eh55_bgo::{
    BgoLiveHealRequest, BgoRepairPlan, apply_bgo_repair_plan_live, build_bgo_repair_plan,
    heal_bgo_live_with_client,
};
use panadiga_core::re::region_compat::{
    LbsRegionPresetId, TvStandardKind, inspect_tv_calibration_bundle,
};
use panadiga_core::re::tbc_appliance::{TbcApplianceMode, build_tbc_appliance_plan};
use panadiga_core::re::unbrick_sd_kit::{SdKitAuditProfile, verify_sd_unbrick_media_with_profile};
use panadiga_core::re::{
    Prog061BgoPatchMode, RebuildFromDumpRequest, SdLoaderProfile, SdMediaVerificationReport,
    SdUnbrickKitRequest, audit_eh55_bgo_nor_8mb, prepare_sd_unbrick_kit_with_tier_policy,
    rebuild_sd_kit_from_dump,
};

use super::super::json_io::write_cli_output_bytes;
use super::render::{
    emit_json_if_requested, format_sd_kit_lines, format_sd_verification_lines, print_lines,
};
use crate::cli::args::unbrick::{
    BgoAuditArgs, BgoLiveApplyArgs, BgoLiveHealArgs, BgoRepairPlanArgs, PrepareSdArgs,
    RebuildFromDumpArgs, TbcPlanArgs, VerifySdArgs,
};

pub(crate) fn parse_lbs_preset(raw: &str) -> Result<LbsRegionPresetId> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "br-pal-m" => Ok(LbsRegionPresetId::BrPalMR4),
        "us-ntsc" => Ok(LbsRegionPresetId::UsNtscR1),
        "eu-pal" => Ok(LbsRegionPresetId::EuPalR2),
        other => other.parse(),
    }
}

pub(crate) fn verify_sd_dir_with_profile(
    dir: &Path,
    profile: SdKitAuditProfile,
) -> SdMediaVerificationReport {
    verify_sd_unbrick_media_with_profile(dir, profile)
}

pub(super) fn handle_prepare_sd(sd: PrepareSdArgs) -> Result<()> {
    let lbs_preset = sd.lbs_preset.as_deref().map(parse_lbs_preset).transpose()?;
    let tv_std = sd
        .tv_standard
        .as_deref()
        .map(str::parse::<TvStandardKind>)
        .transpose()?;

    let req = if sd.hardened_sd {
        let mut hardened = SdUnbrickKitRequest::hardened_panamon1(sd.outdir);
        hardened.pana_dvd_path = sd.frm;
        hardened.prog061_path = sd.prog061;
        hardened.main_path = sd.main;
        hardened.drv_path = sd.drv;
        hardened.main_passthrough_mode = sd.passthrough_mode.parse()?;
        hardened.loader_profile = SdLoaderProfile::Ver061RawSd48;
        if sd.bgo_mode != "bypass" {
            hardened.bgo_mode = sd.bgo_mode.parse()?;
        }
        hardened.include_pana_sd = !sd.diag_only;
        hardened.unlock_hdd_dynamic = !sd.no_hdd_dynamic && sd.target_gb.is_none();
        hardened.hdd_target_gb = sd.target_gb;
        hardened.write_extracted_sources = sd.save_sources;
        hardened.allow_unsafe_auto_format = sd.allow_unsafe_bgo_format;
        if sd.expected_main_build.is_some() {
            hardened.expected_main_build = sd.expected_main_build;
        }
        hardened.allow_target_mismatch = sd.allow_target_mismatch;
        hardened.sata_bridge_reset_fix = sd.sata_bridge_reset_fix;
        hardened.sata_hdd_autoboot = sd.sata_hdd_autoboot;
        hardened.main_bgo_retarget = sd.main_bgo_retarget;
        hardened.nor_autoboot_capsule = sd.nor_autoboot_capsule;
        hardened.m6930lb_localization = sd.m6930lb_localization;
        hardened.localization_tv_profile = sd.tv_standard.clone();
        hardened
    } else {
        let bgo_mode: Prog061BgoPatchMode = sd.bgo_mode.parse()?;
        SdUnbrickKitRequest {
            output_dir: sd.outdir,
            pana_dvd_path: sd.frm,
            prog061_path: sd.prog061,
            main_path: sd.main,
            drv_path: sd.drv,
            bgo_mode,
            main_passthrough_mode: sd.passthrough_mode.parse()?,
            loader_profile: sd.loader_profile.parse()?,
            include_pana_sd: !sd.diag_only,
            unlock_hdd_dynamic: !sd.no_hdd_dynamic && sd.target_gb.is_none(),
            hdd_target_gb: sd.target_gb,
            write_extracted_sources: sd.save_sources,
            allow_unsafe_auto_format: sd.allow_unsafe_bgo_format,
            expected_main_build: sd.expected_main_build,
            allow_target_mismatch: sd.allow_target_mismatch,
            sata_bridge_reset_fix: sd.sata_bridge_reset_fix,
            sata_hdd_autoboot: sd.sata_hdd_autoboot,
            main_bgo_retarget: sd.main_bgo_retarget,
            nor_autoboot_capsule: sd.nor_autoboot_capsule,
            m6930lb_localization: sd.m6930lb_localization,
            localization_tv_profile: sd.tv_standard.clone(),
        }
    };

    let report = prepare_sd_unbrick_kit_with_tier_policy(&req, sd.allow_unverified_profile)?;
    print_lines(&format_sd_kit_lines(&report));

    if sd.hardened_sd {
        let strict = verify_sd_dir_with_profile(
            Path::new(&report.output_dir),
            SdKitAuditProfile::HardenedRecovery,
        );
        if strict.factory_unpatched_prog061_detected || strict.overall_status == SD_STATUS_INVALID {
            bail!("Hardened SD verification failed: {}", strict.overall_status);
        }
        println!("[HARDENED] Dual-family ver0.61 + PANAMON1 parity verified.");
    }

    if lbs_preset.is_some() || tv_std.is_some() {
        let preset = lbs_preset.unwrap_or(LbsRegionPresetId::CustomInspected);
        let main_bytes = report
            .main_source
            .as_deref()
            .filter(|p| Path::new(p).is_file())
            .map(fs::read)
            .transpose()?;
        let frm_bytes = report.pana_sd_path.as_deref().map(fs::read).transpose()?;
        let tv_rep = inspect_tv_calibration_bundle(
            preset,
            tv_std,
            None,
            main_bytes.as_deref(),
            frm_bytes.as_deref(),
            None,
        )?;
        println!(
            "[TV/LBS] preset={} std={} chassis={} mmio_ops={} warnings={}",
            tv_rep.profile.preset.as_str(),
            tv_rep.profile.tv_standard.as_str(),
            tv_rep.profile.chassis_family,
            tv_rep.profile.mmio_plan.len(),
            tv_rep.warnings.len()
        );
    }

    emit_json_if_requested(&report, sd.json.as_deref())
}

pub(super) fn handle_verify_sd(args: VerifySdArgs) -> Result<()> {
    let profile: SdKitAuditProfile = args.audit_profile.parse()?;
    let report = verify_sd_dir_with_profile(&args.dir, profile);
    print_lines(&format_sd_verification_lines(&report));
    if let Some(ref parity) = report.dual_family_parity {
        println!(
            "Dual-family sd48 parity: M6930LB={}, M6930={}, parity_ok={}",
            parity.m6930lb_main.is_some(),
            parity.m6930_main.is_some(),
            parity.dual_parity_ok
        );
    }
    emit_json_if_requested(&report, args.json.as_deref())?;
    if report.overall_status == SD_STATUS_INVALID {
        bail!("SD media validation failed; inspect missing or corrupt artifacts above");
    }
    Ok(())
}

pub(super) fn handle_tbc_plan(args: TbcPlanArgs) -> Result<()> {
    let mode: TbcApplianceMode = args.mode.parse()?;
    let std_kind: TvStandardKind = args.standard.parse()?;
    let plan = build_tbc_appliance_plan(mode, std_kind, args.experimental_ram_chainload)?;
    println!(
        "TBC Plan: mode={} std={} sa0_sa3_untouched={} mmio_ops={}",
        plan.mode.as_str(),
        plan.tv_standard.as_str(),
        plan.nor_sa0_to_sa3_untouched,
        plan.mmio_ops.len()
    );
    for w in &plan.warnings {
        println!("  [WARN] {w}");
    }
    for s in &plan.steps {
        println!("  - {s}");
    }
    emit_json_if_requested(&plan, args.json.as_deref())
}

pub(super) fn handle_bgo_audit(args: BgoAuditArgs) -> Result<()> {
    let nor = fs::read(&args.nor).with_context(|| format!("Reading {}", args.nor.display()))?;
    let report = audit_eh55_bgo_nor_8mb(&nor)?;
    println!(
        "BGO Audit: healthy={} active_p1={} missing_p2={:?} spares={:?} corrupt={:?} ifodev2={:?} nfrom_files={}",
        report.healthy,
        report.p1_active_count,
        report.p2_missing_ids,
        report.spare_sas,
        report.corrupt_sas,
        report.ifodev2_model,
        report.nfrom_file_count
    );
    emit_json_if_requested(&report, args.json.as_deref())
}

pub(super) fn handle_bgo_repair_plan(args: BgoRepairPlanArgs) -> Result<()> {
    let nor = fs::read(&args.nor).with_context(|| format!("Reading {}", args.nor.display()))?;
    let plan = build_bgo_repair_plan(&nor, None)?;
    let json_bytes = serde_json::to_vec_pretty(&plan)?;
    write_cli_output_bytes(&args.out_plan, &json_bytes)?;
    if let Some(ref cow_path) = args.cow_out {
        let img = plan
            .repaired_nor_image
            .as_deref()
            .context("BgoRepairPlan missing repaired_nor_image")?;
        write_cli_output_bytes(cow_path, img)?;
    }
    println!(
        "BGO Repair Plan: missing_p2={:?} ops={} erased={:?} write_bytes={} post_healthy={}",
        plan.missing_f2_ids,
        plan.ops.len(),
        plan.erased_sas,
        plan.total_write_bytes,
        plan.post_audit.healthy
    );
    emit_json_if_requested(&plan, args.json.as_deref())
}

pub fn handle_bgo_live_apply_with_transport<T: Panamon1Transport>(
    transport: T,
    args: BgoLiveApplyArgs,
) -> Result<()> {
    let raw_plan =
        fs::read(&args.plan).with_context(|| format!("Reading {}", args.plan.display()))?;
    let plan: BgoRepairPlan = serde_json::from_slice(&raw_plan)
        .with_context(|| format!("Parsing BgoRepairPlan JSON {}", args.plan.display()))?;
    let mut client = Panamon1Client::new(transport);
    let trace_ms = (!args.no_boottrace).then_some(15_000);
    let report = apply_bgo_repair_plan_live(
        &mut client,
        &plan,
        &args.backup_dir,
        args.confirm_upper_nor,
        trace_ms,
        &mut |p| {
            println!(
                "[BGO {}/{}] {:?}: {}",
                p.current, p.total, p.stage, p.detail
            )
        },
    )?;
    println!(
        "BGO Live Apply: erased={} written={} B backup={} post_sha={}",
        report.erased_sectors, report.written_bytes, report.backup_path, report.post_bgo_sha256
    );
    emit_json_if_requested(&report, args.json.as_deref())
}

pub(super) fn handle_rebuild_from_dump(args: RebuildFromDumpArgs) -> Result<()> {
    let req = RebuildFromDumpRequest {
        model_id: args.model,
        nor_dump_path: args.nor_dump,
        hdd_main_path: args.hdd_main,
        frm_path: args.frm,
        prog061_path: args.prog061,
        target_gb: args.target_gb,
        out_dir: args.out_dir,
        allow_unverified_mix: args.allow_unverified_mix,
        allow_unverified_profile: args.allow_unverified_profile,
    };
    let report = rebuild_sd_kit_from_dump(&req)?;
    print_lines(&format_sd_kit_lines(&report.kit));
    print_lines(&format_sd_verification_lines(&report.verification));
    emit_json_if_requested(&report, args.json.as_deref())
}

pub(super) fn handle_bgo_live_apply(args: BgoLiveApplyArgs) -> Result<()> {
    let transport =
        SerialPanamon1Transport::open(&args.port, DEFAULT_BAUD_RATE, Duration::from_millis(1_500))?;
    handle_bgo_live_apply_with_transport(transport, args)
}

pub fn handle_bgo_live_heal_with_transport<T: Panamon1Transport>(
    transport: Option<T>,
    args: BgoLiveHealArgs,
) -> Result<()> {
    let nor_bytes =
        fs::read(&args.nor).with_context(|| format!("Reading {}", args.nor.display()))?;
    let tv_standard = args
        .tv_standard
        .as_deref()
        .map(str::parse::<TvStandardKind>)
        .transpose()?;
    let req = BgoLiveHealRequest {
        confirm_upper_nor: args.confirm_upper_nor,
        tv_standard,
        dry_run: args.dry_run,
    };
    let mut client = transport.map(Panamon1Client::new);
    let report = heal_bgo_live_with_client(
        client.as_mut(),
        &nor_bytes,
        &args.backup_dir,
        &req,
        &mut |p| {
            println!(
                "[BGO HEAL {}/{}] {:?}: {}",
                p.current, p.total, p.stage, p.detail
            )
        },
    )?;
    println!(
        "BGO Live Heal: dry_run={} erased={:?} write={} B post_healthy={} commands={}",
        report.dry_run,
        report.erased_sas,
        report.total_write_bytes,
        report.post_healthy,
        report.command_plan.commands.len()
    );
    emit_json_if_requested(&report, args.json.as_deref())
}

pub(super) fn handle_bgo_live_heal(args: BgoLiveHealArgs) -> Result<()> {
    if args.dry_run {
        handle_bgo_live_heal_with_transport::<SerialPanamon1Transport>(None, args)
    } else {
        let port = args
            .port
            .as_deref()
            .context("--port <PORT> is required unless --dry-run is specified")?;
        let transport =
            SerialPanamon1Transport::open(port, DEFAULT_BAUD_RATE, Duration::from_millis(1_500))?;
        handle_bgo_live_heal_with_transport(Some(transport), args)
    }
}
