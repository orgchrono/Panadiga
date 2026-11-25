#![deny(unsafe_code)]

//! Deterministic C11 & TypeScript SSOT emitter for `DigaOS`
//! (`firmware/digaos/generated/digaos_ssot.{h,c}` & `apps/desktop/src/models/generated/digaos.ts`).
//!
//! Eliminates manual hexadecimal constant duplication in `hal/am34_mmio.h` and `drivers/` by projecting
//! hardware bounds, MMIO apertures, TBUS/AIOF/AVDD/IIC/SDCD/DAVIS/MEIHFS definitions, declarative menus,
//! themes, max-hardware-power tuning profiles, multi-SoC targets, and `DIGAOS_SSOT_CONTRACT_HASH`
//! straight from `panadiga-constants` and `panadiga-profiles` (`DMR-EH55` `BenchVerified`).

mod header_gen;
mod header_sections;
mod tables_ts_gen;
#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::{Context, Result};
pub use panadiga_profiles::{DigaOsConfigSpec, DigaOsMmioRangeSpec, validate_digaos_config};

use super::pico_header::write_if_changed_or_create;

pub use header_gen::{generate_digaos_ssot_header, generate_digaos_ssot_header_for};
pub use tables_ts_gen::{
    generate_digaos_ssot_tables, generate_digaos_ssot_tables_for, generate_ts_digaos_module,
    generate_ts_digaos_module_for,
};

/// Embedded canonical declarative `DigaOS` configuration JSON (`profiles/digaos/default_digaos_config.json`).
pub const DEFAULT_DIGAOS_CONFIG_JSON: &str =
    include_str!("../../../../../profiles/digaos/default_digaos_config.json");

const fn compute_fnv1a32_normalized(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811C_9DC5;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        i += 1;
        if b == b'\r' {
            continue;
        }
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Deterministic FNV-1a contract hash over `profiles/digaos/default_digaos_config.json`.
pub const DIGAOS_SSOT_CONTRACT_HASH: u32 =
    compute_fnv1a32_normalized(DEFAULT_DIGAOS_CONFIG_JSON.as_bytes());

/// Loads and validates `profiles/digaos/default_digaos_config.json` from `root` (or builtin fallback).
pub fn load_digaos_config_from_root(root: &Path) -> Result<DigaOsConfigSpec> {
    let custom_path = root
        .join("profiles")
        .join("digaos")
        .join("default_digaos_config.json");
    let raw = if custom_path.is_file() {
        std::fs::read_to_string(&custom_path)
            .with_context(|| format!("Reading {}", custom_path.display()))?
    } else {
        DEFAULT_DIGAOS_CONFIG_JSON.to_string()
    };
    let cfg: DigaOsConfigSpec =
        serde_json::from_str(&raw).context("Parsing DigaOS declarative configuration JSON")?;
    validate_digaos_config(&cfg)?;
    Ok(cfg)
}

/// Computes the FNV-1a contract hash for any normalized `DigaOsConfigSpec` JSON representation.
#[must_use]
pub fn compute_digaos_contract_hash_for_raw(raw_json: &str) -> u32 {
    compute_fnv1a32_normalized(raw_json.as_bytes())
}

pub(super) fn load_validated_builtin_digaos_config() -> DigaOsConfigSpec {
    let cfg: DigaOsConfigSpec = serde_json::from_str(DEFAULT_DIGAOS_CONFIG_JSON)
        .expect("Builtin profiles/digaos/default_digaos_config.json must parse");
    validate_digaos_config(&cfg).expect(
        "Builtin profiles/digaos/default_digaos_config.json must satisfy safety invariants",
    );
    cfg
}

/// Exports `generated/digaos_ssot.h` and `generated/digaos_ssot_tables.c` under `digaos_dir`.
pub fn export_digaos_ssot_files(digaos_dir: &Path, root_for_cfg: Option<&Path>) -> Result<()> {
    let (cfg, hash) = match root_for_cfg {
        Some(root) => {
            let p = root.join("profiles/digaos/default_digaos_config.json");
            if p.is_file() {
                let raw = std::fs::read_to_string(&p)
                    .with_context(|| format!("Reading {}", p.display()))?;
                let c: DigaOsConfigSpec = serde_json::from_str(&raw)?;
                validate_digaos_config(&c)?;
                (c, compute_digaos_contract_hash_for_raw(&raw))
            } else {
                (
                    load_validated_builtin_digaos_config(),
                    DIGAOS_SSOT_CONTRACT_HASH,
                )
            }
        }
        None => (
            load_validated_builtin_digaos_config(),
            DIGAOS_SSOT_CONTRACT_HASH,
        ),
    };
    write_if_changed_or_create(
        &digaos_dir.join("generated/digaos_ssot.h"),
        &generate_digaos_ssot_header_for(&cfg, hash),
    )?;
    write_if_changed_or_create(
        &digaos_dir.join("generated/digaos_ssot_tables.c"),
        &generate_digaos_ssot_tables_for(&cfg),
    )?;
    Ok(())
}

/// Exports `apps/desktop/src/models/generated/digaos.ts`.
pub fn export_ts_digaos_module(output_path: &Path) -> Result<()> {
    write_if_changed_or_create(output_path, &generate_ts_digaos_module())
}
