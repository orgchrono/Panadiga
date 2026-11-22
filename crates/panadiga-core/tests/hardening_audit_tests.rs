#![deny(unsafe_code)]

//! P0/P1 Red-Team Audit Hardening & Rust<->TypeScript IPC JSON Contract Tests (R1..R6).

use panadiga_core::constants::{
    AM34_MAIN_BASE_VA, CHK_ID_BGO_ALLOWLIST_INVARIANT, CHK_ID_DIGADIAG_BYPASS_TRAP,
    CHK_ID_FVU_PANAMON_TRAP, CHK_ID_RTSD_LBA48_GATE, NOR_MIN_VALID_DUMP_BYTES,
    PANA_DIGADIAG_RAW_VER061_SIZE, PANA_EXTRHEAD_DEFAULT_ADLER_STEP, SD_STATUS_INVALID,
    SD_STATUS_READY_WITH_WARNINGS,
};
use panadiga_core::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, pack_extrhead, pack_frm,
};
use panadiga_core::hdd::capacity_patch::specs::HDD_CMD_CLAMP_WINDOW_TEMPLATE;
use panadiga_core::hdd::writer::{MountedPartitionInfo, PhysicalDriveInfo, StorageMediaRole};
use panadiga_core::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
use panadiga_core::re::bgo_unbrick::{
    OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
    ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
    Prog061BgoPatchMode, enforce_train_evidence_tier, find_train_by_loader_key,
    patch_prog061_bgo_in_place, retarget_prog_for_loader_profile_in_place,
    validate_bgo_allowlist_invariant,
};
use panadiga_core::re::unbrick_sd_kit::{
    SdKitAuditProfile, inspect_nor_dump_integrity, verify_sd_buffers,
    verify_sd_buffers_with_profile,
};
use panadiga_core::ssot::DigadiagTrustModel;

fn dual_sd48_module(name: &str, data: Vec<u8>) -> FrmPackModule {
    let mut m = FrmPackModule::new(name, data);
    m.hardware_family = "M6930LB".to_string();
    m.hardware_family_aliases = vec!["M6930".to_string()];
    m.model_prefix = b'M';
    m
}

fn synth_prog_fixture() -> Vec<u8> {
    let mut buf = vec![0xFFu8; PROG061_EXPECTED_SIZE];
    let patches: &[(usize, &[u8])] = &[
        (0, &[0xDC, 0x14, 0x00, 0x00]),
        (OFF_BGO_SUM_BEQ, &ORIG_SUM_BEQ),
        (OFF_DEVINFO_CHECK_BEQ, &ORIG_DEVINFO_BEQ),
        (OFF_BGO_INIT_CALL, &ORIG_A2BC_BGO_CALL),
        (OFF_BGO_INIT_FAIL_HOOK, &ORIG_A2CD_HOOK),
    ];
    for &(off, bytes) in patches {
        buf[off..off + bytes.len()].copy_from_slice(bytes);
    }
    buf
}

#[test]
fn positive_bgo_allowlist_accepts_eh55_and_rejects_overlap_spill_and_misaligned() {
    let train = find_train_by_loader_key("ver061").expect("ver061 train");
    let valid_spec = train.bgo_retarget_spec.as_ref().expect("ver061 spec");
    validate_bgo_allowlist_invariant(train, valid_spec).expect("valid ver061 BGO allowlist");

    // 1. Overlaps 256 KiB IPL region (0x8402_0000)
    let mut ipl_overlap = valid_spec.clone();
    ipl_overlap.detect_flash_base = 0x8402_0000;
    ipl_overlap.bgo_table_words[12] = 0x8402_0000;
    assert!(validate_bgo_allowlist_invariant(train, &ipl_overlap).is_err());

    // 2. Overlaps DRV / EXTRHEAD region (0x8410_0000..0x8413_0000)
    let mut drv_overlap = valid_spec.clone();
    drv_overlap.detect_flash_base = 0x8410_0000;
    drv_overlap.bgo_table_words[12] = 0x8410_0000;
    drv_overlap.bgo_table_words[31] = 48;
    assert!(
        validate_bgo_allowlist_invariant(train, &drv_overlap)
            .unwrap_err()
            .to_string()
            .contains("allowlist")
    );

    // 3. Spills beyond BGO top into nor_fs (0x8441_0000)
    let mut norfs_spill = valid_spec.clone();
    norfs_spill.bgo_table_words[13] = 0x8441_0000;
    norfs_spill.bgo_table_words[31] = 46;
    assert!(validate_bgo_allowlist_invariant(train, &norfs_spill).is_err());

    // 4. Misaligned off 64 KiB physical sector boundary (0x8413_1000)
    let mut misaligned = valid_spec.clone();
    misaligned.detect_flash_base = 0x8413_1000;
    misaligned.bgo_table_words[12] = 0x8413_1000;
    misaligned.bgo_table_words[13] = 0x843F_1000;
    misaligned.bgo_table_words[31] = 44;
    assert!(validate_bgo_allowlist_invariant(train, &misaligned).is_err());
}

