//! Thin-Host Structured `@P7501_*` Wire Protocol Parser (`@P7501_IDENT`, `@PROG`, `@EVT`, `@P7501_RES`).
//!
//! Decodes deterministic single-line key=value frames emitted by the Pico C11 Smart Probe
//! without regex dependencies or multi-line heuristics.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ssot::registry::{HardwareProfileRegistry, NorRegionKind, NorRegionPolicy};

/// Structured 4-stage target identification frame (`@P7501_IDENT ...`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicoIdentFrame {
    pub vdd_mv: u32,
    pub nsw5v: bool,
    pub elec: String,
    pub loader: String,
    pub jedec: u32,
    pub flash: String,
    pub ipl_crc32: u32,
    pub hw: String,
    pub tv: String,
    pub state: String,
    pub model: String,
    pub tier: String,
    pub caps: u32,
    pub ssot: String,
}

impl PicoIdentFrame {
    /// Derives the cached/GDB RSP NVRAM base hex string (`0x807F0000`, `0x803F0000`, `0x801F0000`, `0x80FE0000`)
    /// from the relational v2 `HardwareProfileRegistry`, returning `None` (fail-closed) when no writable EEMUL exists.
    #[must_use]
    pub fn nvram_base_hex(&self) -> Option<String> {
        let model_trimmed = self.model.trim();
        if !model_trimmed.is_empty()
            && let Ok(resolved) = HardwareProfileRegistry::builtin().resolve_model(model_trimmed)
        {
            if !resolved.flash_chip.as_ref().is_some_and(|c| c.writable) {
                return None;
            }
            return resolved
                .nor_layout
                .as_ref()
                .and_then(|l| l.find_region(NorRegionKind::Eemul))
                .filter(|r| r.policy == NorRegionPolicy::NvramSanitize)
                .map(|ee| {
                    format!(
                        "0x{:08X}",
                        crate::constants::AM34_FLASH_BASE + ee.start_offset
                    )
                });
        }

        let f = self.flash.to_ascii_uppercase();
        if f.contains("MX29LV320B") || f.contains("MR27V802") {
            None
        } else if f.contains("S29GL128N") || f.contains("16MB") {
            Some("0x80FE0000".to_string())
        } else if f.contains("S29JL064H") || f.contains("8MB") {
            Some(crate::constants::AM34_NVRAM_BASE_HEX.to_string())
        } else if f.contains("S29AL032D")
            || f.contains("S29JL032H")
            || f.contains("MX29LV320T")
            || f.contains("4MB")
        {
            Some("0x803F0000".to_string())
        } else if f.contains("S29AL016D") || f.contains("2MB") {
            Some("0x801F0000".to_string())
        } else {
            None
        }
    }
}

/// Structured atomic job progress event (`@PROG step=<cur>/<total> msg=<token>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicoProgressEvent {
    pub step: u32,
    pub total: u32,
    pub msg: String,
}

/// Structured asynchronous probe event (`@EVT kind=<KIND> ...`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicoEventFrame {
    pub kind: String,
    pub fields: BTreeMap<String, String>,
}

/// Structured command epilogue frame (`@P7501_RES cmd=<NAME> status=<OK|ERR> code=<0|E05|...> elapsed_us=<u64>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicoResultFrame {
    pub cmd: String,
    pub status: String,
    pub code: String,
    pub elapsed_us: u64,
    pub fields: BTreeMap<String, String>,
}

impl PicoResultFrame {
    /// Returns true when the command completed with `status=OK` and `code=0`.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.status.eq_ignore_ascii_case("OK")
            && (self.code == "0" || self.code.eq_ignore_ascii_case("OK"))
    }
}

/// Real-time on-chip AM34 boot classification frame (`@P7501_BOOT_CLASS ...`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PicoBootClassFrame {
    pub verdict: String,
    pub build: String,
    pub stage2: bool,
    pub avout_ok: bool,
    pub vmute0: bool,
    pub ntsc: bool,
    pub rsv_ok: bool,
    pub hdd_fail: u32,
    pub bgo_err: u32,
    pub fpc_err: u32,
    pub fpc_to: u32,
    pub iic_err: u32,
    pub odd_err: u32,
    pub op7f_cnt: u32,
    pub op7f: Vec<String>,
}

