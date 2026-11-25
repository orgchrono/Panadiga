#![deny(unsafe_code)]

//! Declarative SSOT configuration types for `DigaOS` (bare-metal Panasonic DIGA OS).
//!
//! Defines declarative menus, visual/OSD themes, max-hardware-power tuning profiles,
//! verified MMIO write allow-lists, and multi-SoC capability targets (`AM34`, `AM33-2`,
//! `UniPhier`, `SPARC v8 DoMiNo`), projected deterministically into C11 (`digaos_ssot.h`)
//! and TypeScript (`digaos.ts`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::hex_serde::{hex_u16, hex_u32};

/// Auditable MMIO write range permitted for a `DigaOS` hardware power profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigaOsMmioRangeSpec {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub start: u32,
    pub size: u32,
    pub note: String,
}

/// Declarative menu item inside a `DigaOS` interactive/OSD menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigaOsMenuItemSpec {
    pub id: String,
    pub label_key: String,
    pub command: String,
    pub kind: u8,
    pub min_safety_tier: u8,
    pub requires_unlock: bool,
}

/// Declarative `DigaOS` menu group containing ordered menu items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigaOsMenuSpec {
    pub id: String,
    pub title_key: String,
    pub items: Vec<DigaOsMenuItemSpec>,
}

/// Declarative RGB565 visual palette & OSD geometry preset for `DigaOS`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigaOsThemeSpec {
    pub id: String,
    pub name_key: String,
    #[serde(with = "hex_u16")]
    #[schemars(with = "String")]
    pub bg_rgb565: u16,
    #[serde(with = "hex_u16")]
    #[schemars(with = "String")]
    pub fg_rgb565: u16,
    #[serde(with = "hex_u16")]
    #[schemars(with = "String")]
    pub accent_rgb565: u16,
    #[serde(with = "hex_u16")]
    #[schemars(with = "String")]
    pub warn_rgb565: u16,
    #[serde(with = "hex_u16")]
    #[schemars(with = "String")]
    pub danger_rgb565: u16,
    pub osd_font_mode: u8,
    pub overscan_px: u8,
    pub contrast_level: u8,
}

/// Declarative hardware-power tuning preset (TBC/AIOF, AVDD gain, SATA UDMA, DRAM, UART)
/// bound to explicit safety barrier flags and audited MMIO write ranges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigaOsPowerProfileSpec {
    pub id: String,
    pub name_key: String,
    pub soc_core_id: String,
    pub tbc_mode: u8,
    pub aiof_fs0_mode: u8,
    pub avdd_gain_preset: u8,
    pub sata_udma_mode: u8,
    pub dram_refresh_preset: u8,
    pub uart_baud: u32,
    pub unlock_required: bool,
    pub lethal_mmio_guard: bool,
    pub ipl_ro_guard: bool,
    #[serde(default)]
    pub allowed_mmio_writes: Vec<DigaOsMmioRangeSpec>,
}

/// Multi-SoC capability & bring-up readiness entry projected to C11 `DigaOS` HAL guards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigaOsSocTargetSpec {
    pub soc_core_id: String,
    pub soc_id: String,
    pub isa: String,
    pub is_big_endian: bool,
    pub mmio_verified: bool,
    pub is_primary_executable: bool,
    pub logic_voltage_mv: u32,
}

/// Root declarative `DigaOS` SSOT configuration bundle (`profiles/digaos/default_digaos_config.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigaOsConfigSpec {
    pub schema_version: String,
    pub prompt: String,
    pub menus: Vec<DigaOsMenuSpec>,
    pub themes: Vec<DigaOsThemeSpec>,
    pub power_profiles: Vec<DigaOsPowerProfileSpec>,
    pub soc_targets: Vec<DigaOsSocTargetSpec>,
}
