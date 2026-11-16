//! DTO and tuple helper definitions for the Panasonic DIGA Model Database (SSOT).

use serde::{Deserialize, Serialize};

/// Strongly-typed profile entry in the internal Panasonic DIGA Model Database (SSOT).
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

pub(super) type ModelSpecRow = (
    &'static str,
    &'static str,
    u32,
    &'static str,
    &'static str,
    &'static str,
);

pub(super) const fn model_spec(
    model_id: &'static str,
    series: &'static str,
    flash_size_mb: u32,
    nvram_base_hex: &'static str,
    sector_geometry: &'static str,
    soc_name: &'static str,
) -> ModelSpecRow {
    (
        model_id,
        series,
        flash_size_mb,
        nvram_base_hex,
        sector_geometry,
        soc_name,
    )
}
