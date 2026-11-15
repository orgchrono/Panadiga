//! Structured telemetry DTOs for raw SD card / image auditing.

use serde::{Deserialize, Serialize};

use super::super::verify::SdUnbrickCheck;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MbrPartitionRecord {
    pub slot: u8,
    pub boot_flag: u8,
    pub partition_type: u8,
    pub type_name: String,
    pub lba_start: u32,
    pub sector_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FatBpbReport {
    pub oem_name: String,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub cluster_size_bytes: u32,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entry_count: u16,
    pub total_sectors: u32,
    pub media_descriptor: u8,
    pub sectors_per_fat: u32,
    pub hidden_sectors: u32,
    pub fat_type_label: String,
    pub volume_label: String,
    pub data_clusters: u32,
    pub computed_fat_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootDirFileReport {
    pub name_8_3: String,
    pub attr: u8,
    pub hidden: bool,
    pub system: bool,
    pub read_only: bool,
    pub first_cluster: u16,
    pub file_size: u32,
    pub has_preceding_lfn: bool,
    pub chain_clusters: Vec<u16>,
    pub chain_contiguous: bool,
    pub chain_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdDiskInspectionReport {
    pub target: String,
    pub partition_scheme: String,
    pub mbr_partitions: Vec<MbrPartitionRecord>,
    pub active_partition_lba: Option<u64>,
    pub bpb: Option<FatBpbReport>,
    pub digadiag_entry: Option<RootDirFileReport>,
    pub pana_sd_entry: Option<RootDirFileReport>,
    pub root_files_sample: Vec<String>,
    pub digadiag_payload_verified: bool,
    pub overall_status: String,
    pub checks: Vec<SdUnbrickCheck>,
}
