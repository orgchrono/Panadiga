use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::catalog::hardware_catalog;
use super::constants::{
    CATALOG_TIMEOUT_KEY_WDT_CEILING, INTERNAL_OR_ALIAS_CLI_COMMANDS, P7501_WDT_MAX_CMD_WINDOW_MS,
};
use super::*;

fn repo_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed reading {}: {e}", path.display()))
}

fn extract_c11_cli_names(src: &str) -> Vec<String> {
    let start = src
        .find("k_cli_commands[] = {")
        .expect("Missing k_cli_commands[] table in p7501_cli.c");
    let body = &src[start
        ..src[start..]
            .find("};")
            .map(|off| start + off)
            .expect("Missing closing }; for k_cli_commands[]")];

    body.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('{') {
                return None;
            }
            let first_quote = trimmed.find('"')?;
            let rest = &trimmed[first_quote + 1..];
            let end_quote = rest.find('"')?;
            Some(rest[..end_quote].to_string())
        })
        .collect()
}

fn extract_c_define_token<'a>(src: &'a str, name: &str) -> &'a str {
    let needle = format!("#define {name}");
    src.lines()
        .find_map(|line| {
            let rest = line.trim_start().strip_prefix(&needle)?;
            if !rest.starts_with(char::is_whitespace) {
                return None;
            }
            rest.split_whitespace().next()
        })
        .unwrap_or_else(|| panic!("Missing #define {name}"))
}

#[test]
fn ssot_thresholds_match_firmware_and_platform_constants() {
    let spec = build_hardware_ssot_spec();
    assert!((spec.thresholds.min_vdd_threshold_volts - 2.50).abs() < f32::EPSILON);
    assert!((spec.thresholds.max_vdd_ceiling_volts - 3.45).abs() < f32::EPSILON);
    assert_eq!(spec.usb_ids.raspberry_pi_vid, 0x2E8A);
    assert_eq!(spec.usb_ids.rp2350_cdc_pid, 0x000F);
    assert_eq!(spec.pinout.len(), 6);
    assert_eq!(
        spec.pinout[1].signal_name,
        "XRESET Open-Drain (Nunca Push-Pull)"
    );
    assert_eq!(spec.pinout[2].signal_name, "UART TX -> AM34 RX via Q7514");
    assert_eq!(spec.pinout[3].manual_pin, 4);
    assert_eq!(spec.pinout[3].signal_name, "Terra 0.0Ω Chassi");
    assert_eq!(spec.breakin_modes.len(), 4);
    assert_eq!(spec.flash_geometries.len(), 4);
    assert_eq!(spec.flash_geometries[0].nvram_base_hex, "0x801F0000");
    assert_eq!(spec.flash_geometries[1].nvram_base_hex, "0x803F0000");
    assert_eq!(spec.flash_geometries[2].nvram_base_hex, "0x807F0000");
    assert_eq!(spec.flash_geometries[3].nvram_base_hex, "0x80FE0000");
    assert_eq!(spec.pico_commands.len(), 79);

    for cmd in &spec.pico_commands {
        assert!(
            !cmd.syntax.contains("{{"),
            "Unresolved token in {} syntax",
            cmd.id
        );
        if let Some(def) = &cmd.default_arg {
            assert!(
                !def.contains("{{"),
                "Unresolved token in {} default_arg",
                cmd.id
            );
        }
        assert_eq!(
            cmd.arg_placeholder, cmd.default_arg,
            "Placeholder/default drift in {}",
            cmd.id
        );
        for opt in &cmd.arg_options {
            assert!(
                !opt.value.contains("{{"),
                "Unresolved token in {} option",
                cmd.id
            );
        }
    }

    let find_cmd = |id: &str| {
        spec.pico_commands
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("Missing command {id}"))
    };
    assert_eq!(
        find_cmd("DUMP").default_arg.as_deref(),
        Some("0x807F0000 16")
    );
    assert_eq!(find_cmd("BAUD").default_arg.as_deref(), Some("115200"));
    assert_eq!(
        find_cmd("BRIDGE").default_arg.as_deref(),
        Some("115200 JIG:350")
    );
    assert!(
        find_cmd("PANARSP").syntax.contains("XMEM:")
            && find_cmd("PANARSP").syntax.contains("DUMPNOR")
            && find_cmd("PANARSP").syntax.contains("REGS")
    );
    assert_eq!(
        find_cmd("PANARSP").arg_options[4].value,
        "4000 MEM:515B2E38,10"
    );
    assert_eq!(
        find_cmd("PANARSP").arg_options[6].value,
        "180000 DUMPNOR 84000000 800000"
    );
    assert!(find_cmd("DIAGWATCH").syntax.contains("READ[:addr[,len]]"));
    assert_eq!(
        find_cmd("DIAGWATCH").arg_options[1].value,
        "25000 JIG:2500 READ:847F0000,20 RELEASE"
    );
    assert_eq!(
        find_cmd("DRVPANA").arg_options[1].value,
        "PASS 0x4d415453 0x48495441"
    );
    assert!(find_cmd("BRIDGE").description.contains("~~~"));
    assert!(spec.indexed_manuals_count >= 15);
}

