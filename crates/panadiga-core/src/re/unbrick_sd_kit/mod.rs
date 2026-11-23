//! SD Recovery Kit Generator (`DIGADIAG.FRM` + `PANA__SD.FRM`), Media Validator,
//! and Raw SD MBR/BPB/FAT16 Disk Auditor (`inspect_sd`).
//!
//! Splits pure media verification (`verify`), pure artifact composition (`compose`),
//! sector-aligned raw disk inspection (`inspect_sd`), and thin disk I/O (`io`).

pub mod cleanroom_digadiag;
pub mod compose;
pub mod creator;
pub mod dvd_creator;
pub mod fat16_builder;
pub mod inspect_sd;
pub mod io;
pub mod iso9660_builder;
pub mod rebuild;
pub mod verify;

pub use cleanroom_digadiag::{
    CLEANROOM_DIGADIAG_MAGIC, CleanroomDigadiagArtifact, build_cleanroom_digadiag,
};

pub use compose::{
    HardenedPanamon1KitArtifacts, SdKitBuildPreset, SdLoaderProfile, SdUnbrickKitReport,
    SdUnbrickKitRequest, compose_hardened_panamon1_kit,
};
pub use creator::{
    RecoveryPayloadSpec, ResolvedRecoveryMedia, SDXC_ADVISORY_THRESHOLD_BYTES, SdCardAdvisory,
    SdCardCreatorPlan, SdCardCreatorReport, SdCardCreatorRequest, SdCreatorProgress,
    SdCreatorStage, SdTargetDriveInfo, assess_sd_card_size_advisory, build_layperson_sd_steps,
    create_sd_recovery_card_on_device, create_sd_recovery_image, flash_sd_recovery_card,
    inspect_sd_card_targets, plan_sd_recovery_card, resolve_recovery_media_payloads,
};
pub use dvd_creator::{
    DvdCreatorProgress, DvdCreatorReport, DvdCreatorRequest, DvdCreatorStage, OpticalBurnBackend,
    OpticalBurnerDriveInfo, SystemOpticalBurnBackend, create_dvd_recovery_disc,
    create_dvd_recovery_disc_with_backend, create_dvd_recovery_iso, expected_optical_confirm_token,
    inspect_optical_burners,
};
pub use fat16_builder::{
    FAT16_DEFAULT_VOLUME_MB, FAT16_MAX_VOLUME_MB, FAT16_MIN_VOLUME_MB, FAT16_PARTITION_TYPE_LBA,
    FAT16_VBR_LBA_START, Fat16PlacedFileReport, Fat16SdFileInput, Fat16SdGeometry,
    SynthesizedSdImage, compute_fat16_sd_geometry, encode_dos_8_3_name, synthesize_fat16_sd_image,
};
pub use inspect_sd::{
    FatBpbReport, MbrPartitionRecord, RootDirFileReport, SdDiskInspectionReport,
    inspect_sd_disk_bytes, inspect_sd_reader, inspect_sd_target,
};
pub use io::{
    prepare_sd_unbrick_kit, prepare_sd_unbrick_kit_with_tier_policy, verify_sd_unbrick_media,
    verify_sd_unbrick_media_with_profile,
};
pub use iso9660_builder::{
    ISO9660_SECTOR_SIZE, Iso9660FileInput, Iso9660InspectionReport, Iso9660PlacedFileReport,
    SynthesizedIsoImage, inspect_iso9660_image, synthesize_iso9660_level1_image,
};
pub use rebuild::{
    NorDumpIntegrityReport, RebuildFromDumpReport, RebuildFromDumpRequest,
    inspect_nor_dump_integrity, rebuild_sd_kit_from_dump, scan_loader_banner,
};
pub use verify::{
    DualFamilyParityReport, DualFamilySlotResolution, SdKitAuditProfile, SdMediaVerificationReport,
    SdUnbrickCheck, simulate_sd_find_firm_entry_dual, verify_sd_buffers,
    verify_sd_buffers_with_profile,
};