/// Real-time Timer-MCU T-BUS audit frame (`@P7501_TBUS_AUDIT ...`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PicoTbusAuditFrame {
    pub frames: u32,
    pub br_rejected: u32,
    pub nak7f: u32,
    pub ops: Vec<String>,
}

/// Generic SRAM-staged push envelope (`@P7501_FRAME seq=... ts_us=... type=... len=... crc32=... <payload>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PicoPushFrame {
    pub seq: u32,
    pub ts_us: u64,
    pub frame_type: String,
    pub len: u32,
    pub crc32: u32,
    pub payload: String,
    pub fields: BTreeMap<String, String>,
}

/// Generic thin KV telemetry envelope for `@P7501_HOTPLUG`, `@P7501_LIVE_MAP`, `@P7501_DIAG_DEEP`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PicoTaggedKvFrame {
    pub tag: String,
    pub fields: BTreeMap<String, String>,
}

/// Unified parsed wire frame from a `@P7501_*` / `@PROG` / `@EVT` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PicoWireMessage {
    Ident(PicoIdentFrame),
    Progress(PicoProgressEvent),
    Event(PicoEventFrame),
    BootClass(PicoBootClassFrame),
    TbusAudit(PicoTbusAuditFrame),
    PushFrame(PicoPushFrame),
    TaggedKv(PicoTaggedKvFrame),
    Result(PicoResultFrame),
}

/// Tokenizes `key=value` pairs supporting double-quoted values with spaces (`loader="Loader ver0.61"`).
#[must_use]
pub fn parse_kv_tokens(rest: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let bytes = rest.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }
        let key_start = i;
        while i < len && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len || bytes[i] != b'=' {
            while i < len && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            continue;
        }
        let key = rest[key_start..i].trim().to_string();
        i += 1; // skip '='
        let val = if i < len && bytes[i] == b'"' {
            i += 1;
            let val_start = i;
            while i < len && bytes[i] != b'"' {
                i += 1;
            }
            let s = rest[val_start..i].to_string();
            if i < len && bytes[i] == b'"' {
                i += 1;
            }
            s
        } else {
            let val_start = i;
            while i < len && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            rest[val_start..i].to_string()
        };
        if !key.is_empty() {
            map.insert(key, val);
        }
    }
    map
}

fn parse_u32_auto(raw: Option<&String>) -> u32 {
    let Some(s) = raw else {
        return 0;
    };
    let trimmed = s.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u32::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        trimmed.parse::<u32>().unwrap_or(0)
    }
}

/// Parses a `@P7501_IDENT ...` line (or finds the first `@P7501_IDENT` line inside a multi-line response).
#[must_use]
pub fn parse_ident_line(input: &str) -> Option<PicoIdentFrame> {
    let line = input
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("@P7501_IDENT "))?;
    let kv = parse_kv_tokens(&line["@P7501_IDENT ".len()..]);
    Some(PicoIdentFrame {
        vdd_mv: parse_u32_auto(kv.get("vdd_mv")),
        nsw5v: parse_u32_auto(kv.get("nsw5v")) != 0,
        elec: kv.get("elec").cloned().unwrap_or_default(),
        loader: kv.get("loader").cloned().unwrap_or_default(),
        jedec: parse_u32_auto(kv.get("jedec")),
        flash: kv.get("flash").cloned().unwrap_or_default(),
        ipl_crc32: parse_u32_auto(kv.get("ipl_crc32")),
        hw: kv.get("hw").cloned().unwrap_or_default(),
        tv: kv.get("tv").cloned().unwrap_or_default(),
        state: kv.get("state").cloned().unwrap_or_default(),
        model: kv
            .get("model")
            .cloned()
            .unwrap_or_else(|| "DMR-EH55".to_string()),
        tier: kv.get("tier").cloned().unwrap_or_default(),
        caps: parse_u32_auto(kv.get("caps")),
        ssot: kv.get("ssot").cloned().unwrap_or_default(),
    })
}