#[test]
fn runtime_tier_gate_enforces_t3_minimum_unless_explicit_override() {
    assert!(enforce_train_evidence_tier("ver061", false).is_ok());
    let err = enforce_train_evidence_tier("ver063", false).unwrap_err();
    assert!(err.to_string().contains("RUNTIME TIER GATE BLOCKED"));
    assert!(enforce_train_evidence_tier("ver063", true).is_ok());

    let mut prog = synth_prog_fixture();
    assert!(retarget_prog_for_loader_profile_in_place(&mut prog, "ver063").is_err());
    assert!(retarget_prog_for_loader_profile_in_place(&mut prog, "ver061").is_ok());
}

#[test]
fn symmetric_traps_and_rtsd_lba48_gate_enforce_recovery_safety() {
    // DIGADIAG carrying BYPASS_CHECKS_ONLY must fail HardenedRecovery and warn in Standard when PANA__SD present.
    let mut bypass_diag = synth_prog_fixture();
    patch_prog061_bgo_in_place(&mut bypass_diag, Prog061BgoPatchMode::BypassChecksOnly).unwrap();

    let mut synth_main = vec![0u8; 0x300];
    synth_main[0x80..0x80 + HDD_CMD_CLAMP_WINDOW_TEMPLATE.len()]
        .copy_from_slice(&HDD_CMD_CLAMP_WINDOW_TEMPLATE);
    apply_capacity_patch_in_place(
        &mut synth_main,
        HddCapacityPatchMode::FixedCapacity,
        Some(250),
    )
    .unwrap();
    let main_ctr = pack_extrhead(
        &synth_main,
        "MAIN",
        AM34_MAIN_BASE_VA,
        "TEST3410PQ",
        ExtrheadCryptMode::Main,
        PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        0x3410,
    )
    .unwrap();

    let (sd_bytes, _) = pack_frm(
        &[
            dual_sd48_module("PROG061 ", bypass_diag.clone()),
            dual_sd48_module("MAIN3410", main_ctr),
        ],
        FrmContainerMode::Sd48,
        None,
    )
    .unwrap();

    let std_rep = verify_sd_buffers("/mem/sym_std", Some(Ok(&bypass_diag)), Some(&sd_bytes));
    assert_eq!(std_rep.overall_status, SD_STATUS_READY_WITH_WARNINGS);
    assert!(
        std_rep
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_DIGADIAG_BYPASS_TRAP && c.severity == "warn")
    );
    assert!(
        std_rep
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_RTSD_LBA48_GATE && c.severity == "pass")
    );

    let hard_rep = verify_sd_buffers_with_profile(
        "/mem/sym_hard",
        Some(Ok(&bypass_diag)),
        Some(&sd_bytes),
        SdKitAuditProfile::HardenedRecovery,
    );
    assert_eq!(hard_rep.overall_status, SD_STATUS_INVALID);
    assert!(
        hard_rep
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_DIGADIAG_BYPASS_TRAP && c.severity == "fail")
    );
    assert!(
        !hard_rep
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_FVU_PANAMON_TRAP)
    );
    assert!(
        hard_rep
            .checks
            .iter()
            .any(|c| c.id == CHK_ID_BGO_ALLOWLIST_INVARIANT && c.severity == "pass")
    );
}

