//! Pure in-memory codecs for `PROG061`, `MAIN`, `PANA_DVD.FRM`, and `PANA__SD.FRM`.

use anyhow::{Result, bail};

use crate::constants::{
    PANA_EXTRHEAD_DEFAULT_ADLER_STEP, PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256,
};
use crate::crypto::{
    DecryptedFrmTocEntry, ExtrheadCryptMode, TocLayout, decrypt_and_parse_frm_toc,
    feistel_decrypt_buffer, parse_sd_frm_toc_with_layout, unpack_extrhead,
};

use super::super::bgo_unbrick::{PROG061_EXPECTED_SIZE, inspect_prog061_bgo_sites};
use super::super::main_passthrough::inspect_main_firmware_identity;

pub(crate) fn load_prog061_from_bytes(raw: &[u8]) -> Result<Vec<u8>> {
    if raw.len() == PROG061_EXPECTED_SIZE && !raw.starts_with(b"EXTRHEAD") {
        if inspect_prog061_bgo_sites(raw).is_ok() {
            return Ok(raw.to_vec());
        }
        let mut dec = raw.to_vec();
        feistel_decrypt_buffer(
            &mut dec,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );
        if inspect_prog061_bgo_sites(&dec).is_ok() {
            return Ok(dec);
        }
        return Ok(raw.to_vec());
    }
    if raw.len() > 0x40 {
        let mode = if raw.starts_with(b"EXTRHEAD") {
            ExtrheadCryptMode::Plain
        } else {
            ExtrheadCryptMode::Auto
        };
        return Ok(unpack_extrhead(raw, mode)?.decompressed);
    }
    bail!("Unrecognized PROG061 binary ({} bytes)", raw.len());
}

pub(crate) fn load_main_binary_and_meta(raw: &[u8]) -> Result<(Vec<u8>, String, u16, u32)> {
    let crypt_modes: &[ExtrheadCryptMode] = if raw.starts_with(b"EXTRHEAD") {
        &[ExtrheadCryptMode::Plain]
    } else {
        &[ExtrheadCryptMode::Main, ExtrheadCryptMode::Auto]
    };

    for &mode in crypt_modes {
        if let Ok(u) = unpack_extrhead(raw, mode) {
            let identity =
                inspect_main_firmware_identity(&u.decompressed, Some(&u.header.build_name));
            let build = if identity.build_id == "UNKNOWN" {
                u.header.build_name
            } else {
                identity.build_id
            };
            return Ok((u.decompressed, build, u.header.flag, u.header.adler_step));
        }
    }

    if raw.len() >= 10_000_000 {
        let identity = inspect_main_firmware_identity(raw, None);
        return Ok((
            raw.to_vec(),
            identity.build_id,
            1,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        ));
    }

    bail!(
        "Input does not match raw MAIN firmware or EXTRHEAD container ({} B)",
        raw.len()
    );
}

pub(crate) fn detect_and_parse_frm_toc(
    raw: &[u8],
    source_label: &str,
) -> Result<(String, bool, Vec<DecryptedFrmTocEntry>)> {
    let enc_entries = decrypt_and_parse_frm_toc(raw);
    let enc_ok = !enc_entries.is_empty()
        && enc_entries.iter().all(|e| {
            e.list_sum_valid
                && (e.module_offset as usize)
                    .checked_add(e.module_size as usize)
                    .is_some_and(|end| end <= raw.len())
        });
    if enc_ok {
        return Ok(("PANA_DVD_48B_FEISTEL".to_string(), true, enc_entries));
    }

    let (layout_opt, sd_entries) = parse_sd_frm_toc_with_layout(raw);
    if sd_entries.is_empty() {
        bail!("No valid Feistel 48B or SD 64B TOC entries found in {source_label}");
    }
    let label = match layout_opt {
        Some(TocLayout::Sd48B) => "PANA_SD_48B_OBSOLETE",
        _ => "PANA_SD_64B_PLAIN",
    };
    Ok((label.to_string(), false, sd_entries))
}

pub(crate) fn slice_frm_slot(
    buf: &[u8],
    off: u32,
    sz: u32,
    encrypted_frm: bool,
) -> Result<Vec<u8>> {
    let start = off as usize;
    let end = start.saturating_add(sz as usize);
    if end > buf.len() || sz == 0 {
        bail!(
            "FRM module range 0x{start:X}..0x{end:X} exceeds container ({} B)",
            buf.len()
        );
    }
    let mut slice = buf[start..end].to_vec();
    if encrypted_frm {
        feistel_decrypt_buffer(
            &mut slice,
            &PANA_FEISTEL_ROUND_KEY_32B,
            &PANA_FEISTEL_SBOX_256,
        );
    }
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_frm_slot_rejects_empty_and_oob() {
        let buf = vec![0xAAu8; 64];
        assert!(slice_frm_slot(&buf, 0, 0, false).is_err());
        assert!(slice_frm_slot(&buf, 32, 64, false).is_err());
        assert_eq!(
            slice_frm_slot(&buf, 8, 16, false).unwrap(),
            vec![0xAAu8; 16]
        );
    }
}
