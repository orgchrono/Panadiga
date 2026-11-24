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
    AddressReloc, AuthorizationMode, BinaryPatchSiteSpec, BoardProfile, BusWindow, BytePatchEntry,
    Capability, ClaimEvidence, EepromChipProfile, EepromFunction, EvidenceSource, EvidenceTier,
    FirmwareTrainProfile, FlashChipProfile, FlashHwProtect, FlashJedecId, FlashRwBank,
    FlashSectorBank, FrmContainerSpec, LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride,
    MainBgoRetargetAnchors, MainBuildSpec, MediaFormatProfile, ModelClaims, ModelProfileManifest,
    NorLayoutProfile, NorRegion, NorRegionKind, NorRegionPolicy, OffsetRange, ProfilesBundle,
    ProgBgoRetargetSpec, RegionSignature, RuntimeFingerprint, SataBridgeResetSpec, SocCoreProfile,
    SocProfile, Stage2BootSpec, Stage2BootSymbolsSpec,
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
            ghidra_language_id: Some("mn10300:LE:32:default".to_string()),
            ghidra_compiler_id: Some("default".to_string()),
            ghidra_sleigh_provider: Some(
                "GhidraMn10300+am33_panadiga_zero_undef.patch".to_string(),
            ),
            default_base_address: Some(0x4000_0000),
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
            soc_core_ref: Some("mn10300_am34".to_string()),
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
            prog_sata_bridge_reset_spec: None,
            stage2_boot_spec: None,
            main_builds: Vec::new(),
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
    let mut table_words = vec![0u32; 64];
    table_words[12] = 0x8413_0000;
    table_words[13] = 0x8440_0000;
    table_words[15] = 0xFFFF_FFFF;
    table_words[31] = 45;
    let spec = ProgBgoRetargetSpec {
        target_geometry_id: "BGO_45SEC_8MB_0X84130000".to_string(),
        expected_prog_size: 0x0004_0000,
        bgo_table_offset: 0x0003_6E2C,
        bgo_table_marker: 0x8000_0000,
        detect_logical_sectors: 0x28,
        detect_flash_base: 0x8413_0000,
        bgo_table_words: table_words,
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

#[test]
fn validate_bundle_rejects_mismatched_schema_version() {
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let mut b = load_bundle_from_dir(&profiles_dir).expect("load real profiles");
    b.schema_version = 999;
    let report = validate_bundle(&b);
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("schema_version 999"))
    );
}

#[test]
fn validate_bundle_rejects_4mb_vs_8mb_bgo_flash_and_sector_traps() {
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let base_real = load_bundle_from_dir(&profiles_dir).expect("load real profiles");
    assert!(
        validate_bundle(&base_real).valid,
        "Built-in profiles including 4MB ver0.63 layout must pass"
    );

    // 1. 8MB BGO train bound to a 4MB Flash layout fails closed.
    let mut mis_8_on_4 = base_real.clone();
    mis_8_on_4
        .flash_chips
        .get_mut("spansion_s29jl064h_8mb")
        .unwrap()
        .total_bytes = 0x0040_0000;
    let rep_8_on_4 = validate_bundle(&mis_8_on_4);
    assert!(!rep_8_on_4.valid);
    assert!(
        rep_8_on_4
            .errors
            .iter()
            .any(|e| e.contains("declares flash capacity 0x800000"))
    );

    // 2. 4MB BGO geometry bound to an 8MB Flash layout fails closed.
    let mut mis_4_on_8 = base_real.clone();
    mis_4_on_8
        .nor_layouts
        .get_mut("loader_ver063_4mb_top_bgo35")
        .unwrap()
        .flash_ref = "spansion_s29jl064h_8mb".to_string();
    let rep_4_on_8 = validate_bundle(&mis_4_on_8);
    assert!(!rep_4_on_8.valid);
    assert!(
        rep_4_on_8
            .errors
            .iter()
            .any(|e| e.contains("declares flash capacity 0x400000"))
    );

    // 3. Wrong word[31] sector count (35 instead of 45) for 0x84130000..0x84400000 fails closed.
    let mut bad_sectors = base_real.clone();
    bad_sectors
        .firmware_trains
        .get_mut("loader_ver061_toc48_main57")
        .unwrap()
        .bgo_retarget_spec
        .as_mut()
        .unwrap()
        .bgo_table_words[31] = 35;
    let rep_sectors = validate_bundle(&bad_sectors);
    assert!(!rep_sectors.valid);
    assert!(
        rep_sectors
            .errors
            .iter()
            .any(|e| e.contains("word[31]*64KiB"))
    );

    // 4. Wrong digadiag_spec.exact_bytes for extrhead_128k (e.g. 262144 instead of 131072) fails closed.
    let mut bad_diag = base_real;
    bad_diag
        .firmware_trains
        .get_mut("loader_ver063_bootbin")
        .unwrap()
        .digadiag_spec = Some(crate::types::DigadiagSpec {
        format: "extrhead_128k".to_string(),
        filename: "DIGADIAG.FRM".to_string(),
        exact_bytes: 262_144,
        load_va: 0x5040_0000,
        entry_va: 0x5040_0000,
        panamon1_entry_va: None,
        bgo_geometry_target: "BGO_35SEC_4MB_0X84200000".to_string(),
        fvu_decoupled_mode: "bypass_checks_only".to_string(),
        sd_media_requirement: "fat16b_0x06_max_2gb".to_string(),
        trust_model: Some(crate::types::DigadiagTrustModel::EncryptedToc),
    });
    let rep_diag = validate_bundle(&bad_diag);
    assert!(!rep_diag.valid);
    assert!(
        rep_diag
            .errors
            .iter()
            .any(|e| e.contains("requires exact_bytes=131072"))
    );
}

