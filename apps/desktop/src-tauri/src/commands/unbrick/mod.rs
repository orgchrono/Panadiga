//! Thin Tauri IPC shell for the Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Center.

#![deny(unsafe_code)]

mod dvd_creator;
mod frm_patch;
mod hardware_profiles;
mod media_emu;
mod region_calibration;
mod sd_creator;
mod sd_harden;
mod tbc_lab;
mod vault_session;

pub use dvd_creator::*;
pub use frm_patch::*;
pub use hardware_profiles::*;
pub use media_emu::*;
pub use region_calibration::*;
pub use sd_creator::*;
pub use sd_harden::*;
pub use tbc_lab::*;
pub use vault_session::*;

#[cfg(test)]
mod tests {
    use super::*;
    use panadiga_core::constants::PANA_DIGADIAG_FRM_SIZE;

    #[test]
    fn rsp_plan_and_nor_stub_ipc_commands_work() {
        let plan = patch_main_passthrough_command(
            "rsp_plan".to_string(),
            None,
            None,
            None,
            Some("0x50401F3C".to_string()),
        )
        .unwrap();
        assert_eq!(plan["handshake_hex"], "0e0402");
        assert_eq!(plan["callback_write_frame"], "$M515B2E40,4:3c1f4050#95");

        let stub = build_nor_dumper_stub_command(
            Some("0x84000000".to_string()),
            Some("0x10000".to_string()),
            Some(4096),
            None,
            None,
            Some(true),
        )
        .unwrap();
        assert_eq!(stub["output_bytes"], PANA_DIGADIAG_FRM_SIZE);
        assert_eq!(stub["wrapped_digadiag"], true);
        assert_eq!(stub["load_addr"], "0x50400000");

        let boot = run_matshita_boot_emu("unlock_disc_frm".to_string()).unwrap();
        assert!(boot.startup_succeeded);
        assert!(boot.unlock_media_verified);
        assert_eq!(boot.jump_target_va_hex.as_deref(), Some("0x50400000"));

        let inj = inject_matshita_atapi_write(
            Some("0x50400000".to_string()),
            Some(0),
            Some(2),
            Some(true),
            "deadbeef".to_string(),
        )
        .unwrap();
        assert_eq!(inj["record"]["len_bytes"], 4);

        let ipl_err = run_matshita_serial_cmd("WW 0x84000000 0x11223344".to_string(), Some(true));
        assert!(ipl_err.unwrap_err().contains("E05"));

        assert!(inspect_sd_disk_command("   ".to_string()).is_err());
    }
}
