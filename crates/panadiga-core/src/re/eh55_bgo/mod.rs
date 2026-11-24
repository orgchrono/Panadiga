//! Pure read-only auditor for the real Panasonic DMR-EH55 (`Loader ver0.61`) 45-sector BGO FTL.
//!
//! Models the physical flash window `AM34_EH55_FLASH_BGO_BASE..+0x2D0000` (`SA26..SA70`,
//! `0x84130000..0x843FFFFF`) discovered via `0x440337A4` and verified live on `IC5001`.

mod audit;
mod audit_helpers;
pub mod live_apply;
pub mod norfrom_install;
pub mod repair_plan;

pub use audit::{
    audit_bgo_nor_auto, audit_bgo_nor_for_model, audit_bgo_nor_with_geometry,
    audit_eh55_bgo_nor_8mb,
};
pub use audit_helpers::{
    BgoParseError, BgoSectorGeometry, NfdDirEntry, eh55_bgo_sector_offset,
    eval_bgo_migration_index, eval_bgo_migration_target, find_nfd_dir_entry,
    is_active_bgo_p1_sector, parse_bgo_l2p_with_geometry, parse_eh55_bgo_l2p,
    resolve_nfd_file_page_pid, verify_nor_one_to_zero_compatible,
};
pub use live_apply::{
    BgoApplyStage, BgoLiveApplyProgress, BgoLiveApplyReport, BgoLiveHealCommandPlan,
    BgoLiveHealReport, BgoLiveHealRequest, BgoTvStandardNote, apply_bgo_repair_plan_live,
    heal_bgo_live_with_client, plan_live_bgo_heal_commands,
};
pub use norfrom_install::{
    PORTAO4_UNPACKER_MAGIC_LE, PORTAO4_UNPACKER_MAGIC_OFFSET, Portao4InstallReport,
    build_portao4_norfrom_cow_image, build_portao4_self_unpacking_prog061,
    unpack_portao4_self_unpacking_prog061,
};
pub use repair_plan::{
    BgoRepairOp, BgoRepairPlan, build_bgo_repair_plan, hash_bgo_target_sectors,
    validate_and_simulate_bgo_plan, validate_bgo_op_bounds,
};

use serde::{Deserialize, Serialize};

use crate::constants::{
    AM34_EH55_FLASH_BGO_BASE, AM34_EH55_FLASH_BGO_SIZE, AM34_FLASH_UNCACHED_BASE,
    SPANSION_S29JL064H_SECTOR_COUNT,
};

/// First Spansion `S29JL064H` sector index inside the EH55 `Loader ver0.61` BGO region (`0x84130000`).
pub const EH55_BGO_FIRST_SA: usize = 26;
/// Last Spansion `S29JL064H` sector index inside the EH55 `Loader ver0.61` BGO region (`0x843F0000`).
pub const EH55_BGO_LAST_SA: usize = 70;
/// Total number of 64 KiB sectors in the EH55 `Loader ver0.61` BGO region (`SA26..=SA70`).
pub const EH55_BGO_SECTOR_COUNT: usize = 45;
/// Uniform 64 KiB erase sector stride within `SA26..=SA70`.
pub const EH55_BGO_SECTOR_STRIDE: usize = 0x1_0000;
/// Maximum logical BGO page identifier (`0x0F24`, covering `[nfrom]` FAT backup pages).
pub const EH55_BGO_MAX_PID: u16 = 0x0F24;
/// Expected number of Partition 2 (`0xF201..=0xF228`) sectors in a healthy EH55 BGO volume.
pub const EH55_BGO_EXPECTED_P2_COUNT: usize = 40;

const _: () = assert!(
    (AM34_EH55_FLASH_BGO_BASE - AM34_FLASH_UNCACHED_BASE) as usize == 0x13_0000
        && AM34_EH55_FLASH_BGO_SIZE as usize == EH55_BGO_SECTOR_COUNT * EH55_BGO_SECTOR_STRIDE
        && EH55_BGO_LAST_SA < SPANSION_S29JL064H_SECTOR_COUNT
);

/// Physical role classification for a single 64 KiB BGO sector in `SA26..=SA70`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Eh55BgoSectorKind {
    Spare,
    Partition1Active,
    Partition1Migrating,
    Partition2,
    Corrupt,
}

