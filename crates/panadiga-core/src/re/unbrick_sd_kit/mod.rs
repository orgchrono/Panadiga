//! SD Recovery Kit Generator (`DIGADIAG.FRM` + `PANA__SD.FRM`), Media Validator,
//! and Raw SD MBR/BPB/FAT16 Disk Auditor (`inspect_sd`).
//!
//! Splits pure media verification (`verify`), pure artifact composition (`compose`),
//! sector-aligned raw disk inspection (`inspect_sd`), and thin disk I/O (`io`).

pub mod compose;
pub mod inspect_sd;
pub mod io;
pub mod verify;

pub use compose::{
    HardenedPanamon1KitArtifacts, SdKitBuildPreset, SdLoaderProfile, SdUnbrickKitReport,
    SdUnbrickKitRequest, compose_hardened_panamon1_kit,
};
pub use inspect_sd::{
    FatBpbReport, MbrPartitionRecord, RootDirFileReport, SdDiskInspectionReport,
    inspect_sd_disk_bytes, inspect_sd_reader, inspect_sd_target,
};
pub use io::{prepare_sd_unbrick_kit, verify_sd_unbrick_media};
pub use verify::{
    DualFamilyParityReport, DualFamilySlotResolution, SdKitAuditProfile, SdMediaVerificationReport,
    SdUnbrickCheck, simulate_sd_find_firm_entry_dual, verify_sd_buffers,
    verify_sd_buffers_with_profile,
};

pub(crate) fn parse_sd_hardened_profile(
    raw: &str,
    kind: &str,
    hardened_aliases: &[&str],
) -> anyhow::Result<bool> {
    let normalized = raw.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "standard" | "default") {
        return Ok(false);
    }
    if hardened_aliases.contains(&normalized.as_str()) {
        return Ok(true);
    }
    anyhow::bail!(
        "Unsupported SD {kind} '{raw}': expected standard or {}",
        hardened_aliases.join(" / ")
    )
}

#[cfg(test)]
pub(crate) fn synthetic_prog061_fixture() -> Vec<u8> {
    use crate::re::bgo_unbrick::{
        OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
        ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_EXPECTED_SIZE,
    };
    let mut prog = vec![0xFFu8; PROG061_EXPECTED_SIZE];
    prog[0..4].copy_from_slice(&[0xDC, 0x14, 0x00, 0x00]);
    prog[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2].copy_from_slice(&ORIG_SUM_BEQ);
    prog[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + 2].copy_from_slice(&ORIG_DEVINFO_BEQ);
    prog[OFF_BGO_INIT_CALL..OFF_BGO_INIT_CALL + 7].copy_from_slice(&ORIG_A2BC_BGO_CALL);
    prog[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + 20].copy_from_slice(&ORIG_A2CD_HOOK);
    prog
}
