#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

#[test]
fn builtin_v2_profiles_load_validate_and_resolve_all_89_models() {
    let reg = HardwareProfileRegistry::load_builtin().expect("v2 builtin registry must load");
    let report = reg.validate().expect("validate must succeed");
    assert!(report.valid, "validation errors: {:?}", report.errors);
    assert_eq!(report.model_count, 89);
    assert!(report.safety_checks_passed >= 30);

    let eh55 = reg.resolve_model("DMR-EH55").expect("resolve DMR-EH55");
    assert_eq!(
        eh55.flash_chip.as_ref().map(|f| f.part_number.as_str()),
        Some("S29JL064H70TFI00")
    );
    assert_eq!(
        eh55.nvram_sector_range_derived.as_deref(),
        Some("SA134..SA141")
    );
    assert_eq!(eh55.bgo_sector_range_derived.as_deref(), Some("SA26..SA70"));
    assert_eq!(
        eh55.ipl_protected_range_hex.as_deref(),
        Some("0x84000000..0x8403FFFF")
    );
    assert!(eh55.effective_capabilities.nvram_write_live);
    assert!(eh55.effective_capabilities.upper_nor_write_live);
    assert!(eh55.effective_capabilities.custom_recovery_media);

    // Exact region suffix alias resolves to EH55.
    let eh55lb = reg
        .resolve_model("DMR-EH55LB-S")
        .expect("alias DMR-EH55LB-S");
    assert_eq!(eh55lb.manifest.model_id, "DMR-EH55");

    // EH56 is an independent first-class model (T1), NOT silently mapped to EH55.
    let eh56 = reg.resolve_model("DMR-EH56").expect("resolve DMR-EH56");
    assert_eq!(eh56.manifest.model_id, "DMR-EH56");
    assert!(!eh56.effective_capabilities.nvram_write_live);
    assert!(!eh56.effective_capabilities.nvram_sanitize_offline_cow);
    assert!(
        eh56.effective_capabilities
            .lab_override_offline_cow_eligible
    );

    // ES35V is MAIN_XIP-proven (T3 physical_dump_verified) -> offline NVRAM CoW only, live blocked.
    let es35v = reg.resolve_model("DMR-ES35V").expect("resolve DMR-ES35V");
    assert_eq!(
        es35v.nvram_sector_range_derived.as_deref(),
        Some("SA134..SA141")
    );
    assert!(es35v.effective_capabilities.nvram_sanitize_offline_cow);
    assert!(!es35v.effective_capabilities.nvram_write_live);

    // EZ485V is SPARC V8 DoMiNo / VxWorks -> C0 read-only by default.
    let ez485v = reg.resolve_model("DMR-EZ485V").expect("resolve DMR-EZ485V");
    assert_eq!(
        ez485v.soc_core.as_ref().map(|c| c.isa.as_str()),
        Some("SPARC V8 (LSI DoMiNo)")
    );
    assert!(ez485v.effective_capabilities.inspect_read_only);
    assert!(!ez485v.effective_capabilities.nvram_sanitize_offline_cow);
    assert!(!ez485v.effective_capabilities.nvram_write_live);
}

#[test]
fn overlay_rejects_builtin_overwrites_forces_t0_and_enforces_lvttl() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("panadiga_v2_overlay_test_{nanos}"));
    let cores_dir = root.join("soc_cores");
    fs::create_dir_all(&cores_dir).unwrap();

    let reg = HardwareProfileRegistry::load_builtin().unwrap();
    let base_core = reg.bundle().soc_cores.get("mn10300_am34").unwrap().clone();

    // 1. New custom entity with valid 3300 mV succeeds and is downgraded to T0 (CommunityUnverified).
    let mut custom_ok = base_core.clone();
    custom_ok.id = "custom_lab_core".to_string();
    fs::write(
        cores_dir.join("custom_lab_core.json"),
        serde_json::to_string_pretty(&custom_ok).unwrap(),
    )
    .unwrap();
    let overlaid = HardwareProfileRegistry::load_with_overlay(Some(&root))
        .expect("valid new overlay entity must load");
    assert_eq!(
        overlaid.bundle().soc_cores["custom_lab_core"].evidence.tier,
        EvidenceTier::CommunityUnverified
    );

    // 2. New entity violating 3300 mV LVTTL invariant must fail closed.
    custom_ok.logic_voltage_mv = 5000;
    fs::write(
        cores_dir.join("custom_lab_core.json"),
        serde_json::to_string_pretty(&custom_ok).unwrap(),
    )
    .unwrap();
    assert!(
        HardwareProfileRegistry::load_with_overlay(Some(&root)).is_err(),
        "overlay violating 3300 mV LVTTL invariant must be rejected"
    );

    // 3. Attempting to overwrite ANY built-in entity must fail closed.
    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(&cores_dir).unwrap();
    fs::write(
        cores_dir.join("mn10300_am34.json"),
        serde_json::to_string_pretty(&base_core).unwrap(),
    )
    .unwrap();
    assert!(
        HardwareProfileRegistry::load_with_overlay(Some(&root)).is_err(),
        "overlay attempting to overwrite built-in soc_core must be rejected"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn builtin_v2_profiles_lineup_expansion_and_safety_regressions() {
    let reg = HardwareProfileRegistry::builtin();

    let es35v = reg.resolve_model("DMR-ES35V").expect("resolve DMR-ES35V");
    assert_eq!(es35v.manifest.hw_code.as_deref(), Some("M6720EE"));
    assert_eq!(es35v.manifest.tv_standard.as_deref(), Some("PAL/SECAM"));

    let ez485v = reg.resolve_model("DMR-EZ485V").expect("resolve DMR-EZ485V");
    assert_eq!(ez485v.manifest.hw_code.as_deref(), Some("M4831P"));
    let ez_nv = ez485v
        .nor_layout
        .as_ref()
        .and_then(|l| l.regions.last())
        .expect("ez485v vendor_nvram region");
    assert_eq!(ez_nv.start_offset, 0x0036_0000);
    assert_eq!(ez_nv.end_offset, 0x003F_FFFF);

    let eh58 = reg.resolve_model("DMR-EH58").expect("resolve DMR-EH58");
    assert_eq!(
        eh58.board.as_ref().map(|b| b.p7501_header_present),
        Some(false)
    );
    assert_eq!(
        eh58.firmware_train.as_ref().and_then(|t| t.drv_load_va),
        Some(0x9020_2000)
    );

    let eh50 = reg.resolve_model("DMR-EH50").expect("resolve DMR-EH50");
    assert_eq!(eh50.manifest.hw_code.as_deref(), Some("M7820"));
    assert_eq!(
        eh50.firmware_train
            .as_ref()
            .and_then(|t| t.frm.as_ref())
            .and_then(|f| f.key_id.as_deref()),
        Some("le2011_08e0")
    );

    for mid in ["DMR-EX97", "DMR-XW300", "DMR-BS850", "DMR-BWT720"] {
        let m = reg
            .resolve_model(mid)
            .unwrap_or_else(|e| panic!("resolve {mid}: {e}"));
        assert!(!m.effective_capabilities.nvram_write_live);
    }
}