#[test]
fn universal_patch_stage2_and_main_build_specs_enforce_bounds_ipl_and_geometry() {
    let mut good = sample_bundle();
    let trn = good
        .firmware_trains
        .get_mut("loader_ver061_toc48_main57")
        .unwrap();
    trn.prog_sata_bridge_reset_spec = Some(SataBridgeResetSpec {
        target_name: "PROG061".to_string(),
        base_va: 0x5040_0000,
        expected_size: 262_144,
        factory_wait_ms: 2,
        patched_wait_ms: 1000,
        sleep_callee_va: 0x5043_1303,
        site: BinaryPatchSiteSpec {
            name: "prog061_sata_reset".to_string(),
            description: "2 ms -> 1000 ms".to_string(),
            va: 0x5041_D928,
            expect_hex: "2cd00789ddbadaffff0000".to_string(),
            replace_hex: "2c640089ddd73901000000".to_string(),
            symbol_ref: None,
        },
    });
    let sym = |idx: u32| 0x5040_1000 + idx * 0x100;
    trn.stage2_boot_spec = Some(Stage2BootSpec {
        base_va: 0x5040_0000,
        expected_size: 262_144,
        hook_offset: 0x0000_5100,
        hook_expect_hex: "dd084f00000000".to_string(),
        payload_offset: 0x0003_A000,
        require_reset_fix_1000ms: true,
        uart_abort_polls: 25,
        uart_poll_ticks_10ms: 2,
        hdd_primary_lba: 0x0001_2000,
        hdd_mirror_lba_delta: 0x0002_4000,
        hdd_probe_sectors: 128,
        feistel_crypt_span: 0x0001_0000,
        main_va_min: 0x5000_0000,
        main_va_max: 0x57FF_FFFF,
        dcache_purge_base: 0x5710_0000,
        dcache_purge_len: 0x0010_0000,
        sd_info_bss_va: 0x5048_1000,
        hdd_dma_buf_va: 0x90C1_0000,
        fvu_load_uncached_va: 0x9040_0000,
        norfrom_fvu_base_va: 0x8404_0000,
        symbols: Stage2BootSymbolsSpec {
            dcache_purge_va: sym(1),
            uart0_getc_nb_va: sym(2),
            uart0_printf_va: sym(3),
            check_update_flag_va: sym(4),
            hdd_max_lba_va: sym(5),
            hdd_read_va: sym(6),
            sd_init_va: sym(7),
            stop_timer_irq_va: sym(8),
            feistel_decrypt_va: sym(9),
            extrhead_get_size_va: sym(10),
            extrhead_check_hdr_va: sym(11),
            extrhead_is_comp_va: sym(12),
            extrhead_get_src_va: sym(13),
            extrhead_get_entry_va: sym(14),
            extrhead_uncomp_va: sym(15),
            fat_mount_va: sym(16),
            fat_open_va: sym(17),
            fat_close_va: sym(18),
            disable_irq_va: sym(19),
            memcpy_va: sym(20),
            sleep_10ms_va: sym(21),
        },
    });
    trn.main_builds = vec![MainBuildSpec {
        build_id: "3410PQ".to_string(),
        slot_name: "MAIN3410PQ".to_string(),
        frm_version_u16: 0x3410,
        base_va: 0x5700_0000,
        unpacked_size: 13_003_052,
        region_tv_standard: Some("pal_m".to_string()),
        model_markers: vec!["DMR-EH55".to_string()],
        chassis_family: Some("M6930LB".to_string()),
        native_bgo_geometry_id: "BGO_45SEC_8MB_0X84130000".to_string(),
        native_bgo_detect_va: Some(0x5756_EFD4),
        native_bgo_flash_base_va: Some(0x8413_0000),
        supports_bgo_retarget_to: Vec::new(),
        bgo_retarget_anchors: Some(MainBgoRetargetAnchors {
            factory_geometry_id: "BGO_35SEC_4MB_0X84200000".to_string(),
            target_geometry_id: "BGO_45SEC_8MB_0X84130000".to_string(),
            nfd_text_base_va: 0x5740_1000,
            rodata_table_base_va: 0x5780_1000,
            bss_nfd_ctrl_base_va: 0x5301_0000,
            bss_reserved_base_va: 0x534F_0000,
            bss_cached_end_site_va: 0x5741_1000,
            bss_cached_end_orig: 0x534E_0000,
            crt0_uncached_end_site_va: 0x5700_1000,
            sbrk_uncached_end_site_va: 0x5770_1000,
            bss_uncached_end_orig: 0x934E_0000,
            cave_va: 0x5700_0400,
        }),
        sata_bridge_reset_spec: None,
        av_passthrough_sites: Vec::new(),
        unpacked_sha256: Some(
            "c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1".to_string(),
        ),
    }];
    assert!(validate_bundle(&good).valid);

    let mut bad = good;
    let bad_trn = bad
        .firmware_trains
        .get_mut("loader_ver061_toc48_main57")
        .unwrap();
    let bad_prog = bad_trn.prog_sata_bridge_reset_spec.as_mut().unwrap();
    bad_prog.site.va = 0x5045_0000;
    bad_trn
        .stage2_boot_spec
        .as_mut()
        .unwrap()
        .norfrom_fvu_base_va = 0x8401_0000;
    bad_trn.main_builds[0].native_bgo_flash_base_va = Some(0x8420_0000);
    let bad_anc = bad_trn.main_builds[0]
        .bgo_retarget_anchors
        .as_mut()
        .unwrap();
    bad_anc.bss_reserved_base_va = 0x534F_0004;
    bad_anc.nfd_text_base_va = 0x5800_0000;
    bad_trn.main_builds[0]
        .av_passthrough_sites
        .push(BinaryPatchSiteSpec {
            name: "bad_hex_site".to_string(),
            description: "malformed hex test".to_string(),
            va: 0x5700_0000,
            expect_hex: "ZZ".to_string(),
            replace_hex: "00".to_string(),
            symbol_ref: None,
        });
    let rep = validate_bundle(&bad);
    assert!(!rep.valid);
    assert!(rep.errors.iter().any(|e| e.contains("invalid hex bytes")));
    assert!(rep.errors.iter().any(|e| e.contains("out of bounds")));
    assert!(rep.errors.iter().any(|e| e.contains("immutable IPL guard")));
    assert!(
        rep.errors
            .iter()
            .any(|e| e.contains("native_bgo_flash_base_va"))
    );
    assert!(
        rep.errors
            .iter()
            .any(|e| e.contains("bss_reserved_base_va"))
    );
    assert!(rep.errors.iter().any(|e| e.contains("out of MAIN bounds")));
}

