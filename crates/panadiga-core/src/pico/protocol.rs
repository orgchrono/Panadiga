//! Thin-Host Structured `@P7501_*` Wire Protocol Parser (`@P7501_IDENT`, `@PROG`, `@EVT`, `@P7501_RES`).
//!
//! Decodes deterministic single-line key=value frames emitted by the Pico C11 Smart Probe
//! without regex dependencies or multi-line heuristics.

use std::collections::BTreeMap;

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

/// Unified parsed wire frame from a `@P7501_*` / `@PROG` / `@EVT` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PicoWireMessage {
    Ident(PicoIdentFrame),
    Progress(PicoProgressEvent),
    Event(PicoEventFrame),
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
}
