//! Panasonic `DMR-EH55` `MAIN` (`EH55_MAIN_3410PQ.bin`) AV TBC (`E-E`) Passthrough Patcher & RSP Planner
//!
//! Covers the three verified MN103S sites in `EH55_MAIN_3410PQ.bin` (`base VA 0x57000000`, `13,003,052` B)
//! plus `_dev_SERIAL` (`0x574F0775`) GDB RSP `$M` frame synthesis:
//!
//! 1. `0x5771ABFB` (`+0x0071ABFB`, 6 B): FPC status control -> `mov 1, d0; ret; nop; nop` (`80 01 f0 fc cb cb`).
//! 2. `0x574F7B8C` (`+0x004F7B8C`, 4 B): ODD state check -> `jmp16 +0x00D0 -> 0x574F7C5C; nop` (`cc d0 00 cb`).
//! 3. `0x574F7C5C` (`+0x004F7C5C`, 7 B): ODD init status -> `clr d0; nop x5` (`80 00 cb cb cb cb cb`).

use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::bgo_unbrick::ensure_parent_dir;
use crate::nvram::resolve_input_path;

/// Virtual base address where `EH55_MAIN_3410PQ.bin` executes in SDRAM.
pub const MAIN_BASE_VA: u32 = 0x5700_0000;
/// Exact unpacked byte size of `EH55_MAIN_3410PQ.bin`.
pub const EXPECTED_MAIN_3410PQ_SIZE: usize = 13_003_052;

/// File offset of front-panel status control site (`VA 0x5771ABFB`, 6 bytes).
pub const OFF_FPC_STATUS_CTRL: usize = 0x0071_ABFB;
/// File offset of ODD state branch site (`VA 0x574F7B8C`, 4 bytes).
pub const OFF_ODD_STATE_BRANCH: usize = 0x004F_7B8C;
/// File offset of ODD init status site (`VA 0x574F7C5C`, 7 bytes).
pub const OFF_ODD_INIT_STATUS: usize = 0x004F_7C5C;

pub const ORIG_FPC_STATUS_CTRL: [u8; 6] = [0xCF, 0xE0, 0xF8, 0xFE, 0xD0, 0x82];
pub const PATCH_FPC_STATUS_CTRL: [u8; 6] = [0x80, 0x01, 0xF0, 0xFC, 0xCB, 0xCB];

pub const ORIG_ODD_STATE_BRANCH: [u8; 4] = [0xBF, 0x03, 0xC1, 0x10];
pub const PATCH_ODD_STATE_BRANCH: [u8; 4] = [0xCC, 0xD0, 0x00, 0xCB];

pub const ORIG_ODD_INIT_STATUS: [u8; 7] = [0xDD, 0xCD, 0x3C, 0x00, 0x00, 0x00, 0x00];
pub const PATCH_ODD_INIT_STATUS: [u8; 7] = [0x80, 0x00, 0xCB, 0xCB, 0xCB, 0xCB, 0xCB];

/// `_dev_SERIAL` UART0 RSP protocol state table base VA (`0x515B2E60` holds state `0..4`).
pub const DEV_SERIAL_STATE_BASE_VA: u32 = 0x515B_2E00;
/// `_dev_SERIAL` registered serial callback pointer VA (`+0x40`).
pub const DEV_SERIAL_CALLBACK_VA: u32 = 0x515B_2E40;
/// Pointer to active ISR register save context frame.
pub const DEV_SERIAL_ISR_CTX_PTR_VA: u32 = 0x5316_AE8C;
/// Offset of saved `PC` within the ISR register save context frame.
pub const DEV_SERIAL_PC_OFFSET: u32 = 0x64;
/// Verified 4-byte UART state-machine entry sequence (`0 -> 1 -> 2 -> 3 -> 4`).
pub const DEV_SERIAL_HANDSHAKE_BYTES: [u8; 4] = [0x0E, 0x02, 0x02, 0x04];

#[derive(Debug, Clone, Copy)]
struct PassthroughSiteSpec {
    name: &'static str,
    file_offset: usize,
    factory: &'static [u8],
    patched: &'static [u8],
    description: &'static str,
}

