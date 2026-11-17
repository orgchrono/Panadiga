//! Canonical Runtime Operational Settings & Atomic Persistence (`panadiga-core`).
//!
//! Centralizes user-configurable runtime defaults in Rust (keeping physical hardware safety
//! invariants immutable: `NSW5V` GP3 Hi-Z, `XRESET` Open-Drain, IPL `SA0..SA3` read-only).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{DEFAULT_BAUD_RATE, DEFAULT_GDB_LISTEN_SOCKET, resolve_workspace_root};
use crate::logger::{LogLevel, set_level};
use crate::scan::DEFAULT_FORENSIC_KEYWORDS;

pub const ALLOWED_BAUD_RATES: &[u32] = &[
    9_600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 921_600,
];
pub const ALLOWED_NOR_BLOCK_SIZES: &[u32] = &[512, 1_024, 2_048, 4_096, 8_192];
pub const ALLOWED_LOG_LEVELS: &[&str] = &["off", "error", "warn", "info", "debug", "trace"];
pub const RESERVED_P7501_MAX_GPIO: u8 = 5;
pub const MAX_SAFE_RP2040_USER_GPIO: u8 = 28;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialRuntimeSettings {
    pub default_baud_rate: u32,
    pub gdb_listen_addr: String,
    pub diagnostic_timeout_ms: u64,
    pub telemetry_poll_interval_ms: u64,
}

