//! Panasonic DIGA Model Catalog projected directly from the v2 manifest registry (zero hardcoded tables).

mod detect;
mod types;

pub use detect::{detect_diga_model_from_slice, is_valid_nor_flash_size};
pub use types::DigaModelProfileDto;

use crate::ssot::registry::{HardwareProfileRegistry, ResolvedModelProfile};

fn project_model_dto(resolved: &ResolvedModelProfile) -> DigaModelProfileDto {
    let flash_size_bytes = resolved
        .flash_chip
        .as_ref()
        .map_or(0, |f| u64::from(f.total_bytes));
    let flash_size_mb = u32::try_from(flash_size_bytes / (1024 * 1024)).unwrap_or(0);
    let is_otp = resolved.flash_chip.as_ref().is_some_and(|f| !f.writable);

    let nvram_base_hex = resolved.nvram_uncached_va_hex.clone().unwrap_or_else(|| {
        if is_otp {
            "READ_ONLY_OTP".to_string()
        } else {
            "UNKNOWN".to_string()
        }
    });

    let sector_geometry = match (&resolved.flash_chip, &resolved.nvram_sector_range_derived) {
        (Some(chip), _) if !chip.writable => format!(
            "{} ({} Read-Only OTP PROM, No NVRAM)",
            chip.part_number,
            resolved
                .derived_sector_geometry
                .as_deref()
                .unwrap_or("1 MiB")
        ),
        (Some(chip), Some(nv_sa)) => format!(
            "{} [{}] — NVRAM {}",
            chip.part_number,
            resolved.derived_sector_geometry.as_deref().unwrap_or("NOR"),
            nv_sa
        ),
        (Some(chip), None) => format!(
            "{} [{}]",
            chip.part_number,
            resolved.derived_sector_geometry.as_deref().unwrap_or("NOR")
        ),
        (None, _) => "UNKNOWN (Flash/Layout Unverified — Inspect Only)".to_string(),
    };

    let soc_name = match (&resolved.soc_core, &resolved.soc) {
        (Some(core), Some(soc)) => format!("{} ({})", core.isa, soc.part_number),
        (Some(core), None) => core.isa.clone(),
        (None, Some(soc)) => soc.part_number.clone(),
        (None, None) => "UNKNOWN".to_string(),
    };

    DigaModelProfileDto {
        model_id: resolved.manifest.model_id.clone(),
        series: resolved.manifest.series.clone(),
        flash_size_mb,
        flash_size_bytes,
        nvram_base_hex,
        sector_geometry,
        soc_name,
    }
}

/// Complete database of Panasonic DIGA models projected dynamically from `HardwareProfileRegistry`.
pub fn get_diga_model_catalog_dto() -> Vec<DigaModelProfileDto> {
    let mut models = HardwareProfileRegistry::builtin().list_models();
    models.sort_by(|a, b| a.manifest.model_id.cmp(&b.manifest.model_id));

    let mut out = Vec::with_capacity(models.len() + 1);
    out.push(DigaModelProfileDto {
        model_id: "AUTO".to_string(),
        series: "Auto-Detect (Structural Probe & Loader Banner Fingerprinting)".to_string(),
        flash_size_mb: 8,
        flash_size_bytes: 8 * 1024 * 1024,
        nvram_base_hex: "AUTO (Probe-Verified)".to_string(),
        sector_geometry: "Probe-Detected (Derived from Resolved Flash Chip)".to_string(),
        soc_name: "Matsushita AM33/AM34 / LSI DoMiNo".to_string(),
    });
    for resolved in &models {
        out.push(project_model_dto(resolved));
    }
    out
}
