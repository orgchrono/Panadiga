//! Unlock `.FRM` / `DIGADIAG.FRM` packaging and verification helpers for SD and CD/DVD recovery.

use super::{FrmUnlockVerificationSummary, sha256_hex};
use crate::constants::{PANA_DIGADIAG_DEFAULT_LOAD_ADDR, PANA_DIGADIAG_FRM_SIZE};
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, pack_extrhead, pack_frm,
    parse_sd_frm_toc_with_layout, unpack_dvd_frm_to_modules, unpack_extrhead,
    verify_packed_frm_roundtrip,
};
use anyhow::{Result, bail};

/// Decodes whitespace/0x-separated hexadecimal bytes into a raw payload buffer.
pub fn parse_hex_payload(raw_hex: &str) -> Result<Vec<u8>> {
    let cleaned: String = raw_hex
        .split_whitespace()
        .map(|tok| {
            tok.strip_prefix("0x")
                .or_else(|| tok.strip_prefix("0X"))
                .unwrap_or(tok)
        })
        .collect();
    let cleaned = cleaned.replace([':', ',', '-'], "");
    if cleaned.is_empty() || !cleaned.len().is_multiple_of(2) {
        bail!("Invalid hex payload: expected non-empty even number of hex nibbles");
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| anyhow::anyhow!("Invalid hex byte '{}': {e}", &cleaned[i..i + 2]))
        })
        .collect()
}

/// Parses an unlock `.FRM` container kind label (`dvd`/`cd`, `sd`, `sd48`) into [`FrmContainerMode`].
pub fn parse_unlock_container_mode(raw: &str) -> Result<FrmContainerMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "dvd" | "cd" | "pana_dvd" => Ok(FrmContainerMode::Dvd),
        "sd" | "pana_sd" => Ok(FrmContainerMode::Sd),
        "sd48" | "pana_sd48" => Ok(FrmContainerMode::Sd48),
        other => bail!("Unsupported unlock FRM mode '{other}' (expected dvd/cd, sd, or sd48)"),
    }
}

/// Decodes and cryptographically verifies SD (`DIGADIAG.FRM`, `PANA__SD.FRM`) or CD (`PANA_DVD.FRM`) unlock media.
pub fn verify_unlock_frm_container(
    container_kind: &str,
    frm_bytes: &[u8],
) -> Result<FrmUnlockVerificationSummary> {
    let sha = sha256_hex(frm_bytes);
    let norm = container_kind.trim().to_ascii_lowercase();
    let jump_hex = format!("0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X}");

    match norm.as_str() {
        "dvd" | "cd" | "pana_dvd" => {
            let modules = unpack_dvd_frm_to_modules(frm_bytes)?;
            let mut notes = vec![format!(
                "Verified Feistel 48B TOC with {} active module(s)",
                modules.len()
            )];
            for m in &modules {
                notes.push(format!("Slot '{}' ({} B)", m.name.trim(), m.payload.len()));
            }
            Ok(FrmUnlockVerificationSummary {
                container_kind: "PANA_DVD.FRM".to_string(),
                container_bytes: frm_bytes.len(),
                sha256: sha,
                entry_count: modules.len(),
                bootloader_jump_va_hex: jump_hex,
                extrhead_valid: None,
                verified: true,
                notes,
            })
        }
        "sd" | "pana_sd" => {
            let (layout_opt, entries) = parse_sd_frm_toc_with_layout(frm_bytes);
            let layout = layout_opt.ok_or_else(|| {
                anyhow::anyhow!("Input does not contain a valid PANA__SD.FRM TOC")
            })?;
            let mode = match layout {
                crate::crypto::TocLayout::Sd64B => FrmContainerMode::Sd,
                crate::crypto::TocLayout::Sd48B => FrmContainerMode::Sd48,
                crate::crypto::TocLayout::Dvd48B | crate::crypto::TocLayout::Dvd64B => {
                    FrmContainerMode::Dvd
                }
            };
            let verified_entries = verify_packed_frm_roundtrip(frm_bytes, mode, entries.len())?;
            Ok(FrmUnlockVerificationSummary {
                container_kind: layout.title().to_string(),
                container_bytes: frm_bytes.len(),
                sha256: sha,
                entry_count: verified_entries.len(),
                bootloader_jump_va_hex: jump_hex,
                extrhead_valid: None,
                verified: true,
                notes: verified_entries
                    .iter()
                    .map(|e| format!("SD slot {} '{}' sum=OK", e.index, e.module_name.trim()))
                    .collect(),
            })
        }
        "digadiag" => {
            if frm_bytes.len() < PANA_DIGADIAG_FRM_SIZE {
                bail!(
                    "DIGADIAG.FRM must be >= {} bytes ({} B found; BOOT.BIN reads 0x20000 B)",
                    PANA_DIGADIAG_FRM_SIZE,
                    frm_bytes.len()
                );
            }
            let unpacked = unpack_extrhead(frm_bytes, ExtrheadCryptMode::Auto)?;
            let mut notes = vec![format!(
                "EXTRHEAD dst_addr=0x{:08X} decompressed={} B; BOOT.BIN FUN_44006dbc always jumps to 0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X}",
                unpacked.header.dst_addr,
                unpacked.decompressed.len()
            )];
            if unpacked.header.dst_addr != PANA_DIGADIAG_DEFAULT_LOAD_ADDR {
                notes.push(format!(
                    "WARNING: dst_addr 0x{:08X} != mandatory DIGADIAG jump 0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X}",
                    unpacked.header.dst_addr
                ));
            }
            Ok(FrmUnlockVerificationSummary {
                container_kind: "DIGADIAG.FRM".to_string(),
                container_bytes: frm_bytes.len(),
                sha256: sha,
                entry_count: 1,
                bootloader_jump_va_hex: jump_hex,
                extrhead_valid: Some(unpacked.checksum_valid),
                verified: unpacked.checksum_valid && unpacked.footer_present,
                notes,
            })
        }
        other => bail!("Unsupported unlock FRM container kind '{other}'"),
    }
}

/// Generates a valid 3-slot (`DRV`, `PROG061 `, `MAIN3410PQ`) unlock `.FRM` container (CD/DVD or SD).
pub fn build_sample_unlock_frm(
    mode: FrmContainerMode,
    prog_payload: &[u8],
) -> Result<(Vec<u8>, FrmUnlockVerificationSummary)> {
    let wrapped_prog = pack_extrhead(
        prog_payload,
        "FVU ",
        PANA_DIGADIAG_DEFAULT_LOAD_ADDR,
        "PROG061",
        ExtrheadCryptMode::Plain,
        16,
        1,
    )?;
    let mut m_drv = FrmPackModule::new("DRV S211", vec![0xCB; 64]);
    m_drv.version_u16 = 0x975E;
    let mut m_prog = FrmPackModule::new("PROG061 ", wrapped_prog);
    m_prog.version_u16 = 0x1202;
    m_prog.target_flag = 0; // Safeguard: never set AUTO_FORMAT_BGO/FORMAT_HDD here
    let mut m_main = FrmPackModule::new("MAIN3410PQ", vec![0xCB; 64]);
    m_main.version_u16 = 0x3410;
    let (bytes, _) = pack_frm(&[m_drv, m_prog, m_main], mode, None)?;
    let kind = if mode == FrmContainerMode::Dvd {
        "dvd"
    } else {
        "sd"
    };
    let summary = verify_unlock_frm_container(kind, &bytes)?;
    Ok((bytes, summary))
}