/// Per-sector structural state reported by [`audit_eh55_bgo_nor_8mb`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Eh55BgoSectorReport {
    pub sa: usize,
    pub flash_offset: usize,
    pub phys_va: u32,
    pub sector_id: u16,
    pub kind: Eh55BgoSectorKind,
    pub migration_idx: Option<usize>,
    pub migration_target: u16,
    pub zero_marker_ok: bool,
}

/// Full structural audit of the DMR-EH55 45-sector BGO FTL inside an 8 MiB `IC5001` dump.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Eh55BgoAuditReport {
    pub healthy: bool,
    pub total_sectors: usize,
    pub p1_active_count: usize,
    pub p1_migrating_count: usize,
    pub p2_present_ids: Vec<u16>,
    pub p2_missing_ids: Vec<u16>,
    pub spare_sas: Vec<usize>,
    pub corrupt_sas: Vec<usize>,
    pub active_l2p_pages: usize,
    pub l2p_tag_mismatches: usize,
    pub fat_mirror_mismatches: usize,
    pub nfrom_file_count: usize,
    pub ifodev2_model: Option<String>,
    pub sectors: Vec<Eh55BgoSectorReport>,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::constants::resolve_workspace_root;

    #[test]
    fn migration_bitmap_and_one_to_zero_helpers_match_loader_ver061_semantics() {
        // Real SA41+0x14..+0x3A (38 B) before repair (`mig14 == 0xFFFF` at index 2 vs trailing word set).
        let mut before = hex::decode("f8fff8fff8fffcffffff0400").unwrap();
        before.resize(38, 0xFF);
        assert_eq!(eval_bgo_migration_target(&before, 0), 0xFFFF);

        // Repaired SA41+0x14..+0x3A (exact 6 pure 1->0 writes: slot 3 -> physical sector index 4 = SA29).
        let mut after = before.clone();
        after[0] = 0xF0;
        after[2] = 0xF0;
        after[4] = 0xF0;
        after[6] = 0xF8;
        after[14] = 0x04;
        after[15] = 0x00;
        assert_eq!(eval_bgo_migration_index(&after, 0), Some(3));
        assert_eq!(eval_bgo_migration_target(&after, 0), 4);

        assert_eq!(
            verify_nor_one_to_zero_compatible(&before, &after).unwrap(),
            6
        );
        assert!(verify_nor_one_to_zero_compatible(&after, &before).is_err());
    }

    #[test]
    fn optional_real_ic5001_dumps_audit_broken_and_repaired_bgo_states() {
        let ws = resolve_workspace_root();
        let dump_path = ws.join("firmware_and_images/eh55_nor_8mb_ic5001_dump.bin");
        let repaired_path = ws.join("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
        if !Path::new(&dump_path).is_file() || !Path::new(&repaired_path).is_file() {
            return;
        }

        let orig = std::fs::read(&dump_path).unwrap();
        let rep = std::fs::read(&repaired_path).unwrap();

        let broken_audit = audit_eh55_bgo_nor_8mb(&orig).unwrap();
        assert!(!broken_audit.healthy);
        assert_eq!(broken_audit.total_sectors, EH55_BGO_SECTOR_COUNT);
        assert_eq!(broken_audit.p2_missing_ids.len(), 13);

        let clean_audit = audit_eh55_bgo_nor_8mb(&rep).unwrap();
        assert!(clean_audit.healthy, "repaired BGO must pass full audit");
        assert_eq!(clean_audit.total_sectors, EH55_BGO_SECTOR_COUNT);
        assert_eq!(clean_audit.p1_active_count, 1);
        assert_eq!(clean_audit.p1_migrating_count, 0);
        assert_eq!(clean_audit.spare_sas.len(), 4);
        assert_eq!(clean_audit.p2_present_ids.len(), EH55_BGO_EXPECTED_P2_COUNT);
        assert!(clean_audit.p2_missing_ids.is_empty());
        assert_eq!(clean_audit.corrupt_sas.len(), 0);
        assert_eq!(clean_audit.active_l2p_pages, 996);
        assert_eq!(clean_audit.l2p_tag_mismatches, 0);
        assert_eq!(clean_audit.fat_mirror_mismatches, 0);
        assert_eq!(clean_audit.nfrom_file_count, 19);
        assert_eq!(clean_audit.ifodev2_model.as_deref(), Some("M6930LB"));
    }
}
