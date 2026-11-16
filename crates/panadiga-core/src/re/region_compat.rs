//! Deterministic Region & Model Compatibility Patcher (`MAIN` `<deviceinfo>`, `DEV.BIN`, `.FRM` TOC).
//!
//! Targets three verified compatibility surfaces across Panasonic DIGA firmware trains:
//! 1. `MAIN` updater (`src/fmupre/pre_update_firm.c` / `FUN_5749859a`) 8-byte `model` + 1-byte `oem`
//!    guards in both `3410PQ` (`VA 0x57498738`) and `3060NP` (`VA 0x574A01AA`).
//! 2. `DEV.BIN` (`512`-byte `AA 55 FF FF` / `PANA_DVD_REC` container device descriptor).
//! 3. `PANA_DVD.FRM` (`48B` Feistel) & `PANA__SD.FRM` (`64B` plaintext) TOC model/family metadata.

use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::main_passthrough::{EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA};
use super::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use crate::constants::{
    PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256, PANA_FRM_TOC_ENTRY_COUNT,
    PANA_FRM_TOC_REGION_SIZE, PANA_SD_TOC_MAX_ENTRIES, PANA_SD_TOC_REGION_SIZE,
};
use crate::crypto::codec::{extract_c_ascii, write_padded_ascii, write_toc_entry_list_sum};
use crate::crypto::{
    DecryptedFrmTocEntry, TocLayout, decrypt_frm_toc_with_detected_key, feistel_decrypt_buffer,
    feistel_encrypt_buffer, parse_sd_frm_toc_with_layout,
};
use crate::nvram::calculate_hashes;

/// Exact unpacked byte size of `MAIN3060NP` (`PANA_DVD.FRM` slot 2 / `PANA__SD.FRM`).
pub const EXPECTED_MAIN_3060NP_SIZE: usize = 13_427_852;

/// `3410PQ` `src/fmupre/pre_update_firm.c` model/OEM comparison pair (`VA 0x57498738`, 21 B).
pub const OFF_FMU_DEVINFO_3410PQ: usize = 0x0049_8738;
/// `3060NP` `src/fmupre/pre_update_firm.c` model/OEM comparison pair (`VA 0x574A01AA`, 21 B).
pub const OFF_FMU_DEVINFO_3060NP: usize = 0x004A_01AA;

pub const ORIG_FMU_DEVINFO_3410PQ: [u8; 21] = [
    0xA0, 0x00, 0xC8, 0x05, 0xCC, 0xE2, 0x00, 0xFB, 0xAA, 0x90, 0x64, 0xF8, 0xBB, 0x2B, 0xF1, 0xAD,
    0xC8, 0x05, 0xCC, 0xD4, 0x00,
];
pub const PATCH_FMU_DEVINFO_3410PQ: [u8; 21] = [
    0xA0, 0x00, 0xCA, 0x05, 0xCC, 0xE2, 0x00, 0xFB, 0xAA, 0x90, 0x64, 0xF8, 0xBB, 0x2B, 0xF1, 0xAD,
    0xCA, 0x05, 0xCC, 0xD4, 0x00,
];

pub const ORIG_FMU_DEVINFO_3060NP: [u8; 21] = [
    0xA0, 0x00, 0xC8, 0x05, 0xCC, 0x0D, 0x01, 0xF8, 0xBA, 0x48, 0xFB, 0xAA, 0x90, 0x23, 0xF1, 0x96,
    0xC8, 0x05, 0xCC, 0xFF, 0x00,
];
pub const PATCH_FMU_DEVINFO_3060NP: [u8; 21] = [
    0xA0, 0x00, 0xCA, 0x05, 0xCC, 0x0D, 0x01, 0xF8, 0xBA, 0x48, 0xFB, 0xAA, 0x90, 0x23, 0xF1, 0x96,
    0xCA, 0x05, 0xCC, 0xFF, 0x00,
];

const SPEC_DEVINFO_3410PQ: PatchSpec<'static> = PatchSpec {
    name: "fmu_devinfo_3410pq",
    description: "3410PQ FUN_5749859a <deviceinfo> model + oem beq +5 -> bra +5",
    va: MAIN_BASE_VA + OFF_FMU_DEVINFO_3410PQ as u32,
    base_va: MAIN_BASE_VA,
    expect: &ORIG_FMU_DEVINFO_3410PQ,
    replace: &PATCH_FMU_DEVINFO_3410PQ,
    required_len: Some(EXPECTED_MAIN_3410PQ_SIZE),
    input_sha256: None,
};