/// Parses a `@PROG step=<cur>/<total> msg=<token>` line.
#[must_use]
pub fn parse_prog_line(line: &str) -> Option<PicoProgressEvent> {
    let rest = line.trim().strip_prefix("@PROG ")?;
    let kv = parse_kv_tokens(rest);
    let step_raw = kv.get("step")?;
    let (cur_str, total_str) = step_raw.split_once('/')?;
    let step = cur_str.parse::<u32>().ok()?;
    let total = total_str.parse::<u32>().ok()?;
    Some(PicoProgressEvent {
        step,
        total,
        msg: kv.get("msg").cloned().unwrap_or_default(),
    })
}

/// Parses an `@EVT kind=<KIND> ...` line.
#[must_use]
pub fn parse_evt_line(line: &str) -> Option<PicoEventFrame> {
    let rest = line.trim().strip_prefix("@EVT ")?;
    let mut fields = parse_kv_tokens(rest);
    let kind = fields.remove("kind")?;
    Some(PicoEventFrame { kind, fields })
}

/// Parses a `@P7501_RES cmd=<NAME> status=<OK|ERR> code=<...> elapsed_us=<u64>` line.
#[must_use]
pub fn parse_res_line(input: &str) -> Option<PicoResultFrame> {
    let line = input
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with("@P7501_RES "))?;
    let mut fields = parse_kv_tokens(&line["@P7501_RES ".len()..]);
    let cmd = fields.remove("cmd")?;
    let status = fields.remove("status").unwrap_or_else(|| "OK".to_string());
    let code = fields.remove("code").unwrap_or_else(|| "0".to_string());
    let elapsed_us = fields
        .remove("elapsed_us")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    Some(PicoResultFrame {
        cmd,
        status,
        code,
        elapsed_us,
        fields,
    })
}

fn parse_csv_hex_ops(raw: Option<&String>) -> Vec<String> {
    let Some(s) = raw else {
        return Vec::new();
    };
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("NONE") {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(str::trim)
        .filter(|tok| !tok.is_empty() && !tok.eq_ignore_ascii_case("NONE"))
        .map(|tok| tok.to_ascii_uppercase())
        .collect()
}

/// Parses a `@P7501_BOOT_CLASS ...` line (selecting the final matching summary if multiple exist).
#[must_use]
pub fn parse_boot_class_line(input: &str) -> Option<PicoBootClassFrame> {
    let line = input
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with("@P7501_BOOT_CLASS "))?;
    let kv = parse_kv_tokens(&line["@P7501_BOOT_CLASS ".len()..]);
    let verdict = kv.get("verdict")?.clone();
    Some(PicoBootClassFrame {
        verdict,
        build: kv
            .get("build")
            .cloned()
            .unwrap_or_else(|| "UNKNOWN".to_string()),
        stage2: parse_u32_auto(kv.get("stage2")) != 0,
        avout_ok: parse_u32_auto(kv.get("avout_ok")) != 0,
        vmute0: parse_u32_auto(kv.get("vmute0")) != 0,
        ntsc: parse_u32_auto(kv.get("ntsc")) != 0,
        rsv_ok: parse_u32_auto(kv.get("rsv_ok")) != 0,
        hdd_fail: parse_u32_auto(kv.get("hdd_fail")),
        bgo_err: parse_u32_auto(kv.get("bgo_err")),
        fpc_err: parse_u32_auto(kv.get("fpc_err")),
        fpc_to: parse_u32_auto(kv.get("fpc_to")),
        iic_err: parse_u32_auto(kv.get("iic_err")),
        odd_err: parse_u32_auto(kv.get("odd_err")),
        op7f_cnt: parse_u32_auto(kv.get("op7f_cnt")),
        op7f: parse_csv_hex_ops(kv.get("op7f")),
    })
}