const PASSTHROUGH_SITES: [PassthroughSiteSpec; 3] = [
    PassthroughSiteSpec {
        name: "fpc_status_ctrl",
        file_offset: OFF_FPC_STATUS_CTRL,
        factory: &ORIG_FPC_STATUS_CTRL,
        patched: &PATCH_FPC_STATUS_CTRL,
        description: "Front-panel status control -> mov 1, d0; ret; nop; nop",
    },
    PassthroughSiteSpec {
        name: "odd_state_branch",
        file_offset: OFF_ODD_STATE_BRANCH,
        factory: &ORIG_ODD_STATE_BRANCH,
        patched: &PATCH_ODD_STATE_BRANCH,
        description: "ODD state check branch -> jmp16 +0x00D0 (0x574F7C5C); nop",
    },
    PassthroughSiteSpec {
        name: "odd_init_status",
        file_offset: OFF_ODD_INIT_STATUS,
        factory: &ORIG_ODD_INIT_STATUS,
        patched: &PATCH_ODD_INIT_STATUS,
        description: "ODD init return status -> clr d0; nop x5",
    },
];

/// Supported configuration modes for `MAIN` AV TBC (`E-E`) passthrough sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainPassthroughMode {
    AvPassthrough,
    FactoryOriginal,
}

impl MainPassthroughMode {
    pub fn from_str_mode(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "av_passthrough" | "passthrough" | "ee" | "tbc" => Ok(Self::AvPassthrough),
            "factory_original" | "factory" | "restore" => Ok(Self::FactoryOriginal),
            other => bail!(
                "Unsupported MAIN passthrough mode '{other}': expected av_passthrough or factory_original"
            ),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AvPassthrough => "av_passthrough",
            Self::FactoryOriginal => "factory_original",
        }
    }
}

impl FromStr for MainPassthroughMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_str_mode(s)
    }
}

/// Inspection status and live RSP `$M` write frame for a single `MAIN` site.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainPassthroughSiteReport {
    pub name: String,
    pub description: String,
    pub va_hex: String,
    pub file_offset_hex: String,
    pub length: usize,
    pub state: String,
    pub current_bytes_hex: String,
    pub rsp_passthrough_frame: String,
    pub rsp_factory_frame: String,
}

/// Complete report of `MAIN` AV TBC passthrough sites (used for inspect & patch).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainPassthroughReport {
    pub file_size: usize,
    pub expected_size_match: bool,
    pub detected_mode: String,
    pub sites: Vec<MainPassthroughSiteReport>,
    pub output_path: Option<String>,
    pub applied_mode: Option<String>,
}

/// Structured live `_dev_SERIAL` RSP sequence returned by MCP `rsp_plan` mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainPassthroughRspPlan {
    pub handshake_hex: String,
    pub nak_byte_hex: String,
    pub serial_state_base_va: String,
    pub serial_callback_va: String,
    pub isr_ctx_ptr_va: String,
    pub isr_saved_pc_offset: String,
    pub passthrough_frames: Vec<String>,
    pub factory_restore_frames: Vec<String>,
    pub callback_write_frame: Option<String>,
    pub pico_cli_shortcuts: Vec<String>,
}

/// Compute 8-bit modulo-256 GDB RSP checksum over a payload string.
pub fn rsp_checksum(payload: &str) -> u8 {
    payload.bytes().fold(0u8, |acc, b| acc.wrapping_add(b))
}

/// Format a payload string as `$<payload>#<xx>` with lowercase hex checksum.
pub fn format_rsp_frame(payload: &str) -> String {
    format!("${payload}#{:02x}", rsp_checksum(payload))
}

/// Format an arbitrary memory write as `$M<ADDR>,<LEN>:<lowercase_hex>#<xx>`.
pub fn format_rsp_write_mem_frame(va: u32, data: &[u8]) -> String {
    let hex_bytes = hex::encode(data);
    let payload = format!("M{va:08X},{}:{hex_bytes}", data.len());
    format_rsp_frame(&payload)
}

/// Format a 32-bit little-endian word write as `$M<ADDR>,4:<le_hex>#<xx>`.
pub fn format_rsp_write_u32_frame(va: u32, value_le: u32) -> String {
    format_rsp_write_mem_frame(va, &value_le.to_le_bytes())
}

