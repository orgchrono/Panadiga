//! Hardware safety, memory map, Pico P7501 UART catalog, and repository health MCP tools.

use anyhow::Context as _;
use panadiga_core::constants::{
    AM34_FLASH_BASE, AM34_FLASH_SIZE, AM34_FORBIDDEN_LETHAL_REG, AM34_SYSTEM_MEMORY_REGIONS,
    DEFAULT_MANUALS_DIR, SPANSION_FLASH_SIZE_BYTES,
};
use panadiga_core::manuals::count_pdf_files;
use panadiga_core::pico::rsp_codec::policy::{
    NorWritePolicy, is_critical_ipl_boot_address, is_nor_write_blocked_for_flash,
    is_nvram_address_for_flash,
};
use panadiga_core::ssot::{
    Capability, HardwareProfileRegistry, LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride,
    RuntimeFingerprint, build_hardware_ssot_spec, get_diga_model_catalog_dto,
};
use panadiga_rag::config::MAX_SAFE_VOLTAGE_VOLTS;
use serde_json::{Map, Value, json};
use std::path::Path;

use crate::workspace::{optional_non_empty_str, parse_u64_hex_or_dec, require_non_empty_str};

/// Extracts optional audited `LabOverride` parameters shared across MCP tools.
pub(crate) fn parse_lab_override_from_args(args: &Value) -> anyhow::Result<Option<LabOverride>> {
    let ack = optional_non_empty_str(args, "lab_override_disclaimer");
    let reason = optional_non_empty_str(args, "lab_override_reason");
    match (ack, reason) {
        (None, None) => Ok(None),
        (Some(a), Some(r)) => {
            if a != LAB_OVERRIDE_DISCLAIMER_ACK {
                anyhow::bail!(
                    "lab_override_disclaimer must equal '{}'",
                    LAB_OVERRIDE_DISCLAIMER_ACK
                );
            }
            if r.len() < 5 {
                anyhow::bail!("lab_override_reason must be >= 5 characters");
            }
            Ok(Some(LabOverride {
                disclaimer_ack: a.to_string(),
                operator_reason: r.to_string(),
            }))
        }
        _ => anyhow::bail!(
            "Must provide BOTH 'lab_override_disclaimer' and 'lab_override_reason' to activate LabOverride"
        ),
    }
}

pub fn handle_profiles_authorize(args: &Value) -> anyhow::Result<Value> {
    let model_id = require_non_empty_str(args, "model_id")?;
    let cap_raw = require_non_empty_str(args, "capability")?;
    let capability: Capability =
        serde_json::from_value(Value::String(cap_raw.to_ascii_lowercase()))
            .with_context(|| format!("Unknown capability '{cap_raw}'"))?;
    let lab_override = parse_lab_override_from_args(args)?;

    let jedec_mfg_id = match args.get("jedec_mfg_id") {
        Some(Value::Number(n)) => n.as_u64().and_then(|v| u32::try_from(v).ok()),
        Some(Value::String(s)) => Some(u32::try_from(
            parse_u64_hex_or_dec(s).map_err(|e| anyhow::anyhow!(e))?,
        )?),
        _ => None,
    };
    let mut jedec_device_cycles = Vec::new();
    if let Some(arr) = args.get("jedec_device_cycles").and_then(|v| v.as_array()) {
        for item in arr {
            let v = match item {
                Value::Number(n) => u32::try_from(
                    n.as_u64()
                        .ok_or_else(|| anyhow::anyhow!("Invalid JEDEC device word"))?,
                )?,
                Value::String(s) => {
                    u32::try_from(parse_u64_hex_or_dec(s).map_err(|e| anyhow::anyhow!(e))?)?
                }
                _ => anyhow::bail!("jedec_device_cycles elements must be strings or integers"),
            };
            jedec_device_cycles.push(v);
        }
    }

    let fp = RuntimeFingerprint {
        dump_size_bytes: args.get("dump_size_bytes").and_then(|v| v.as_u64()),
        jedec_mfg_id,
        jedec_device_cycles,
        loader_banner: optional_non_empty_str(args, "loader_banner").map(str::to_string),
        has_eemul_f1_marker: args
            .get("has_eemul_f1_marker")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        wp_pin_deasserted: args.get("wp_pin_deasserted").and_then(|v| v.as_bool()),
        frm_sha256: optional_non_empty_str(args, "frm_sha256").map(str::to_string),
        requested_loader_profile: optional_non_empty_str(args, "requested_loader_profile")
            .map(str::to_string),
    };

    match HardwareProfileRegistry::builtin().authorize(
        model_id,
        capability,
        &fp,
        lab_override.as_ref(),
    )? {
        Ok(auth) => Ok(json!({
            "allowed": true,
            "authorization": auth,
        })),
        Err(denial) => Ok(json!({
            "allowed": false,
            "denial": denial,
        })),
    }
}

