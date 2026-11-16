//! Panasonic All-Line Infrared (IR) Engine & Dynamic Keymap SSOT (Android APK & Test Builds).
//!
//! Compiles into `panadiga-core` under `target_os = "android"` and `cfg(test)` so desktop
//! executables remain free of IR-blaster UI/catalog overhead while sharing canonical Kaseikyo
//! math with `crypto::pana_fw_go`.

pub mod adapter;
pub mod catalog;
pub mod classify;
pub mod kaseikyo;
pub mod keymap;
pub mod macros;
pub mod types;

#[cfg(test)]
mod tests;

pub use adapter::{
    IrPrepareParams, encode_pattern, format_sysfs_payload, is_carrier_supported,
    is_cycle_register_mcu_driver, is_period_dividing_fpga_driver, prepare_transmission,
    recommended_timing_unit, resolve_carrier,
};
pub use catalog::{all_codesets, catalog_stats, find_codeset, search_codesets};
pub use kaseikyo::{
    CARRIER_MATSUSHITA_HZ, KASEIKYO_PRONTO_WORD_COUNT, KASEIKYO_TRANSITION_COUNT,
    decode_flipper_kaseikyo, decode_kaseikyo_from_pronto, effective_subdevice,
    format_kaseikyo_wire_hex, parse_pronto_hex, synthesize_kaseikyo_pattern,
    synthesize_kaseikyo_pronto,
};
pub use keymap::{
    assemble_profile_from_codeset, export_irplus_xml, import_flipper_ir, import_irplus_xml,
};
pub use macros::verified_service_macros;
pub use types::{
    CatalogCommandSpec, DynamicKeymapProfile, DynamicRemoteKey, IrDeviceInfo, IrFrequencyRange,
    IrMacroRiskLevel, IrMacroStep, IrPreparedSignal, IrTimingUnit, KeyProtocol, KeySection,
    KeyTone, PanasonicCatalogCodeset, PanasonicCatalogSummary, PanasonicCategory, VerifiedIrMacro,
};
