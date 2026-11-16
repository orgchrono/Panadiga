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