#[test]
fn locales_catalog_matches_traps_and_interpolates_bilingual() {
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let bundle = load_bundle_from_dir(&profiles_dir).expect("load real profiles");
    let catalog = crate::i18n::load_locales_from_dir(&profiles_dir).expect("load locales");
    crate::i18n::validate_locale_catalog(&catalog, &bundle)
        .expect("real profiles locales must pass parity, no-hex, and trap coverage");

    let mut params = BTreeMap::new();
    params.insert(
        "factory_geometry_id",
        "BGO_35SEC_4MB_0X84200000".to_string(),
    );
    params.insert("target_geometry_id", "BGO_45SEC_8MB_0X84130000".to_string());
    params.insert("base_va", "0x84130000".to_string());
    params.insert("sector_count", "45".to_string());

    let pt = catalog
        .render(
            crate::i18n::LocaleId::PtBr,
            "train.loader_ver061_toc48_main57.bgo_retarget_notes",
            &params,
        )
        .expect("pt-BR render");
    let en = catalog
        .render(
            crate::i18n::LocaleId::EnUs,
            "train.loader_ver061_toc48_main57.bgo_retarget_notes",
            &params,
        )
        .expect("en-US render");
    assert!(pt.contains("45 setores uniformes"));
    assert!(en.contains("45 uniform 64 KiB sectors"));

    params.remove("sector_count");
    assert!(
        catalog
            .render(
                crate::i18n::LocaleId::EnUs,
                "train.loader_ver061_toc48_main57.bgo_retarget_notes",
                &params,
            )
            .is_err()
    );
}

