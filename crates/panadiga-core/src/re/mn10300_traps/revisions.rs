//! Trap 7 cross-revision firmware matcher (`DRV_decomp` vs `EH55_DRV_S211` vs `BOOT`/`PROG061`).

use super::{
    DRV_DECOMP_CLI_TABLE_VA, DRV_DECOMP_PASS_HANDLER_VA, DRV_S211_PASS_HANDLER_VA,
    FirmwareRevisionProfile,
};

/// Trap 7: distinguishes `DRV_decomp` (`0x4013420a`) vs `EH55_DRV_S211` (`0x40130efa`) vs `BOOT`/`PROG061`.
pub fn identify_firmware_revision(data: &[u8]) -> FirmwareRevisionProfile {
    let has_slogon = data
        .windows(8)
        .any(|w| w == b"SLOGON\0\0" || w == b"MMODE\0\0\0");
    let has_s211 = data.windows(6).any(|w| w == b"R6-MV4") || data.windows(4).any(|w| w == b"S211");
    if has_slogon && data.len() < 0x18_0000 {
        return FirmwareRevisionProfile {
            stage: "DRV".to_string(),
            revision: "DRV_decomp".to_string(),
            pass_handler_va: Some(DRV_DECOMP_PASS_HANDLER_VA),
            cli_table_va: Some(DRV_DECOMP_CLI_TABLE_VA),
        };
    }
    if has_s211 || data.len() == 0x20_0000 {
        return FirmwareRevisionProfile {
            stage: "DRV".to_string(),
            revision: "EH55_DRV_S211".to_string(),
            pass_handler_va: Some(DRV_S211_PASS_HANDLER_VA),
            cli_table_va: None,
        };
    }
    if data.windows(14).any(|w| w == b"Loader ver0.61") {
        return FirmwareRevisionProfile {
            stage: "BOOT_OR_PROG061".to_string(),
            revision: "ver0.61".to_string(),
            pass_handler_va: None,
            cli_table_va: None,
        };
    }
    FirmwareRevisionProfile {
        stage: "UNKNOWN".to_string(),
        revision: "generic".to_string(),
        pass_handler_va: None,
        cli_table_va: None,
    }
}
