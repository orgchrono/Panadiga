#![deny(unsafe_code)]

//! Deterministic multi-generation DIGA firmware fingerprinter & artifact harvester.

use super::{
    CipherFamily, ContainerFormat, HarvestedKeySchedule, HarvestedSbox, IngestFingerprintReport,
    extract_handshake_tokens, scan_256b_sboxes, scan_key_schedules,
};
use crate::constants::PANA_FEISTEL_ROUND_KEY_32B;
use crate::crypto::{
    ExtrheadCryptMode, PANA_DVD_KEY_CATALOG, TocLayout, decrypt_frm_toc_with_detected_key,
    derive_round_key_from_custom_key_8b, parse_sd_frm_toc_with_layout, unpack_extrhead,
};
use crate::ssot::catalog::{detect_diga_model_from_slice, is_valid_nor_flash_size};

/// Harvests S-boxes, key schedules, and handshake tokens from `data`.
pub fn harvest_crypto_artifacts(
    data: &[u8],
    base_off: usize,
    sboxes: &mut Vec<HarvestedSbox>,
    schedules: &mut Vec<HarvestedKeySchedule>,
    tokens: &mut Vec<String>,
) {
    scan_256b_sboxes(data, base_off, sboxes);
    scan_key_schedules(data, base_off, schedules);
    extract_handshake_tokens(data, tokens);
}

fn resolve_key_name(rk: &[u8; 32]) -> Option<String> {
    if *rk == PANA_FEISTEL_ROUND_KEY_32B {
        return Some("old_df72".to_string());
    }
    PANA_DVD_KEY_CATALOG
        .iter()
        .find(|&&(_, k8)| derive_round_key_from_custom_key_8b(&k8) == *rk)
        .map(|&(name, _)| name.to_string())
}

