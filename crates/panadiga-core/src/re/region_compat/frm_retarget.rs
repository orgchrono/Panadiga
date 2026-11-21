//! `.FRM` TOC metadata retargeting (`PANA_DVD.FRM` Feistel & `PANA__SD.FRM` plaintext).

use anyhow::{Result, bail};

use crate::constants::{
    PANA_FEISTEL_ROUND_KEY_32B, PANA_FEISTEL_SBOX_256, PANA_FRM_TOC_ENTRY_COUNT,
    PANA_FRM_TOC_REGION_SIZE, PANA_SD_TOC_MAX_ENTRIES, PANA_SD_TOC_REGION_SIZE,
};
use crate::crypto::codec::{write_padded_ascii, write_toc_entry_list_sum};
use crate::crypto::{
    TocLayout, decrypt_frm_toc_with_detected_key, feistel_decrypt_buffer, feistel_encrypt_buffer,
    parse_sd_frm_toc_with_layout,
};
use crate::nvram::calculate_hashes;

use super::types::{FrmRetargetReport, validate_ascii_field};

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
        match layout {
            TocLayout::Sd64B => {
                if let Some(flag) = target_flag {
                    entry[0x18] = flag;
                }
                if let Some(grp) = model_group {
                    write_padded_ascii(&mut entry[0x30..0x3C], grp.trim(), 0);
                }
            }
            TocLayout::Sd48B => {
                // `Loader ver0.61` `sd_find_firm_entry` matches `[0x18] == model_prefix` (e.g. `'M'`) and `[0x19] == target_flag`.
                if let Some(oem) = oem_char {
                    entry[0x18] = oem as u8;
                }
                if let Some(flag) = target_flag {
                    if flag.is_ascii_alphabetic() {
                        entry[0x18] = flag;
                    } else {
                        entry[0x19] = flag;
                    }
                }
                if let Some(grp) = model_group {
                    write_padded_ascii(&mut entry[0x1A..0x20], grp.trim(), 0);
                }
            }
            TocLayout::Dvd48B | TocLayout::Dvd64B => {
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
