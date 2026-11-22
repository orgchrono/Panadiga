#![deny(unsafe_code)]

//! Deterministic region-based interpolation for DIGA TV standards, line systems,
//! `AIOF_VMODESEL`, DVD regions, OEM markers, mains power, tuners, and regional HW codes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Regionally interpolated hardware & broadcast parameters derived from `region_suffix`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InterpolatedRegionVariant {
    pub region_code: String,
    pub tv_standard: String,
    pub video_line_system: String,
    pub aiof_vmodesel_hex: String,
    pub dvd_region_code: u8,
    pub oem_marker: String,
    pub ac_mains_spec: String,
    pub tuner_system: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpolated_hw_code: Option<String>,
}

const KNOWN_HW_SUFFIXES: &[&str] = &[
    "VEB", "VEG", "CEG", "SEG", "DEB", "DEG", "VPC", "GCS", "EB", "EG", "EC", "EE", "EF", "EP",
    "EN", "EX", "LB", "GN", "GL", "GZ", "GC", "VP", "PC",
];

fn derive_tv_standard(code: &str, model_id: &str) -> &'static str {
    let is_atsc = model_id.to_ascii_uppercase().starts_with("DMR-EZ");
    match code {
        "LB" | "LBS" => "PAL-M",
        "EE" => "PAL/SECAM",
        "GC" | "GCS" | "GA" | "GD" | "GT" | "GS" => "PAL/NTSC",
        "P" | "VP" | "PC" | "VPC" => {
            if is_atsc {
                "NTSC/ATSC"
            } else {
                "NTSC"
            }
        }
        _ => "PAL",
    }
}

fn derive_line_system(tv_standard: &str) -> &'static str {
    match tv_standard {
        "NTSC" | "NTSC/ATSC" | "PAL-M" => "525/59.94Hz",
        "PAL/NTSC" => "525/59.94Hz & 625/50Hz",
        _ => "625/50Hz",
    }
}

fn derive_vmodesel_hex(tv_standard: &str) -> &'static str {
    if tv_standard == "PAL-M" {
        "0x0002"
    } else if tv_standard.starts_with("NTSC") {
        "0x0000"
    } else {
        "0x0001"
    }
}

fn derive_dvd_region(code: &str) -> u8 {
    match code {
        "P" | "VP" | "PC" | "VPC" => 1,
        "GCS" | "GA" | "GD" | "GT" => 3,
        "LB" | "LBS" | "GN" | "GL" | "GZ" => 4,
        "EE" => 5,
        _ => 2,
    }
}

fn derive_oem_marker(code: &str) -> &'static str {
    match code {
        "LB" | "LBS" => "M",
        "P" | "VP" | "PC" | "VPC" => "P",
        _ => "E",
    }
}

fn derive_ac_mains(code: &str) -> &'static str {
    match code {
        "P" | "VP" | "PC" | "VPC" => "120V 60Hz",
        "LB" | "LBS" | "GC" | "GCS" => "110-240V 50/60Hz",
        _ => "220-240V 50Hz",
    }
}

fn derive_tuner_system(code: &str, model_id: &str, series: &str) -> &'static str {
    let model_up = model_id.to_ascii_uppercase();
    let series_up = series.to_ascii_uppercase();
    if model_up.starts_with("DMR-EZ") && matches!(code, "P" | "VP" | "PC" | "VPC") {
        return "ATSC + NTSC-M";
    }
    if code == "SEG"
        || model_up.starts_with("DMR-BS")
        || model_up.starts_with("DMR-XS")
        || series_up.contains("DVB-S")
    {
        return "DVB-S/S2 + PAL";
    }
    if code == "CEG" || model_up.starts_with("DMR-EX84C") || model_up.starts_with("DMR-EX93") {
        return "DVB-C/T + PAL";
    }
    if model_up.starts_with("DMR-EX")
        || model_up.starts_with("DMR-BW")
        || model_up.starts_with("DMR-BWT")
        || model_up.starts_with("DMR-XW")
    {
        return "DVB-T + PAL";
    }
    match code {
        "LB" | "LBS" => "Analog M (PAL-M)",
        "P" | "VP" | "PC" | "VPC" => "Analog M (NTSC)",
        "EE" => "Analog B/G, D/K (PAL/SECAM)",
        "GC" | "GCS" => "Analog Multi-System (PAL/NTSC)",
        _ => "Analog B/G, I (PAL)",
    }
}