impl Default for SerialRuntimeSettings {
    fn default() -> Self {
        Self {
            default_baud_rate: DEFAULT_BAUD_RATE,
            gdb_listen_addr: DEFAULT_GDB_LISTEN_SOCKET.to_string(),
            diagnostic_timeout_ms: 4_000,
            telemetry_poll_interval_ms: 1_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageRuntimeSettings {
    pub hdd_default_capacity_gb: u32,
    pub nor_dumper_block_size: u32,
    pub hex_page_bytes: u32,
    pub forensic_keywords: Vec<String>,
}

impl Default for StorageRuntimeSettings {
    fn default() -> Self {
        Self {
            hdd_default_capacity_gb: 250,
            nor_dumper_block_size: 4_096,
            hex_page_bytes: 384,
            forensic_keywords: DEFAULT_FORENSIC_KEYWORDS
                .iter()
                .map(|kw| (*kw).to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRuntimeSettings {
    pub default_ir_pico_gpio: u8,
    pub ir_catalog_search_limit: u32,
}

impl Default for IrRuntimeSettings {
    fn default() -> Self {
        Self {
            default_ir_pico_gpio: 6,
            ir_catalog_search_limit: 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchRuntimeSettings {
    pub default_split_left_pct: u8,
    pub manuals_split_initial_pct: u8,
    pub hex_split_initial_pct: u8,
    pub log_level: String,
}

impl Default for WorkbenchRuntimeSettings {
    fn default() -> Self {
        Self {
            default_split_left_pct: 40,
            manuals_split_initial_pct: 64,
            hex_split_initial_pct: 34,
            log_level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeSettings {
    pub serial: SerialRuntimeSettings,
    pub storage: StorageRuntimeSettings,
    pub ir: IrRuntimeSettings,
    pub workbench: WorkbenchRuntimeSettings,
}

impl RuntimeSettings {
    /// Returns a normalized copy trimming whitespace and filtering blank forensic keywords.
    pub fn normalized(&self) -> Self {
        let mut copy = self.clone();
        copy.serial.gdb_listen_addr = copy.serial.gdb_listen_addr.trim().to_string();
        copy.storage.forensic_keywords = copy
            .storage
            .forensic_keywords
            .iter()
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
            .collect();
        copy.workbench.log_level = copy.workbench.log_level.trim().to_ascii_lowercase();
        copy
    }

    /// Validates operational bounds and enforces non-negotiable P7501 GPIO safety invariants.
    pub fn validate(&self) -> Result<(), String> {
        if !ALLOWED_BAUD_RATES.contains(&self.serial.default_baud_rate) {
            return Err(format!(
                "Unsupported baud rate {} (allowed: {:?})",
                self.serial.default_baud_rate, ALLOWED_BAUD_RATES
            ));
        }
        validate_socket_addr_spec(&self.serial.gdb_listen_addr)?;
        if !(500..=60_000).contains(&self.serial.diagnostic_timeout_ms) {
            return Err("diagnostic_timeout_ms must be between 500 and 60000 ms".to_string());
        }
        if !(200..=30_000).contains(&self.serial.telemetry_poll_interval_ms) {
            return Err("telemetry_poll_interval_ms must be between 200 and 30000 ms".to_string());
        }

        if !(80..=2_000).contains(&self.storage.hdd_default_capacity_gb) {
            return Err("hdd_default_capacity_gb must be between 80 and 2000 GB".to_string());
        }
        if !ALLOWED_NOR_BLOCK_SIZES.contains(&self.storage.nor_dumper_block_size) {
            return Err(format!(
                "nor_dumper_block_size must be one of {:?}",
                ALLOWED_NOR_BLOCK_SIZES
            ));
        }
        if !(128..=4_096).contains(&self.storage.hex_page_bytes)
            || !self.storage.hex_page_bytes.is_multiple_of(16)
        {
            return Err(
                "hex_page_bytes must be between 128 and 4096 bytes and divisible by 16".to_string(),
            );
        }
        if self.storage.forensic_keywords.is_empty() || self.storage.forensic_keywords.len() > 32 {
            return Err("forensic_keywords must contain between 1 and 32 entries".to_string());
        }

        if self.ir.default_ir_pico_gpio <= RESERVED_P7501_MAX_GPIO
            || self.ir.default_ir_pico_gpio > MAX_SAFE_RP2040_USER_GPIO
        {
            return Err(format!(
                "GPIO {} is reserved/unsafe on P7501 (GP0..GP5 protect UART, NSW5V Hi-Z GP3, and XRESET GP5); select GP6..GP28",
                self.ir.default_ir_pico_gpio
            ));
        }
        if !(10..=500).contains(&self.ir.ir_catalog_search_limit) {
            return Err("ir_catalog_search_limit must be between 10 and 500".to_string());
        }

        if !(24..=74).contains(&self.workbench.default_split_left_pct) {
            return Err("default_split_left_pct must be between 24 and 74%".to_string());
        }
        if !(42..=76).contains(&self.workbench.manuals_split_initial_pct) {
            return Err("manuals_split_initial_pct must be between 42 and 76%".to_string());
        }
        if !(22..=55).contains(&self.workbench.hex_split_initial_pct) {
            return Err("hex_split_initial_pct must be between 22 and 55%".to_string());
        }
        if !ALLOWED_LOG_LEVELS.contains(&self.workbench.log_level.as_str()) {
            return Err(format!(
                "log_level '{}' is invalid (allowed: {:?})",
                self.workbench.log_level, ALLOWED_LOG_LEVELS
            ));
        }
        Ok(())
    }
}

fn validate_socket_addr_spec(spec: &str) -> Result<(), String> {
    let Some((host, port_str)) = spec.rsplit_once(':') else {
        return Err("gdb_listen_addr must use host:port format".to_string());
    };
    if host.trim().is_empty() {
        return Err("gdb_listen_addr host cannot be empty".to_string());
    }
    let port: u16 = port_str
        .parse()
        .map_err(|_| format!("gdb_listen_addr port '{port_str}' is invalid"))?;
    if port == 0 {
        return Err("gdb_listen_addr port must be non-zero".to_string());
    }
    Ok(())
}

/// Resolves the canonical OS configuration file path for runtime operational settings.
pub fn resolve_settings_file_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("PANADIGA_SETTINGS_PATH") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs::config_dir()
        .unwrap_or_else(|| resolve_workspace_root().join(".panadiga-config"))
        .join("panadiga")
        .join("runtime_settings.json")
}

/// Loads and validates `RuntimeSettings` from `path`, returning defaults when absent or invalid.
pub fn load_settings_from(path: &Path) -> RuntimeSettings {
    let Ok(raw) = fs::read_to_string(path) else {
        return RuntimeSettings::default();
    };
    match serde_json::from_str::<RuntimeSettings>(&raw) {
        Ok(parsed) => {
            let norm = parsed.normalized();
            match norm.validate() {
                Ok(()) => norm,
                Err(reason) => {
                    tracing::warn!(path = %path.display(), %reason, "Invalid runtime settings on disk; restoring defaults");
                    RuntimeSettings::default()
                }
            }
        }
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "Failed parsing runtime settings JSON; restoring defaults");
            RuntimeSettings::default()
        }
    }
}

/// Validates and atomically writes `RuntimeSettings` to `path`, applying live log level reloads.
pub fn save_settings_to(path: &Path, settings: &RuntimeSettings) -> Result<RuntimeSettings> {
    let normalized = settings.normalized();
    if let Err(reason) = normalized.validate() {
        bail!("{reason}");
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed creating settings directory '{}'", parent.display())
        })?;
    }
    let json = serde_json::to_string_pretty(&normalized)?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, format!("{json}\n"))
        .with_context(|| format!("Failed writing temporary settings '{}'", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("Failed persisting runtime settings '{}'", path.display()))?;

    set_level(LogLevel::parse_lossy(&normalized.workbench.log_level));
    tracing::info!(
        path = %path.display(),
        baud = normalized.serial.default_baud_rate,
        log_level = %normalized.workbench.log_level,
        "Persisted canonical Panadiga runtime settings"
    );
    Ok(normalized)
}

/// Loads persisted runtime settings from the canonical OS configuration location.
pub fn load_runtime_settings() -> RuntimeSettings {
    load_settings_from(&resolve_settings_file_path())
}

/// Validates and persists runtime settings to the canonical OS configuration location.
pub fn save_runtime_settings(settings: &RuntimeSettings) -> Result<RuntimeSettings> {
    save_settings_to(&resolve_settings_file_path(), settings)
}

/// Resets runtime settings to factory defaults and removes or overwrites the persisted file atomically.
pub fn reset_runtime_settings() -> Result<RuntimeSettings> {
    let defaults = RuntimeSettings::default();
    save_runtime_settings(&defaults)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_persists_and_reloads_cleanly() {
        let dir = std::env::temp_dir().join(format!("panadiga_settings_{}", std::process::id()));
        let file = dir.join("runtime_settings.json");
        let _ = fs::remove_dir_all(&dir);

        let mut custom = RuntimeSettings::default();
        custom.serial.default_baud_rate = 57_600;
        custom.serial.gdb_listen_addr = "127.0.0.1:4444".to_string();
        custom.storage.hdd_default_capacity_gb = 500;
        custom.workbench.log_level = "DEBUG".to_string();

        let saved = save_settings_to(&file, &custom).expect("valid custom settings must save");
        assert_eq!(saved.workbench.log_level, "debug");
        assert_eq!(saved.serial.default_baud_rate, 57_600);

        let reloaded = load_settings_from(&file);
        assert_eq!(reloaded, saved);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_reject_reserved_p7501_gpios_including_nsw5v_gp3() {
        let mut bad = RuntimeSettings::default();
        bad.ir.default_ir_pico_gpio = 3;
        let err = bad
            .validate()
            .expect_err("GP3 (NSW5V Hi-Z) MUST be rejected by hardware safety shield");
        assert!(err.contains("NSW5V"));
    }

    #[test]
    fn settings_reject_invalid_baud_and_log_level() {
        let mut bad = RuntimeSettings::default();
        bad.serial.default_baud_rate = 123_456;
        assert!(bad.validate().is_err());

        let mut bad_log = RuntimeSettings::default();
        bad_log.workbench.log_level = "verbose".to_string();
        assert!(bad_log.validate().is_err());
    }
}
