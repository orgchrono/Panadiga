#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::derive::{
    derive_sector_table, derived_geometry_label, format_sa_range, sector_of_offset,
    sector_range_for_aligned_span,
};
use crate::gate::CapabilityGate;
use crate::loader::{
    load_bundle_from_dir, merge_overlay_dir, model_storage_key, resolve_model_in_bundle,
};
use crate::schema::verify_schemas_match_dir;
use crate::types::{
    AddressReloc, AuthorizationMode, BoardProfile, BusWindow, BytePatchEntry, Capability,
    ClaimEvidence, EepromChipProfile, EepromFunction, EvidenceSource, EvidenceTier,
    FirmwareTrainProfile, FlashChipProfile, FlashHwProtect, FlashJedecId, FlashRwBank,
    FlashSectorBank, FrmContainerSpec, LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride,
    MediaFormatProfile, ModelClaims, ModelProfileManifest, NorLayoutProfile, NorRegion,
    NorRegionKind, NorRegionPolicy, OffsetRange, ProfilesBundle, ProgBgoRetargetSpec,
    RegionSignature, RuntimeFingerprint, SocCoreProfile, SocProfile,
};
use crate::validate::{AM34_LETHAL_MMIO_ADDR, REQUIRED_LVTTL_MV, validate_bundle};

fn ev(tier: EvidenceTier, reference: &str) -> ClaimEvidence {
    ClaimEvidence {
        tier,
        sources: vec![EvidenceSource {
            kind: "test".to_string(),
            reference: reference.to_string(),
            sha256: None,
            notes: None,
        }],
        notes: None,
    }
}

fn s29jl064h_chip() -> FlashChipProfile {
    FlashChipProfile {
        id: "spansion_s29jl064h_8mb".to_string(),
        vendor: "Spansion".to_string(),
        part_number: "S29JL064H70TFI00".to_string(),
        package: "TSOP-48".to_string(),
        voltage_min_mv: 2700,
        voltage_max_mv: 3600,
        nominal_voltage_mv: REQUIRED_LVTTL_MV,
        writable: true,
        total_bytes: 0x0080_0000,
        jedec: Some(FlashJedecId {
            mfg: 0x01,
            device_cycles: vec![0x227E, 0x2202, 0x2201],
        }),
        sector_banks: vec![
            FlashSectorBank {
                name: "bottom_boot".to_string(),
                count: 8,
                sector_size: 0x2000,
            },
            FlashSectorBank {
                name: "main_uniform".to_string(),
                count: 126,
                sector_size: 0x10000,
            },
            FlashSectorBank {
                name: "top_nvram".to_string(),
                count: 8,
                sector_size: 0x2000,
            },
        ],
        rw_banks: vec![
            FlashRwBank {
                start_sa: 0,
                end_sa: 22,
            },
            FlashRwBank {
                start_sa: 23,
                end_sa: 70,
            },
            FlashRwBank {
                start_sa: 71,
                end_sa: 118,
            },
            FlashRwBank {
                start_sa: 119,
                end_sa: 141,
            },
        ],
        hw_protect: None,
        evidence: ev(EvidenceTier::PhysicalDumpVerified, "S29JL064H"),
    }
}

