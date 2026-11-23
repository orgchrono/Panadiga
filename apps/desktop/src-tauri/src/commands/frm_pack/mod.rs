//! Tauri IPC Commands for Panasonic `.FRM` Packing and `EXTRHEAD`/`EXTRFOOT` Unpacking.

mod ops;
mod paths;

pub use ops::*;
pub use paths::{
    ExtrheadUnpackReportDto, FrmPackModuleInputDto, PackFrmRequestDto, list_frm_containers_command,
};
pub(crate) use paths::{resolve_read_path, resolve_safe_read_path, resolve_write_path};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::constants::DEFAULT_DIGADIAG_DST_HEX;
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
            dst_addr: Some(DEFAULT_DIGADIAG_DST_HEX.to_string()),
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

    #[test]
    fn ipc_path_confinement_blocks_traversal_devices_and_protected_dirs() {
        static TEST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let safe_dir = std::env::temp_dir().join(format!(
            "panadiga_ipc_path_test_{}_{}",
            std::process::id(),
            seq
        ));
        std::fs::create_dir_all(&safe_dir).unwrap();
        let safe_file = safe_dir.join("sample.bin");
        std::fs::write(&safe_file, b"PANADIGA").unwrap();

        assert!(resolve_write_path(&safe_dir.join("out.bin").to_string_lossy()).is_ok());
        assert!(resolve_safe_read_path(&safe_file.to_string_lossy()).is_ok());

        assert!(resolve_write_path("../escape.bin").is_err());
        assert!(resolve_safe_read_path("../escape.bin").is_err());
        assert!(resolve_write_path("\\\\.\\PhysicalDrive0").is_err());
        assert!(resolve_safe_read_path("\\\\.\\PhysicalDrive0").is_err());
        assert!(resolve_write_path("bad\0path.bin").is_err());
        assert!(resolve_safe_read_path("bad\0path.bin").is_err());

        let ws_root = panadiga_core::constants::resolve_workspace_root();
        assert!(resolve_write_path(&ws_root.join(".git/config").to_string_lossy()).is_err());
        assert!(
            resolve_write_path(
                &ws_root
                    .join("profiles/schema/tamper.json")
                    .to_string_lossy()
            )
            .is_err()
        );
        #[cfg(windows)]
        assert!(resolve_write_path("C:\\Windows\\System32\\evil.bin").is_err());
        #[cfg(not(windows))]
        assert!(resolve_write_path("/etc/passwd").is_err());

        let _ = std::fs::remove_dir_all(&safe_dir);
    }
}
