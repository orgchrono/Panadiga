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
}