#[test]
fn validate_locale_catalog_rejects_mismatch_hardcoded_hex_and_missing_trap() {
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let bundle = load_bundle_from_dir(&profiles_dir).expect("load real profiles");
    let mut bad = crate::i18n::load_locales_from_dir(&profiles_dir).expect("load locales");

    bad.en_us.entries.insert(
        "train.loader_ver061_toc48_main57.bgo_retarget_notes".to_string(),
        "Retarget {factory_geometry_id} -> {target_geometry_id} at {wrong_var}".to_string(),
    );
    bad.pt_br.entries.insert(
        "trap.TRAP_BGO_4MB_VS_8MB.preventive_guard".to_string(),
        "Hardcoded 0x84200000 inside {guard_paths}".to_string(),
    );
    bad.en_us.entries.remove("trap.TRAP_BGO_4MB_VS_8MB.title");

    let failures = crate::i18n::validate_locale_catalog(&bad, &bundle)
        .expect_err("tampered catalog must fail validation");
    assert!(
        failures
            .iter()
            .any(|f| f.rule == "locale_placeholder_parity"),
        "expected locale_placeholder_parity in {failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|f| f.rule == "locale_no_hardcoded_hex_address"),
        "expected locale_no_hardcoded_hex_address in {failures:?}"
    );
    assert!(
        failures.iter().any(|f| f.rule == "locale_key_parity"),
        "expected locale_key_parity in {failures:?}"
    );
    assert!(
        failures.iter().any(|f| f.rule == "locale_trap_coverage"),
        "expected locale_trap_coverage in {failures:?}"
    );
}

