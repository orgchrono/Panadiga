#![deny(unsafe_code)]

//! Blu-ray Feistel 48-byte TOC sub-container serialization and Adler-32 checksum update.

use anyhow::Result;

use super::super::types::{ModuleEncryptionKind, UniversalFrmSubContainer};
use super::options::{FrmRepackOptions, align_up, bump_ascii_digits, rebuild_modified_module_slot};
use crate::codec::compute_fvu_byte_sum;
use crate::constants::{PANA_FEISTEL_SBOX_256, PANA_FRM_TOC_ENTRY_SIZE, PANA_FRM_TOC_REGION_SIZE};
use crate::feistel::{adler32_panasonic_strided, feistel_encrypt_buffer};

pub(super) fn pack_bluray_subcontainer(
    sub: &UniversalFrmSubContainer,
    opts: &FrmRepackOptions,
    modified_flags: &[bool],
) -> Result<Vec<u8>> {
    let toc_start = if sub.has_panasonic_header {
        48usize
    } else {
        0usize
    };
    let first_offset = sub
        .modules
        .iter()
        .map(|m| m.module_offset as usize)
        .min()
        .unwrap_or(toc_start + PANA_FRM_TOC_REGION_SIZE)
        .max(toc_start + sub.modules.len() * PANA_FRM_TOC_ENTRY_SIZE);
    let first_offset = align_up(first_offset, 0x200).max(if sub.has_panasonic_header {
        0x2000
    } else {
        PANA_FRM_TOC_REGION_SIZE
    });

    let mut image = vec![0u8; first_offset];
    if let Some(hdr48) = sub.raw_outer_48b {
        image[..48].copy_from_slice(&hdr48);
    }

    for (idx, mod_info) in sub.modules.iter().enumerate() {
        let up = mod_info.name.trim().to_ascii_uppercase();
        let is_main = up.starts_with("MAIN") || up.starts_with("SMAI");
        let is_pad_mod = up.starts_with("PROG") || up.starts_with("DIAG");

        let mut raw_data = rebuild_modified_module_slot(
            mod_info,
            modified_flags[idx],
            &sub.feistel_round_key,
            opts,
        )?;
        if is_pad_mod && !raw_data.len().is_multiple_of(16) {
            raw_data.resize(align_up(raw_data.len(), 16), 0);
        }

        let data_sum = if sub.uses_adler32_sums {
            adler32_panasonic_strided(&raw_data, 1)
        } else {
            compute_fvu_byte_sum(&raw_data)
        };
        let plain_size = raw_data.len();

        let skip_outer = is_main
            || up == "VGD"
            || mod_info.encryption_kind == ModuleEncryptionKind::BlurayDrvEdge;
        let mut disk_bytes = raw_data;
        if !skip_outer && sub.toc_encrypted {
            let aligned = align_up(disk_bytes.len(), 16);
            disk_bytes.resize(aligned, 0);
            feistel_encrypt_buffer(
                &mut disk_bytes,
                &sub.feistel_round_key,
                &PANA_FEISTEL_SBOX_256,
            );
        }

        let mod_off = if idx == 0 {
            first_offset
        } else {
            let cur = align_up(image.len(), 0x1000);
            image.resize(cur, 0);
            cur
        };
        image.extend_from_slice(&disk_bytes);

        let mut slot = mod_info.raw_toc_entry.clone();
        if slot.len() < PANA_FRM_TOC_ENTRY_SIZE {
            slot.resize(PANA_FRM_TOC_ENTRY_SIZE, 0);
        }
        slot[0x0C..0x10].copy_from_slice(&u32::try_from(mod_off)?.to_le_bytes());
        slot[0x19] = mod_info.target_flag;
        slot[0x20..0x24].copy_from_slice(&u32::try_from(plain_size)?.to_le_bytes());
        slot[0x24..0x28].copy_from_slice(&data_sum.to_le_bytes());
        if let Some(delta) = opts.patch_version_offset
            && is_main
        {
            if !bump_ascii_digits(&mut slot[0x04..0x08], delta) {
                let ver = mod_info.version_u16;
                slot[0x0A..0x0C].copy_from_slice(&ver.to_le_bytes());
            }
            let _ = bump_ascii_digits(&mut slot[0x1C..0x20], delta);
        }

        let hdr_sum = adler32_panasonic_strided(&slot[..0x2C], 1);
        slot[0x2C..0x30].copy_from_slice(&hdr_sum.to_le_bytes());

        let slot_pos = toc_start + mod_info.index * PANA_FRM_TOC_ENTRY_SIZE;
        image[slot_pos..slot_pos + PANA_FRM_TOC_ENTRY_SIZE].copy_from_slice(&slot[..48]);
    }

    if sub.has_panasonic_header {
        let content_sz = u32::try_from(image.len().saturating_sub(48))?;
        image[36..40].copy_from_slice(&content_sz.to_le_bytes());
        image[43] = u8::from(sub.toc_encrypted);
        let hsum = adler32_panasonic_strided(&image[..44], 1);
        image[44..48].copy_from_slice(&hsum.to_le_bytes());
    }

    if sub.toc_encrypted {
        let enc_len = (first_offset - toc_start) & !7;
        feistel_encrypt_buffer(
            &mut image[toc_start..toc_start + enc_len],
            &sub.feistel_round_key,
            &PANA_FEISTEL_SBOX_256,
        );
    }

    Ok(image)
}
