//! Tauri IPC Commands for Panasonic `.FRM` Packing and `EXTRHEAD`/`EXTRFOOT` Unpacking.

mod ops;
mod paths;

pub use ops::*;
pub use paths::{
    ExtrheadUnpackReportDto, FrmPackModuleInputDto, PackFrmRequestDto, list_frm_containers_command,
};
pub(crate) use paths::{resolve_read_path, resolve_write_path};

#[cfg(test)]
mod tests {
    use super::*;
    use panadiga_core::constants::PANA_DIGADIAG_FRM_SIZE;

    #[test]
    fn desktop_pack_and_unpack_commands_roundtrip() {
        let tmp = std::env::temp_dir().join("panadiga_tauri_frm_pack_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let payload_path = tmp.join("diag_payload.bin");
        let digadiag_out = tmp.join("DIGADIAG.FRM");
        let unpacked_out = tmp.join("diag_out.bin");
        let sd_out = tmp.join("PANA__SD.FRM");

        let payload = vec![0xA5u8; 512];
        std::fs::write(&payload_path, &payload).unwrap();

        let pack_diag = pack_frm_command(PackFrmRequestDto {
            mode: "digadiag".to_string(),
            out_path: digadiag_out.to_string_lossy().to_string(),
            modules: Vec::new(),
            input_path: Some(payload_path.to_string_lossy().to_string()),
            dst_addr: Some("0x50400000".to_string()),
            tag: Some("FVU ".to_string()),
            build_name: Some("TEST01".to_string()),
            hw_family: None,
            model_group: None,
            version_u16: None,
            target_flag: None,
        })
        .unwrap();
        assert_eq!(pack_diag.mode, "digadiag");
        assert_eq!(pack_diag.total_bytes, PANA_DIGADIAG_FRM_SIZE);

        let unpacked = unpack_extrhead_command(
            digadiag_out.to_string_lossy().to_string(),
            Some("auto".to_string()),
            Some(unpacked_out.to_string_lossy().to_string()),
        )
        .unwrap();
        assert!(unpacked.checksum_valid);
        assert!(unpacked.footer_present);
        assert_eq!(unpacked.decompressed_bytes, 512);
        assert_eq!(std::fs::read(&unpacked_out).unwrap(), payload);

        let pack_sd = pack_frm_command(PackFrmRequestDto {
            mode: "sd".to_string(),
            out_path: sd_out.to_string_lossy().to_string(),
            modules: vec![FrmPackModuleInputDto {
                name: "DIAG01".to_string(),
                path: payload_path.to_string_lossy().to_string(),
            }],
            input_path: None,
            dst_addr: None,
            tag: None,
            build_name: None,
            hw_family: Some("M6930".to_string()),
            model_group: Some("NP-309".to_string()),
            version_u16: Some("0x0061".to_string()),
            target_flag: Some(0),
        })
        .unwrap();
        assert_eq!(pack_sd.mode, "sd");
        assert_eq!(pack_sd.modules.len(), 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
