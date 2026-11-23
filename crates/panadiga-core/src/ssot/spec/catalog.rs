//! Data-driven loader for the shared SSOT hardware catalog (`hardware_catalog.json`)
//! and canonical default hardware catalog labels (`hardware_catalog_labels.json`).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

const RAW_HARDWARE_CATALOG_JSON: &str = include_str!("../hardware_catalog.json");
const RAW_PT_BR_LOCALE_JSON: &str = include_str!("../hardware_catalog_labels.json");

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CatalogPinEntry {
    pub key: String,
    pub manual_pin: u8,
    pub physical_pin: u8,
    pub pico_gpio: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CatalogBreakInEntry {
    pub key: String,
    pub mode: u8,
    pub badge_variant: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CatalogFlashEntry {
    pub key: String,
    pub size_mb: u32,
    pub microsector_range: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CatalogCommandEntry {
    pub id: String,
    pub category: String,
    pub syntax: String,
    pub default_arg: Option<String>,
    #[serde(default)]
    pub options: Vec<[String; 2]>,
    #[serde(default)]
    pub dangerous: bool,
    pub timeout: String,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct CatalogMemoryRegionBlueprintEntry {
    pub key: String,
    pub start: String,
    pub end: String,
    pub size_kb: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CatalogSystemMemoryRegionEntry {
    pub name: String,
    pub start: String,
    pub end: String,
    pub size_kb: usize,
    pub perms: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct HardwareCatalog {
    pub timeouts_ms: BTreeMap<String, u64>,
    pub pinout: Vec<CatalogPinEntry>,
    pub breakin_modes: Vec<CatalogBreakInEntry>,
    pub flash_geometries: Vec<CatalogFlashEntry>,
    pub commands: Vec<CatalogCommandEntry>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub memory_region_blueprints: Vec<CatalogMemoryRegionBlueprintEntry>,
    pub system_memory_regions: Vec<CatalogSystemMemoryRegionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LocalePinEntry {
    pub pin_label: String,
    pub pico_pin: String,
    pub signal_name: String,
    pub direction: String,
    pub voltage: String,
    pub critical_note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LocaleBreakInEntry {
    pub label: String,
    pub badge: String,
    pub description: String,
    pub signal_diagram: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LocaleFlashEntry {
    pub chip_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LocaleCommandEntry {
    pub label: String,
    pub description: String,
    pub arg_label: Option<String>,
    pub arg_placeholder: Option<String>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LocaleSsotSection {
    pub pins: BTreeMap<String, LocalePinEntry>,
    #[serde(rename = "breakinModes")]
    pub breakin_modes: BTreeMap<String, LocaleBreakInEntry>,
    #[serde(rename = "flashGeometries")]
    pub flash_geometries: BTreeMap<String, LocaleFlashEntry>,
    pub commands: BTreeMap<String, LocaleCommandEntry>,
}

#[derive(Debug, Deserialize)]
struct LocaleFileRoot {
    ssot: LocaleSsotSection,
}

static CATALOG_CACHE: OnceLock<HardwareCatalog> = OnceLock::new();
static LOCALE_CACHE: OnceLock<LocaleSsotSection> = OnceLock::new();

/// Loads and caches the shared technical hardware & CLI catalog (`hardware_catalog.json`).
pub(super) fn hardware_catalog() -> &'static HardwareCatalog {
    CATALOG_CACHE.get_or_init(|| {
        serde_json::from_str(RAW_HARDWARE_CATALOG_JSON).expect("Valid ssot/hardware_catalog.json")
    })
}

/// Loads and caches the canonical `pt-BR` SSOT localization table used for DTO text.
pub(super) fn pt_br_ssot_locale() -> &'static LocaleSsotSection {
    LOCALE_CACHE.get_or_init(|| {
        let root: LocaleFileRoot =
            serde_json::from_str(RAW_PT_BR_LOCALE_JSON).expect("Valid pt-BR.json SSOT table");
        root.ssot
    })
}