fn sample_bundle() -> ProfilesBundle {
    let mut bundle = crate::loader::empty_bundle();
    bundle.soc_cores.insert(
        "mn10300_am34".to_string(),
        SocCoreProfile {
            id: "mn10300_am34".to_string(),
            isa: "MN10300/AM34".to_string(),
            endianness: "little".to_string(),
            logic_voltage_mv: REQUIRED_LVTTL_MV,
            evidence: ev(EvidenceTier::BenchVerified, "bench"),
        },
    );
    bundle.socs.insert(
        "mn2ds0009".to_string(),
        SocProfile {
            id: "mn2ds0009".to_string(),
            part_number: "MN2DS0009".to_string(),
            core_ref: "mn10300_am34".to_string(),
            bus_windows: vec![BusWindow {
                name: "nor_cached".to_string(),
                base: 0x8000_0000,
                size: 0x0400_0000,
                uncached_alias_base: Some(0x8400_0000),
            }],
            mmio: BTreeMap::from([("sc0_uart".to_string(), 0xD400_2000)]),
            lethal_mmio: vec![AM34_LETHAL_MMIO_ADDR],
            p7501_uart_baud: 115200,
            p7501_nsw5v_hiz_only: true,
            evidence: ev(EvidenceTier::BenchVerified, "bench"),
        },
    );
    bundle
        .flash_chips
        .insert("spansion_s29jl064h_8mb".to_string(), s29jl064h_chip());
    bundle.flash_chips.insert(
        "spansion_s29al016d_2mb_top".to_string(),
        FlashChipProfile {
            id: "spansion_s29al016d_2mb_top".to_string(),
            vendor: "Spansion".to_string(),
            part_number: "S29AL016D70TFI01".to_string(),
            package: "TSOP-48".to_string(),
            voltage_min_mv: 2700,
            voltage_max_mv: 3600,
            nominal_voltage_mv: REQUIRED_LVTTL_MV,
            writable: true,
            total_bytes: 0x0020_0000,
            jedec: Some(FlashJedecId {
                mfg: 0x01,
                device_cycles: vec![0x22C4],
            }),
            sector_banks: vec![
                FlashSectorBank {
                    name: "main_64k".to_string(),
                    count: 31,
                    sector_size: 0x10000,
                },
                FlashSectorBank {
                    name: "top_32k".to_string(),
                    count: 1,
                    sector_size: 0x8000,
                },
                FlashSectorBank {
                    name: "top_8k".to_string(),
                    count: 2,
                    sector_size: 0x2000,
                },
                FlashSectorBank {
                    name: "top_16k".to_string(),
                    count: 1,
                    sector_size: 0x4000,
                },
            ],
            rw_banks: vec![],
            hw_protect: None,
            evidence: ev(EvidenceTier::OfficialFrmVerified, "S29AL016D datasheet"),
        },
    );
    bundle.eeprom_chips.insert(
        "panasonic_c3ebfc000053_timer".to_string(),
        EepromChipProfile {
            id: "panasonic_c3ebfc000053_timer".to_string(),
            vendor: "Panasonic".to_string(),
            part_number: "C3EBFC000053".to_string(),
            service_replacement_part: Some("RFKFM6016KT".to_string()),
            bus: "i2c_timer_main_pcb".to_string(),
            function: EepromFunction::TimerMainConfig,
            i2c_addr_7bit: 0x50,
            total_bytes: 0x0400,
            page_size: 16,
            voltage_mv: REQUIRED_LVTTL_MV,
            evidence: ev(EvidenceTier::ServiceManualOnly, "DSD0603003CE"),
        },
    );
    bundle.media_formats.insert(
        "sd_fat16b_0x06".to_string(),
        MediaFormatProfile {
            id: "sd_fat16b_0x06".to_string(),
            media_kind: "sd_card".to_string(),
            filesystem: "FAT16B".to_string(),
            partition_type: 0x06,
            sector_size: 512,
            start_lba: 63,
            default_volume_mb: 32,
            root_filenames: vec!["DIGADIAG.FRM".to_string(), "PANA__SD.FRM".to_string()],
            magic_hex: "55AA".to_string(),
            write_speed_policy: "raw_block_verified".to_string(),
            evidence: ev(EvidenceTier::BenchVerified, "bench"),
        },
    );
    bundle.firmware_trains.insert(
        "loader_ver061_toc48_main57".to_string(),
        FirmwareTrainProfile {
            id: "loader_ver061_toc48_main57".to_string(),
            loader_version: "0.61".to_string(),
            loader_date: Some("06/03/06".to_string()),
            loader_profile_key: "ver061".to_string(),
            loader_fingerprint: None,
            frm: Some(FrmContainerSpec {
                toc_entry_bytes: Some(48),
                toc_size: Some(0x1800),
                cipher: Some("feistel64_ecb".to_string()),
                key_id: Some("pana_dvd_eh55".to_string()),
                official_frm_sha256: None,
            }),
            prog_load_va: Some(0x5040_0000),
            main_load_va: Some(0x5700_0000),
            drv_load_va: Some(0x9020_2000),
            symbols: BTreeMap::from([("panamon1_entry".to_string(), 0x5043_9494)]),
            sd_boot_filenames: vec!["DIGADIAG.FRM".to_string(), "PANA__SD.FRM".to_string()],
            digadiag_raw_bytes: Some(262_144),
            media_refs: vec!["sd_fat16b_0x06".to_string()],
            digadiag_spec: None,
            pana_rebuild_recipe: None,
            bgo_retarget_spec: None,
            evidence: ev(EvidenceTier::BenchVerified, "bench"),
        },
    );
    bundle.nor_layouts.insert(
        "eh55_ver061_on_s29jl064h".to_string(),
        NorLayoutProfile {
            id: "eh55_ver061_on_s29jl064h".to_string(),
            flash_ref: "spansion_s29jl064h_8mb".to_string(),
            train_ref: "loader_ver061_toc48_main57".to_string(),
            regions: vec![
                NorRegion {
                    kind: NorRegionKind::Ipl,
                    start_offset: 0x00_0000,
                    end_offset: 0x03_FFFF,
                    policy: NorRegionPolicy::ReadOnlyPermanent,
                    tag: Some("IPL".to_string()),
                    signature: None,
                },
                NorRegion {
                    kind: NorRegionKind::Bgo,
                    start_offset: 0x13_0000,
                    end_offset: 0x3F_FFFF,
                    policy: NorRegionPolicy::GatedUpperNor,
                    tag: Some("BGO".to_string()),
                    signature: None,
                },
                NorRegion {
                    kind: NorRegionKind::Eemul,
                    start_offset: 0x7F_0000,
                    end_offset: 0x7F_FFFF,
                    policy: NorRegionPolicy::NvramSanitize,
                    tag: Some("EEMUL".to_string()),
                    signature: Some(RegionSignature {
                        marker_byte_offset: 5,
                        marker_value: 0xF1,
                    }),
                },
            ],
            evidence: ev(EvidenceTier::BenchVerified, "bench"),
        },
    );
    bundle.boards.insert(
        "vep79138".to_string(),
        BoardProfile {
            id: "vep79138".to_string(),
            pcb_code: "VEP79138".to_string(),
            service_module_code: None,
            soc_ref: Some("mn2ds0009".to_string()),
            flash_ref: Some("spansion_s29jl064h_8mb".to_string()),
            eeprom_refs: vec!["panasonic_c3ebfc000053_timer".to_string()],
            p7501_header_present: true,
            has_hdd_interface: true,
            has_sd_slot: true,
            evidence: ev(EvidenceTier::BenchVerified, "bench"),
        },
    );
    bundle.models.insert(
        "dmr_eh55".to_string(),
        ModelProfileManifest {
            model_id: "DMR-EH55".to_string(),
            family_label: "DMR-EH55".to_string(),
            series: "Série EH (HDD+DVD)".to_string(),
            region_suffix: Some("EC".to_string()),
            tv_standard: None,
            hw_code: Some("M6930".to_string()),
            board_ref: Some("vep79138".to_string()),
            train_ref: Some("loader_ver061_toc48_main57".to_string()),
            nor_layout_ref: Some("eh55_ver061_on_s29jl064h".to_string()),
            exact_aliases: vec!["DMR-EH55LB-S".to_string()],
            fixture_ref: None,
            claims: ModelClaims {
                board: ev(EvidenceTier::BenchVerified, "bench"),
                flash_chip: ev(EvidenceTier::BenchVerified, "bench"),
                nor_layout: ev(EvidenceTier::BenchVerified, "bench"),
                train: ev(EvidenceTier::BenchVerified, "bench"),
                eeprom: ev(EvidenceTier::ServiceManualOnly, "DSD0603003CE"),
            },
        },
    );
    bundle
}

