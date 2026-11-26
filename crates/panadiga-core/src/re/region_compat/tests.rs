//! Unit tests for `region_compat` (`MAIN` `<deviceinfo>`, `DEV.BIN`, `.FRM` TOC retarget).

use super::super::main_passthrough::EXPECTED_MAIN_3410PQ_SIZE;
use super::*;
use crate::constants::{PANA_DEV_BIN_MAGIC, PANA_DEV_BIN_SIG};
use crate::crypto::codec::write_padded_ascii;
use crate::crypto::{FrmContainerMode, FrmPackModule, decrypt_and_parse_frm_toc, pack_frm};

#[test]
fn region_compat_main_bypass_and_restore_work_atomically() {
    let mut m3410 = vec![0u8; EXPECTED_MAIN_3410PQ_SIZE];
    m3410[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 21]
        .copy_from_slice(&ORIG_FMU_DEVINFO_3410PQ);
    assert_eq!(inspect_main_region_compat(&m3410).unwrap().state, "FACTORY");
    let patched =
        patch_main_region_compat_in_place(&mut m3410, RegionCompatMode::BypassModelAndOem).unwrap();
    assert_eq!(patched.build_profile, "3410PQ");
    assert_eq!(patched.state, "BYPASS_MODEL_AND_OEM");
    let restored =
        patch_main_region_compat_in_place(&mut m3410, RegionCompatMode::FactoryStrict).unwrap();
    assert_eq!(restored.state, "FACTORY");

    let mut m3060 = vec![0u8; EXPECTED_MAIN_3060NP_SIZE];
    m3060[OFF_FMU_DEVINFO_3060NP..OFF_FMU_DEVINFO_3060NP + 21]
        .copy_from_slice(&ORIG_FMU_DEVINFO_3060NP);
    let p2 =
        patch_main_region_compat_in_place(&mut m3060, RegionCompatMode::BypassModelAndOem).unwrap();
    assert_eq!(p2.build_profile, "3060NP");
    assert_eq!(p2.state, "BYPASS_MODEL_AND_OEM");
}

