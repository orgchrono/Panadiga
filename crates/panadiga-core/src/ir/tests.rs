//! Unit tests validating Kaseikyo synthesis, Exynos IR HAL adaptation, `.irplus`/Flipper codecs,
//! and the offline 1,763-codeset Panasonic IR catalog.

use super::*;

#[test]
fn kaseikyo_wire_frames_and_pronto_roundtrip_match_hardware() {
    // Verified DMR-EH55 POWER toggle (0xB0, SET 1 0x00, OBC 0x3D) -> 02 20 B0 00 3D 8D
    assert_eq!(
        format_kaseikyo_wire_hex(0xB0, 0x00, 0x3D),
        "02 20 B0 00 3D 8D"
    );
    // Teletext RED (0xB0, SET 1 ext 0x01, OBC 0x41) -> 02 20 B0 01 41 F0
    assert_eq!(
        format_kaseikyo_wire_hex(0xB0, 0x01, 0x41),
        "02 20 B0 01 41 F0"
    );
    // SET 2 Audio [A] (0x10 | 0x00, 0x33) -> 02 20 B0 10 33 93
    assert_eq!(effective_subdevice(0x00, 2, true), 0x10);
    assert_eq!(effective_subdevice(0x01, 3, true), 0x21);
    assert_eq!(effective_subdevice(0x00, 3, false), 0x00);

    let pronto = synthesize_kaseikyo_pronto(0xB0, 0x01, 0x41);
    assert_eq!(
        pronto.split_whitespace().count(),
        KASEIKYO_PRONTO_WORD_COUNT
    );
    assert_eq!(
        decode_kaseikyo_from_pronto(&pronto),
        Some((0xB0, 0x01, 0x41))
    );

    let (carrier, pat) = parse_pronto_hex(&pronto).expect("pronto parse succeeds");
    assert_eq!(carrier, CARRIER_MATSUSHITA_HZ);
    assert_eq!(pat.len(), KASEIKYO_TRANSITION_COUNT);
}

#[test]
fn exynos_fpga_and_mcu_timing_adaptation_restores_all_device_signatures() {
    let sm_t805m = IrDeviceInfo {
        manufacturer: "samsung".to_string(),
        model: "SM-T805M".to_string(),
        hardware: "samsungexynos5420".to_string(),
        board: "chagalllte".to_string(),
        product: "chagalllteub".to_string(),
        device: "chagalllte".to_string(),
        ..Default::default()
    };
    assert!(is_period_dividing_fpga_driver(&sm_t805m, false));
    assert_eq!(
        recommended_timing_unit(&sm_t805m, true, false),
        IrTimingUnit::PeriodScaled
    );

    // Even when generic ROM obscures Build.HARDWARE, chagall/t805 product still triggers FPGA driver
    let lineage_sm_t805m = IrDeviceInfo {
        hardware: "chagall".to_string(),
        model: "SM-T805M".to_string(),
        ..Default::default()
    };
    assert!(is_period_dividing_fpga_driver(&lineage_sm_t805m, false));

    let gt_n8000 = IrDeviceInfo {
        model: "GT-N8000".to_string(),
        hardware: "smdk4x12".to_string(),
        ..Default::default()
    };
    assert!(is_cycle_register_mcu_driver(&gt_n8000, true, false));
    assert_eq!(
        recommended_timing_unit(&gt_n8000, true, false),
        IrTimingUnit::CarrierCycles
    );

    let pattern = synthesize_kaseikyo_pattern(0xB0, 0x00, 0x3D);
    let prepared = prepare_transmission(IrPrepareParams {
        raw_carrier_hz: CARRIER_MATSUSHITA_HZ,
        pattern_us: pattern,
        wire_hex: "02 20 B0 00 3D 8D".to_string(),
        pronto_hex: String::new(),
        ranges: &[IrFrequencyRange {
            min_hz: 36_000,
            max_hz: 40_000,
        }],
        device_info: &sm_t805m,
        has_sec_ir_node: true,
        has_fpga_node: true,
        unit_override: None,
    });
    assert_eq!(prepared.carrier_hz, 37_010);
    assert!(!prepared.expects_cycles_in_sysfs);
    assert!(prepared.sysfs_payload.starts_with("37010,3459,1729,"));
    assert_eq!(prepared.encoded_pattern[0], 3459 * 27);
}

#[test]
fn offline_catalog_loads_all_panasonic_models_and_roundtrips_irplus() {
    let (codesets, aliases) = catalog_stats();
    assert!(codesets > 1500, "expected >=1500 codesets, got {codesets}");
    assert!(aliases >= codesets);

    let hits = search_codesets("DMR-EH55", PanasonicCategory::Diga, Some(10));
    assert!(!hits.is_empty());
    assert_eq!(hits[0].id, "curated_diga_eh55_eur7659");

    let cs = find_codeset("curated_diga_eh55_eur7659").expect("curated EH55 present");
    let profile = assemble_profile_from_codeset(cs, None, false);
    assert!(profile.keys.iter().any(|k| k.obc == 0x3D));
    let xml = export_irplus_xml(&profile, 2);
    assert!(xml.contains("<irplus>"));
    let reimported = import_irplus_xml(&xml, Some("TestMode2"));
    assert_eq!(reimported.keys.len(), profile.keys.len());

    let flipper = "\
Filetype: IR signals file
Version: 1
#
name: Audio_test
type: parsed
protocol: Kaseikyo
address: B0 02 20 00
command: 30 03 00 00
";
    let f_prof = import_flipper_ir(flipper, "FlipperTest");
    assert_eq!(f_prof.keys.len(), 1);
    assert_eq!(f_prof.keys[0].device, 0xB0);
    assert_eq!(f_prof.keys[0].obc, 0x33);
}

#[test]
fn verified_service_macros_are_strictly_audited_and_cross_checked() {
    let macros = verified_service_macros();
    assert!(macros.len() >= 12);

    // Every service sequence must use real EH55 digits from panasonic_ir_catalog.dat.gz
    let eh55 = find_codeset("curated_diga_eh55_eur7659").expect("EH55 master");
    for m in macros {
        assert!(
            !m.verification_source.trim().is_empty(),
            "missing source on {}",
            m.id
        );
        assert!(!m.steps.is_empty(), "empty steps on {}", m.id);
        for s in &m.steps {
            let matching = eh55
                .commands
                .iter()
                .find(|c| c.name == s.label && c.device == s.device && c.subdevice == s.subdevice);
            assert_eq!(
                matching.map(|c| c.obc),
                Some(s.obc),
                "Macro '{}' step '{}' OBC mismatch vs catalog",
                m.id,
                s.label
            );
        }
    }

    // Confirm VFD test uses real [5][1], AV internal patterns use [1][1]/[1][2],
    // and unverified/broken macros remain excluded.
    let vfd = macros
        .iter()
        .find(|m| m.id == "eh55_vfd_all_segments")
        .expect("VFD 5,1");
    assert_eq!((vfd.steps[0].obc, vfd.steps[1].obc), (0x14, 0x10));
    let clr = macros
        .iter()
        .find(|m| m.id == "eh55_service_initialize_clr")
        .expect("CLR 9,9");
    assert_eq!(clr.risk, IrMacroRiskLevel::Destructive);
    for forbidden in [
        "multi_region",
        "child_lock",
        "video_io_test_80",
        "auto_restore_set2",
    ] {
        assert!(
            !macros.iter().any(|m| m.id.contains(forbidden)),
            "Forbidden unverified macro {forbidden} leaked into SSOT"
        );
    }
}