pub(crate) fn parse_sd_hardened_profile(
    raw: &str,
    kind: &str,
    hardened_aliases: &[&str],
) -> anyhow::Result<bool> {
    let normalized = raw.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "standard" | "default") {
        return Ok(false);
    }
    if hardened_aliases.contains(&normalized.as_str()) {
        return Ok(true);
    }
    anyhow::bail!(
        "Unsupported SD {kind} '{raw}': expected standard or {}",
        hardened_aliases.join(" / ")
    )
}

#[cfg(test)]
pub(crate) fn synthetic_prog061_fixture() -> Vec<u8> {
    use crate::re::bgo_unbrick::{
        OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
        ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
    };
    use crate::re::sata_bridge_fix::{
        OFF_PROG061_SATA_BRIDGE_RESET_WAIT, ORIG_PROG061_SATA_BRIDGE_RESET_WAIT,
    };
    use crate::re::sata_bridge_stage2::{
        OFF_PROG061_STAGE2_HOOK_SITE, ORIG_PROG061_STAGE2_HOOK_BYTES,
    };
    let mut prog = vec![0xFFu8; PROG061_EXPECTED_SIZE];
    prog[0..4].copy_from_slice(&[0xDC, 0x14, 0x00, 0x00]);
    prog[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
    prog[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
    prog[OFF_BGO_INIT_CALL..OFF_BGO_INIT_CALL + 7].copy_from_slice(&ORIG_A2BC_BGO_CALL);
    prog[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);
    prog[OFF_PROG061_SATA_BRIDGE_RESET_WAIT..OFF_PROG061_SATA_BRIDGE_RESET_WAIT + 11]
        .copy_from_slice(&ORIG_PROG061_SATA_BRIDGE_RESET_WAIT);
    prog[OFF_PROG061_STAGE2_HOOK_SITE..OFF_PROG061_STAGE2_HOOK_SITE + 7]
        .copy_from_slice(&ORIG_PROG061_STAGE2_HOOK_BYTES);
    prog
}

#[cfg(test)]
pub(crate) fn synthetic_eh55_prog061_fixture() -> (Vec<u8>, String) {
    use crate::re::bgo_unbrick::{ORIG_SUM_BEQ, find_retarget_spec_by_loader_key};
    let mut prog = synthetic_prog061_fixture();
    prog[0x03_6E2C..0x03_6E30].copy_from_slice(&0x8000_0000u32.to_le_bytes());
    prog[0x03_6E2C + 0x0C..0x03_6E2C + 0x10].copy_from_slice(&0x28u32.to_le_bytes());
    prog[0x03_6E2C + 0x30..0x03_6E2C + 0x34].copy_from_slice(&0x8413_0000u32.to_le_bytes());
    let ver061_spec = find_retarget_spec_by_loader_key("ver061").unwrap();
    let eh55_sum_off = ver061_spec.bgo_sum_beq_offset as usize;
    prog[eh55_sum_off..eh55_sum_off + 2].copy_from_slice(&ORIG_SUM_BEQ);
    (prog, ver061_spec.target_geometry_id.clone())
}

#[cfg(test)]
pub(crate) fn synthetic_packed_main_fixture(
    build_name: &str,
    version_u16: u16,
    fixed_gb: Option<u32>,
) -> Vec<u8> {
    use crate::constants::PANA_EXTRHEAD_DEFAULT_ADLER_STEP;
    use crate::crypto::{ExtrheadCryptMode, pack_extrhead};
    use crate::hdd::capacity_patch::codec::test_helpers::build_synthetic_hdd_cmd_main;
    use crate::hdd::capacity_patch::specs::MAIN_DEFAULT_BASE_VA;
    use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};

    let mut synth_main = build_synthetic_hdd_cmd_main();
    if let Some(gb) = fixed_gb {
        apply_capacity_patch_in_place(
            &mut synth_main,
            HddCapacityPatchMode::FixedCapacity,
            Some(gb),
        )
        .unwrap();
    }
    pack_extrhead(
        &synth_main,
        "MAIN",
        MAIN_DEFAULT_BASE_VA,
        build_name,
        ExtrheadCryptMode::Main,
        PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        version_u16,
    )
    .unwrap()
}