#[test]
fn dev_bin_and_frm_toc_retarget_roundtrip_preserve_checksums() {
    let mut dev = vec![0xFFu8; 512];
    dev[0x20..0x2C].copy_from_slice(PANA_DEV_BIN_SIG);
    dev[0..4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
    write_padded_ascii(&mut dev[0x04..0x0C], "EH58EE", 0);
    dev[0x0C] = b'M';
    let initial = inspect_dev_bin_bytes(&dev).unwrap();
    assert_eq!(initial.model, "EH58EE");
    assert_eq!(initial.oem, "M");
    let (edited, rep) = patch_dev_bin_bytes(&dev, "EH55EG", Some('E')).unwrap();
    assert_eq!(rep.model, "EH55EG");
    assert_eq!(rep.oem, "E");
    assert_eq!(edited.len(), 512);

    let mod0 = FrmPackModule::new("MAIN3410PQ", vec![0xAAu8; 64]);
    let (dvd_bytes, _) = pack_frm(&[mod0], FrmContainerMode::Dvd, None).unwrap();
    let (retargeted, frm_rep) = retarget_frm_toc_bytes(
        &dvd_bytes,
        Some("M6931"),
        Some("EG-500"),
        Some('E'),
        Some(1),
    )
    .unwrap();
    assert_eq!(frm_rep.updated_entries, 1);
    assert!(frm_rep.entries[0].list_sum_valid);
    assert_eq!(frm_rep.entries[0].hardware_family, "M6931");
    assert_eq!(frm_rep.entries[0].model_group, "EG-500");
    assert_eq!(frm_rep.entries[0].target_flag, 1);
    assert_eq!(
        decrypt_and_parse_frm_toc(&retargeted).len(),
        frm_rep.entries.len()
    );
}

#[test]
fn ifodev2_2048b_and_sd48_retarget_preserve_tail_blocks_and_prefix_semantics() {
    let ifodev2 = make_synthetic_ifodev2("M6930LB", b'M', &[0x01, 0x01]);

    let initial = inspect_dev_bin_bytes(&ifodev2).unwrap();
    assert_eq!(initial.size_bytes, PANA_IFODEV2_SIZE);
    assert_eq!(initial.model, "M6930LB");
    assert_eq!(initial.oem, "M");

    let (patched, rep) = patch_dev_bin_bytes(&ifodev2, "M6930", Some('E')).unwrap();
    assert_eq!(patched.len(), PANA_IFODEV2_SIZE);
    assert_eq!(rep.model, "M6930");
    assert_eq!(rep.oem, "E");
    assert_eq!(&patched[0x400..0x402], &[0x01, 0x01]);

    let mut sd_mod = FrmPackModule::new("MAIN3410PQ", vec![0xAAu8; 64]);
    sd_mod.model_prefix = b'E';
    sd_mod.target_flag = 0;
    let (sd48_bytes, _) = pack_frm(&[sd_mod], FrmContainerMode::Sd48, None).unwrap();
    let (retargeted, frm_rep) = retarget_frm_toc_bytes(
        &sd48_bytes,
        Some("M6930LB"),
        Some("NP-309"),
        None,
        Some(b'M'),
    )
    .unwrap();
    assert_eq!(frm_rep.layout, "sd-48b-plaintext");
    assert!(frm_rep.entries[0].list_sum_valid);
    assert_eq!(frm_rep.entries[0].hardware_family, "M6930LB");
    assert_eq!(retargeted[0x18], b'M');
    assert_eq!(retargeted[0x19], 0x00);

    let mut prog_mod = FrmPackModule::new("PROG061 ", vec![0xBBu8; 64]);
    prog_mod.hardware_family = "M6930LB".to_string();
    prog_mod.hardware_family_aliases = vec!["M6930".to_string()];
    prog_mod.model_prefix = b'E';
    let mut main_mod = FrmPackModule::new("MAIN3410PQ", vec![0xCCu8; 64]);
    main_mod.hardware_family = "M6930LB".to_string();
    main_mod.hardware_family_aliases = vec!["M6930".to_string()];
    main_mod.model_prefix = b'E';
    let (sd48_dual, _) = pack_frm(&[prog_mod, main_mod], FrmContainerMode::Sd48, None).unwrap();
    let (retargeted_dual, dual_rep) =
        retarget_frm_toc_bytes(&sd48_dual, None, Some("NP-309"), Some('E'), Some(b'M')).unwrap();
    assert_eq!(dual_rep.updated_entries, 4);
    assert!(
        crate::re::unbrick_sd_kit::simulate_sd_find_firm_entry_dual(&retargeted_dual)
            .dual_parity_ok
    );

    // Passing numeric target_flag=1 must update [0x19] WITHOUT clobbering [0x18] model_prefix 'M'
    let (retargeted_num_flag, _) =
        retarget_frm_toc_bytes(&retargeted, None, None, Some('M'), Some(1)).unwrap();
    assert_eq!(retargeted_num_flag[0x18], b'M');
    assert_eq!(retargeted_num_flag[0x19], 1);
}

#[test]
fn tv_calibration_bundle_accepts_extrhead_packed_main3410pq() {
    use crate::constants::AM34_MAIN_BASE_VA;
    use crate::crypto::{ExtrheadCryptMode, pack_extrhead};
    use crate::re::main_passthrough::EXPECTED_MAIN_3410PQ_SIZE;
    use crate::re::region_compat::{
        LbsRegionPresetId, OFF_FMU_DEVINFO_3410PQ, ORIG_FMU_DEVINFO_3410PQ, TvStandardKind,
        inspect_tv_calibration_bundle,
    };

    let mut raw = vec![0u8; EXPECTED_MAIN_3410PQ_SIZE];
    raw[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + ORIG_FMU_DEVINFO_3410PQ.len()]
        .copy_from_slice(&ORIG_FMU_DEVINFO_3410PQ);
    let packed = pack_extrhead(
        &raw,
        "MAIN",
        AM34_MAIN_BASE_VA,
        "3410PQ",
        ExtrheadCryptMode::Main,
        16,
        1,
    )
    .unwrap();
    let rep = inspect_tv_calibration_bundle(
        LbsRegionPresetId::BrPalMR4,
        Some(TvStandardKind::PalM),
        None,
        Some(&packed),
        None,
        None,
    )
    .unwrap();
    let main_reg = rep.main_region.as_ref().unwrap();
    assert_eq!(main_reg.build_profile, "3410PQ");
    assert_eq!(main_reg.state, "FACTORY");
}

#[test]
fn ifodev2_full_factory_options_and_19_market_codes_decode_cleanly() {
    let mut ifodev2 = make_synthetic_ifodev2("M6930LB", b'M', &[0x01, 0x01]);
    ifodev2[0x0D..0x13].copy_from_slice(&[0x31, 0xC5, 0x60, 0x00, 0x00, 0x00]);
    ifodev2[0x18..0x1A].copy_from_slice(&[0x00, 0x03]);
    ifodev2[0x30] = 0x07;
    ifodev2[0x31] = 0x04;
    ifodev2[0x50..0x5E].copy_from_slice(&[
        0x00, 0x33, 0x00, 0x92, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x10, 0x20, 0xFF, 0xFF, 0x02,
    ]);

    let rep = inspect_dev_bin_bytes(&ifodev2).unwrap();
    assert_eq!(rep.model, "M6930LB");
    assert_eq!(rep.oem, "M");
    assert_eq!(rep.option_code_hex, "31c560000000");
    assert_eq!(rep.hw_version_u16, 3);
    assert_eq!(rep.model_code, 7);
    assert_eq!(rep.market_code, 4);
    assert_eq!(rep.market_code_name, "PL");
    assert_eq!(rep.main_option_btn, [0x0033, 0x0092, 0xFFFF, 0xFFFF]);
    assert_eq!(rep.main_option_rcmode, 0);
    assert_eq!(rep.main_option_gui, [0x10, 0x20, 0xFF, 0xFF]);
    assert_eq!(rep.main_option_chmode, 2);

    let expected_lut = [
        (0x00u8, "JP"),
        (0x01, "P"),
        (0x02, "PC"),
        (0x03, "PP"),
        (0x04, "PL"),
        (0x05, "PX"),
        (0x06, "GT"),
        (0x80, "EG"),
        (0x81, "GCS"),
        (0x82, "GN"),
        (0x83, "EB"),
        (0x84, "UNKNOWN"),
        (0x85, "EF"),
        (0x86, "EBL"),
        (0x87, "EE"),
        (0x88, "EP"),
        (0x89, "GC"),
        (0x8A, "GK"),
        (0x8B, "EC"),
        (0xFF, "--"),
    ];
    for (code, name) in expected_lut {
        assert_eq!(decode_market_code_name(code), name);
    }
}

#[test]
fn m6930lb_ssot_localization_executes_cleanly_in_mn103vm_and_roundtrips() {
    use crate::constants::AM34_MAIN_BASE_VA;
    use crate::re::Mn103Vm;
    use crate::re::ssot_patches::find_main_build_by_size;

    let (_, build_3410) = find_main_build_by_size(EXPECTED_MAIN_3410PQ_SIZE).unwrap();
    assert_eq!(build_3410.region_localization_sites.len(), 58);
    let mut main_3410 = vec![0xFFu8; EXPECTED_MAIN_3410PQ_SIZE];
    let market_window = hex::decode(
        "14a006c31cfac88000c024fac88300c340fac88b00c118fac88500c012ca32a003c22ea000c82ac0068503ca24fccce42aae5715dda5fdffff000000ddfefdffff00008064dd586c210000002d8000fc86bc6d1853",
    )
    .unwrap();
    assert_eq!(market_window.len(), 0x574F_722D - 0x574F_71D8);
    let off_win = 0x574F_71D8usize - AM34_MAIN_BASE_VA as usize;
    main_3410[off_win..off_win + market_window.len()].copy_from_slice(&market_window);
    main_3410[0x570D_625E - AM34_MAIN_BASE_VA as usize..][..5]
        .copy_from_slice(&[0xCF, 0xF2, 0xF8, 0xFE, 0xE8]);
    main_3410[0x5706_13C7 - AM34_MAIN_BASE_VA as usize..][..7]
        .copy_from_slice(&[0xDD, 0x0B, 0x7D, 0x00, 0x00, 0x00, 0x00]);
    main_3410[0x5706_B337 - AM34_MAIN_BASE_VA as usize..][..7]
        .copy_from_slice(&[0xDD, 0x9B, 0xDD, 0xFF, 0xFF, 0x00, 0x00]);
    main_3410[0x574F_659D - AM34_MAIN_BASE_VA as usize..][..7]
        .copy_from_slice(&[0xDD, 0x1E, 0x08, 0x02, 0x00, 0x00, 0x00]);
    main_3410[0x574F_4BF5 - AM34_MAIN_BASE_VA as usize..][..7]
        .copy_from_slice(&[0xDD, 0x53, 0x39, 0xB7, 0xFF, 0x00, 0x00]);
    main_3410[0x5750_BD6A - AM34_MAIN_BASE_VA as usize..][..2].copy_from_slice(&[0xF1, 0xE2]);
    main_3410[0x5713_09CF - AM34_MAIN_BASE_VA as usize..][..2].copy_from_slice(&[0xF1, 0xE2]);
    // Sites 8-15: config1 default, cav_boot_ntsc, boot_pal_ntsc_force, AIOF 7.5 IRE, AIOF cps_pal_m, setinfo lock, sysboot AVOUT
    main_3410[0x57AE_FBAC - AM34_MAIN_BASE_VA as usize..][..1].copy_from_slice(&[0x00]);
    main_3410[0x574F_6F2D - AM34_MAIN_BASE_VA as usize..][..7]
        .copy_from_slice(&[0xF8, 0xBB, 0x13, 0xA3, 0xC9, 0x03, 0x88]);
    main_3410[0x5750_41FF - AM34_MAIN_BASE_VA as usize..][..2].copy_from_slice(&[0xC9, 0x1B]);
    main_3410[0x570D_674D - AM34_MAIN_BASE_VA as usize..][..2].copy_from_slice(&[0x80, 0x55]);
    main_3410[0x570D_679F - AM34_MAIN_BASE_VA as usize..][..2].copy_from_slice(&[0x80, 0x02]);
    main_3410[0x574F_691B - AM34_MAIN_BASE_VA as usize..][..13].copy_from_slice(&[
        0x08, 0xFB, 0xAA, 0x00, 0x4D, 0xFB, 0xD8, 0x00, 0x00, 0xC9, 0x04, 0x8A, 0x01,
    ]);
    let setinfo_code_a = hex::decode(
        "cf20f540f1e65910fb690004fcdda0d0ae57f580f17470a1c8072c1727ca38a502c828c108a501c80aca2ba504c811ca25f580f174f82004f040f052ca18f580f174f820047062ca0df580f174f82004f060f07200df2004",
    )
    .unwrap();
    main_3410[0x5751_6C9F - AM34_MAIN_BASE_VA as usize..][..setinfo_code_a.len()]
        .copy_from_slice(&setinfo_code_a);
    let setinfo_code_b = hex::decode(
        "cfa0f8fee8f89716f1e0f1d25656f1e8fcd0a8d0ae573e2216f1d99500470c4710f000f0f0dfa020cf80f8feec5d24f1e0f1d25656f1e8fcd0acd0ae57470c95004710f000f0f0df8018",
    )
    .unwrap();
    main_3410[0x5751_6EA5 - AM34_MAIN_BASE_VA as usize..][..setinfo_code_b.len()]
        .copy_from_slice(&setinfo_code_b);
    main_3410[0x5750_C377 - AM34_MAIN_BASE_VA as usize..][..3].copy_from_slice(&[0x00, 0xF0, 0xFC]);
    main_3410[0x57AE_E210 - AM34_MAIN_BASE_VA as usize..][..16]
        .copy_from_slice(&hex::decode("01000000305e18534d6c51579f6c5157").unwrap());
    for (va, bytes) in [
        (0x5706_75F8usize, &[0xCFu8, 0xE0, 0xF8, 0xFE, 0xCC][..]),
        (0x5706_7931, &[0xCF, 0xF2, 0xF8, 0xFE, 0xCC]),
        (0x5706_83FF, &[0xCF, 0xA0, 0xF8, 0xFE, 0xD0]),
        (0x5706_843D, &[0xCF, 0xF0, 0xF8, 0xFE, 0xD0]),
        (0x5706_8499, &[0xCF, 0xA0, 0xF8, 0xFE, 0xD0]),
        (0x5706_84E0, &[0xCF, 0xE0, 0xF8, 0xFE, 0xD0]),
        (0x5706_8548, &[0xCF, 0xA0, 0xF8, 0xFE, 0xD0]),
        (0x5750_971C, &[0xCF, 0x80, 0xF8, 0xFE, 0xF4]),
        (0x5750_9770, &[0xCF, 0x80, 0xF8, 0xFE, 0xE8]),
        (0x5750_4B28, &[0xC4, 0x11]),
        (0x5750_6476, &[0xDD, 0x8C, 0xA4, 0x02, 0x00, 0x00, 0x00]),
        (0x5750_64B0, &[0xDD, 0x52, 0xA4, 0x02, 0x00, 0x00, 0x00]),
        (0x5751_EAB3, &[0xDD, 0x4F, 0x1E, 0x01, 0x00, 0x00, 0x00]),
        (0x5751_EB37, &[0xDD, 0xCB, 0x1D, 0x01, 0x00, 0x00, 0x00]),
        (0x570B_623C, &[0x00, 0xF0, 0xFC]),
        (0x5772_4096, &[0xDD, 0xDD, 0x57, 0x99, 0xFF, 0x00, 0x00]),
        (0x570B_98EE, &[0x2C, 0x10, 0x04]),
        (0x5772_546D, &[0x2C, 0x00, 0x02]),
        (0x5772_5518, &[0x2C, 0xB1, 0x4B]),
        (0x5772_553F, &[0xFC, 0xCC, 0x00, 0x8D, 0x00, 0x00]),
        (0x570D_6630, &[0xC8, 0x6F]),
        (0x570D_66B1, &[0x80, 0x02]),
        (0x570D_66E3, &[0x80, 0x51]),
        (0x570D_66ED, &[0x80, 0x57]),
        (0x570D_66F7, &[0x80, 0x53]),
        (0x570D_7C07, &[0xAA, 0x01, 0xC9, 0x11]),
        (0x570D_7CE1, &[0xBA, 0x01, 0xC9, 0x10]),
        (0x570C_E4F2, &[0xCF, 0xF2, 0xF8, 0xFE, 0xDC]),
        (0x570D_C732, &[0xCF, 0xC0, 0xF8, 0xFE, 0xEC]),
        (0x5706_34B2, &[0xCF, 0xC0, 0xF8, 0xFE, 0xE8]),
        (0x5706_34F7, &[0xCF, 0xF2, 0xF8, 0xFE, 0xB0]),
        (0x570A_C466, &[0xF8, 0xFE, 0xE0, 0x00, 0x42]),
        (0x576A_F438, &[0xCF, 0x20, 0xF8, 0xFE, 0xF0]),
        (0x5771_AB1B, &[0xCF, 0x20, 0xF8, 0xFE, 0xD0, 0xF1]),
        (0x5751_7163, &[0xCF, 0x80, 0xF8, 0xFE, 0xE4]),
        (0x570D_731E, &[0x80, 0x57]),
        (0x570D_7374, &[0x80, 0x57]),
        (0x570D_739A, &[0x80, 0x57]),
        (0x5706_2BD7, &[0xCF, 0xF2, 0xF8, 0xFE, 0xE0]),
        (0x5706_5E05, &[0xCF, 0x80, 0xF8, 0xFE, 0xD0]),
        (0x5706_26A0, &[0x80, 0x02]),
        (0x5700_0A20, &[0u8; 58][..]),
        (0x5750_EE35, &[0xDD, 0x61, 0x61, 0xB5, 0xFF, 0x00, 0x00]),
        (0x5750_7927, &[0xDD, 0xE5, 0x1A, 0x01, 0x00, 0x00, 0x00]),
    ] {
        main_3410[va - AM34_MAIN_BASE_VA as usize..][..bytes.len()].copy_from_slice(bytes);
    }
    let orig_3410_sha = inspect_main_localization(&main_3410).unwrap().sha256;
    assert_eq!(
        inspect_main_localization(&main_3410).unwrap().overall_state,
        "FACTORY"
    );

    let run_market_window = |buf: &[u8]| -> u8 {
        let mut vm = Mn103Vm::new();
        vm.map_segment(AM34_MAIN_BASE_VA, buf.to_vec(), false);
        vm.map_segment(0x5318_6000, vec![0xFFu8; 0x1000], true);
        vm.d[0] = 0x04;
        vm.d[1] = 0x04;
        vm.pc = 0x574F_71D8;
        vm.run_until(0x574F_722D, 32).unwrap();
        vm.read_u8(0x5318_6DBC).unwrap()
    };
    assert_eq!(run_market_window(&main_3410), 0x04);

    let applied =
        patch_main_localization_in_place(&mut main_3410, LocalizationPatchMode::M6930LbBrazil)
            .unwrap();
    assert_eq!(applied.build_profile, "3410PQ");
    assert_eq!(applied.overall_state, "M6930LB_BRAZIL");
    assert_eq!(run_market_window(&main_3410), 0x80);

    // Patched IC3001 I2C 0x94 + 9 FPC/U99 stubbed functions return D0 = 0 and restore SP
    for entry in [
        0x570D_625Eu32,
        0x5706_75F8,
        0x5706_7931,
        0x5706_83FF,
        0x5706_843D,
        0x5706_8499,
        0x5706_84E0,
        0x5706_8548,
        0x5750_971C,
        0x5750_9770,
        0x5706_2BD7,
        0x5706_5E05,
    ] {
        let mut vm_ret = Mn103Vm::new();
        vm_ret.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
        vm_ret.map_segment(0x51FE_0000, vec![0u8; 0x1000], true);
        vm_ret.sp = 0x51FE_0800;
        vm_ret.write_u32(vm_ret.sp, 0x577F_BEEF).unwrap();
        vm_ret.d[0] = 0xDEAD_BEEF;
        vm_ret.pc = entry;
        vm_ret.run_until(0x577F_BEEF, 16).unwrap();
        assert_eq!(vm_ret.sp, 0x51FE_0800);
        assert_eq!(vm_ret.d[0], 0);
    }

    // DACOUT1 now 0x0DDF, FPC_StartPowerOff_InEPGDL mode=1, and three runtime AIOF_Set_Standard sites select 0x56 (0x40F0)
    assert_eq!(
        &main_3410[0x5772_553F - AM34_MAIN_BASE_VA as usize..][..6],
        &[0xFC, 0xCC, 0xDF, 0x0D, 0x00, 0x00]
    );
    assert_eq!(
        &main_3410[0x5706_26A0 - AM34_MAIN_BASE_VA as usize..][..2],
        &[0x80, 0x01]
    );
    for va in [0x570D_731Eu32, 0x570D_7374, 0x570D_739A] {
        assert_eq!(
            &main_3410[va as usize - AM34_MAIN_BASE_VA as usize..][..2],
            &[0x80, 0x56]
        );
    }

    // Patched FPC / AutoClock / WakeupReason / MEDIA RESET branches land on exact join VAs
    for (entry, target) in [
        (0x5706_13C7u32, 0x5706_13F5u32),
        (0x5706_B337, 0x5706_B35B),
        (0x574F_659D, 0x574F_6600),
        (0x5750_4B28, 0x5750_4B39),
        (0x5750_6476, 0x5750_6486),
        (0x5750_64B0, 0x5750_64C0),
    ] {
        let mut vm_br = Mn103Vm::new();
        vm_br.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
        vm_br.pc = entry;
        vm_br.step().unwrap();
        assert_eq!(vm_br.pc, target);
    }

    // Patched pre-WakeupReason FPC 0xF9 GATE=15 + MEDIA RESET wait poff suppressions clear d0 and advance past call
    for (entry, after_call) in [
        (0x574F_4BF5u32, 0x574F_4BFCu32),
        (0x5751_EAB3, 0x5751_EABA),
        (0x5751_EB37, 0x5751_EB3E),
    ] {
        let mut vm_skip = Mn103Vm::new();
        vm_skip.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
        vm_skip.d[0] = 0xDEAD_BEEF;
        vm_skip.pc = entry;
        vm_skip.run_until(after_call, 16).unwrap();
        assert_eq!(vm_skip.pc, after_call);
        assert_eq!(vm_skip.d[0], 0);
    }

    // Patched SYS_CNTL_PAL_NTSC_Change & AVOUT_V_Mute force a2=0 (9A 00 -> NTSC 60Hz + DACOUT1 unmuted)
    for (entry, expected_next_pc) in [(0x5750_BD6Au32, 0x5750_BD6Cu32), (0x5713_09CF, 0x5713_09D1)]
    {
        let mut vm_a2 = Mn103Vm::new();
        vm_a2.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
        vm_a2.d[0] = 1;
        vm_a2.a[2] = 0xDEAD_BEEF;
        vm_a2.pc = entry;
        vm_a2.step().unwrap();
        assert_eq!(vm_a2.pc, expected_next_pc);
        assert_eq!(vm_a2.a[2], 0);
    }

    // Verify setinfo_get(0x0117) returns 1 (TV_NTSC) even when [nfrom] config1 RAM holds 0 (TV_PAL),
    // and setinfo_set(0x0117, 0) returns 0 without mutating 0x57AE_FBAC.
    let mut vm_setinfo = Mn103Vm::new();
    vm_setinfo.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
    vm_setinfo.map_segment(0x5318_5000, vec![0u8; 0x1000], true);
    vm_setinfo.map_segment(0x51FE_0000, vec![0u8; 0x1000], true);
    vm_setinfo.write_u8(0x5318_5E30, 0x00).unwrap();
    vm_setinfo.sp = 0x51FE_0800;
    let call_setinfo = |vm: &mut Mn103Vm, entry: u32, buf_va: u32, init_val: u8| {
        vm.write_u8(buf_va, init_val).unwrap();
        vm.write_u32(vm.sp + 12, 0x0000_0001).unwrap();
        vm.write_u32(vm.sp, 0x577F_BEEF).unwrap();
        vm.d[0] = 0x0117;
        vm.d[1] = buf_va;
        vm.pc = entry;
        vm.run_until(0x577F_BEEF, 128).unwrap();
        assert_eq!(vm.sp, 0x51FE_0800);
        assert_eq!(vm.d[0], 0);
    };
    call_setinfo(&mut vm_setinfo, 0x5751_6ECD, 0x51FE_0900, 0xAA);
    assert_eq!(vm_setinfo.read_u8(0x51FE_0900).unwrap(), 1);
    call_setinfo(&mut vm_setinfo, 0x5751_6EA5, 0x51FE_0904, 0x00);
    assert_eq!(vm_setinfo.read_u8(0x57AE_FBAC).unwrap(), 1);

    // Verify sys_boot AVOUT_SetInitParam caller (0x574F_691B..0x574F_6928) keeps d2 = 0 (TV_NTSC)
    // even when [sp + 0x4D] == 0.
    let mut vm_sysboot_av = Mn103Vm::new();
    vm_sysboot_av.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
    vm_sysboot_av.map_segment(0x51FE_0000, vec![0u8; 0x1000], true);
    vm_sysboot_av.sp = 0x51FE_0800;
    vm_sysboot_av.write_u8(vm_sysboot_av.sp + 0x4D, 0).unwrap();
    vm_sysboot_av.d[2] = 0xDEAD_BEEF;
    vm_sysboot_av.pc = 0x574F_691B;
    vm_sysboot_av.run_until(0x574F_6928, 32).unwrap();
    assert_eq!(vm_sysboot_av.pc, 0x574F_6928);
    assert_eq!(vm_sysboot_av.d[2], 0);

    let restored =
        patch_main_localization_in_place(&mut main_3410, LocalizationPatchMode::FactoryOriginal)
            .unwrap();
    assert_eq!(restored.overall_state, "FACTORY");
    assert_eq!(restored.sha256, orig_3410_sha);

    // Verify tv_profiles filtering produces 4 distinct builds (pal=36, ntsc-j=53, ntsc-m=56, pal-m=55)
    let mut distinct_shas = std::collections::HashSet::new();
    for (tv_profile, expected_count) in [
        ("pal", 36usize),
        ("ntsc-j", 53),
        ("ntsc-m", 56),
        ("pal-m", 55),
    ] {
        let mut variant = main_3410.clone();
        let rep = patch_main_localization_for_tv_profile_in_place(
            &mut variant,
            LocalizationPatchMode::M6930LbBrazil,
            Some(tv_profile),
        )
        .unwrap();
        assert_eq!(rep.tv_profile, tv_profile);
        assert_eq!(rep.site_count, expected_count);
        assert_eq!(rep.overall_state, "M6930LB_BRAZIL");
        distinct_shas.insert(rep.sha256.clone());

        let reverted = patch_main_localization_for_tv_profile_in_place(
            &mut variant,
            LocalizationPatchMode::FactoryOriginal,
            Some(tv_profile),
        )
        .unwrap();
        assert_eq!(reverted.overall_state, "FACTORY");
        assert_eq!(reverted.sha256, orig_3410_sha);
    }
    assert_eq!(distinct_shas.len(), 4);

    // Execute new NTSC 525, IIC5 BH7629KS2/Audio cave, ATP skip & Standby unblock patches in Mn103Vm
    let mut ntsc_vm_buf = main_3410.clone();
    patch_main_localization_for_tv_profile_in_place(
        &mut ntsc_vm_buf,
        LocalizationPatchMode::M6930LbBrazil,
        Some("ntsc-m"),
    )
    .unwrap();
    let mut vm_ntsc = Mn103Vm::new();
    vm_ntsc.map_segment(AM34_MAIN_BASE_VA, ntsc_vm_buf, false);
    vm_ntsc.map_segment(0x51FE_0000, vec![0u8; 0x1000], true);

    // Execute FPC_VIDEO_OUT_ON hook (0x5750_EE35) -> 0x5700_0A20 cave -> iic5_write(0x90, 6) & iic5_write(0x84, 4) -> 0x5750_EE3C
    vm_ntsc.steps = 0;
    vm_ntsc.sp = 0x51FE_0800;
    vm_ntsc.d[0] = 2;
    vm_ntsc.pc = 0x5750_EE35;
    vm_ntsc.run_until(0x5740_0722, 16).unwrap();
    assert_eq!(vm_ntsc.d[0], 0x90);
    assert_eq!(vm_ntsc.d[1], 6);
    let bh_ptr = vm_ntsc.read_u32(vm_ntsc.sp + 0x0C).unwrap();
    let bh_bytes: Vec<u8> = (0..6)
        .map(|i| vm_ntsc.read_u8(bh_ptr + i).unwrap())
        .collect();
    assert_eq!(bh_bytes, vec![0x10, 0x08, 0x07, 0x63, 0xC0, 0x30]);
    vm_ntsc.d[0] = 0x0960;
    vm_ntsc.pc = vm_ntsc.read_u32(vm_ntsc.sp).unwrap();
    vm_ntsc.steps = 0;
    vm_ntsc.run_until(0x5740_0722, 16).unwrap();
    assert_eq!(vm_ntsc.d[0], 0x84);
    assert_eq!(vm_ntsc.d[1], 4);
    let aud_ptr = vm_ntsc.read_u32(vm_ntsc.sp + 0x0C).unwrap();
    let aud_bytes: Vec<u8> = (0..4)
        .map(|i| vm_ntsc.read_u8(aud_ptr + i).unwrap())
        .collect();
    assert_eq!(aud_bytes, vec![0x03, 0x80, 0x00, 0x22]);
    vm_ntsc.d[0] = 0x0960;
    vm_ntsc.pc = vm_ntsc.read_u32(vm_ntsc.sp).unwrap();
    vm_ntsc.steps = 0;
    vm_ntsc.run_until(0x5750_EE3C, 16).unwrap();
    assert_eq!(vm_ntsc.sp, 0x51FE_0800);
    assert_eq!(vm_ntsc.d[0], 0);

    // Execute front-panel [CH UP]+[CH DOWN] ATP_START skip (0x5750_7927 -> epilogue 0x5750_7B86 with D0 = 0)
    vm_ntsc.steps = 0;
    vm_ntsc.d[0] = 0xDEAD_BEEF;
    vm_ntsc.pc = 0x5750_7927;
    vm_ntsc.run_until(0x5750_7B86, 8).unwrap();
    assert_eq!(vm_ntsc.pc, 0x5750_7B86);
    assert_eq!(vm_ntsc.d[0], 0);

    vm_ntsc.steps = 0;
    vm_ntsc.sp = 0x51FE_0800;
    vm_ntsc.write_u32(vm_ntsc.sp, 0x577F_BEEF).unwrap();
    vm_ntsc.d[0] = 0;
    vm_ntsc.pc = 0x570C_E4F2;
    vm_ntsc.run_until(0x577F_BEEF, 16).unwrap();
    assert_eq!(vm_ntsc.d[0], 0x09C4);

    for stub_va in [
        0x570D_C732u32,
        0x5706_34B2,
        0x5706_34F7,
        0x570A_C466,
        0x576A_F438,
        0x5771_AB1B,
        0x5751_7163,
    ] {
        vm_ntsc.steps = 0;
        vm_ntsc.sp = 0x51FE_0800;
        vm_ntsc.write_u32(vm_ntsc.sp, 0x577F_BEEF).unwrap();
        vm_ntsc.d[0] = 0xDEAD_BEEF;
        vm_ntsc.pc = stub_va;
        vm_ntsc.run_until(0x577F_BEEF, 16).unwrap();
        assert_eq!(vm_ntsc.d[0], 0);
    }

    vm_ntsc.steps = 0;
    vm_ntsc.pc = 0x570D_6630;
    vm_ntsc.run_until(0x570D_669F, 4).unwrap();
    vm_ntsc.steps = 0;
    vm_ntsc.pc = 0x570D_7C07;
    vm_ntsc.run_until(0x570D_7C1A, 8).unwrap();
    vm_ntsc.steps = 0;
    vm_ntsc.pc = 0x570D_7CE1;
    vm_ntsc.run_until(0x570D_7CF3, 8).unwrap();

    vm_ntsc.steps = 0;
    vm_ntsc.sp = 0x51FE_0800;
    vm_ntsc.d[0] = 0xDEAD_BEEF;
    vm_ntsc.pc = 0x5772_4096;
    vm_ntsc.run_until(0x5772_409D, 16).unwrap();
    assert_eq!(vm_ntsc.d[0], 0);

    vm_ntsc.steps = 0;
    vm_ntsc.pc = 0x5772_5518;
    vm_ntsc.run_until(0x5772_551B, 4).unwrap();
    assert_eq!(vm_ntsc.d[0] & 0xFFFF, 0x4FB3);

    // MAIN3060NP (1 site @ 0x570DF34A) synthetic + optional real binary verification
    let (_, build_3060) = find_main_build_by_size(EXPECTED_MAIN_3060NP_SIZE).unwrap();
    assert_eq!(build_3060.region_localization_sites.len(), 1);
    let mut main_3060 = vec![0xFFu8; EXPECTED_MAIN_3060NP_SIZE];
    main_3060[0x570D_F34A - AM34_MAIN_BASE_VA as usize..][..5]
        .copy_from_slice(&[0xCF, 0xF2, 0xF8, 0xFE, 0xE8]);
    let orig_3060_sha = inspect_main_localization(&main_3060).unwrap().sha256;
    let p3060 =
        patch_main_localization_in_place(&mut main_3060, LocalizationPatchMode::M6930LbBrazil)
            .unwrap();
    assert_eq!(p3060.overall_state, "M6930LB_BRAZIL");
    let mut vm_3060 = Mn103Vm::new();
    vm_3060.map_segment(AM34_MAIN_BASE_VA, main_3060.clone(), false);
    vm_3060.map_segment(0x51FE_0000, vec![0u8; 0x1000], true);
    vm_3060.sp = 0x51FE_0800;
    vm_3060.write_u32(vm_3060.sp, 0x577F_BEEF).unwrap();
    vm_3060.d[0] = 0xDEAD_BEEF;
    vm_3060.pc = 0x570D_F34A;
    vm_3060.run_until(0x577F_BEEF, 16).unwrap();
    assert_eq!(vm_3060.d[0], 0);
    let r3060 =
        patch_main_localization_in_place(&mut main_3060, LocalizationPatchMode::FactoryOriginal)
            .unwrap();
    assert_eq!(r3060.sha256, orig_3060_sha);

    let real_3060_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../firmware_and_images/extracted_partitions/frm_unpacked/main3060np_unpacked.bin",
    );
    if real_3060_path.is_file() {
        let mut real_buf = std::fs::read(&real_3060_path).unwrap();
        assert_eq!(
            inspect_main_localization(&real_buf).unwrap().overall_state,
            "FACTORY"
        );
        // Prove MAIN3060NP factory tim_iic_init (0x570AD8A2) + tim_Video_sel_set_all Encoder path
        // synthesize exact 6-byte Rohm BH7629KS2 (0x90) frame [0x10, 0x08, 0x07, 0x63, 0xC0, 0x30]
        let mut vm_real30 = Mn103Vm::new();
        vm_real30.map_segment(AM34_MAIN_BASE_VA, real_buf.clone(), false);
        vm_real30.map_segment(0x5203_2000, vec![0xAAu8; 0x2000], true);
        vm_real30.map_segment(0x51FE_0000, vec![0u8; 0x1000], true);
        vm_real30.sp = 0x51FE_0800;
        vm_real30.write_u32(vm_real30.sp, 0x577F_BEEF).unwrap();
        vm_real30.pc = 0x570A_D8A2;
        vm_real30.run_until(0x577F_BEEF, 512).unwrap();
        vm_real30.steps = 0;
        vm_real30.write_u8(0x5203_2DA0, 0x10).unwrap();
        vm_real30.sp = 0x51FE_07C0;
        vm_real30.write_u32(vm_real30.sp, 0x577F_BEEF).unwrap();
        vm_real30.d[0] = vm_real30.sp + 0x27;
        vm_real30.d[1] = vm_real30.sp + 0x26;
        vm_real30.pc = 0x570A_D882;
        vm_real30.run_until(0x577F_BEEF, 64).unwrap();
        vm_real30.steps = 0;
        vm_real30.pc = 0x570A_D676;
        vm_real30.run_until(0x570A_D6D3, 64).unwrap();
        let synth_bh6: Vec<u8> = (0..6)
            .map(|i| vm_real30.read_u8(vm_real30.sp + 0x31 + i * 2).unwrap())
            .collect();
        assert_eq!(synth_bh6, vec![0x10, 0x08, 0x07, 0x63, 0xC0, 0x30]);

        let p =
            patch_main_localization_in_place(&mut real_buf, LocalizationPatchMode::M6930LbBrazil)
                .unwrap();
        assert_eq!(p.overall_state, "M6930LB_BRAZIL");
    }
}