/// Build the complete `_dev_SERIAL` live RSP frame plan for Pico / UART injection.
pub fn build_passthrough_rsp_plan(callback_target_va: Option<u32>) -> MainPassthroughRspPlan {
    let passthrough_frames = PASSTHROUGH_SITES
        .iter()
        .map(|s| {
            format_rsp_write_mem_frame(MAIN_BASE_VA.wrapping_add(s.file_offset as u32), s.patched)
        })
        .collect();
    let factory_restore_frames = PASSTHROUGH_SITES
        .iter()
        .map(|s| {
            format_rsp_write_mem_frame(MAIN_BASE_VA.wrapping_add(s.file_offset as u32), s.factory)
        })
        .collect();
    let callback_write_frame =
        callback_target_va.map(|va| format_rsp_write_u32_frame(DEV_SERIAL_CALLBACK_VA, va));
    let mut pico_cli_shortcuts = vec![
        "PANARSP PASSTHROUGH".to_string(),
        "PANARSP NORMAL MAGIC PASSTHROUGH".to_string(),
    ];
    if let Some(va) = callback_target_va {
        pico_cli_shortcuts.push(format!("PANARSP CALL:{va:08X}"));
    }

    MainPassthroughRspPlan {
        handshake_hex: hex::encode(DEV_SERIAL_HANDSHAKE_BYTES),
        nak_byte_hex: "15".to_string(),
        serial_state_base_va: format!("0x{DEV_SERIAL_STATE_BASE_VA:08X}"),
        serial_callback_va: format!("0x{DEV_SERIAL_CALLBACK_VA:08X}"),
        isr_ctx_ptr_va: format!("0x{DEV_SERIAL_ISR_CTX_PTR_VA:08X}"),
        isr_saved_pc_offset: format!("0x{DEV_SERIAL_PC_OFFSET:02X}"),
        passthrough_frames,
        factory_restore_frames,
        callback_write_frame,
        pico_cli_shortcuts,
    }
}

fn site_slice<'a>(buf: &'a [u8], spec: &PassthroughSiteSpec) -> Result<&'a [u8]> {
    let end = spec.file_offset + spec.factory.len();
    if end > buf.len() {
        bail!(
            "MAIN binary too small ({} bytes) for site '{}' (needs end 0x{:06X})",
            buf.len(),
            spec.name,
            end
        );
    }
    Ok(&buf[spec.file_offset..end])
}

/// Inspect the 3 AV TBC passthrough sites inside an unpacked `MAIN` binary buffer.
pub fn inspect_main_passthrough_sites(buf: &[u8]) -> Result<MainPassthroughReport> {
    let mut sites = Vec::with_capacity(PASSTHROUGH_SITES.len());
    let mut all_factory = true;
    let mut all_patched = true;

    for spec in &PASSTHROUGH_SITES {
        let cur = site_slice(buf, spec)?;
        let va = MAIN_BASE_VA.wrapping_add(spec.file_offset as u32);
        let state = if cur == spec.factory {
            all_patched = false;
            "FACTORY_ORIGINAL"
        } else if cur == spec.patched {
            all_factory = false;
            "AV_PASSTHROUGH"
        } else {
            all_factory = false;
            all_patched = false;
            "UNKNOWN_OR_MODIFIED"
        };

        sites.push(MainPassthroughSiteReport {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            va_hex: format!("0x{va:08X}"),
            file_offset_hex: format!("0x{:06X}", spec.file_offset),
            length: cur.len(),
            state: state.to_string(),
            current_bytes_hex: hex::encode(cur),
            rsp_passthrough_frame: format_rsp_write_mem_frame(va, spec.patched),
            rsp_factory_frame: format_rsp_write_mem_frame(va, spec.factory),
        });
    }

    let detected_mode = if all_factory {
        "FACTORY_ORIGINAL"
    } else if all_patched {
        "AV_PASSTHROUGH"
    } else {
        "MIXED_OR_UNKNOWN"
    };

    Ok(MainPassthroughReport {
        file_size: buf.len(),
        expected_size_match: buf.len() == EXPECTED_MAIN_3410PQ_SIZE,
        detected_mode: detected_mode.to_string(),
        sites,
        output_path: None,
        applied_mode: None,
    })
}

