//! Reverse Engineering, MN103S Disassembly, UART NOR Dumper & BGO Unbrick Primitives

pub mod bench_runtime;
pub mod bgo_unbrick;
pub mod davis_probe;
pub mod eh55_bgo;
/// Domain-neutral alias for the Panasonic DIGA BGO flash-translation-layer (FTL) auditor and repair planner.
pub use eh55_bgo as bgo_ftl;
pub mod emu_patch_suite;
pub mod error_dict;
pub mod main_bgo_retarget;
pub mod main_passthrough;
pub mod matshita_emu;
pub mod mn10300;
pub mod mn10300_emit;
pub mod mn10300_emu;
pub mod mn10300_traps;
pub mod nor_dumper;
pub mod patch_spec;
pub mod prewrite_gate;
pub mod prog061_uart_monitor;
pub mod region_compat;
pub mod sata_bridge_fix;
pub mod sata_hdd_autoboot;
pub mod sig_locator;
pub mod ssot_patches;
pub mod symbol_index;
pub mod tbc_appliance;
pub mod unbrick_sd_kit;
pub mod unbrick_workbench;
pub mod verify_loop;

pub use bench_runtime::{
    BenchRuntimePlan, BenchRuntimeStageDescriptor, build_dynamic_bench_runtime_plan,
    build_dynamic_bench_runtime_plan_with_keyring,
};
pub use bgo_unbrick::{
    BASE_FACTORY_BGO_GEOMETRY_ID, DetectedBgoGeometry, OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK,
    OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ,
    ORIG_SUM_BEQ, PATCH_A2BC_BRA_MONITOR, PROG061_BASE_VA, PROG061_EXPECTED_SIZE,
    PROG061_NFROM_FORMAT_VA, Prog061BgoInspection, Prog061BgoPatchMode, Prog061BgoPatchReport,
    Prog061BgoSiteStatus, decode_prog061_payload, detect_prog_bgo_geometry,
    enforce_train_evidence_tier, find_recipe_by_loader_key, find_retarget_spec_by_geometry_id,
    find_retarget_spec_by_loader_key, find_train_by_geometry_id, find_train_by_loader_key,
    inspect_prog_bgo_geometry_with_spec, inspect_prog061_bgo_sites,
    inspect_prog061_container_bytes, load_retarget_drv_for_spec, load_train_retarget_spec,
    load_train_retarget_spec_and_drv, patch_prog061_bgo_in_place,
    patch_prog061_bgo_in_place_with_policy, patch_prog061_or_build_digadiag,
    patch_prog061_or_build_digadiag_with_policy, prog_has_bgo_table_marker,
    retarget_prog_for_geometry_in_place, retarget_prog_for_loader_profile_in_place,
    retarget_prog_for_loader_profile_with_policy, retarget_prog_from_spec_in_place,
    validate_bgo_allowlist_invariant,
};
pub use eh55_bgo::{
    BgoParseError, BgoSectorGeometry, EH55_BGO_EXPECTED_P2_COUNT, EH55_BGO_FIRST_SA,
    EH55_BGO_LAST_SA, EH55_BGO_MAX_PID, EH55_BGO_SECTOR_COUNT, Eh55BgoAuditReport,
    Eh55BgoSectorKind, Eh55BgoSectorReport, NOR_AUTOBOOT_CAPSULE_HEADER_BYTES,
    NOR_AUTOBOOT_CAPSULE_MAGIC_LE, NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET, NorAutobootInstallReport,
    audit_bgo_nor_with_geometry, audit_eh55_bgo_nor_8mb, build_nor_autoboot_capsule,
    build_nor_autoboot_cow_image, eh55_bgo_sector_offset, eval_bgo_migration_index,
    eval_bgo_migration_target, is_active_bgo_p1_sector, parse_bgo_l2p_with_geometry,
    parse_eh55_bgo_l2p, unpack_nor_autoboot_capsule, verify_nor_one_to_zero_compatible,
};
pub use emu_patch_suite::{
    NorEmuPatchSuiteReport, PatchVerificationCheck, run_nor_emu_patch_verification_suite,
};
pub use main_bgo_retarget::{
    BytePatchSite, MAIN_BGO_STATE_8MB, MAIN_BGO_STATE_FACTORY_4MB, MAIN_BGO_STATE_MISMATCH,
    MainBgoGeometryStatus, MainBgoRetargetSpec, OwnedMainBgoRetargetSpec, OwnedSlicePatchSite,
    U32RelocSpec, WordPatchSite, apply_main_bgo_retarget_for_spec_in_place,
    apply_main_bgo_retarget_in_place, build_main_bgo_retarget_spec_from_ssot,
    inspect_main_bgo_geometry, inspect_main_bgo_geometry_for_spec,
    inspect_or_verify_main_bgo_geometry,
};
pub use main_passthrough::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_HANDSHAKE_BYTES, DEV_SERIAL_ISR_CTX_PTR_VA,
    DEV_SERIAL_PC_OFFSET, DEV_SERIAL_STATE_BASE_VA, EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA,
    MainFirmwareIdentity, MainPassthroughMode, MainPassthroughReport, MainPassthroughRspPlan,
    MainPassthroughSiteReport, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
    ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH, PATCH_FPC_STATUS_CTRL,
    PATCH_ODD_INIT_STATUS, PATCH_ODD_STATE_BRANCH, apply_main_passthrough_in_place,
    build_passthrough_rsp_plan, format_rsp_frame, format_rsp_write_mem_frame,
    format_rsp_write_mem_payload, format_rsp_write_u32_frame, inspect_main_firmware_identity,
    inspect_main_passthrough_sites, passthrough_rsp_payloads, patch_main_passthrough_file,
    rsp_checksum,
};
pub use mn10300::{
    CallFrameRef, HardwareLoopRef, ImmediateLoadRef, InstructionRef, condition_suffix,
    decode_bcc_refs, decode_branch_call_refs, decode_call_frames, decode_regmask_names,
    decode_setlb_loops, find_callers_to, find_fc_immediate_loads, find_pointer_xrefs,
};
pub use mn10300_emu::{CfiStage, Mn103Fault, Mn103Vm};
pub use nor_dumper::{
    DEFAULT_NOR_STUB_LOAD_ADDR, DIGADIAG_NOR_STUB_LOAD_ADDR, NOR_ALLOWED_WINDOWS, NOR_MAGIC_FOOTER,
    NOR_MAGIC_HEADER, NorDumpResult, build_nor_dumper_artifact, build_readonly_nor_dumper_stub,
    decode_nor_dump_stream, resolve_nor_stub_load_addr, validate_nor_dump_range,
};
pub use prewrite_gate::{
    PreWriteCheckItem, PreWriteGateReport, PreWriteTarget, require_prewrite_gate,
    verify_prewrite_gate,
};
pub use prog061_uart_monitor::{
    OFF_UART_MONITOR_SLACK, PATCH_A2CD_UART_MONITOR, PROG061_UART_MONITOR_VA,
    UART_MONITOR_MAX_SLACK_BYTES, build_prog061_uart_monitor_payload,
};
pub use region_compat::{
    DevBinReport, EXPECTED_MAIN_3060NP_SIZE, FrmRetargetReport, LocalizationPatchMode,
    MainLocalizationReport, MainLocalizationSiteReport, MainRegionCompatReport,
    OFF_FMU_DEVINFO_3060NP, OFF_FMU_DEVINFO_3410PQ, ORIG_FMU_DEVINFO_3060NP,
    ORIG_FMU_DEVINFO_3410PQ, PATCH_FMU_DEVINFO_3060NP, PATCH_FMU_DEVINFO_3410PQ, RegionCompatMode,
    decode_market_code_name, inspect_dev_bin_bytes, inspect_main_localization,
    inspect_main_region_compat, patch_dev_bin_bytes, patch_main_localization_in_place,
    patch_main_region_compat_in_place, retarget_frm_toc_bytes,
};
pub use sata_bridge_fix::{
    OFF_MAIN3060NP_SATA_BRIDGE_RESET_WAIT, OFF_MAIN3410PQ_SATA_BRIDGE_RESET_WAIT,
    OFF_PROG061_SATA_BRIDGE_RESET_WAIT, ORIG_MAIN3060NP_SATA_BRIDGE_RESET_WAIT,
    ORIG_MAIN3410PQ_SATA_BRIDGE_RESET_WAIT, ORIG_PROG061_SATA_BRIDGE_RESET_WAIT,
    PATCH_MAIN3060NP_SATA_BRIDGE_RESET_WAIT, PATCH_MAIN3410PQ_SATA_BRIDGE_RESET_WAIT,
    PATCH_PROG061_SATA_BRIDGE_RESET_WAIT, SATA_BRIDGE_STATE_FACTORY_2MS,
    SATA_BRIDGE_STATE_MISMATCH, SATA_BRIDGE_STATE_WAIT_1000MS, SataBridgeResetFixMode,
    SataBridgeResetSiteStatus, apply_main_sata_bridge_reset_fix_in_place,
    apply_main3060np_sata_bridge_reset_fix_in_place,
    apply_main3410pq_sata_bridge_reset_fix_in_place,
    apply_prog_sata_bridge_reset_fix_for_profile_in_place,
    apply_prog061_sata_bridge_reset_fix_in_place, inspect_main_sata_bridge_reset_site,
    inspect_main3060np_sata_bridge_reset_site, inspect_main3410pq_sata_bridge_reset_site,
    inspect_prog_sata_bridge_reset_site_for_profile, inspect_prog061_sata_bridge_reset_site,
};
pub use sata_hdd_autoboot::{
    HddAutobootTrainSpec, HddAutobootTrainSpec as HddAutobootSpec,
    OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE, ORIG_PROG061_HDD_AUTOBOOT_HOOK_BYTES,
    PROG061_HDD_AUTOBOOT_EXPECTED_MAIN_CSUM_OFFSET, PROG061_HDD_AUTOBOOT_PAYLOAD_OFFSET,
    PROG061_HDD_AUTOBOOT_PAYLOAD_VA, PROG061_HDD_AUTOBOOT_STATE_ENABLED,
    PROG061_HDD_AUTOBOOT_STATE_FACTORY, PROG061_HDD_AUTOBOOT_STATE_MISMATCH,
    Prog061HddAutobootStatus, apply_hdd_autoboot_for_loader_profile_in_place,
    apply_hdd_autoboot_in_place_for_spec, apply_prog061_hdd_autoboot_in_place,
    assemble_hdd_autoboot_payload_for_spec, assemble_prog061_hdd_autoboot_payload,
    compute_hdd_autoboot_hook_bytes, hdd_autoboot_spec_for_loader_profile,
    inspect_hdd_autoboot_for_loader_profile, inspect_hdd_autoboot_site_for_spec,
    inspect_prog061_hdd_autoboot_site, stamp_hdd_autoboot_expected_main_checksum_for_profile,
    stamp_hdd_autoboot_expected_main_checksum_for_spec,
};
pub use sig_locator::{
    CfgFunctionCandidate, LocatedFunctionMatch, StringXrefHit, detect_prologue_at,
    discover_cfg_functions, locate_functions_by_string, summarize_function_cfg,
};
pub use ssot_patches::{
    find_main_build_by_id, find_main_build_by_size, find_prog_sata_reset_spec,
    hdd_autoboot_spec_for_loader, verify_main_native_bgo_from_spec,
};
pub use symbol_index::{
    FirmwareSymbolIndex, FunctionSymbolEntry, SymbolAbiDetail, SymbolAnnotationOverlay,
    SymbolLookupMatch, SymbolLookupQuery, SymbolLookupResponse, SymbolLookupSafetyGate,
    SymbolSideEffect, build_symbol_index, list_symbol_indexes, load_all_destructive_symbols,
    load_symbol_index, lookup_symbols, write_symbol_index,
};
pub use tbc_appliance::{
    TbcApplianceMode, TbcAppliancePlan, VerifiedMainPassthroughSiteRef,
    build_ic7501_vfd_text_frame, build_ic7501_wdt_255s_frame, build_tbc_appliance_plan,
    compute_ic7501_tbus_checksum, emit_m0_tbus_wdt_keepalive_stub,
};
pub use unbrick_sd_kit::{
    CLEANROOM_DIGADIAG_MAGIC, CleanroomDigadiagArtifact, DvdCreatorProgress, DvdCreatorReport,
    DvdCreatorRequest, DvdCreatorStage, Fat16PlacedFileReport, Fat16SdFileInput, Fat16SdGeometry,
    FatBpbReport, ISO9660_SECTOR_SIZE, Iso9660FileInput, Iso9660InspectionReport,
    Iso9660PlacedFileReport, MbrPartitionRecord, NorDumpIntegrityReport, OpticalBurnBackend,
    OpticalBurnerDriveInfo, RebuildFromDumpReport, RebuildFromDumpRequest, RootDirFileReport,
    SdCardAdvisory, SdCardCreatorPlan, SdCardCreatorReport, SdCardCreatorRequest,
    SdCreatorProgress, SdCreatorStage, SdDiskInspectionReport, SdLoaderProfile,
    SdMediaVerificationReport, SdTargetDriveInfo, SdUnbrickCheck, SdUnbrickKitReport,
    SdUnbrickKitRequest, SynthesizedIsoImage, SynthesizedSdImage, SystemOpticalBurnBackend,
    assess_sd_card_size_advisory, build_cleanroom_digadiag, build_layperson_sd_steps,
    compute_fat16_sd_geometry, create_dvd_recovery_disc, create_dvd_recovery_disc_with_backend,
    create_dvd_recovery_iso, create_sd_recovery_card_on_device, create_sd_recovery_image,
    expected_optical_confirm_token, flash_sd_recovery_card, inspect_iso9660_image,
    inspect_nor_dump_integrity, inspect_optical_burners, inspect_sd_card_targets,
    inspect_sd_disk_bytes, inspect_sd_reader, inspect_sd_target, plan_sd_recovery_card,
    prepare_sd_unbrick_kit, prepare_sd_unbrick_kit_with_tier_policy, rebuild_sd_kit_from_dump,
    scan_loader_banner, synthesize_fat16_sd_image, synthesize_iso9660_level1_image,
    verify_sd_buffers, verify_sd_unbrick_media, verify_sd_unbrick_media_with_profile,
};
pub use unbrick_workbench::{
    ExtractedFrmModuleReport, FrmModuleExtractionReport, UnbrickHardwareFact,
    UnbrickWorkspaceReport, extract_modules_from_frm, inspect_unbrick_workspace,
};
pub use verify_loop::{VerifyExpectation, VerifyLoopReport, VerifySegment, run_verify_loop};

/// Domain-neutral aliases over [`eh55_bgo`] and model-aware BGO FTL entrypoints.
pub use eh55_bgo::{
    BgoApplyStage, BgoLiveApplyProgress, BgoLiveApplyReport, BgoLiveHealCommandPlan,
    BgoLiveHealReport, BgoLiveHealRequest, BgoRepairOp, BgoRepairPlan, BgoTvStandardNote,
    apply_bgo_repair_plan_live, audit_bgo_nor_auto, audit_bgo_nor_for_model, build_bgo_repair_plan,
    heal_bgo_live_with_client, plan_live_bgo_heal_commands, validate_and_simulate_bgo_plan,
};
pub type BgoFtlAuditReport = Eh55BgoAuditReport;
pub type BgoFtlSectorKind = Eh55BgoSectorKind;
pub type BgoFtlSectorReport = Eh55BgoSectorReport;
pub type BgoFtlRepairPlan = BgoRepairPlan;
pub type BgoFtlLiveApplyReport = BgoLiveApplyReport;