#[test]
fn adversarial_nor_dump_integrity_blocks_swapped_mirrored_truncated_and_erased_dumps() {
    // 1. Truncated dump (< 2 MiB)
    let trunc = vec![0xAAu8; NOR_MIN_VALID_DUMP_BYTES / 2];
    let r_trunc = inspect_nor_dump_integrity(&trunc);
    assert!(!r_trunc.ok);

    // 2. All-0xFF erased dump
    let erased = vec![0xFFu8; NOR_MIN_VALID_DUMP_BYTES];
    let r_erased = inspect_nor_dump_integrity(&erased);
    assert!(!r_erased.ok);
    assert!(r_erased.is_all_ff);

    // 3. 16-bit byte-swapped dump ("XERTDAEH")
    let mut swapped = vec![0xFFu8; NOR_MIN_VALID_DUMP_BYTES];
    swapped[0x100..0x108].copy_from_slice(b"XERTDAEH");
    let r_swap = inspect_nor_dump_integrity(&swapped);
    assert!(!r_swap.ok);
    assert!(r_swap.byte_swapped_16bit);

    // 4. Upper-half address-pin mirrored dump
    let mut half = vec![0x5Au8; NOR_MIN_VALID_DUMP_BYTES / 2];
    half[0x100..0x119].copy_from_slice(b"Loader ver0.61 [06/03/06]");
    let mut mirrored = half.clone();
    mirrored.extend_from_slice(&half);
    let r_mirror = inspect_nor_dump_integrity(&mirrored);
    assert!(!r_mirror.ok);
    assert_eq!(
        r_mirror.mirrored_block_size,
        Some(NOR_MIN_VALID_DUMP_BYTES / 2)
    );

    // 5. Healthy non-mirrored NOR image with valid IPL banner
    let mut valid_nor = vec![0xFFu8; NOR_MIN_VALID_DUMP_BYTES];
    valid_nor[0x100..0x119].copy_from_slice(b"Loader ver0.61 [06/03/06]");
    valid_nor[0x12_0000..0x12_0008].copy_from_slice(b"EXTRHEAD");
    let r_ok = inspect_nor_dump_integrity(&valid_nor);
    assert!(r_ok.ok);
    assert_eq!(r_ok.loader_version.as_deref(), Some("0.61"));
}

#[test]
fn rust_to_typescript_ipc_json_contracts_preserve_exact_keys() {
    // 1. PhysicalDriveInfo <-> apps/desktop/src/models/hdd.ts
    let drive = PhysicalDriveInfo {
        device_path: r"\\.\PhysicalDrive3".to_string(),
        display_name: "Hitachi PATA 250GB".to_string(),
        size_bytes: 250_059_350_016,
        size_gb: 250.06,
        is_system_drive: false,
        safe_to_write: true,
        lockout_reason: None,
        drive_role: StorageMediaRole::HardDisk,
        bus_type: "USB".to_string(),
        removable: false,
        is_mounted: false,
        mount_points: vec![],
        partitions: vec![MountedPartitionInfo {
            device_path: r"\\.\F:".to_string(),
            label: "MEIHFS".to_string(),
            fs_type: "RAW".to_string(),
            mount_point: r"F:\".to_string(),
            size_bytes: 250_000_000_000,
        }],
    };
    let drive_json = serde_json::to_value(&drive).unwrap();
    for key in [
        "device_path",
        "display_name",
        "size_bytes",
        "size_gb",
        "is_system_drive",
        "safe_to_write",
        "lockout_reason",
        "drive_role",
        "bus_type",
        "removable",
        "is_mounted",
        "mount_points",
        "partitions",
    ] {
        assert!(
            drive_json.get(key).is_some(),
            "Missing PhysicalDriveInfo key {key}"
        );
    }

    // 2. SdMediaVerificationReport <-> apps/desktop/src/models/unbrick.ts
    let raw_diag = vec![0u8; PANA_DIGADIAG_RAW_VER061_SIZE];
    let sd_rep = verify_sd_buffers("/mem/contract", Some(Ok(&raw_diag)), None);
    let sd_json = serde_json::to_value(&sd_rep).unwrap();
    for key in [
        "sd_root_dir",
        "overall_status",
        "digadiag_present",
        "digadiag_exact_size_ok",
        "pana_sd_present",
        "pana_sd_toc_entries",
        "pana_sd_list_sums_ok",
        "factory_unpatched_prog061_detected",
        "checks",
    ] {
        assert!(
            sd_json.get(key).is_some(),
            "Missing SdMediaVerificationReport key {key}"
        );
    }

    // 3. FirmwareTrainProfile + DigadiagSpec.trust_model <-> apps/desktop/src/models/generated/profiles.ts
    let train = find_train_by_loader_key("ver061").expect("ver061 train");
    let train_json = serde_json::to_value(train).unwrap();
    assert_eq!(
        train_json["digadiag_spec"]["trust_model"],
        serde_json::json!(DigadiagTrustModel::Checksum32Only)
    );
}
