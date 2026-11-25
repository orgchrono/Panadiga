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
    assert_eq!(build_3410.region_localization_sites.len(), 11);
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
    // Sites 8-10 (added 2026-11-25): config1 default, cav_boot_ntsc, boot_pal_ntsc_force
    main_3410[0x57AE_FBAC - AM34_MAIN_BASE_VA as usize..][..1].copy_from_slice(&[0x00]);
    main_3410[0x574F_6F2D - AM34_MAIN_BASE_VA as usize..][..7]
        .copy_from_slice(&[0xF8, 0xBB, 0x13, 0xA3, 0xC9, 0x03, 0x88]);
    main_3410[0x5750_41FF - AM34_MAIN_BASE_VA as usize..][..2].copy_from_slice(&[0xC9, 0x1B]);
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

    // Patched IC3001 I2C 0x94 helper returns D0 = 0
    let mut vm_iic = Mn103Vm::new();
    vm_iic.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
    vm_iic.map_segment(0x51FE_0000, vec![0u8; 0x1000], true);
    vm_iic.sp = 0x51FE_0800;
    vm_iic.write_u32(vm_iic.sp, 0x577F_BEEF).unwrap();
    vm_iic.d[0] = 0xDEAD_BEEF;
    vm_iic.pc = 0x570D_625E;
    vm_iic.run_until(0x577F_BEEF, 16).unwrap();
    assert_eq!(vm_iic.d[0], 0);

    // Patched FPC / AutoClock branches land on exact join VAs
    for (entry, target) in [
        (0x5706_13C7u32, 0x5706_13F5u32),
        (0x5706_B337, 0x5706_B35B),
        (0x574F_659D, 0x574F_6600),
    ] {
        let mut vm_br = Mn103Vm::new();
        vm_br.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
        vm_br.pc = entry;
        vm_br.step().unwrap();
        assert_eq!(vm_br.pc, target);
    }

    // Patched pre-WakeupReason FPC 0xF9 GATE=15 skip: clears d0 and steps to 0x574F_4BFC
    let mut vm_gate15 = Mn103Vm::new();
    vm_gate15.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
    vm_gate15.d[0] = 0xDEAD_BEEF;
    vm_gate15.pc = 0x574F_4BF5;
    vm_gate15.run_until(0x574F_4BFC, 16).unwrap();
    assert_eq!(vm_gate15.pc, 0x574F_4BFC);
    assert_eq!(vm_gate15.d[0], 0);

    // Patched SYS_CNTL_PAL_NTSC_Change & AVOUT_V_Mute force d2=0 (NTSC 60Hz + DACOUT1 unmuted)
    for (entry, expected_next_pc) in [(0x5750_BD6Au32, 0x5750_BD6Cu32), (0x5713_09CF, 0x5713_09D1)]
    {
        let mut vm_d2 = Mn103Vm::new();
        vm_d2.map_segment(AM34_MAIN_BASE_VA, main_3410.clone(), false);
        vm_d2.d[0] = 1;
        vm_d2.d[2] = 0xDEAD_BEEF;
        vm_d2.pc = entry;
        vm_d2.step().unwrap();
        assert_eq!(vm_d2.pc, expected_next_pc);
        assert_eq!(vm_d2.d[2], 0);
    }

    let restored =
        patch_main_localization_in_place(&mut main_3410, LocalizationPatchMode::FactoryOriginal)
            .unwrap();
    assert_eq!(restored.overall_state, "FACTORY");
    assert_eq!(restored.sha256, orig_3410_sha);

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
        let p =
            patch_main_localization_in_place(&mut real_buf, LocalizationPatchMode::M6930LbBrazil)
                .unwrap();
        assert_eq!(p.overall_state, "M6930LB_BRAZIL");
    }
}