#[test]
fn c11_firmware_and_catalog_ssot_parity() {
    let spec = build_hardware_ssot_spec();
    let c11_cli = repo_file("firmware/picofw/cli/picofw_cli.c");
    let c11_names = extract_c11_cli_names(&c11_cli);
    let c11_set: HashSet<&str> = c11_names.iter().map(String::as_str).collect();
    let ssot_set: HashSet<&str> = spec.pico_commands.iter().map(|c| c.id.as_str()).collect();
    let internal_set: HashSet<&str> = INTERNAL_OR_ALIAS_CLI_COMMANDS.iter().copied().collect();

    for cmd_id in &ssot_set {
        assert!(
            c11_set.contains(cmd_id),
            "SSOT command {cmd_id} missing from C11 k_cli_commands[]"
        );
    }
    for c_name in &c11_names {
        let covered = ssot_set.contains(c_name.as_str()) || internal_set.contains(c_name.as_str());
        assert!(
            covered,
            "C11 command {c_name} neither in SSOT catalog nor INTERNAL_OR_ALIAS_CLI_COMMANDS"
        );
    }

    let wdt_hdr = repo_file("firmware/picofw/common/picofw_wdt_logic.h");
    let wdt_max_c: u64 = extract_c_define_token(&wdt_hdr, "PICOFW_WDT_MAX_CMD_WINDOW_MS")
        .trim_end_matches(&['u', 'U', 'l', 'L'][..])
        .parse()
        .expect("numeric PICOFW_WDT_MAX_CMD_WINDOW_MS");
    assert_eq!(wdt_max_c, P7501_WDT_MAX_CMD_WINDOW_MS);
    assert_eq!(
        hardware_catalog()
            .timeouts_ms
            .get(CATALOG_TIMEOUT_KEY_WDT_CEILING)
            .copied(),
        Some(P7501_WDT_MAX_CMD_WINDOW_MS)
    );

    let types_hdr = repo_file("firmware/picofw/picofw_types.h");
    let baud_c: u32 = extract_c_define_token(&types_hdr, "PICOFW_UART_BAUD_RATE")
        .trim_end_matches(&['u', 'U', 'l', 'L'][..])
        .parse()
        .expect("numeric PICOFW_UART_BAUD_RATE");
    let vdd_min_c: f32 = extract_c_define_token(&types_hdr, "PICOFW_VDD_THRESHOLD_V")
        .trim_end_matches(&['f', 'F'][..])
        .parse()
        .expect("f32 PICOFW_VDD_THRESHOLD_V");
    let vdd_ceil_c: f32 = extract_c_define_token(&types_hdr, "PICOFW_VDD_FLOATING_CEIL_V")
        .trim_end_matches(&['f', 'F'][..])
        .parse()
        .expect("f32 PICOFW_VDD_FLOATING_CEIL_V");
    assert_eq!(baud_c, spec.thresholds.default_baud_rate);
    assert!((vdd_min_c - spec.thresholds.min_vdd_threshold_volts).abs() < f32::EPSILON);
    assert!((vdd_ceil_c - spec.thresholds.max_vdd_ceiling_volts).abs() < f32::EPSILON);
}

