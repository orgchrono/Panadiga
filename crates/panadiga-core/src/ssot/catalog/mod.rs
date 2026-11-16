//! Panasonic DIGA Model Catalog and binary signature identification helpers.

mod detect;
mod profiles_auto_8mb;
mod profiles_legacy_hdmi;
mod types;

pub use detect::{detect_diga_model_from_slice, is_valid_nor_flash_size};
pub use types::DigaModelProfileDto;

/// Complete internal database of Panasonic DIGA recorders across all 4 NOR Flash geometries (2MB/4MB/8MB/16MB).
pub fn get_diga_model_catalog_dto() -> Vec<DigaModelProfileDto> {
    profiles_auto_8mb::SPECS
        .iter()
        .chain(profiles_legacy_hdmi::SPECS.iter())
        .map(
            |&(model_id, series, flash_size_mb, nvram_base_hex, sector_geometry, soc_name)| {
                DigaModelProfileDto {
                    model_id: model_id.to_string(),
                    series: series.to_string(),
                    flash_size_mb,
                    flash_size_bytes: (flash_size_mb as u64) * 1024 * 1024,
                    nvram_base_hex: nvram_base_hex.to_string(),
                    sector_geometry: sector_geometry.to_string(),
                    soc_name: soc_name.to_string(),
                }
            },
        )
        .collect()
}