const SPEC_DEVINFO_3060NP: PatchSpec<'static> = PatchSpec {
    name: "fmu_devinfo_3060np",
    description: "3060NP pre_update_firm <deviceinfo> model + oem beq +5 -> bra +5",
    va: MAIN_BASE_VA + OFF_FMU_DEVINFO_3060NP as u32,
    base_va: MAIN_BASE_VA,
    expect: &ORIG_FMU_DEVINFO_3060NP,
    replace: &PATCH_FMU_DEVINFO_3060NP,
    required_len: Some(EXPECTED_MAIN_3060NP_SIZE),
    input_sha256: None,
};

const DEV_BIN_SIZE: usize = 512;
const DEV_BIN_MAGIC: [u8; 4] = [0xAA, 0x55, 0xFF, 0xFF];
const DEV_BIN_SIG: &[u8; 12] = b"PANA_DVD_REC";

/// Patch mode for `MAIN` updater `<deviceinfo>` model/OEM guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionCompatMode {
    BypassModelAndOem,
    FactoryStrict,
}

impl FromStr for RegionCompatMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bypass" | "bypass_model_and_oem" | "unlock" => Ok(Self::BypassModelAndOem),
            "factory" | "factory_strict" | "restore" => Ok(Self::FactoryStrict),
            other => {
                bail!("Unsupported region-compat mode '{other}' (expected bypass|factory_strict)")
            }
        }
    }
}

/// Inspection/patch summary for unpacked `MAIN` (`3410PQ` / `3060NP`) `<deviceinfo>` checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainRegionCompatReport {
    pub build_profile: String,
    pub size_bytes: usize,
    pub va_hex: String,
    pub file_offset_hex: String,
    pub state: String,
    pub actual_hex: String,
    pub sha256: String,
}

/// Decoded/verified `DEV.BIN` (`512`-byte) device descriptor report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevBinReport {
    pub size_bytes: usize,
    pub model: String,
    pub oem: String,
    pub signature: String,
    pub md5: String,
    pub sha256: String,
}

/// Summary of retargeted `.FRM` TOC metadata slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrmRetargetReport {
    pub layout: String,
    pub total_bytes: usize,
    pub updated_entries: usize,
    pub entries: Vec<DecryptedFrmTocEntry>,
    pub sha256: String,
}

fn resolve_main_spec(len: usize) -> Result<(&'static str, &'static PatchSpec<'static>)> {
    match len {
        EXPECTED_MAIN_3410PQ_SIZE => Ok(("3410PQ", &SPEC_DEVINFO_3410PQ)),
        EXPECTED_MAIN_3060NP_SIZE => Ok(("3060NP", &SPEC_DEVINFO_3060NP)),
        other => bail!(
            "Unsupported MAIN size {other} B (expected 3410PQ={} B or 3060NP={} B)",
            EXPECTED_MAIN_3410PQ_SIZE,
            EXPECTED_MAIN_3060NP_SIZE
        ),
    }
}

/// Inspects the `MAIN` `<deviceinfo>` model/OEM comparison site deterministically.
pub fn inspect_main_region_compat(bin: &[u8]) -> Result<MainRegionCompatReport> {
    let (profile, spec) = resolve_main_spec(bin.len())?;
    let cls = classify_patch_site(bin, spec, "BYPASS_MODEL_AND_OEM")?;
    let (_, sha256) = calculate_hashes(bin);
    Ok(MainRegionCompatReport {
        build_profile: profile.to_string(),
        size_bytes: bin.len(),
        va_hex: format!("0x{:08X}", cls.va),
        file_offset_hex: format!("0x{:08X}", cls.file_offset),
        state: cls.state.to_string(),
        actual_hex: hex::encode(cls.actual_bytes),
        sha256,
    })
}

/// Applies or restores `RegionCompatMode` atomically on an unpacked `MAIN` image in memory.
pub fn patch_main_region_compat_in_place(
    bin: &mut [u8],
    mode: RegionCompatMode,
) -> Result<MainRegionCompatReport> {
    let (_, spec) = resolve_main_spec(bin.len())?;
    let action = match mode {
        RegionCompatMode::BypassModelAndOem => PatchApplyAction::ApplyReplace,
        RegionCompatMode::FactoryStrict => PatchApplyAction::RestoreExpect,
    };
    apply_patch_specs_in_place(bin, &[(spec, action)])?;
    inspect_main_region_compat(bin)
}

fn validate_ascii_field(label: &str, text: &str, max_len: usize) -> Result<()> {
    let t = text.trim();
    if t.is_empty() || t.len() > max_len || !t.bytes().all(|b| (0x20..0x7F).contains(&b)) {
        bail!("{label} must be 1..={max_len} printable ASCII characters, got '{text}'");
    }
    Ok(())
}