#[test]
fn memory_regions_catalog_matches_am34_constants_and_blueprints() {
    use crate::constants::AM34_SYSTEM_MEMORY_REGIONS;

    let catalog = hardware_catalog();
    assert_eq!(
        catalog.system_memory_regions.len(),
        AM34_SYSTEM_MEMORY_REGIONS.len()
    );
    for (json_reg, rust_reg) in catalog
        .system_memory_regions
        .iter()
        .zip(AM34_SYSTEM_MEMORY_REGIONS.iter())
    {
        assert_eq!(json_reg.name, rust_reg.name);
        assert_eq!(json_reg.start, format!("0x{:08X}", rust_reg.start));
        assert_eq!(json_reg.end, format!("0x{:08X}", rust_reg.end));
        assert_eq!(json_reg.size_kb, rust_reg.size_kb);
        assert_eq!(json_reg.perms, rust_reg.perms);
    }

    let bp_keys: Vec<&str> = catalog
        .memory_region_blueprints
        .iter()
        .map(|b| b.key.as_str())
        .collect();
    assert_eq!(
        bp_keys,
        vec![
            "sram",
            "mmio",
            "flash_ipl",
            "rtos_fw",
            "flash_nvram",
            "sdram"
        ]
    );
    for bp in &catalog.memory_region_blueprints {
        assert!(bp.start.starts_with("0x") && bp.start.len() == 10);
        assert!(bp.end.starts_with("0x") && bp.end.len() == 10);
        assert!(bp.size_kb > 0);
    }

    let sram_bp = &catalog.memory_region_blueprints[0];
    let ipl_bp = &catalog.memory_region_blueprints[2];
    let rtos_bp = &catalog.memory_region_blueprints[3];
    assert_eq!(
        (
            sram_bp.start.as_str(),
            sram_bp.end.as_str(),
            sram_bp.size_kb
        ),
        ("0x00000000", "0x0003FFFF", 256)
    );
    assert_eq!(
        (ipl_bp.start.as_str(), ipl_bp.end.as_str(), ipl_bp.size_kb),
        ("0x80000000", "0x8003FFFF", 256)
    );
    assert_eq!(
        (
            rtos_bp.start.as_str(),
            rtos_bp.end.as_str(),
            rtos_bp.size_kb
        ),
        ("0x80040000", "0x807EFFFF", 7872)
    );
}

#[test]
fn core_catalogs_and_decoupled_labels_stay_valid_and_synced() {
    let labels_core: serde_json::Value = serde_json::from_str(&repo_file(
        "crates/panadiga-core/src/ssot/hardware_catalog_labels.json",
    ))
    .expect("valid hardware_catalog_labels.json");
    let labels_ui: serde_json::Value =
        serde_json::from_str(&repo_file("apps/desktop/src/locales/pt-BR/ssot.json"))
            .expect("valid apps/desktop/src/locales/pt-BR/ssot.json");
    assert_eq!(
        labels_core, labels_ui,
        "crates/panadiga-core/src/ssot/hardware_catalog_labels.json drifted from pt-BR/ssot.json"
    );

    for rel in [
        "crates/panadiga-core/src/ssot/hardware_catalog.json",
        "crates/panadiga-core/src/ghidra/specs/ghidra_maps_ssot.json",
        "crates/panadiga-ir/src/ir_macros_ssot.json",
        "crates/panadiga-core/src/re/error_dict/firmware_error_dict.json",
    ] {
        let raw = repo_file(rel);
        let parsed: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("Invalid JSON {rel}: {e}"));
        let non_empty = parsed.as_object().is_some_and(|m| !m.is_empty())
            || parsed.as_array().is_some_and(|a| !a.is_empty());
        assert!(non_empty, "{rel} must be a non-empty JSON object or array");
        assert!(
            !raw.contains("SA0..SA3"),
            "Stale SA0..SA3 token found in {rel}"
        );
    }
}