#[test]
fn promote_and_review_proposal_verify_real_on_disk_sha256_and_ledger() {
    let real_profiles = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let root = std::env::temp_dir().join(format!(
        "panadiga_promote_verify_{}_{}",
        std::process::id(),
        file!().len()
    ));
    let _ = fs::remove_dir_all(&root);
    let profiles_dir = root.join("profiles");
    let models_dir = profiles_dir.join("models");
    let staging_dir = profiles_dir.join("staging").join("manual_proposals");
    fs::create_dir_all(&models_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();

    // Copy relational dictionaries and blobs needed for load_bundle_from_dir
    for sub in
        crate::loader::ENTITY_SUBDIRS
            .iter()
            .copied()
            .chain(["blobs", "traps", "forensic_atlas"])
    {
        let src_sub = real_profiles.join(sub);
        if !src_sub.is_dir() {
            continue;
        }
        let dst_sub = profiles_dir.join(sub);
        fs::create_dir_all(&dst_sub).unwrap();
        for entry in fs::read_dir(&src_sub).unwrap() {
            let p = entry.unwrap().path();
            if p.is_file() {
                fs::copy(&p, dst_sub.join(p.file_name().unwrap())).unwrap();
            }
        }
    }

    let fixture_rel = "fixtures/eh55_probe.json";
    let fixture_abs = root.join(fixture_rel);
    fs::create_dir_all(fixture_abs.parent().unwrap()).unwrap();
    fs::write(&fixture_abs, br#"{"model":"DMR-EH55","vdd_mv":3300}"#).unwrap();
    let real_sha = crate::promote::compute_file_sha256_hex(&fixture_abs).unwrap();
    let spoofed_sha = "0".repeat(64);

    // 1. Spoofed 64-hex SHA-256 MUST fail closed
    let err = crate::promote::promote_model_claim(&crate::promote::PromoteClaimRequest {
        workspace_root: &root,
        profiles_dir: &profiles_dir,
        model_id: "DMR-EH55",
        axis: crate::ClaimAxis::Eeprom,
        new_tier: EvidenceTier::PhysicalDumpVerified,
        evidence_sha256: &spoofed_sha,
        fixture_rel_path: fixture_rel,
        bench_log_rel_path: None,
        notes: None,
    })
    .unwrap_err();
    assert!(err.to_string().contains("SHA-256 verification failed"));

    // 2. Staged proposal with spoofed SHA-256 fails review cleanly
    let bad_prop = crate::staging::ManualClaimProposal {
        proposal_id: "PROP-BAD".to_string(),
        model_id: "DMR-EH55".to_string(),
        axis: crate::ClaimAxis::Eeprom,
        proposed_tier: EvidenceTier::PhysicalDumpVerified,
        fixture_rel_path: fixture_rel.to_string(),
        bench_log_rel_path: None,
        declared_sha256: spoofed_sha,
        notes: Some("spoofed hash check".to_string()),
        submitted_by: Some("test".to_string()),
        ledger_entry_id: Some("EV-TEST-1".to_string()),
    };
    let bad_prop_path = staging_dir.join("bad.json");
    fs::write(
        &bad_prop_path,
        serde_json::to_string_pretty(&bad_prop).unwrap(),
    )
    .unwrap();
    let ledger_path = root.join("ledger.json");
    fs::write(
        &ledger_path,
        serde_json::json!({
            "schema_version": 1,
            "entries": [{
                "id": "EV-TEST-1",
                "sha256_assertions": [{ "path": fixture_rel, "sha256": real_sha.to_uppercase() }]
            }]
        })
        .to_string(),
    )
    .unwrap();
    let bad_rep = crate::staging::review_manual_proposal(
        &root,
        &profiles_dir,
        &bad_prop_path,
        true,
        Some(&ledger_path),
    )
    .unwrap();
    assert!(!bad_rep.valid);
    assert!(!bad_rep.sha256_verified);
    assert!(!bad_rep.promoted);

    // 3. Valid staged proposal passes SHA-256 + ledger check and promotes when apply=true
    let ok_prop = crate::staging::ManualClaimProposal {
        declared_sha256: real_sha.clone(),
        proposal_id: "PROP-OK".to_string(),
        ..bad_prop
    };
    let ok_prop_path = staging_dir.join("ok.json");
    fs::write(
        &ok_prop_path,
        serde_json::to_string_pretty(&ok_prop).unwrap(),
    )
    .unwrap();
    assert_eq!(
        crate::staging::list_manual_proposals(&profiles_dir)
            .unwrap()
            .len(),
        2
    );

    let ok_rep = crate::staging::review_manual_proposal(
        &root,
        &profiles_dir,
        &ok_prop_path,
        true,
        Some(&ledger_path),
    )
    .unwrap();
    assert!(ok_rep.valid);
    assert!(ok_rep.sha256_verified);
    assert!(ok_rep.ledger_synced);
    assert!(ok_rep.promoted);
    assert_eq!(ok_rep.computed_sha256.as_deref(), Some(real_sha.as_str()));

    let updated = load_bundle_from_dir(&profiles_dir)
        .unwrap()
        .models
        .get("dmr_eh55")
        .unwrap()
        .clone();
    assert_eq!(
        updated.claims.eeprom.tier,
        EvidenceTier::PhysicalDumpVerified
    );
    assert!(
        updated
            .claims
            .eeprom
            .sources
            .iter()
            .any(|s| s.sha256.as_deref() == Some(real_sha.as_str()))
    );

    let _ = fs::remove_dir_all(&root);
}