fn derive_interpolated_hw_code(base_hw_code: Option<&str>, code: &str) -> Option<String> {
    let base = base_hw_code.map(str::trim).filter(|s| !s.is_empty())?;
    for suf in KNOWN_HW_SUFFIXES {
        if let Some(stem) = base.strip_suffix(suf)
            && !stem.is_empty()
        {
            return Some(format!("{stem}{code}"));
        }
    }
    if matches!(code, "LB" | "LBS") && !base.ends_with("LB") {
        return Some(format!("{base}LB"));
    }
    Some(base.to_string())
}

/// Interpolates all variable broadcast and regional hardware properties for a single `region_code`.
#[must_use]
pub fn interpolate_region_variant(
    region_code: &str,
    model_id: &str,
    series: &str,
    base_hw_code: Option<&str>,
) -> InterpolatedRegionVariant {
    let code = region_code.trim().to_ascii_uppercase();
    let tv_standard = derive_tv_standard(&code, model_id).to_string();
    let video_line_system = derive_line_system(&tv_standard).to_string();
    let aiof_vmodesel_hex = derive_vmodesel_hex(&tv_standard).to_string();
    InterpolatedRegionVariant {
        dvd_region_code: derive_dvd_region(&code),
        oem_marker: derive_oem_marker(&code).to_string(),
        ac_mains_spec: derive_ac_mains(&code).to_string(),
        tuner_system: derive_tuner_system(&code, model_id, series).to_string(),
        interpolated_hw_code: derive_interpolated_hw_code(base_hw_code, &code),
        region_code: code,
        tv_standard,
        video_line_system,
        aiof_vmodesel_hex,
    }
}

/// Expands a slash-separated `region_suffix` (e.g. `"EC/LB"`) into per-region variants.
#[must_use]
pub fn interpolate_model_regions(
    region_suffix: Option<&str>,
    model_id: &str,
    series: &str,
    base_hw_code: Option<&str>,
) -> Vec<InterpolatedRegionVariant> {
    let Some(raw) = region_suffix.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    raw.split('/')
        .map(str::trim)
        .filter(|tok| !tok.is_empty())
        .map(|tok| interpolate_region_variant(tok, model_id, series, base_hw_code))
        .collect()
}

/// Deduplicates atomic TV standards across regional variants (`"PAL/PAL-M"`, `"NTSC/PAL/SECAM"`).
#[must_use]
pub fn interpolate_composite_tv_standard(variants: &[InterpolatedRegionVariant]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for var in variants {
        for sub in var.tv_standard.split('/') {
            let trimmed = sub.trim();
            if !trimmed.is_empty() && !parts.iter().any(|p| p == trimmed) {
                parts.push(trimmed.to_string());
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// Matches an explicit regional alias query (e.g. `"DMR-EH55LB-S"`) or sole regional variant.
#[must_use]
pub fn match_queried_region_variant(
    model_query: &str,
    model_id: &str,
    variants: &[InterpolatedRegionVariant],
) -> Option<InterpolatedRegionVariant> {
    if variants.is_empty() {
        return None;
    }
    if variants.len() == 1 {
        return Some(variants[0].clone());
    }
    let query_up = model_query.trim().to_ascii_uppercase();
    let id_up = model_id.trim().to_ascii_uppercase();
    if query_up == id_up {
        return None;
    }
    let remainder = query_up
        .strip_prefix(&id_up)
        .unwrap_or(query_up.as_str())
        .trim_start_matches(['-', '_', ' ']);
    if remainder.is_empty() {
        return None;
    }

    let mut ordered: Vec<&InterpolatedRegionVariant> = variants.iter().collect();
    ordered.sort_by_key(|a| std::cmp::Reverse(a.region_code.len()));
    for var in ordered {
        let code = var.region_code.as_str();
        if remainder.starts_with(code)
            || remainder
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|tok| tok == code)
        {
            return Some(var.clone());
        }
    }
    None
}
