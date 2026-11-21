//! DTO definitions for the Panasonic DIGA Model Catalog projected from `panadiga-profiles`.

use serde::{Deserialize, Serialize};

/// Strongly-typed profile entry projected from the validated v2 profile registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigaModelProfileDto {
    pub model_id: String,
    pub series: String,
    pub flash_size_mb: u32,
    pub flash_size_bytes: u64,
    pub nvram_base_hex: String,
    pub sector_geometry: String,
    pub soc_name: String,
}