/// Deterministically identifies container format, cipher family, S-boxes, key schedules, and model tokens.
pub fn fingerprint_binary(bytes: &[u8], label: &str) -> IngestFingerprintReport {
    let (mut sboxes, mut key_schedules, mut handshake_tokens) =
        (Vec::new(), Vec::new(), Vec::new());
    harvest_crypto_artifacts(
        bytes,
        0,
        &mut sboxes,
        &mut key_schedules,
        &mut handshake_tokens,
    );

    let (sd_layout, sd_entries) = parse_sd_frm_toc_with_layout(bytes);
    let (dvd_key, _, dvd_layout, dvd_entries) = decrypt_frm_toc_with_detected_key(bytes);

    let (container, mut cipher_family, detected_key_label, double_encrypted_extrhead, toc_entries) =
        if let Some(layout) = sd_layout {
            let cf = match layout {
                TocLayout::Sd48B => ContainerFormat::Sd48B,
                TocLayout::Sd64B | TocLayout::Dvd64B | TocLayout::Dvd48B => ContainerFormat::Sd64B,
            };
            (cf, CipherFamily::Plaintext, None, false, sd_entries)
        } else if let Some(key_name) = dvd_key
            && !dvd_entries.is_empty()
        {
            let cf = if dvd_layout == TocLayout::Dvd64B {
                ContainerFormat::Dvd64B
            } else {
                ContainerFormat::Dvd48B
            };
            (
                cf,
                CipherFamily::MatsushitaFeistel64,
                Some(key_name.into()),
                false,
                dvd_entries,
            )
        } else if bytes.len() >= 0x40
            && let Ok(unpacked) = unpack_extrhead(bytes, ExtrheadCryptMode::Auto)
        {
            harvest_crypto_artifacts(
                &unpacked.decompressed,
                0x40,
                &mut sboxes,
                &mut key_schedules,
                &mut handshake_tokens,
            );
            let upper = label.to_ascii_uppercase();
            let cf = if upper.contains("DIGADIAG") || bytes.len() == 0x2_0000 {
                ContainerFormat::Digadiag
            } else {
                ContainerFormat::Extrhead
            };
            let (cipher, klabel, dbl) = if bytes.starts_with(b"EXTRHEAD") {
                (CipherFamily::Plaintext, None, false)
            } else if let Some((rk, dbl)) =
                crate::crypto::extrhead::detect_extrhead_key_and_double(bytes, None)
            {
                (
                    CipherFamily::MatsushitaFeistel64,
                    resolve_key_name(&rk),
                    dbl,
                )
            } else {
                (CipherFamily::MatsushitaFeistel64, None, false)
            };
            (cf, cipher, klabel, dbl, Vec::new())
        } else if bytes.len() == 0x4_0000 && label.to_ascii_uppercase().contains("DIGADIAG") {
            (
                ContainerFormat::Digadiag,
                CipherFamily::Plaintext,
                None,
                false,
                Vec::new(),
            )
        } else if bytes.starts_with(b"SH") && bytes.len() >= 512 {
            (
                ContainerFormat::MeihfsHdd,
                CipherFamily::Plaintext,
                None,
                false,
                Vec::new(),
            )
        } else if is_valid_nor_flash_size(bytes.len() as u64) {
            (
                ContainerFormat::RawNor,
                CipherFamily::Plaintext,
                None,
                false,
                Vec::new(),
            )
        } else {
            (
                ContainerFormat::Unknown,
                CipherFamily::Unknown,
                None,
                false,
                Vec::new(),
            )
        };

    for entry in toc_entries.iter().take(4) {
        let name = entry.module_name.trim().to_ascii_uppercase();
        let off = entry.module_offset as usize;
        let end = off
            .saturating_add(entry.module_size as usize)
            .min(bytes.len());
        if (name.starts_with("BOOT") || name.starts_with("PROG") || name.starts_with("MAIN"))
            && off < end
        {
            let slice = &bytes[off..end];
            if let Ok(unpacked) = unpack_extrhead(slice, ExtrheadCryptMode::Auto) {
                harvest_crypto_artifacts(
                    &unpacked.decompressed,
                    off,
                    &mut sboxes,
                    &mut key_schedules,
                    &mut handshake_tokens,
                );
            } else {
                harvest_crypto_artifacts(
                    slice,
                    off,
                    &mut sboxes,
                    &mut key_schedules,
                    &mut handshake_tokens,
                );
            }
        }
    }
    if matches!(
        cipher_family,
        CipherFamily::Plaintext | CipherFamily::Unknown
    ) && let Some(first) = key_schedules.first()
    {
        cipher_family = first.family;
    }

    IngestFingerprintReport {
        label: label.to_string(),
        byte_len: bytes.len(),
        container,
        cipher_family,
        detected_key_label,
        double_encrypted_extrhead,
        detected_model: detect_diga_model_from_slice(bytes),
        sboxes,
        key_schedules,
        handshake_tokens,
        toc_entry_count: toc_entries.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BLOWFISH_P_INIT_BE, DES_IP_HEAD, DES_SBOX1_UNPACKED};
    use super::*;
    use crate::constants::PANA_FEISTEL_SBOX_256;
    use crate::crypto::{
        ExtrheadCompressionAlgo, FrmContainerMode, FrmPackModule, build_digadiag_frm_with_algo,
        pack_frm_with_key,
    };

    #[test]
    fn test_fingerprint_sd48b_and_harvest_boot_prog_artifacts() {
        let mut prog = Vec::from(b"SDDIAG VER0.61 DMR-EH55 MN103S ".as_slice());
        prog.extend_from_slice(&PANA_FEISTEL_SBOX_256);
        prog.extend_from_slice(&BLOWFISH_P_INIT_BE);
        prog.extend_from_slice(&PANA_FEISTEL_ROUND_KEY_32B);
        let packed = pack_frm_with_key(
            &[FrmPackModule::new("PROG061", prog)],
            FrmContainerMode::Sd48,
            Some(0x1800),
            &PANA_FEISTEL_ROUND_KEY_32B,
        )
        .unwrap();
        let rep = fingerprint_binary(&packed.0, "PANA__SD.FRM");
        assert_eq!(rep.container, ContainerFormat::Sd48B);
        assert!(rep.sboxes.iter().any(|s| s.matches_known_matsushita));
        assert!(
            rep.key_schedules
                .iter()
                .any(|k| k.family == CipherFamily::Blowfish)
        );
        assert!(rep.handshake_tokens.iter().any(|t| t.contains("DMR-EH55")));
    }

    #[test]
    fn test_fingerprint_dvd64b_digadiag_des_and_hdd_nor() {
        let mut rk_38bb = PANA_FEISTEL_ROUND_KEY_32B;
        if let Some((_, k8)) = PANA_DVD_KEY_CATALOG
            .iter()
            .find(|&&(n, _)| n == "y2013_38bb")
        {
            rk_38bb = derive_round_key_from_custom_key_8b(k8);
        }
        let dvd = pack_frm_with_key(
            &[FrmPackModule::new("MAIN", vec![0xAA; 64])],
            FrmContainerMode::Dvd64B,
            Some(0x2000),
            &rk_38bb,
        )
        .unwrap();
        let rep_dvd = fingerprint_binary(&dvd.0, "PANA_DVD.FRM");
        assert_eq!(rep_dvd.container, ContainerFormat::Dvd64B);
        assert_eq!(rep_dvd.detected_key_label.as_deref(), Some("y2013_38bb"));

        let mut diag_body = vec![0u8; 256];
        diag_body[..16].copy_from_slice(&DES_SBOX1_UNPACKED);
        diag_body[32..40].copy_from_slice(&DES_IP_HEAD);
        let (diag, _) = build_digadiag_frm_with_algo(
            &diag_body,
            0x4400_0000,
            "PROG",
            "EH55",
            ExtrheadCompressionAlgo::Zlib,
            1,
            0x2_0000,
        )
        .unwrap();
        let rep_diag = fingerprint_binary(&diag, "DIGADIAG.FRM");
        assert_eq!(rep_diag.container, ContainerFormat::Digadiag);
        assert_eq!(rep_diag.cipher_family, CipherFamily::Des);

        let mut hdd = vec![0u8; 512];
        hdd[..2].copy_from_slice(b"SH");
        assert_eq!(
            fingerprint_binary(&hdd, "hdd.bin").container,
            ContainerFormat::MeihfsHdd
        );
        let nor = vec![0xFFu8; 8 * 1024 * 1024];
        assert_eq!(
            fingerprint_binary(&nor, "nor8m.bin").container,
            ContainerFormat::RawNor
        );
    }
}
