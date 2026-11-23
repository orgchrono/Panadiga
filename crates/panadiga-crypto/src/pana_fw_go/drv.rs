//! Optical-drive `DRV` module (`0x20`-byte `DriveHeader` + optional `EXTRHEAD` + tail `-48` cipher).

use anyhow::{Result, bail};

use super::super::codec::{extract_c_ascii, pad_to_block, write_padded_ascii};
use super::super::extrhead::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, pack_extrhead_with_options,
    unpack_extrhead_with_key,
};
use super::crypto_utils::{apply_edge_crypt_order, candidate_round_keys};
use super::types::{
    DRV_HEADER_SIZE, DRV_TAIL_CLEAR_BYTES, DriveHeaderInfo, MAIN_LIST_LEGACY_CRYPT_SPAN,
    UnpackedDrvModule,
};
use crate::constants::{PANA_EXTRHEAD_DEFAULT_ADLER_STEP, PANA_FEISTEL_ROUND_KEY_32B};

fn looks_like_drv_plain(decrypted: &[u8]) -> bool {
    if decrypted.starts_with(b"EXTRHEAD") {
        return true;
    }
    if decrypted.len() >= DRV_HEADER_SIZE + 8
        && &decrypted[DRV_HEADER_SIZE..DRV_HEADER_SIZE + 8] == b"EXTRHEAD"
    {
        return true;
    }
    decrypted.len() >= DRV_HEADER_SIZE
        && decrypted[..8]
            .iter()
            .all(|&b| b == 0 || (0x20..0x7F).contains(&b))
        && decrypted[8..24]
            .iter()
            .all(|&b| b == 0 || (0x20..0x7F).contains(&b))
        && decrypted[..4].iter().any(|&b| b.is_ascii_alphanumeric())
}

/// Unpacks an optical-drive `DRV` module protected by head+tail (`len - 48`) partial Feistel encryption.
pub fn extract_drv_module(raw: &[u8], round_key: Option<&[u8; 32]>) -> Result<UnpackedDrvModule> {
    let min_len = MAIN_LIST_LEGACY_CRYPT_SPAN + DRV_TAIL_CLEAR_BYTES;
    if raw.len() < min_len {
        bail!(
            "DRV module too short ({} B < minimum {min_len} B)",
            raw.len()
        );
    }
    let aligned_total = raw.len() & !7;
    let body_end = aligned_total - DRV_TAIL_CLEAR_BYTES;

    let mut decrypted = None;
    if looks_like_drv_plain(raw) {
        decrypted = Some(raw.to_vec());
    } else {
        for rk in candidate_round_keys(round_key) {
            let mut probe = raw.to_vec();
            apply_edge_crypt_order(
                &mut probe[..body_end],
                MAIN_LIST_LEGACY_CRYPT_SPAN,
                &rk,
                false,
            );
            if looks_like_drv_plain(&probe) {
                decrypted = Some(probe);
                break;
            }
        }
    }

    let plain = decrypted.unwrap_or_else(|| {
        let rk = round_key.copied().unwrap_or(PANA_FEISTEL_ROUND_KEY_32B);
        let mut fallback = raw.to_vec();
        apply_edge_crypt_order(
            &mut fallback[..body_end],
            MAIN_LIST_LEGACY_CRYPT_SPAN,
            &rk,
            false,
        );
        fallback
    });

    if plain.starts_with(b"EXTRHEAD") {
        let unpacked = unpack_extrhead_with_key(&plain, ExtrheadCryptMode::Plain, None)?;
        let payload = unpacked.decompressed.clone();
        return Ok(UnpackedDrvModule {
            header: None,
            extrhead: Some(unpacked),
            payload,
        });
    }

    let header = DriveHeaderInfo {
        manufacturer: extract_c_ascii(&plain[0..8]),
        model: extract_c_ascii(&plain[8..24]),
        version: extract_c_ascii(&plain[24..31]),
        unknown: plain[31],
    };
    let rest = &plain[DRV_HEADER_SIZE..];
    if rest.starts_with(b"EXTRHEAD") {
        let unpacked = unpack_extrhead_with_key(rest, ExtrheadCryptMode::Plain, None)?;
        let payload = unpacked.decompressed.clone();
        return Ok(UnpackedDrvModule {
            header: Some(header),
            extrhead: Some(unpacked),
            payload,
        });
    }

    Ok(UnpackedDrvModule {
        header: Some(header),
        extrhead: None,
        payload: rest.to_vec(),
    })
}

/// Packs an optical-drive `DRV` payload with optional 32B `DriveHeader`, optional `EXTRHEAD`, and edge Feistel.
pub fn pack_drv_module(
    payload: &[u8],
    drive_header: Option<&DriveHeaderInfo>,
    compress: Option<ExtrheadCompressionAlgo>,
    build_name: &str,
    round_key: &[u8; 32],
) -> Result<Vec<u8>> {
    let mut out =
        Vec::with_capacity(MAIN_LIST_LEGACY_CRYPT_SPAN + DRV_TAIL_CLEAR_BYTES + payload.len());
    if let Some(hdr) = drive_header {
        let mut hdr_buf = [0u8; DRV_HEADER_SIZE];
        write_padded_ascii(&mut hdr_buf[0..8], &hdr.manufacturer, 0);
        write_padded_ascii(&mut hdr_buf[8..24], &hdr.model, 0);
        write_padded_ascii(&mut hdr_buf[24..31], &hdr.version, 0);
        hdr_buf[31] = hdr.unknown;
        out.extend_from_slice(&hdr_buf);
    }

    if let Some(algo) = compress {
        let container = pack_extrhead_with_options(
            payload,
            "DRV ",
            0,
            build_name,
            ExtrheadCryptMode::Plain,
            algo,
            &PANA_FEISTEL_ROUND_KEY_32B,
            PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
            1,
        )?;
        out.extend_from_slice(&container);
    } else {
        out.extend_from_slice(payload);
    }

    let min_total = MAIN_LIST_LEGACY_CRYPT_SPAN + DRV_TAIL_CLEAR_BYTES;
    if out.len() < min_total {
        out.resize(min_total, 0);
    }
    pad_to_block(&mut out);

    let body_end = out.len() - DRV_TAIL_CLEAR_BYTES;
    apply_edge_crypt_order(
        &mut out[..body_end],
        MAIN_LIST_LEGACY_CRYPT_SPAN,
        round_key,
        true,
    );
    Ok(out)
}