/// Parses and validates a 512-byte Panasonic `DEV.BIN` service container descriptor.
pub fn inspect_dev_bin_bytes(bytes: &[u8]) -> Result<DevBinReport> {
    if bytes.len() != DEV_BIN_SIZE {
        bail!(
            "DEV.BIN must be exactly {DEV_BIN_SIZE} bytes, got {}",
            bytes.len()
        );
    }
    if bytes[0..4] != DEV_BIN_MAGIC {
        bail!(
            "Invalid DEV.BIN magic (expected AA55FFFF, got {})",
            hex::encode(&bytes[0..4])
        );
    }
    if &bytes[0x20..0x2C] != DEV_BIN_SIG {
        bail!("Missing DEV.BIN 'PANA_DVD_REC' signature at offset 0x20");
    }
    let oem_byte = bytes[0x0C];
    let oem = if oem_byte.is_ascii_graphic() {
        (oem_byte as char).to_string()
    } else {
        format!("0x{oem_byte:02X}")
    };
    let (md5, sha256) = calculate_hashes(bytes);
    Ok(DevBinReport {
        size_bytes: bytes.len(),
        model: extract_c_ascii(&bytes[0x04..0x0C]),
        oem,
        signature: extract_c_ascii(&bytes[0x20..0x30]),
        md5,
        sha256,
    })
}

/// Edits `model` (bytes `0x04..0x0C`) and optional `oem` char (`0x0C`) in a validated `DEV.BIN`.
pub fn patch_dev_bin_bytes(
    bytes: &[u8],
    new_model: &str,
    new_oem: Option<char>,
) -> Result<(Vec<u8>, DevBinReport)> {
    let _ = inspect_dev_bin_bytes(bytes)?;
    let model = new_model.trim();
    validate_ascii_field("DEV.BIN model", model, 8)?;
    let mut out = bytes.to_vec();
    write_padded_ascii(&mut out[0x04..0x0C], model, 0);
    if let Some(ch) = new_oem {
        if !ch.is_ascii_graphic() {
            bail!("DEV.BIN oem must be a printable ASCII character");
        }
        out[0x0C] = ch as u8;
    }
    let rep = inspect_dev_bin_bytes(&out)?;
    Ok((out, rep))
}