pub fn handle_memory_map(args: &Value) -> Value {
    let filter_upper = optional_non_empty_str(args, "filter_name").map(|s| s.to_uppercase());

    let regions: Vec<Value> = AM34_SYSTEM_MEMORY_REGIONS
        .iter()
        .filter(|r| match &filter_upper {
            Some(f) => r.name.to_uppercase().contains(f),
            None => true,
        })
        .map(|r| r.to_json_value())
        .collect();

    let total = regions.len();
    json!({ "regions": regions, "total": total })
}

pub fn handle_safety_check(args: &Value) -> Value {
    let mut safe = true;
    let mut violations: Vec<String> = Vec::new();
    let mut result = Map::new();

    if let Some(v_val) = args.get("voltage").filter(|v| !v.is_null()) {
        match v_val.as_f64() {
            Some(v) => {
                if !v.is_finite() || v < 0.0 {
                    safe = false;
                    violations.push(format!(
                        "NEGATIVE_VOLTAGE_VIOLATION: {v}V is invalid/negative for 3.3V LVTTL logic (and Pin 5 NSW5V GP3 must stay Hi-Z Input with no pulls)!"
                    ));
                } else if v > MAX_SAFE_VOLTAGE_VOLTS {
                    safe = false;
                    violations.push(format!(
                        "VOLTAGE_VIOLATION: {}V exceeds maximum safe threshold of {:.2}V (3.3V LVTTL nominal)!",
                        v, MAX_SAFE_VOLTAGE_VOLTS
                    ));
                } else {
                    result.insert(
                        "voltage_status".to_string(),
                        Value::String(format!(
                            "{}V is within safe bounds (0.00V..={:.2}V; keep Pin 5 NSW5V GP3 Hi-Z Input without pulls).",
                            v, MAX_SAFE_VOLTAGE_VOLTS
                        )),
                    );
                }
            }
            None => {
                safe = false;
                violations.push(format!("Invalid numeric voltage: {}", v_val));
            }
        }
    }

    if let Some(addr_val) = args.get("address").filter(|v| !v.is_null()) {
        let parsed = match addr_val.as_str() {
            Some(s) => parse_u64_hex_or_dec(s),
            None => addr_val
                .as_u64()
                .ok_or_else(|| format!("Invalid address type: {}", addr_val)),
        };

        let confirm_upper_nor = args
            .get("confirm_upper_nor")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let flash_size_bytes = args
            .get("flash_size_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(AM34_FLASH_SIZE as u64);

        match parsed {
            Ok(addr) => {
                let u32_addr = u32::try_from(addr).ok();
                let lethal_lo = AM34_FORBIDDEN_LETHAL_REG as u64;
                if (lethal_lo..=lethal_lo + 3).contains(&addr) {
                    safe = false;
                    violations.push(format!(
                        "LETHAL_REGISTER_VIOLATION: Address 0x{:08X} targets forbidden AM34 register 0x{:08X}; reading or writing halts the SoC bus/WDT!",
                        addr, AM34_FORBIDDEN_LETHAL_REG
                    ));
                } else {
                    let bus_addr = if addr < SPANSION_FLASH_SIZE_BYTES as u64 {
                        Some(AM34_FLASH_BASE + addr as u32)
                    } else {
                        u32_addr
                    };
                    if let Some(check_addr) = bus_addr {
                        let policy = if confirm_upper_nor {
                            NorWritePolicy::UpperNorAppUnlocked
                        } else {
                            NorWritePolicy::NvramOnly
                        };
                        if is_critical_ipl_boot_address(check_addr, 1) {
                            safe = false;
                            violations.push(format!(
                                "FLASH_WRITE_VIOLATION: Address 0x{:08X} resides in permanently locked Layer 1 Critical Boot IPL (SA0..SA10, 256 KiB)! Write forbidden (E05).",
                                addr
                            ));
                        } else if is_nor_write_blocked_for_flash(
                            check_addr,
                            1,
                            flash_size_bytes,
                            policy,
                        ) {
                            safe = false;
                            violations.push(format!(
                                "FLASH_WRITE_VIOLATION: Address 0x{:08X} resides in Layer 2 Upper NOR/BGO area (blocked by default E05 unless confirm_upper_nor=true with verified CoW backup)!",
                                addr
                            ));
                        } else if is_nvram_address_for_flash(check_addr, 1, flash_size_bytes) {
                            result.insert(
                                "address_status".to_string(),
                                Value::String(format!(
                                    "Address 0x{:08X} is inside mutable NVRAM (U99 unbrick area).",
                                    addr
                                )),
                            );
                        } else if confirm_upper_nor
                            && !is_critical_ipl_boot_address(check_addr, 1)
                            && panadiga_core::pico::rsp_codec::policy::is_flash_address_in_range(
                                check_addr,
                                1,
                                flash_size_bytes,
                            )
                        {
                            result.insert(
                                "address_status".to_string(),
                                Value::String(format!(
                                    "Address 0x{:08X} is inside Layer 2 Upper NOR/BGO (authorized via confirm_upper_nor; Layer 1 SA0..SA10 remains E05-locked).",
                                    addr
                                )),
                            );
                        } else {
                            result.insert(
                                "address_status".to_string(),
                                Value::String(format!(
                                    "Address 0x{:08X} is outside NOR Flash and lethal MMIO ranges.",
                                    addr
                                )),
                            );
                        }
                    } else {
                        result.insert(
                            "address_status".to_string(),
                            Value::String(format!(
                                "Address 0x{:08X} is outside 32-bit AM34 bus range.",
                                addr
                            )),
                        );
                    }
                }
            }
            Err(err_msg) => {
                safe = false;
                violations.push(err_msg);
            }
        }
    }

    result.insert("safe".to_string(), Value::Bool(safe));
    result.insert(
        "violations".to_string(),
        Value::Array(violations.into_iter().map(Value::String).collect()),
    );
    Value::Object(result)
}

pub fn handle_pico_catalog(args: &Value) -> Value {
    let spec = build_hardware_ssot_spec();
    let filter_cat = optional_non_empty_str(args, "category").map(|s| s.to_lowercase());

    let commands: Vec<_> = spec
        .pico_commands
        .into_iter()
        .filter(|c| match &filter_cat {
            Some(f) => {
                c.category.to_lowercase().contains(f)
                    || c.id.to_lowercase().contains(f)
                    || c.syntax.to_lowercase().contains(f)
            }
            None => true,
        })
        .collect();

    json!({
        "thresholds": spec.thresholds,
        "usb_ids": spec.usb_ids,
        "indexed_manuals_count": spec.indexed_manuals_count,
        "pins": spec.pinout,
        "pinout": spec.pinout,
        "breakin_modes": spec.breakin_modes,
        "flash_geometries": spec.flash_geometries,
        "memory_regions": spec.memory_regions,
        "pico_commands": commands,
    })
}

pub fn handle_project_status(args: &Value, workspace_root: &Path) -> Value {
    let include_rag_stats = args
        .get("include_rag_stats")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let manuals_dir = workspace_root.join(DEFAULT_MANUALS_DIR);
    let pdf_count = count_pdf_files(&manuals_dir);
    let catalog = get_diga_model_catalog_dto();

    let rag_stats = if include_rag_stats {
        serde_json::to_value(panadiga_rag::corpus_stats()).unwrap_or(Value::Null)
    } else {
        Value::Null
    };

    json!({
        "workspace_root": workspace_root.to_string_lossy(),
        "cargo_workspace_crates": [
            "panadiga-profiles",
            "panadiga-core",
            "panadiga-rag",
            "panadiga-mcp",
            "panadiga-cli",
            "panadiga-desktop"
        ],
        "manuals_pdf_count": pdf_count,
        "supported_diga_models": catalog.len(),
        "diga_models": catalog,
        "rag_corpus_stats": rag_stats,
    })
}
