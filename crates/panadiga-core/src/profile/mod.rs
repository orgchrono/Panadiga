#![deny(unsafe_code)]

//! Dynamic target hardware geometry inference from arbitrary NOR, `.FRM`, or HDD dumps.

pub mod dynamic_target;

use serde::{Deserialize, Serialize};

use crate::ingest::{CipherFamily, ContainerFormat};

pub use dynamic_target::build_dynamic_target_profile;

/// Inferred NOR Flash physical layout and read-only IPL bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NorFlashGeometry {
    pub flash_size_bytes: u64,
    pub sector_stride_bytes: usize,
    pub bulk_sector_count: usize,
    pub ipl_readonly_bytes: u64,
}

/// Discovered BGO FTL partition window inside NOR Flash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoBounds {
    pub detected: bool,
    pub start_offset: u64,
    pub end_offset: u64,
    pub first_sa: usize,
    pub last_sa: usize,
    pub active_p1_sectors: Vec<usize>,
    pub data_sector_count: usize,
}

/// Inferred NVRAM/EEMUL sector geometry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvramGeometry {
    pub base_offset: u64,
    pub size_bytes: u64,
    pub microsector_size_bytes: usize,
    pub sector_count: usize,
    pub non_erased_bytes: usize,
}

/// Expected HDD / MEIHFS storage profile and capacity unlock capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HddExpectation {
    pub meihfs_mbr_present: bool,
    pub mirror_mbr_present: bool,
    pub clamp_site_offset: Option<usize>,
    pub min_expected_gb: u32,
    pub max_expected_gb: u32,
    pub supports_dynamic_unlock: bool,
}

/// Complete inferred target hardware & firmware profile without relying solely on static catalogs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicTargetProfile {
    pub label: String,
    pub model: String,
    pub container: ContainerFormat,
    pub cipher_family: CipherFamily,
    pub nor_flash: NorFlashGeometry,
    pub bgo_bounds: BgoBounds,
    pub nvram: NvramGeometry,
    pub hdd: HddExpectation,
    pub notes: Vec<String>,
}
