#![deny(unsafe_code)]

//! Opaque-box Tier 1-4 E2E Contract & Real-Hardware Replay Suite for R1/R2/R3/R4.
//!
//! Covers:
//! - R1: Dual `PicoFW` (`PICO>`, `@PICO_*`) primary + legacy (`PANADIGA>`, `P7501>`, `@P7501_*`) fallback
//!   and physical `P7501` DTO preservation (`P7501PinMappingDto`).
//! - R2: `DigaOsConfigSpec` validation, Draft 2020-12 schema parity, `DIGAOS_SSOT_CONTRACT_HASH`
//!   cross-layer parity (`Rust` <-> `digaos_ssot.h` <-> `digaos.ts`), multi-SoC isolation,
//!   and lethal MMIO / IPL RO boundary rejection.
//! - R3: Deterministic `ndb>` replay of `com10_live_volatile_patch_20261124.log` &
//!   `com10_live_video_osd_diag_20261124.log`, LE-word `.to_le_bytes()` decoding, BCU `BCBERR`
//!   57-task abort, `STANDBY_GATE_STOP`, fail-closed lethal MMIO + NOR write blocks, hot-diff,
//!   reverse-order `rollback()`, and `MAIN3060NP` `/HDDEX/UDA_M6930_H_JPN_V006` caveat.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use panadiga_core::constants::{
    AIOF_FS0_NON_STANDARD, AIOF_FS0_STANDARD, AM34_AIOF_BASE, AM34_LETHAL_MMIO_ADDR,
    PICOFW_RES_CODES, PICOFW_WIRE_TAGS, resolve_workspace_root,
};
use panadiga_core::pico::bench_decoders::{evaluate_live_rail_gate, parse_status_output_report};
use panadiga_core::pico::ndb::{
    NdbAccessKind, NdbSession, NdbTransport, VolatilePatchOp, detect_ndb_bcu_fault,
    parse_ndb_dd_output, parse_ndb_write_echo, strip_ndb_ansi_prompt, validate_ndb_access,
};
use panadiga_core::pico::protocol::{
    PicoWireMessage, parse_boot_class_line, parse_ident_line, parse_push_frame_line,
    parse_res_line, parse_wire_line,
};
use panadiga_core::pico::rsp_codec::{
    NorWritePolicy, is_am34_lethal_mmio_address, is_nor_write_blocked_for_flash,
};
use panadiga_core::re::region_compat::decode_market_code_name;
use panadiga_core::re::region_compat::tv_standard::AVDD_NPALCTL0_PRESERVED_HI_WORD;
use panadiga_core::ssot::{
    DIGAOS_SSOT_CONTRACT_HASH, P7501PinMappingDto, build_hardware_ssot_spec,
    export_all_ssot_projections,
};
use panadiga_profiles::types::digaos::{DigaOsConfigSpec, DigaOsMmioRangeSpec};
use panadiga_profiles::{validate_digaos_config, verify_schemas_match_dir};