/// Parses a `@P7501_TBUS_AUDIT ...` line (selecting the final matching summary if multiple exist).
#[must_use]
pub fn parse_tbus_audit_line(input: &str) -> Option<PicoTbusAuditFrame> {
    let line = input
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with("@P7501_TBUS_AUDIT "))?;
    let kv = parse_kv_tokens(&line["@P7501_TBUS_AUDIT ".len()..]);
    if !kv.contains_key("frames") {
        return None;
    }
    Some(PicoTbusAuditFrame {
        frames: parse_u32_auto(kv.get("frames")),
        br_rejected: parse_u32_auto(kv.get("br_rejected")),
        nak7f: parse_u32_auto(kv.get("nak7f")),
        ops: parse_csv_hex_ops(kv.get("ops")),
    })
}

/// Parses a `@P7501_FRAME seq=<u32> ts_us=<u64> type=<TAG> len=<u16> crc32=<hex8> <payload>` envelope.
#[must_use]
pub fn parse_push_frame_line(line: &str) -> Option<PicoPushFrame> {
    let rest = line.trim().strip_prefix("@P7501_FRAME ")?;
    let mut kv = parse_kv_tokens(rest);
    let seq = kv.remove("seq").and_then(|s| s.parse::<u32>().ok())?;
    let ts_us = kv.remove("ts_us").and_then(|s| s.parse::<u64>().ok())?;
    let frame_type = kv.remove("type")?;
    let len = kv.remove("len").and_then(|s| s.parse::<u32>().ok())?;
    let crc_str = kv.remove("crc32")?;
    let crc32 = u32::from_str_radix(
        crc_str.trim_start_matches("0x").trim_start_matches("0X"),
        16,
    )
    .ok()?;
    let payload = rest
        .split("crc32=")
        .nth(1)
        .and_then(|after_crc| after_crc.split_once(char::is_whitespace))
        .map(|(_, body)| body.trim().to_string())
        .unwrap_or_default();
    Some(PicoPushFrame {
        seq,
        ts_us,
        frame_type,
        len,
        crc32,
        payload,
        fields: kv,
    })
}

/// Parses `@P7501_HOTPLUG`, `@P7501_LIVE_MAP`, or `@P7501_DIAG_DEEP` as pure thin KV map.
#[must_use]
pub fn parse_tagged_kv_line(line: &str) -> Option<PicoTaggedKvFrame> {
    let trimmed = line.trim();
    for (prefix, tag) in [
        ("@P7501_HOTPLUG ", "HOTPLUG"),
        ("@P7501_LIVE_MAP ", "LIVE_MAP"),
        ("@P7501_DIAG_DEEP ", "DIAG_DEEP"),
        ("@P7501_GPS_ENV ", "GPS_ENV"),
        ("@P7501_ROUTE ", "ROUTE"),
        ("@P7501_XRAY ", "XRAY"),
        ("@P7501_SURG ", "SURG"),
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(PicoTaggedKvFrame {
                tag: tag.to_string(),
                fields: parse_kv_tokens(rest),
            });
        }
    }
    None
}

