#![deny(unsafe_code)]

//! Panasonic `.FRM` Container Builders (`PANA_DVD.FRM`, `PANA__SD.FRM`, and `DIGADIAG.FRM`)

mod pack;
mod types;
mod verify;

pub use pack::{build_digadiag_frm, build_digadiag_frm_with_algo, pack_frm, pack_frm_with_key};
pub use types::{FrmContainerMode, FrmPackModule, FrmPackReport, PackedFrmModuleSummary};
pub use verify::{
    convert_pana_dvd_to_factory_sd, unpack_dvd_frm_to_modules, verify_packed_frm_roundtrip,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::resolve_workspace_root;
    use crate::nvram::calculate_hashes;

    #[test]
    fn test_verify_packed_frm_roundtrip_for_sd_sd48_and_dvd() {
        let mut main_mod = FrmPackModule::new("MAIN3410PQ", vec![0xAB; 64]);
        main_mod.target_flag = 1;
        let mut prog_mod = FrmPackModule::new("PROG061 ", vec![0x11; 32]);
        prog_mod.target_flag = 0;

        for mode in [
            FrmContainerMode::Sd,
            FrmContainerMode::Sd48,
            FrmContainerMode::Dvd,
            FrmContainerMode::Dvd64B,
        ] {
            let (mut image, _report) = pack_frm(&[main_mod.clone(), prog_mod.clone()], mode, None)
                .expect("pack container");
            let verified = verify_packed_frm_roundtrip(&image, mode, 2).expect("verify roundtrip");
            assert_eq!(verified.len(), 2);
            assert_eq!(verified[0].target_flag, 1);
            assert!(verified[0].list_sum_valid);

            let last = image.len() - 1;
            image[last] ^= 0x55;
            assert!(verify_packed_frm_roundtrip(&image, mode, 2).is_err());
        }
    }

    #[test]
    fn test_pack_and_unpack_dvd_with_non_default_catalog_key() {
        use super::super::feistel::{PANA_DVD_KEY_CATALOG, derive_round_key_from_custom_key_8b};

        let mut m_prog = FrmPackModule::new("PROG071 ", vec![0x33u8; 64]);
        m_prog.version_u16 = 0x0701;
        m_prog.target_flag = 1;
        let mut m_main = FrmPackModule::new("MAIN3360NA", vec![0x77u8; 128]);
        m_main.version_u16 = 0x3360;

        let alt_key = derive_round_key_from_custom_key_8b(&PANA_DVD_KEY_CATALOG[1].1);
        let (dvd_bytes, _) =
            pack_frm_with_key(&[m_prog, m_main], FrmContainerMode::Dvd, None, &alt_key).unwrap();

        let verified = verify_packed_frm_roundtrip(&dvd_bytes, FrmContainerMode::Dvd, 2).unwrap();
        assert_eq!(verified[0].module_name, "PROG071 ");
        assert_eq!(verified[1].module_name, "MAIN3360NA");

        let unpacked = unpack_dvd_frm_to_modules(&dvd_bytes).unwrap();
        assert_eq!(unpacked.len(), 2);
        assert_eq!(unpacked[0].payload, vec![0x33u8; 64]);
        assert_eq!(unpacked[1].payload, vec![0x77u8; 128]);
    }

    #[test]
    fn test_convert_pana_dvd_to_factory_sd_synthetic_and_workspace_roundtrip() {
        let mut m_drv = FrmPackModule::new("DRV S211", vec![0x12u8; 64]);
        m_drv.version_u16 = 0x975E;
        let mut m_prog = FrmPackModule::new("PROG061 ", vec![0x34u8; 32]);
        m_prog.version_u16 = 0x1202;
        m_prog.target_flag = 1;
        let mut m_main = FrmPackModule::new("MAIN3060NP", vec![0x56u8; 96]);
        m_main.version_u16 = 0x96AA;
        let mods = vec![m_drv, m_prog, m_main];

        let (dvd_frm, _) = pack_frm(&mods, FrmContainerMode::Dvd, None).unwrap();
        let (expected_sd, _) = pack_frm(&mods, FrmContainerMode::Sd, None).unwrap();
        let (converted_sd, _) = convert_pana_dvd_to_factory_sd(&dvd_frm).unwrap();
        assert_eq!(converted_sd, expected_sd);

        let ws_dvd = resolve_workspace_root().join("firmware_and_images/PANA_DVD.FRM");
        if ws_dvd.is_file() {
            let dvd_real = std::fs::read(&ws_dvd).unwrap();
            let (sd_real, _) = convert_pana_dvd_to_factory_sd(&dvd_real).unwrap();
            assert_eq!(sd_real.len(), 7_533_792);
            let (_, sha) = calculate_hashes(&sd_real);
            assert_eq!(
                sha,
                "edbe9ea8ec8a4905c5c891d37484205e2897c12a50f73d272a268ebe91bb7a1a"
            );
        }
    }
}
