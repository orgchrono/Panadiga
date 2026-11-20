#![deny(unsafe_code)]

//! Unit tests for TV standard MMIO plans, LBS region presets, and read-only bundle audits.

use crate::constants::{
    AIOF_CONT_OFF, AIOF_FS0_OFF, AIOF_VMODESEL_NTSC, AIOF_VMODESEL_OFF, AIOF_VMODESEL_PAL,
    AIOF_VMODESEL_PAL_M, AIOF_YC3_OFF, AM34_AIOF_BASE, AM34_AVDD_BASE, SPANSION_NVRAM_SIZE_BYTES,
};
use crate::crypto::{FrmContainerMode, FrmPackModule, pack_frm};
use crate::re::main_passthrough::EXPECTED_MAIN_3410PQ_SIZE;

use super::super::main_fmu::{OFF_FMU_DEVINFO_3410PQ, ORIG_FMU_DEVINFO_3410PQ};
use super::super::make_synthetic_ifodev2;
use super::mmio::AVDD_NPALCTL0_OFF;
use super::{
    LbsRegionPresetId, TvStandardKind, build_tv_calibration_mmio_plan,
    inspect_tv_calibration_bundle,
};

#[test]
fn tv_standard_mmio_plans_emit_verified_aiof_vmodesel_values() {
    let ntsc = build_tv_calibration_mmio_plan(TvStandardKind::Ntsc);
    assert_eq!(ntsc[0].addr, AM34_AIOF_BASE + AIOF_VMODESEL_OFF);
    assert_eq!(ntsc[0].value, u32::from(AIOF_VMODESEL_NTSC));
    assert_eq!(ntsc[0].value, 0x8020);
    assert_eq!(ntsc[3].value, 0x8000);

    let palm = build_tv_calibration_mmio_plan(TvStandardKind::PalM);
    assert_eq!(palm[0].value, u32::from(AIOF_VMODESEL_PAL_M));
    assert_eq!(palm[0].value, 0x8021);
    assert_eq!(palm[1].addr, AM34_AIOF_BASE + AIOF_FS0_OFF);
    assert_eq!(palm[2].addr, AM34_AIOF_BASE + AIOF_YC3_OFF);
    assert_eq!(palm[2].value, 0x0020);
    assert_eq!(palm[3].addr, AM34_AIOF_BASE + AIOF_CONT_OFF);
    assert_eq!(palm[3].value, 0x4000);
    assert_eq!(palm[4].addr, AM34_AVDD_BASE + AVDD_NPALCTL0_OFF);

    let pal = build_tv_calibration_mmio_plan(TvStandardKind::Pal);
    assert_eq!(pal[0].value, u32::from(AIOF_VMODESEL_PAL));
    assert_eq!(pal[0].value, 0x8027);

    let secam = build_tv_calibration_mmio_plan(TvStandardKind::Secam);
    assert!(secam[0].note.contains("SECAM0..1"));
}

#[test]
fn preset_and_standard_parsing_and_bundle_inspection_tag_ifodev2_tail_readonly() {
    assert_eq!(
        "pal-m".parse::<TvStandardKind>().unwrap(),
        TvStandardKind::PalM
    );
    assert_eq!(
        "lbs".parse::<LbsRegionPresetId>().unwrap(),
        LbsRegionPresetId::BrPalMR4
    );
    assert_eq!(
        LbsRegionPresetId::BrPalMR4.expected_chassis_family(),
        "M6930LB"
    );
    assert_eq!(LbsRegionPresetId::BrPalMR4.dvd_region_code(), Some(4));

    let ifodev2 = make_synthetic_ifodev2("M6930LB", b'M', &[0x01, 0x02, 0x03, 0x04]);

    let mut main_buf = vec![0u8; EXPECTED_MAIN_3410PQ_SIZE];
    main_buf[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 21]
        .copy_from_slice(&ORIG_FMU_DEVINFO_3410PQ);

    let mut sd_mod = FrmPackModule::new("MAIN3410PQ", vec![0x55u8; 64]);
    sd_mod.hardware_family = "M6930LB".to_string();
    let (frm_bytes, _) = pack_frm(&[sd_mod], FrmContainerMode::Sd48, None).unwrap();

    let nvram = vec![0xFFu8; SPANSION_NVRAM_SIZE_BYTES];
    let report = inspect_tv_calibration_bundle(
        LbsRegionPresetId::BrPalMR4,
        None,
        Some(&ifodev2),
        Some(&main_buf),
        Some(&frm_bytes),
        Some(&nvram),
    )
    .unwrap();

    assert!(report.metadata_and_dev_oem_only);
    assert!(report.profile.metadata_and_dev_oem_only);
    assert_eq!(report.profile.tv_standard, TvStandardKind::PalM);
    assert_eq!(report.dev_bin.as_ref().unwrap().model, "M6930LB");
    let tail = report.ifodev2_tail.as_ref().unwrap();
    assert!(tail.metadata_and_dev_oem_only);
    assert_eq!(tail.byte_len, 0x400);
    assert_eq!(tail.non_ff_bytes, 4);
    assert_eq!(report.main_region.as_ref().unwrap().build_profile, "3410PQ");
    assert!(report.frm_toc.as_ref().unwrap().all_list_sums_valid);
    assert_eq!(
        report
            .nvram_sa134_sa141
            .as_ref()
            .unwrap()
            .microsectors
            .len(),
        8
    );
}

#[test]
fn bundle_inspection_rejects_oversized_nvram_and_invalid_frm() {
    let oversized = vec![0u8; SPANSION_NVRAM_SIZE_BYTES + 1];
    assert!(
        inspect_tv_calibration_bundle(
            LbsRegionPresetId::CustomInspected,
            None,
            None,
            None,
            None,
            Some(&oversized),
        )
        .is_err()
    );

    let bad_frm = vec![0u8; 128];
    assert!(
        inspect_tv_calibration_bundle(
            LbsRegionPresetId::CustomInspected,
            None,
            None,
            None,
            Some(&bad_frm),
            None,
        )
        .is_err()
    );
}