/// Retargets `.FRM` TOC model/hardware fields cleanly and recalculates `list_sum_u16` (+ Feistel for DVD).
pub fn retarget_frm_toc_bytes(
    bytes: &[u8],
    hw_family: Option<&str>,
    model_group: Option<&str>,
    oem_char: Option<char>,
    target_flag: Option<u8>,
) -> Result<(Vec<u8>, FrmRetargetReport)> {
    if let Some(hw) = hw_family {
        validate_ascii_field("TOC hardware_family", hw, 8)?;
    }
    if let Some(oem) = oem_char
        && !oem.is_ascii_graphic()
    {
        bail!("TOC oem_char must be a printable ASCII character");
    }

    let (sd_layout, sd_entries) = parse_sd_frm_toc_with_layout(bytes);
    let (layout, active_indices, is_dvd, detected_key) = if let Some(l) = sd_layout {
        let active: Vec<usize> = sd_entries
            .iter()
            .filter(|e| e.list_sum_valid && !e.module_name.trim().is_empty() && e.module_size > 0)
            .map(|e| e.index)
            .collect();
        (l, active, false, PANA_FEISTEL_ROUND_KEY_32B)
    } else {
        let (_kname, rk, dvd_l, dvd_entries) = decrypt_frm_toc_with_detected_key(bytes);
        let active: Vec<usize> = dvd_entries
            .iter()
            .filter(|e| e.list_sum_valid && !e.module_name.trim().is_empty() && e.module_size > 0)
            .map(|e| e.index)
            .collect();
        if active.is_empty() {
            bail!(
                "Unrecognized .FRM container (neither valid plaintext SD TOC nor Feistel DVD TOC)"
            );
        }
        (dvd_l, active, true, rk)
    };

    let max_grp = if layout == TocLayout::Sd64B { 12 } else { 6 };
    if let Some(grp) = model_group {
        validate_ascii_field("TOC model_group", grp, max_grp)?;
    }

    let stride = layout.stride_bytes();
    let max_slots = if is_dvd {
        if layout == TocLayout::Dvd64B {
            PANA_FRM_TOC_REGION_SIZE / 64
        } else {
            PANA_FRM_TOC_ENTRY_COUNT
        }
    } else {
        PANA_SD_TOC_MAX_ENTRIES
    };
    let toc_span = if is_dvd {
        PANA_FRM_TOC_REGION_SIZE.min(bytes.len() & !7)
    } else {
        PANA_SD_TOC_REGION_SIZE.min(bytes.len())
    };
    if toc_span < max_slots.min(active_indices.iter().copied().max().unwrap_or(0) + 1) * stride {
        bail!(
            "Container too small ({} B) for {} TOC span",
            bytes.len(),
            layout.label()
        );
    }

    let mut out = bytes.to_vec();
    if is_dvd {
        feistel_decrypt_buffer(&mut out[..toc_span], &detected_key, &PANA_FEISTEL_SBOX_256);
    }

    for &idx in &active_indices {
        let entry = &mut out[idx * stride..(idx + 1) * stride];
        if let Some(hw) = hw_family {
            write_padded_ascii(&mut entry[0x10..0x18], hw.trim(), 0);
        }
        if layout == TocLayout::Sd64B {
            if let Some(flag) = target_flag {
                entry[0x18] = flag;
            }
            if let Some(grp) = model_group {
                write_padded_ascii(&mut entry[0x30..0x3C], grp.trim(), 0);
            }
        } else {
            if let Some(oem) = oem_char {
                entry[0x18] = oem as u8;
            }
            if let Some(flag) = target_flag {
                entry[0x19] = flag;
            }
            if let Some(grp) = model_group {
                write_padded_ascii(&mut entry[0x1A..0x20], grp.trim(), 0);
            }
        }
        write_toc_entry_list_sum(entry);
    }

    if is_dvd {
        feistel_encrypt_buffer(&mut out[..toc_span], &detected_key, &PANA_FEISTEL_SBOX_256);
    }

    let final_entries = if is_dvd {
        decrypt_frm_toc_with_detected_key(&out).3
    } else {
        parse_sd_frm_toc_with_layout(&out).1
    };
    let (_, sha256) = calculate_hashes(&out);
    Ok((
        out,
        FrmRetargetReport {
            layout: layout.label().to_string(),
            total_bytes: bytes.len(),
            updated_entries: active_indices.len(),
            entries: final_entries,
            sha256,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{FrmContainerMode, FrmPackModule, decrypt_and_parse_frm_toc, pack_frm};

    #[test]
    fn region_compat_main_bypass_and_restore_work_atomically() {
        let mut m3410 = vec![0u8; EXPECTED_MAIN_3410PQ_SIZE];
        m3410[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 21]
            .copy_from_slice(&ORIG_FMU_DEVINFO_3410PQ);
        assert_eq!(inspect_main_region_compat(&m3410).unwrap().state, "FACTORY");
        let patched =
            patch_main_region_compat_in_place(&mut m3410, RegionCompatMode::BypassModelAndOem)
                .unwrap();
        assert_eq!(patched.build_profile, "3410PQ");
        assert_eq!(patched.state, "BYPASS_MODEL_AND_OEM");
        let restored =
            patch_main_region_compat_in_place(&mut m3410, RegionCompatMode::FactoryStrict).unwrap();
        assert_eq!(restored.state, "FACTORY");

        let mut m3060 = vec![0u8; EXPECTED_MAIN_3060NP_SIZE];
        m3060[OFF_FMU_DEVINFO_3060NP..OFF_FMU_DEVINFO_3060NP + 21]
            .copy_from_slice(&ORIG_FMU_DEVINFO_3060NP);
        let p2 = patch_main_region_compat_in_place(&mut m3060, RegionCompatMode::BypassModelAndOem)
            .unwrap();
        assert_eq!(p2.build_profile, "3060NP");
        assert_eq!(p2.state, "BYPASS_MODEL_AND_OEM");
    }

    #[test]
    fn dev_bin_and_frm_toc_retarget_roundtrip_preserve_checksums() {
        let mut dev = vec![0xFFu8; 512];
        dev[0..4].copy_from_slice(&DEV_BIN_MAGIC);
        write_padded_ascii(&mut dev[0x04..0x0C], "EH58EE", 0);
        dev[0x0C] = b'M';
        dev[0x20..0x2C].copy_from_slice(DEV_BIN_SIG);
        let (edited, rep) = patch_dev_bin_bytes(&dev, "EH55EG", Some('E')).unwrap();
        assert_eq!(rep.model, "EH55EG");
        assert_eq!(rep.oem, "E");
        assert_eq!(edited.len(), 512);

        let mod0 = FrmPackModule::new("MAIN3410PQ", vec![0xAAu8; 64]);
        let (dvd_bytes, _) = pack_frm(&[mod0], FrmContainerMode::Dvd, None).unwrap();
        let (retargeted, frm_rep) = retarget_frm_toc_bytes(
            &dvd_bytes,
            Some("M6931"),
            Some("EG-500"),
            Some('E'),
            Some(1),
        )
        .unwrap();
        assert_eq!(frm_rep.updated_entries, 1);
        assert!(frm_rep.entries[0].list_sum_valid);
        assert_eq!(frm_rep.entries[0].hardware_family, "M6931");
        assert_eq!(frm_rep.entries[0].model_group, "EG-500");
        assert_eq!(frm_rep.entries[0].target_flag, 1);
        assert_eq!(
            decrypt_and_parse_frm_toc(&retargeted).len(),
            frm_rep.entries.len()
        );
    }
}