/// Classifies any single structured `@P7501_*` / `@PROG` / `@EVT` wire line.
#[must_use]
pub fn parse_wire_line(line: &str) -> Option<PicoWireMessage> {
    let trimmed = line.trim();
    if trimmed.starts_with("@P7501_IDENT ") {
        parse_ident_line(trimmed).map(PicoWireMessage::Ident)
    } else if trimmed.starts_with("@PROG ") {
        parse_prog_line(trimmed).map(PicoWireMessage::Progress)
    } else if trimmed.starts_with("@EVT ") {
        parse_evt_line(trimmed).map(PicoWireMessage::Event)
    } else if trimmed.starts_with("@P7501_BOOT_CLASS ") {
        parse_boot_class_line(trimmed).map(PicoWireMessage::BootClass)
    } else if trimmed.starts_with("@P7501_TBUS_AUDIT ") {
        parse_tbus_audit_line(trimmed).map(PicoWireMessage::TbusAudit)
    } else if trimmed.starts_with("@P7501_FRAME ") {
        parse_push_frame_line(trimmed).map(PicoWireMessage::PushFrame)
    } else if trimmed.starts_with("@P7501_HOTPLUG ")
        || trimmed.starts_with("@P7501_LIVE_MAP ")
        || trimmed.starts_with("@P7501_DIAG_DEEP ")
        || trimmed.starts_with("@P7501_GPS_ENV ")
        || trimmed.starts_with("@P7501_ROUTE ")
        || trimmed.starts_with("@P7501_XRAY ")
        || trimmed.starts_with("@P7501_SURG ")
    {
        parse_tagged_kv_line(trimmed).map(PicoWireMessage::TaggedKv)
    } else if trimmed.starts_with("@P7501_RES ") {
        parse_res_line(trimmed).map(PicoWireMessage::Result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ident_frame_with_quoted_loader_and_hw() {
        let raw = "@P7501_IDENT vdd_mv=3295 nsw5v=1 elec=AWAKE_READY loader=\"Loader ver0.61\" jedec=0x0001227E flash=S29JL064H ipl_crc32=0xA5B6C7D8 hw=\"VEP79138\" tv=PAL/NTSC state=READY model=DMR-EH55 tier=VERIFIED_BANNER caps=0x07 ssot=1234ABCD\nIDENTIFY: OK";
        let ident = parse_ident_line(raw).expect("valid @P7501_IDENT");
        assert_eq!(ident.vdd_mv, 3295);
        assert!(ident.nsw5v);
        assert_eq!(ident.loader, "Loader ver0.61");
        assert_eq!(ident.jedec, 0x0001_227E);
        assert_eq!(ident.flash, "S29JL064H");
        assert_eq!(ident.ipl_crc32, 0xA5B6_C7D8);
        assert_eq!(ident.hw, "VEP79138");
        assert_eq!(ident.model, "DMR-EH55");
        assert_eq!(ident.caps, 0x07);
        assert_eq!(ident.nvram_base_hex().as_deref(), Some("0x807F0000"));
    }

    #[test]
    fn resolves_nvram_base_via_ssot_and_fails_closed_for_readonly_targets() {
        let es35v = parse_ident_line(
            "@P7501_IDENT vdd_mv=3295 nsw5v=1 loader=\"ver0.18\" flash=S29JL064H model=DMR-ES35V",
        )
        .expect("es35v ident");
        assert_eq!(es35v.nvram_base_hex().as_deref(), Some("0x807F0000"));

        let ez485v =
            parse_ident_line("@P7501_IDENT vdd_mv=3295 nsw5v=1 flash=MX29LV320B model=DMR-EZ485V")
                .expect("ez485v ident");
        assert_eq!(ez485v.nvram_base_hex(), None);

        let es10 =
            parse_ident_line("@P7501_IDENT vdd_mv=3295 nsw5v=1 flash=MR27V802 model=DMR-ES10")
                .expect("es10 ident");
        assert_eq!(es10.nvram_base_hex(), None);

        let custom_16m =
            parse_ident_line("@P7501_IDENT vdd_mv=3295 nsw5v=1 flash=S29GL128N model=UNKNOWN-16M")
                .expect("16m ident");
        assert_eq!(custom_16m.nvram_base_hex().as_deref(), Some("0x80FE0000"));
    }

    #[test]
    fn parses_prog_evt_and_res_frames() {
        let p = parse_prog_line("@PROG step=2/3 msg=WRITE_P2_VAULT_SLOT_1").expect("prog");
        assert_eq!(p.step, 2);
        assert_eq!(p.total, 3);
        assert_eq!(p.msg, "WRITE_P2_VAULT_SLOT_1");

        let e = parse_evt_line("@EVT kind=SDS_TRIG trig=91006 clk=12840").expect("evt");
        assert_eq!(e.kind, "SDS_TRIG");
        assert_eq!(e.fields.get("trig").map(String::as_str), Some("91006"));

        let r =
            parse_res_line("@P7501_RES cmd=JOB status=ERR code=E05 elapsed_us=412").expect("res");
        assert_eq!(r.cmd, "JOB");
        assert!(!r.is_ok());
        assert_eq!(r.code, "E05");
        assert_eq!(r.elapsed_us, 412);
    }

    #[test]
    fn parses_boot_class_and_tbus_audit_frames() {
        let bc_line = "@P7501_BOOT_CLASS verdict=FPC_IC7501_MISMATCH build=3410PQ stage2=1 avout_ok=0 vmute0=0 ntsc=1 rsv_ok=1 hdd_fail=3 bgo_err=0 fpc_err=29 fpc_to=9 iic_err=1 odd_err=0 op7f_cnt=2 op7f=A9,F9";
        let ta_line = "@P7501_TBUS_AUDIT frames=7 br_rejected=2 nak7f=1 ops=A9,F9";
        let bc = parse_boot_class_line(bc_line).expect("boot class");
        assert_eq!(bc.verdict, "FPC_IC7501_MISMATCH");
        assert_eq!(bc.build, "3410PQ");
        assert!(bc.stage2 && !bc.avout_ok && bc.ntsc);
        assert_eq!(bc.fpc_err, 29);
        assert_eq!(bc.op7f, vec!["A9", "F9"]);
        assert!(matches!(
            parse_wire_line(bc_line),
            Some(PicoWireMessage::BootClass(_))
        ));

        let ta = parse_tbus_audit_line(ta_line).expect("tbus audit");
        assert_eq!(ta.frames, 7);
        assert_eq!(ta.br_rejected, 2);
        assert_eq!(ta.nak7f, 1);
        assert_eq!(ta.ops, vec!["A9", "F9"]);
        assert!(matches!(
            parse_wire_line(ta_line),
            Some(PicoWireMessage::TbusAudit(_))
        ));
    }

    #[test]
    fn parses_push_frame_and_tagged_kv_envelopes() {
        let frame_line = "@P7501_FRAME seq=42 ts_us=1234567 type=MMIO len=36 crc32=A1B2C3D4 reg=AIOF_VMOD val=0x00008020";
        let frame = parse_push_frame_line(frame_line).expect("push frame");
        assert_eq!(frame.seq, 42);
        assert_eq!(frame.ts_us, 1_234_567);
        assert_eq!(frame.frame_type, "MMIO");
        assert_eq!(frame.len, 36);
        assert_eq!(frame.crc32, 0xA1B2_C3D4);
        assert_eq!(frame.payload, "reg=AIOF_VMOD val=0x00008020");
        assert!(matches!(
            parse_wire_line(frame_line),
            Some(PicoWireMessage::PushFrame(_))
        ));

        for line in [
            "@P7501_HOTPLUG event=ONLINE_AWAKE vdd_mv=3260 nsw5v=1 recoveries=2",
            "@P7501_LIVE_MAP mode=FULL bus=PANAMON1 frames=19 crc_err=0",
            "@P7501_DIAG_DEEP hw=M6930LB market=PL(0x04) cav=N/PAL avsel=8/1 gui=6 avin_iic=0:0x0@3756",
            "@P7501_GPS_ENV build=3060NP hw=M6930LB cal=0x03 affine=1 delta=0",
            "@P7501_ROUTE dom=VIDEO target=0x88000000 payload=video_mmio_dump guard=ALLOW next=PAYLOAD_RUN",
            "@P7501_XRAY max_stage=S10_VFD_VV fault_stage=NONE stages_mask=0x07FF events=12 errors=0",
            "@P7501_SURG op=LOG entries=2 cursor=2 undo=2 redo=0",
        ] {
            assert!(matches!(
                parse_wire_line(line),
                Some(PicoWireMessage::TaggedKv(_))
            ));
        }
    }
}