#[test]
fn datasheet_sector_derivation_and_ipl_256k_floor() {
    let mut b = sample_bundle();
    let al016 = b.flash_chips.get("spansion_s29al016d_2mb_top").unwrap();
    let table_16 = derive_sector_table(al016).unwrap();
    assert_eq!(table_16.len(), 35);
    assert_eq!(table_16[31].size_bytes, 0x8000);
    assert_eq!(table_16[34].size_bytes, 0x4000);
    assert_eq!(sector_of_offset(al016, 0x001F_0000).unwrap().index, 31);
    let (sa_a, sa_b) = sector_range_for_aligned_span(al016, 0x001F_0000, 0x001F_FFFF).unwrap();
    assert_eq!(format_sa_range(sa_a, sa_b), "SA31..SA34");
    assert!(derived_geometry_label(al016).contains("SA0..SA34"));

    // Resolve EH55 & verify derived IPL / BGO / EEMUL labels
    let eh55 = resolve_model_in_bundle(&b, "DMR-EH55LB-S").unwrap();
    assert_eq!(eh55.bgo_sector_range_derived.as_deref(), Some("SA26..SA70"));
    assert_eq!(
        eh55.nvram_sector_range_derived.as_deref(),
        Some("SA134..SA141")
    );
    assert_eq!(eh55.nvram_uncached_va_hex.as_deref(), Some("0x847F0000"));
    assert!(eh55.effective_capabilities.nvram_write_live);
    assert!(eh55.effective_capabilities.upper_nor_write_live);

    // Regression A3: IPL < 256 KiB (e.g. 128 KiB) MUST fail validation
    b.nor_layouts
        .get_mut("eh55_ver061_on_s29jl064h")
        .unwrap()
        .regions[0]
        .end_offset = 0x01_FFFF;
    let rep = validate_bundle(&b);
    assert!(!rep.valid);
    assert!(rep.errors.iter().any(|e| e.contains("256 KiB")));
    assert_eq!(rep.checks_failed, rep.errors.len());
}