fn workspace_root() -> Result<PathBuf> {
    let resolved = resolve_workspace_root();
    if resolved.join("Cargo.toml").is_file() {
        return Ok(resolved);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("Failed to resolve Panadiga workspace root")
}

fn read_evidence_log(name: &str) -> Result<String> {
    let path = workspace_root()?.join("docs").join("evidence").join(name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read evidence log {}", path.display()))
}

fn load_default_digaos_config() -> Result<(PathBuf, DigaOsConfigSpec)> {
    let root = workspace_root()?;
    let cfg_path = root
        .join("profiles")
        .join("digaos")
        .join("default_digaos_config.json");
    let raw = std::fs::read_to_string(&cfg_path)
        .with_context(|| format!("Failed to read {}", cfg_path.display()))?;
    let cfg: DigaOsConfigSpec = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse {}", cfg_path.display()))?;
    Ok((root, cfg))
}

fn decode_ndb_words_le(words: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

struct MockNdbTransport {
    sent_commands: RefCell<Vec<String>>,
    memory: RefCell<BTreeMap<u32, u8>>,
}

impl MockNdbTransport {
    fn from_le_words(base_va: u32, words: &[u32]) -> Self {
        let mut map = BTreeMap::new();
        for (idx, word) in words.iter().enumerate() {
            let addr = base_va + (idx as u32) * 4;
            for (b_off, byte) in word.to_le_bytes().into_iter().enumerate() {
                map.insert(addr + (b_off as u32), byte);
            }
        }
        Self {
            sent_commands: RefCell::new(Vec::new()),
            memory: RefCell::new(map),
        }
    }

    fn seed_u16_le(&self, addr: u32, val: u16) {
        let mut mem = self.memory.borrow_mut();
        let [lo, hi] = val.to_le_bytes();
        mem.insert(addr, lo);
        mem.insert(addr + 1, hi);
    }

    fn read_words(&self, base_va: u32, byte_len: usize) -> String {
        let mem = self.memory.borrow();
        let word_count = byte_len.div_ceil(4);
        let mut line = format!("{base_va:08X}:");
        for w_idx in 0..word_count {
            let a = base_va + (w_idx as u32) * 4;
            let raw = [
                *mem.get(&a).unwrap_or(&0),
                *mem.get(&(a + 1)).unwrap_or(&0),
                *mem.get(&(a + 2)).unwrap_or(&0),
                *mem.get(&(a + 3)).unwrap_or(&0),
            ];
            let word = u32::from_le_bytes(raw);
            line.push_str(&format!(" {word:08X}"));
        }
        line.push_str("\n\x1b[31mndb> \x1b[m");
        line
    }

    fn commands(&self) -> Vec<String> {
        self.sent_commands.borrow().clone()
    }
}

impl NdbTransport for MockNdbTransport {
    fn execute_command(&mut self, cmd: &str) -> Result<String> {
        let trimmed = cmd.trim().to_string();
        self.sent_commands.borrow_mut().push(trimmed.clone());
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        match parts.as_slice() {
            ["dd", addr_hex, len_dec] => {
                let addr = u32::from_str_radix(addr_hex.trim_start_matches("0x"), 16)?;
                let len = len_dec.parse::<usize>()?;
                Ok(self.read_words(addr, len))
            }
            ["eb", addr_hex, val_hex] => {
                let addr = u32::from_str_radix(addr_hex.trim_start_matches("0x"), 16)?;
                let new_val = u8::from_str_radix(val_hex.trim_start_matches("0x"), 16)?;
                let mut mem = self.memory.borrow_mut();
                let old_val = *mem.get(&addr).unwrap_or(&0);
                mem.insert(addr, new_val);
                Ok(format!("{addr:08X}: {old_val:02X} -> {new_val:02X}\nndb> "))
            }
            ["ew", addr_hex, val_hex] => {
                let addr = u32::from_str_radix(addr_hex.trim_start_matches("0x"), 16)?;
                let new_val = u16::from_str_radix(val_hex.trim_start_matches("0x"), 16)?;
                let mut mem = self.memory.borrow_mut();
                let lo = *mem.get(&addr).unwrap_or(&0);
                let hi = *mem.get(&(addr + 1)).unwrap_or(&0);
                let old_val = u16::from_le_bytes([lo, hi]);
                let [nlo, nhi] = new_val.to_le_bytes();
                mem.insert(addr, nlo);
                mem.insert(addr + 1, nhi);
                Ok(format!("{addr:08X}: {old_val:04X} -> {new_val:04X}\nndb> "))
            }
            _ => bail!("Unsupported mock ndb> command: {trimmed}"),
        }
    }
}

#[test]
fn r1_wire_dual_protocol_parses_picofw_primary_and_legacy_p7501_fallback() -> Result<()> {
    assert!(
        PICOFW_WIRE_TAGS.iter().any(|(_, tag)| *tag == "@PICO_RES"),
        "PICOFW_WIRE_TAGS must define @PICO_RES"
    );
    assert!(
        PICOFW_RES_CODES.iter().any(|(_, code)| *code == "E05"),
        "PICOFW_RES_CODES must preserve E05"
    );

    let primary_ident = "@PICO_IDENT vdd_mv=3260 nsw5v=1 elec=AWAKE_READY loader=\"Loader ver0.61\" jedec=0x0001227E flash=S29JL064H ipl_crc32=0xA5B6C7D8 hw=\"VEP79138\" tv=PAL-M/NTSC state=READY model=DMR-EH55 tier=bench_verified caps=0x07 ssot=1234ABCD";
    let legacy_ident = "@P7501_IDENT vdd_mv=3250 nsw5v=1 elec=AWAKE_READY loader=\"Loader ver0.61\" jedec=0x0001227E flash=S29JL064H ipl_crc32=0xA5B6C7D8 hw=\"VEP79138\" tv=NTSC state=READY model=DMR-EH55 tier=bench_verified caps=0x07 ssot=1234ABCD";

    let p_id = parse_ident_line(primary_ident).context("Primary @PICO_IDENT failed")?;
    let l_id = parse_ident_line(legacy_ident).context("Legacy @P7501_IDENT failed")?;
    assert_eq!(p_id.model, "DMR-EH55");
    assert_eq!(l_id.hw, "VEP79138");

    let primary_res =
        parse_res_line("PICO>\n@PICO_RES cmd=STATUS status=OK code=0 elapsed_us=1120")
            .context("Primary @PICO_RES failed")?;
    assert!(primary_res.is_ok());

    let legacy_prompt_prefix = ["PANA", "DIGA>"].concat();
    let legacy_res = parse_res_line(&format!(
        "{legacy_prompt_prefix}\n@P7501_RES cmd=STATUS status=OK code=0 elapsed_us=1735"
    ))
    .context("Legacy @P7501_RES failed")?;
    assert!(legacy_res.is_ok());

    let primary_frame = parse_push_frame_line(
        "@PICO_FRAME seq=1 ts_us=100 type=CRYPTO_KEY len=8 crc32=01020304 idx=0 name=k0 key=0123456789abcdef",
    )
    .context("Primary @PICO_FRAME failed")?;
    assert_eq!(primary_frame.frame_type, "CRYPTO_KEY");

    let wire_msg = parse_wire_line(
        "@PICO_BOOT_CLASS verdict=CLEAN_MAIN_OK build=3410PQ hdd_autoboot=1 avout_ok=1 vmute0=1 ntsc=1 rsv_ok=1 hdd_fail=0 bgo_err=0 fpc_err=0 fpc_to=0 iic_err=0 odd_err=0 op7f_cnt=0 op7f=NONE",
    )
    .context("Primary @PICO_BOOT_CLASS failed")?;
    assert!(matches!(wire_msg, PicoWireMessage::BootClass(_)));

    let ssot = build_hardware_ssot_spec();
    let pinout: &[P7501PinMappingDto] = &ssot.pinout;
    assert!(
        pinout.iter().any(|p| p.signal_name.contains("NSW5V")),
        "Physical P7501 pinout must retain NSW5V"
    );
    assert!(
        pinout.iter().any(|p| p.signal_name.contains("SW_UART")),
        "Physical P7501 pinout must retain SW_UART"
    );
    assert!(
        pinout.iter().any(|p| p.signal_name.contains("XRESET")),
        "Physical P7501 pinout must retain XRESET"
    );
    Ok(())
}

#[test]
fn r2_digaos_ssot_schema_default_config_and_cross_layer_hash_parity() -> Result<()> {
    let (root, cfg) = load_default_digaos_config()?;
    validate_digaos_config(&cfg)?;
    verify_schemas_match_dir(&root.join("profiles").join("schema"))?;

    let digaos_h_path = root
        .join("firmware")
        .join("digaos")
        .join("generated")
        .join("digaos_ssot.h");
    let digaos_ts_path = root
        .join("apps")
        .join("desktop")
        .join("src")
        .join("models")
        .join("generated")
        .join("digaos.ts");

    let digaos_h = std::fs::read_to_string(&digaos_h_path)
        .with_context(|| format!("Missing {}", digaos_h_path.display()))?;
    let digaos_ts = std::fs::read_to_string(&digaos_ts_path)
        .with_context(|| format!("Missing {}", digaos_ts_path.display()))?;

    let expected_hash_hex = format!("0x{DIGAOS_SSOT_CONTRACT_HASH:08X}");
    assert!(digaos_h.contains("DIGAOS_PROMPT \"DIGAOS>\""));
    assert!(digaos_h.contains("NOT LEGALLY CLEAN-ROOM"));
    assert!(
        digaos_h.contains(&expected_hash_hex)
            || digaos_h.contains(&format!("0x{DIGAOS_SSOT_CONTRACT_HASH:08x}")),
        "digaos_ssot.h missing DIGAOS_SSOT_CONTRACT_HASH {expected_hash_hex}"
    );
    assert!(
        digaos_ts.contains(&expected_hash_hex)
            || digaos_ts.contains(&format!("0x{DIGAOS_SSOT_CONTRACT_HASH:08x}")),
        "digaos.ts missing DIGAOS_SSOT_CONTRACT_HASH {expected_hash_hex}"
    );

    for struct_name in [
        "digaos_menu_item_t",
        "digaos_menu_t",
        "digaos_theme_t",
        "digaos_power_profile_t",
        "digaos_soc_target_t",
    ] {
        assert!(
            digaos_h.contains(struct_name),
            "digaos_ssot.h missing C11 contract struct {struct_name}"
        );
    }

    let am34 = cfg
        .soc_targets
        .iter()
        .find(|t| t.soc_core_id.to_ascii_lowercase().contains("am34"))
        .context("Missing AM34 target in default_digaos_config.json")?;
    assert!(am34.is_primary_executable && am34.mmio_verified && am34.logic_voltage_mv == 3300);

    for family in ["am33", "uniphier", "sparc"] {
        let target = cfg
            .soc_targets
            .iter()
            .find(|t| t.soc_core_id.to_ascii_lowercase().contains(family))
            .with_context(|| format!("Missing extensible SoC target for {family}"))?;
        assert!(!target.mmio_verified && !target.is_primary_executable);
        assert!(target.logic_voltage_mv <= 3300);
    }

    let temp_root =
        std::env::temp_dir().join(format!("panadiga_e2e_ssot_proj_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(temp_root.join("profiles").join("digaos"))?;
    std::fs::copy(
        root.join("profiles")
            .join("digaos")
            .join("default_digaos_config.json"),
        temp_root
            .join("profiles")
            .join("digaos")
            .join("default_digaos_config.json"),
    )?;
    let export_res = export_all_ssot_projections(&temp_root);
    let _ = std::fs::remove_dir_all(&temp_root);
    export_res?;
    Ok(())
}

#[test]
fn r2_digaos_safety_validator_rejects_lethal_mmio_and_ipl_writes() -> Result<()> {
    let (_root, base_cfg) = load_default_digaos_config()?;
    assert!(!base_cfg.power_profiles.is_empty());

    let mut no_lethal = base_cfg.clone();
    no_lethal.power_profiles[0].lethal_mmio_guard = false;
    assert!(validate_digaos_config(&no_lethal).is_err());

    let mut no_ipl = base_cfg.clone();
    no_ipl.power_profiles[0].ipl_ro_guard = false;
    assert!(validate_digaos_config(&no_ipl).is_err());

    for (addr, len) in [
        (0xC000_100Cu32, 4u32),
        (0xC000_100Fu32, 1u32),
        (0xDC0F_0000u32, 4u32),
        (0xDC0F_0FFCu32, 4u32),
        (0x8000_0000u32, 4u32),
        (0x8003_FFFCu32, 4u32),
        (0x8400_0000u32, 4u32),
        (0x8403_FFFCu32, 4u32),
    ] {
        let mut bad_mmio = base_cfg.clone();
        bad_mmio.power_profiles[0]
            .allowed_mmio_writes
            .push(DigaOsMmioRangeSpec {
                start: addr,
                size: len,
                note: "e2e boundary probe".to_string(),
            });
        assert!(
            validate_digaos_config(&bad_mmio).is_err(),
            "validate_digaos_config must reject forbidden write at 0x{addr:08X}..+0x{len:X}"
        );
    }

    let mut bad_unverified = base_cfg.clone();
    if let Some(unverified) = bad_unverified
        .soc_targets
        .iter_mut()
        .find(|t| !t.mmio_verified)
    {
        unverified.is_primary_executable = true;
    }
    assert!(validate_digaos_config(&bad_unverified).is_err());
    Ok(())
}

#[test]
fn r3_replay_live_volatile_patch_log_le_words_and_reverse_rollback() -> Result<()> {
    let raw_log = read_evidence_log("com10_live_volatile_patch_20261124.log")?;
    let cleaned = strip_ndb_ansi_prompt(&raw_log);
    assert!(!cleaned.contains('\x1b'));

    let dd_lines: Vec<&str> = cleaned
        .lines()
        .filter(|l| l.starts_with("570DF348:"))
        .collect();
    assert_eq!(dd_lines.len(), 2, "Expected pre and post dd 570df348 lines");

    let pre_words = parse_ndb_dd_output(dd_lines[0])?;
    assert_eq!(
        pre_words.words,
        vec![0xF2CF_20F0, 0xF5E8_FEF8, 0x009A_8744, 0xFB00_9F08]
    );
    let pre_bytes = decode_ndb_words_le(&pre_words.words);
    assert_eq!(&pre_bytes[2..7], &[0xCF, 0xF2, 0xF8, 0xFE, 0xE8]);

    let post_words = parse_ndb_dd_output(dd_lines[1])?;
    assert_eq!(
        post_words.words,
        vec![0x0080_20F0, 0xF5CB_FCF0, 0x009A_8744, 0xFB00_9F08]
    );
    let post_bytes = decode_ndb_words_le(&post_words.words);
    assert_eq!(&post_bytes[2..7], &[0x80, 0x00, 0xF0, 0xFC, 0xCB]);

    let aiof_echo = cleaned
        .lines()
        .find(|l| l.contains("8800002C:"))
        .context("Missing AIOF_FS0 echo line in volatile patch log")?;
    let parsed_aiof_echo = parse_ndb_write_echo(aiof_echo)?;
    assert_eq!(parsed_aiof_echo.addr, AM34_AIOF_BASE + 0x2C);
    assert_eq!(parsed_aiof_echo.old_value, u32::from(AIOF_FS0_NON_STANDARD));
    assert_eq!(parsed_aiof_echo.new_value, u32::from(AIOF_FS0_STANDARD));

    let transport = MockNdbTransport::from_le_words(0x570D_F348, &pre_words.words);
    transport.seed_u16_le(AM34_AIOF_BASE + 0x2C, AIOF_FS0_NON_STANDARD);

    let mut session = NdbSession::new(transport);
    let ops = [
        VolatilePatchOp::write_bytes(0x570D_F34A, vec![0x80, 0x00, 0xF0, 0xFC, 0xCB]),
        VolatilePatchOp::write_u16(AM34_AIOF_BASE + 0x2C, AIOF_FS0_STANDARD),
    ];
    let diff = session.apply_patches_with_diff(&ops)?;
    assert!(!diff.changes.is_empty());

    let rollback_report = session.rollback()?;
    assert!(rollback_report.verified_clean);

    let cmds = session.into_transport().commands();
    let rollback_tail: Vec<&str> = cmds.iter().map(String::as_str).collect();
    let idx_aiof = rollback_tail
        .iter()
        .rposition(|c| *c == "ew 8800002c c0f0")
        .context("Expected ew 8800002c c0f0 rollback command")?;
    let idx_e8 = rollback_tail
        .iter()
        .rposition(|c| *c == "eb 570df34e e8")
        .context("Expected reverse-order eb 570df34e e8")?;
    let idx_cf = rollback_tail
        .iter()
        .rposition(|c| *c == "eb 570df34a cf")
        .context("Expected reverse-order eb 570df34a cf")?;
    assert!(idx_aiof < idx_e8 && idx_e8 < idx_cf);
    Ok(())
}

#[test]
fn r3_replay_live_video_osd_diag_log_bcu_abort_and_standby_gate_stop() -> Result<()> {
    let raw_log = read_evidence_log("com10_live_video_osd_diag_20261124.log")?;
    let cleaned = strip_ndb_ansi_prompt(&raw_log);

    let dd_main_line = cleaned
        .lines()
        .find(|l| l.starts_with("570DF348:"))
        .context("Missing 570DF348 dd line in video_osd_diag log")?;
    let parsed_main = parse_ndb_dd_output(dd_main_line)?;
    assert_eq!(
        &decode_ndb_words_le(&parsed_main.words)[2..7],
        &[0xCF, 0xF2, 0xF8, 0xFE, 0xE8]
    );

    let bcu_fault =
        detect_ndb_bcu_fault(&cleaned).context("Expected BCU BCBERR fault detection")?;
    assert_eq!(bcu_fault.bcberr_hex, "00002102");
    assert_eq!(bcu_fault.fault_address, 0xD400_4000);
    assert!(bcu_fault.task_dump_count >= 50);

    let run3_idx = raw_log
        .rfind("VDD: 0.04V")
        .context("Missing Run 3 STANDBY STATUS block in video_osd_diag log")?;
    let run3_status = parse_status_output_report(&raw_log[run3_idx..]);
    let gate_err = evaluate_live_rail_gate(&run3_status)
        .err()
        .context("Run 3 VDD=0.04V NSW5V=0 fault_latch=1 must trigger STANDBY_GATE_STOP")?;
    assert!(gate_err.to_string().contains("STANDBY_GATE_STOP"));
    Ok(())
}

#[test]
fn r3_silicon_explorer_blocks_lethal_mmio_bcu_fault_and_nor_writes() -> Result<()> {
    assert!(is_am34_lethal_mmio_address(AM34_LETHAL_MMIO_ADDR, 4));
    assert!(is_am34_lethal_mmio_address(0xDC0F_0000, 4));
    assert!(is_nor_write_blocked_for_flash(
        0x8000_0000,
        4,
        0x0080_0000,
        NorWritePolicy::NvramOnly
    ));

    let transport = MockNdbTransport::from_le_words(0x570D_F348, &[0xF2CF_20F0]);
    let mut session = NdbSession::new(transport);

    for (addr, kind) in [
        (0xC000_100Cu32, NdbAccessKind::Read),
        (0xC000_100Cu32, NdbAccessKind::Write),
        (0xDC0F_0000u32, NdbAccessKind::Read),
        (0xDC0F_0FFFu32, NdbAccessKind::Write),
        (0xD400_4000u32, NdbAccessKind::Read),
        (0x8000_0000u32, NdbAccessKind::Write),
        (0x807F_0000u32, NdbAccessKind::Write),
        (0x8400_0000u32, NdbAccessKind::Write),
        (0x8413_0000u32, NdbAccessKind::Write),
    ] {
        assert!(
            validate_ndb_access(addr, 4, kind).is_err(),
            "validate_ndb_access must reject {kind:?} at 0x{addr:08X}"
        );
        if kind == NdbAccessKind::Write {
            assert!(
                session
                    .apply_patches_with_diff(&[VolatilePatchOp::write_bytes(addr, vec![0x00])])
                    .is_err()
            );
        } else {
            assert!(session.read_words(addr, 4).is_err());
        }
    }

    assert!(
        session.into_transport().commands().is_empty(),
        "Fail-closed guards must transmit ZERO commands on blocked addresses"
    );
    Ok(())
}

#[test]
fn r3_main3060np_missing_western_osd_font_evidence_preserved() -> Result<()> {
    let boot_log = read_evidence_log("boottrace_3060np_s14_boot2_live_20261124.log")?;
    assert!(boot_log.contains("/HDDEX/UDA_M6930_H_JPN_V006"));
    assert!(boot_log.contains("0x90000001"));
    assert!(boot_log.contains("SYS_APP_VoiceGuide_CheckData() NG!"));

    let boot_class =
        parse_boot_class_line(&boot_log).context("Missing @P7501_BOOT_CLASS in 3060NP boot log")?;
    assert_eq!(boot_class.verdict, "CLEAN_MAIN_OK");
    assert_eq!(boot_class.build, "3060NP");

    assert_eq!(AVDD_NPALCTL0_PRESERVED_HI_WORD, 0x0005_0000);
    assert_eq!(decode_market_code_name(0x04), "PL");
    assert_eq!(decode_market_code_name(0x80), "EG");
    Ok(())
}