/// Apply the requested `MainPassthroughMode` across all 3 verified sites in `buf`.
pub fn apply_main_passthrough_in_place(
    buf: &mut [u8],
    mode: MainPassthroughMode,
) -> Result<MainPassthroughReport> {
    for spec in &PASSTHROUGH_SITES {
        let slice = site_slice(buf, spec)?;
        if slice != spec.factory && slice != spec.patched {
            bail!(
                "Unexpected bytes at {} (0x{:06X}): got {} (expected factory {} or passthrough {})",
                spec.name,
                spec.file_offset,
                hex::encode(slice),
                hex::encode(spec.factory),
                hex::encode(spec.patched),
            );
        }
    }

    for spec in &PASSTHROUGH_SITES {
        let end = spec.file_offset + spec.factory.len();
        let target = match mode {
            MainPassthroughMode::AvPassthrough => spec.patched,
            MainPassthroughMode::FactoryOriginal => spec.factory,
        };
        buf[spec.file_offset..end].copy_from_slice(target);
    }

    let mut report = inspect_main_passthrough_sites(buf)?;
    report.applied_mode = Some(mode.as_str().to_string());
    Ok(report)
}

/// Read an unpacked `MAIN` binary, apply `mode` non-destructively, and write `output_path`.
pub fn patch_main_passthrough_file(
    input_path: &Path,
    output_path: &Path,
    mode: MainPassthroughMode,
) -> Result<MainPassthroughReport> {
    let resolved = resolve_input_path(input_path)?;
    let mut buf =
        fs::read(&resolved).with_context(|| format!("Failed to read {}", resolved.display()))?;
    let mut report = apply_main_passthrough_in_place(&mut buf, mode)?;
    ensure_parent_dir(output_path)?;
    fs::write(output_path, &buf)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;
    report.output_path = Some(output_path.to_string_lossy().into_owned());
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_synthetic_main_image() -> Vec<u8> {
        let mut buf = vec![0xFFu8; EXPECTED_MAIN_3410PQ_SIZE];
        for spec in &PASSTHROUGH_SITES {
            buf[spec.file_offset..spec.file_offset + spec.factory.len()]
                .copy_from_slice(spec.factory);
        }
        buf
    }

    #[test]
    fn test_rsp_frame_formatting_and_checksums() {
        assert_eq!(format_rsp_frame("g"), "$g#67");
        let wr_cb = format_rsp_write_u32_frame(DEV_SERIAL_CALLBACK_VA, 0x5040_1F3C);
        assert_eq!(wr_cb, "$M515B2E40,4:3c1f4050#95");
        let wr_branch = format_rsp_write_mem_frame(0x574F_7B8C, &PATCH_ODD_STATE_BRANCH);
        assert!(wr_branch.starts_with("$M574F7B8C,4:ccd000cb#"));

        let plan = build_passthrough_rsp_plan(Some(0x5040_1F3C));
        assert_eq!(plan.handshake_hex, "0e020204");
        assert_eq!(plan.passthrough_frames.len(), 3);
        assert_eq!(plan.factory_restore_frames.len(), 3);
        assert_eq!(
            plan.callback_write_frame.as_deref(),
            Some("$M515B2E40,4:3c1f4050#95")
        );
    }

    #[test]
    fn test_synthetic_inspect_patch_and_restore_round_trip() {
        let tmp = std::env::temp_dir().join(format!("panadiga_main_pass_{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let in_path = tmp.join("EH55_MAIN_3410PQ_synth.bin");
        let out_path = tmp.join("EH55_MAIN_PASSTHROUGH.bin");

        let synth = build_synthetic_main_image();
        fs::write(&in_path, &synth).unwrap();

        let r_orig = inspect_main_passthrough_sites(&synth).unwrap();
        assert_eq!(r_orig.detected_mode, "FACTORY_ORIGINAL");
        assert!(r_orig.expected_size_match);

        let r_patch =
            patch_main_passthrough_file(&in_path, &out_path, MainPassthroughMode::AvPassthrough)
                .unwrap();
        assert_eq!(r_patch.detected_mode, "AV_PASSTHROUGH");
        assert_eq!(r_patch.applied_mode.as_deref(), Some("av_passthrough"));

        let mut patched = fs::read(&out_path).unwrap();
        let r_restored =
            apply_main_passthrough_in_place(&mut patched, MainPassthroughMode::FactoryOriginal)
                .unwrap();
        assert_eq!(r_restored.detected_mode, "FACTORY_ORIGINAL");
        assert_eq!(patched, synth);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_optional_real_eh55_main_3410pq_inspection() {
        let real = Path::new(r"C:\ghidra\binaries\unpacked\EH55_MAIN_3410PQ.bin");
        if !real.is_file() {
            return;
        }
        let data = fs::read(real).unwrap();
        let report = inspect_main_passthrough_sites(&data).unwrap();
        assert!(report.expected_size_match);
        assert_eq!(report.detected_mode, "FACTORY_ORIGINAL");
    }
}