#[test]
fn alias_collision_and_overlay_downgrade_are_fail_closed() {
    let mut b = sample_bundle();
    let mut second = b.models.get("dmr_eh55").unwrap().clone();
    second.model_id = "DMR-EH56".to_string();
    second.exact_aliases = vec!["DMR-EH55LB-S".to_string()];
    b.models.insert("dmr_eh56".to_string(), second);
    let rep = validate_bundle(&b);
    assert!(!rep.valid);
    assert!(
        rep.errors
            .iter()
            .any(|e| e.contains("duplicate model identifier/alias"))
    );

    // Regression A2: runtime overlay cannot overwrite built-in flash chip ID
    let tmp = std::env::temp_dir().join(format!("panadiga_overlay_sec_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("flash_chips")).unwrap();
    fs::write(
        tmp.join("flash_chips/override.json"),
        serde_json::to_string_pretty(&s29jl064h_chip()).unwrap(),
    )
    .unwrap();
    let base = sample_bundle();
    assert!(merge_overlay_dir(base.clone(), &tmp).is_err());

    // Brand-new overlay model claiming T4 MUST be stripped down to T0
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("models")).unwrap();
    let mut custom = base.models.get("dmr_eh55").unwrap().clone();
    custom.model_id = "DMR-CUSTOM-LAB".to_string();
    custom.exact_aliases = vec![];
    fs::write(
        tmp.join("models/dmr_custom_lab.json"),
        serde_json::to_string_pretty(&custom).unwrap(),
    )
    .unwrap();
    let merged = merge_overlay_dir(base, &tmp).unwrap();
    let resolved = resolve_model_in_bundle(&merged, "DMR-CUSTOM-LAB").unwrap();
    assert_eq!(
        resolved.manifest.claims.board.tier,
        EvidenceTier::CommunityUnverified
    );
    assert!(!resolved.effective_capabilities.nvram_write_live);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn capability_gate_enforces_tiers_wp_otp_and_lab_override() {
    let mut b = sample_bundle();
    let eh55 = resolve_model_in_bundle(&b, "DMR-EH55").unwrap();

    // C5 Critical IPL / 0xC000100C ALWAYS denied, even with LabOverride
    let lab = LabOverride {
        disclaimer_ack: LAB_OVERRIDE_DISCLAIMER_ACK.to_string(),
        operator_reason: "custom firmware testing".to_string(),
    };
    assert!(
        CapabilityGate::authorize(
            &eh55,
            Capability::CriticalIplOrLethalMmio,
            &RuntimeFingerprint::default(),
            Some(&lab)
        )
        .is_err()
    );

    // C1 without F1 marker on 8 MiB dump denied unless LabOverride
    let fp_no_f1 = RuntimeFingerprint {
        dump_size_bytes: Some(0x0080_0000),
        has_eemul_f1_marker: false,
        ..Default::default()
    };
    assert!(
        CapabilityGate::authorize(&eh55, Capability::NvramSanitizeOfflineCow, &fp_no_f1, None)
            .is_err()
    );
    let auth_cov = CapabilityGate::authorize(
        &eh55,
        Capability::NvramSanitizeOfflineCow,
        &fp_no_f1,
        Some(&lab),
    )
    .unwrap();
    assert_eq!(auth_cov.mode, AuthorizationMode::LabOverrideUnverified);

    // Regression A12: top-sector WP# denies C2 live write unless wp_pin_deasserted == Some(true)
    b.flash_chips
        .get_mut("spansion_s29jl064h_8mb")
        .unwrap()
        .hw_protect = Some(FlashHwProtect {
        pin: "WP#".to_string(),
        protects_region: "SA141".to_string(),
        affects_top_sector: true,
    });
    let wp_eh55 = resolve_model_in_bundle(&b, "DMR-EH55").unwrap();
    assert!(
        CapabilityGate::authorize(
            &wp_eh55,
            Capability::NvramWriteLive,
            &RuntimeFingerprint::default(),
            None
        )
        .is_err()
    );
    assert!(
        CapabilityGate::authorize(
            &wp_eh55,
            Capability::NvramWriteLive,
            &RuntimeFingerprint {
                wp_pin_deasserted: Some(true),
                ..Default::default()
            },
            None
        )
        .is_ok()
    );

    // OTP PROM refuses even LabOverride C1/C4b? C1 must reject non-writable OTP!
    let otp = FlashChipProfile {
        id: "oki_mr27v802".to_string(),
        vendor: "OKI".to_string(),
        part_number: "MR27V802".to_string(),
        package: "SOP-44".to_string(),
        voltage_min_mv: 2700,
        voltage_max_mv: 3600,
        nominal_voltage_mv: REQUIRED_LVTTL_MV,
        writable: false,
        total_bytes: 0x0010_0000,
        jedec: None,
        sector_banks: vec![],
        rw_banks: vec![],
        hw_protect: None,
        evidence: ev(EvidenceTier::CommunityUnverified, "forum"),
    };
    b.flash_chips.insert("oki_mr27v802".to_string(), otp);
    b.boards.get_mut("vep79138").unwrap().flash_ref = Some("oki_mr27v802".to_string());
    b.models.get_mut("dmr_eh55").unwrap().nor_layout_ref = None;
    let otp_model = resolve_model_in_bundle(&b, "DMR-EH55").unwrap();
    assert!(
        CapabilityGate::authorize(
            &otp_model,
            Capability::NvramSanitizeOfflineCow,
            &RuntimeFingerprint::default(),
            Some(&lab)
        )
        .is_err()
    );
}

#[test]
fn real_profiles_dir_and_schemas_stay_synchronized() {
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let bundle =
        load_bundle_from_dir(&profiles_dir).expect("Real profiles/ tree must validate cleanly");
    assert_eq!(model_storage_key("DMR-EH55"), "dmr_eh55");
    assert!(bundle.models.contains_key("dmr_eh55"));
    assert!(bundle.models.contains_key("dmr_es35v"));
    assert!(bundle.models.contains_key("dmr_ez485v"));

    let report = validate_bundle(&bundle);
    assert!(
        report.valid,
        "Real profiles/ bundle validation failed: {:?}",
        report.errors
    );
    assert_eq!(report.checks_failed, 0);

    for (storage_key, manifest) in &bundle.models {
        let resolved = resolve_model_in_bundle(&bundle, &manifest.model_id).unwrap_or_else(|e| {
            panic!(
                "Model '{storage_key}' ('{}') must resolve cleanly: {e}",
                manifest.model_id
            )
        });
        let model_id = &manifest.model_id;
        if let Some(board_ref) = &resolved.manifest.board_ref {
            assert!(
                resolved.board.is_some(),
                "Model '{model_id}' dangling board_ref '{board_ref}'"
            );
        }
        if let Some(train_ref) = &resolved.manifest.train_ref {
            assert!(
                resolved.firmware_train.is_some(),
                "Model '{model_id}' dangling train_ref '{train_ref}'"
            );
        }
        if let Some(layout_ref) = &resolved.manifest.nor_layout_ref {
            assert!(
                resolved.nor_layout.is_some(),
                "Model '{model_id}' dangling nor_layout_ref '{layout_ref}'"
            );
        }
        if let Some(board) = &resolved.board
            && let Some(flash_ref) = &board.flash_ref
        {
            assert!(
                resolved.flash_chip.is_some(),
                "Model '{model_id}' dangling flash_ref '{flash_ref}'"
            );
        }
    }

    let schema_dir = profiles_dir.join("schema");
    assert!(
        schema_dir.is_dir(),
        "Committed profiles/schema directory must exist"
    );
    let schema_count = fs::read_dir(&schema_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .count();
    assert_eq!(
        schema_count, 12,
        "profiles/schema must contain all 12 schemas"
    );
    verify_schemas_match_dir(&schema_dir).expect("Committed JSON Schemas match schemars");
}

#[test]
fn bgo_retarget_spec_and_blob_validation_enforce_bounds_and_integrity() {
    let mut b = sample_bundle();
    let blob_rel = "blobs/test_drv.bin.gz".to_string();
    b.blobs.insert(
        blob_rel.clone(),
        crate::blob_b64::encode_base64(b"test-drv"),
    );
    let spec = ProgBgoRetargetSpec {
        target_geometry_id: "TEST_GEO".to_string(),
        expected_prog_size: 0x0004_0000,
        bgo_table_offset: 0x0003_6E2C,
        bgo_table_marker: 0x8000_0000,
        detect_logical_sectors: 0x28,
        detect_flash_base: 0x8413_0000,
        bgo_table_words: vec![0u32; 64],
        bgo_sum_beq_offset: 0x0002_30CF,
        nfrom_drv_start: 0x0002_1E6A,
        nfrom_drv_end: 0x0002_9356,
        nfrom_drv_uncompressed_bytes: 29_770,
        nfrom_drv_blob_relpath: blob_rel.clone(),
        nfrom_drv_uncompressed_sha256: "a".repeat(64),
        nop_fill_byte: 0xCB,
        bss_zero_end_offset: 0x07C0,
        orig_bss_zero_end: 0x504F_9178,
        target_bss_zero_end: 0x504F_E000,
        init2_call_offset: 0x24C6,
        orig_init2_rel32: 0x0001_FE93,
        target_init2_rel32: 0x0001_FE82,
        hi_ram_ranges: vec![OffsetRange {
            start: 0x100,
            end: 0x200,
        }],
        hi_ram_relocs: vec![AddressReloc {
            from: 0x5049_0000,
            to: 0x504F_A000,
        }],
        byte_patches: vec![BytePatchEntry {
            offset: 0x150,
            expect: 0x0A,
            replace: 0x0F,
            note: None,
        }],
    };
    b.firmware_trains
        .get_mut("loader_ver061_toc48_main57")
        .unwrap()
        .bgo_retarget_spec = Some(spec.clone());
    assert!(validate_bundle(&b).valid);

    // Mismatched flash base against model's nor_layout BGO region must fail validation.
    let mut bad_base = b.clone();
    bad_base
        .firmware_trains
        .get_mut("loader_ver061_toc48_main57")
        .unwrap()
        .bgo_retarget_spec
        .as_mut()
        .unwrap()
        .detect_flash_base = 0x8420_0000;
    let rep_bad_base = validate_bundle(&bad_base);
    assert!(!rep_bad_base.valid);
    assert!(
        rep_bad_base
            .errors
            .iter()
            .any(|e| e.contains("BGO start 0x130000"))
    );

    // Malformed base64 or missing blob fails validation.
    let mut bad_blob = b.clone();
    bad_blob.blobs.insert(blob_rel, "!!!".to_string());
    assert!(!validate_bundle(&bad_blob).valid);

    // Path traversal rejected by validate_bundle.
    let mut bad_rel = b.clone();
    bad_rel
        .firmware_trains
        .get_mut("loader_ver061_toc48_main57")
        .unwrap()
        .bgo_retarget_spec
        .as_mut()
        .unwrap()
        .nfrom_drv_blob_relpath = "../escape.gz".to_string();
    assert!(!validate_bundle(&bad_rel).valid);
}

#[test]
fn region_interpolation_derives_tv_standard_hw_code_and_electrical_specs() {
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let b = load_bundle_from_dir(&profiles_dir).expect("load real profiles");

    let eh55 = resolve_model_in_bundle(&b, "DMR-EH55").unwrap();
    assert_eq!(eh55.manifest.tv_standard.as_deref(), Some("PAL/PAL-M"));
    assert!(eh55.active_region.is_none());
    assert_eq!(eh55.regional_variants.len(), 2);

    let eh55_lb = resolve_model_in_bundle(&b, "DMR-EH55LB-S").unwrap();
    assert_eq!(eh55_lb.manifest.tv_standard.as_deref(), Some("PAL-M"));
    assert_eq!(eh55_lb.manifest.hw_code.as_deref(), Some("M6930LB"));
    let active_lb = eh55_lb.active_region.expect("active LB region");
    assert_eq!(active_lb.dvd_region_code, 4);
    assert_eq!(active_lb.oem_marker, "M");
    assert_eq!(active_lb.aiof_vmodesel_hex, "0x0002");

    let eh55_ec = resolve_model_in_bundle(&b, "DMR-EH55EC").unwrap();
    assert_eq!(eh55_ec.manifest.tv_standard.as_deref(), Some("PAL"));
    assert_eq!(eh55_ec.manifest.hw_code.as_deref(), Some("M6930"));
    assert_eq!(eh55_ec.active_region.expect("active EC").dvd_region_code, 2);

    let es35v = resolve_model_in_bundle(&b, "DMR-ES35V").unwrap();
    assert_eq!(es35v.manifest.tv_standard.as_deref(), Some("PAL/SECAM"));
    assert_eq!(es35v.active_region.expect("active EE").dvd_region_code, 5);

    let ez485v = resolve_model_in_bundle(&b, "DMR-EZ485V").unwrap();
    assert_eq!(ez485v.manifest.tv_standard.as_deref(), Some("NTSC/ATSC"));
    assert_eq!(ez485v.manifest.hw_code.as_deref(), Some("M4831P"));
    assert_eq!(ez485v.active_region.expect("active VP").dvd_region_code, 1);

    let eh58 = resolve_model_in_bundle(&b, "DMR-EH58").unwrap();
    assert_eq!(eh58.manifest.tv_standard.as_deref(), Some("PAL/SECAM"));
    assert_eq!(
        eh58.regional_variants[1].interpolated_hw_code.as_deref(),
        Some("EH58EC")
    );
}
