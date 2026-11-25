#![deny(unsafe_code)]

//! Deterministic SSOT-driven Brazilian (`M6930LB`) localization patcher for `MAIN` firmware.
//!
//! Resolves `region_localization_sites` declaratively from `profiles/firmware_trains/*.json`
//! (`market_code` OSD unlock, `IC3001 BH7629KS2` I2C `0x94` bypass, and `MN67788RA` `FPC 0x7F`/`GATE=15`).

use std::str::FromStr;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::nvram::calculate_hashes;
use crate::re::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use crate::re::ssot_patches::{find_main_build_by_size, parse_site_pair};
use crate::ssot::registry::{BinaryPatchSiteSpec, MainBuildSpec};

/// Localization patch mode for Panasonic DIGA `MAIN` (`3060NP` / `3410PQ`) binaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalizationPatchMode {
    M6930LbBrazil,
    FactoryOriginal,
}

impl LocalizationPatchMode {
    /// Canonical lowercase mode identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::M6930LbBrazil => "m6930lb_brazil",
            Self::FactoryOriginal => "factory_original",
        }
    }
}

impl FromStr for LocalizationPatchMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "m6930lb" | "m6930lb_brazil" | "brazil" | "br" | "pal-m" | "apply" | "unlock" => {
                Ok(Self::M6930LbBrazil)
            }
            "factory" | "factory_original" | "restore" => Ok(Self::FactoryOriginal),
            other => bail!(
                "Unsupported localization mode '{other}' (expected m6930lb_brazil|factory_original)"
            ),
        }
    }
}

/// Per-site status inside an unpacked `MAIN` binary for `region_localization_sites`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainLocalizationSiteReport {
    pub name: String,
    pub description: String,
    pub va_hex: String,
    pub file_offset_hex: String,
    pub state: String,
    pub actual_hex: String,
}

/// Aggregated inspection or patch summary for SSOT `region_localization_sites`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainLocalizationReport {
    pub build_profile: String,
    pub size_bytes: usize,
    pub overall_state: String,
    pub site_count: usize,
    pub sites: Vec<MainLocalizationSiteReport>,
    pub sha256: String,
}

struct PreparedSite<'a> {
    spec_ref: &'a BinaryPatchSiteSpec,
    expect: Vec<u8>,
    replace: Vec<u8>,
}

fn resolve_localization_build(buf: &[u8]) -> Result<&'static MainBuildSpec> {
    let (_, build) = find_main_build_by_size(buf.len()).with_context(|| {
        format!(
            "Unsupported MAIN binary length {} B for SSOT localization lookup",
            buf.len()
        )
    })?;
    if build.region_localization_sites.is_empty() {
        bail!(
            "MAIN{} ({} B) has no declarative SSOT region_localization_sites",
            build.build_id,
            buf.len()
        );
    }
    Ok(build)
}

fn prepare_sites(build: &'static MainBuildSpec) -> Result<Vec<PreparedSite<'static>>> {
    let mut prepared = Vec::with_capacity(build.region_localization_sites.len());
    for site in &build.region_localization_sites {
        let (expect, replace) = parse_site_pair(site)?;
        prepared.push(PreparedSite {
            spec_ref: site,
            expect,
            replace,
        });
    }
    Ok(prepared)
}

fn as_patch_spec<'a>(build: &MainBuildSpec, entry: &'a PreparedSite<'a>) -> PatchSpec<'a> {
    PatchSpec {
        name: &entry.spec_ref.name,
        description: &entry.spec_ref.description,
        va: entry.spec_ref.va,
        base_va: build.base_va,
        expect: &entry.expect,
        replace: &entry.replace,
        required_len: Some(build.unpacked_size as usize),
        input_sha256: None,
    }
}

fn evaluate_localization_report(
    buf: &[u8],
    build: &MainBuildSpec,
    prepared: &[PreparedSite<'_>],
) -> Result<MainLocalizationReport> {
    let mut site_reports = Vec::with_capacity(prepared.len());
    let mut all_patched = true;
    let mut all_factory = true;
    for entry in prepared {
        let spec = as_patch_spec(build, entry);
        let cls = classify_patch_site(buf, &spec, "PATCHED")?;
        if cls.state != "PATCHED" {
            all_patched = false;
        }
        if cls.state != "FACTORY" {
            all_factory = false;
        }
        site_reports.push(MainLocalizationSiteReport {
            name: entry.spec_ref.name.clone(),
            description: entry.spec_ref.description.clone(),
            va_hex: format!("0x{:08X}", cls.va),
            file_offset_hex: format!("0x{:06X}", cls.file_offset),
            state: cls.state.to_string(),
            actual_hex: hex::encode(&cls.actual_bytes),
        });
    }
    let overall_state = if all_patched {
        "M6930LB_BRAZIL"
    } else if all_factory {
        "FACTORY"
    } else {
        "MIXED_OR_UNKNOWN"
    };
    let (_, sha256) = calculate_hashes(buf);
    Ok(MainLocalizationReport {
        build_profile: build.build_id.clone(),
        size_bytes: buf.len(),
        overall_state: overall_state.to_string(),
        site_count: site_reports.len(),
        sites: site_reports,
        sha256,
    })
}

/// Inspects all SSOT `region_localization_sites` on an unpacked `MAIN` binary without mutating it.
pub fn inspect_main_localization(buf: &[u8]) -> Result<MainLocalizationReport> {
    let build = resolve_localization_build(buf)?;
    let prepared = prepare_sites(build)?;
    evaluate_localization_report(buf, build, &prepared)
}

/// Atomically applies or restores all SSOT `region_localization_sites` on an unpacked `MAIN` binary.
pub fn patch_main_localization_in_place(
    buf: &mut [u8],
    mode: LocalizationPatchMode,
) -> Result<MainLocalizationReport> {
    let build = resolve_localization_build(buf)?;
    let prepared = prepare_sites(build)?;
    let action = match mode {
        LocalizationPatchMode::M6930LbBrazil => PatchApplyAction::ApplyReplace,
        LocalizationPatchMode::FactoryOriginal => PatchApplyAction::RestoreExpect,
    };
    let specs: Vec<PatchSpec<'_>> = prepared
        .iter()
        .map(|entry| as_patch_spec(build, entry))
        .collect();
    let pairs: Vec<(&PatchSpec<'_>, PatchApplyAction)> =
        specs.iter().map(|spec| (spec, action)).collect();
    apply_patch_specs_in_place(buf, &pairs)?;
    evaluate_localization_report(buf, build, &prepared)
}
