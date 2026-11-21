#![deny(unsafe_code)]

//! Pure-Rust deterministic ISO 9660 Level 1 image synthesizer and structural verifier
//! (`2048 B` logical sectors, pure uppercase 8.3 root entries, 32 KiB DVD ECC alignment).

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::fat16_builder::encode_dos_8_3_name;
use crate::nvram::calculate_hashes;

pub const ISO9660_SECTOR_SIZE: usize = 2048;
pub const ISO9660_PVD_LBA: u32 = 16;
pub const ISO9660_TERMINATOR_LBA: u32 = 17;
pub const ISO9660_PATH_L_LBA: u32 = 18;
pub const ISO9660_PATH_M_LBA: u32 = 19;
pub const ISO9660_ROOT_DIR_LBA: u32 = 20;
pub const ISO9660_DATA_START_LBA: u32 = 21;
pub const ISO9660_ECC_ALIGN_SECTORS: u32 = 16;

#[derive(Debug, Clone)]
pub struct Iso9660FileInput {
    pub name_8_3: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Iso9660PlacedFileReport {
    pub name_8_3: String,
    pub extent_lba: u32,
    pub size_bytes: u32,
    pub sector_count: u32,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct SynthesizedIsoImage {
    pub volume_id: String,
    pub total_sectors: u32,
    pub image_bytes: Vec<u8>,
    pub image_sha256: String,
    pub files: Vec<Iso9660PlacedFileReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Iso9660InspectionReport {
    pub valid_pvd: bool,
    pub valid_terminator: bool,
    pub volume_id: String,
    pub logical_block_size: u16,
    pub volume_space_sectors: u32,
    pub files: Vec<Iso9660PlacedFileReport>,
    pub overall_status: String,
}

fn write_both_u16(buf: &mut [u8], val: u16) {
    buf[0..2].copy_from_slice(&val.to_le_bytes());
    buf[2..4].copy_from_slice(&val.to_be_bytes());
}

fn write_both_u32(buf: &mut [u8], val: u32) {
    buf[0..4].copy_from_slice(&val.to_le_bytes());
    buf[4..8].copy_from_slice(&val.to_be_bytes());
}

fn write_ascii_padded(dst: &mut [u8], text: &str) {
    dst.fill(b' ');
    let bytes = text.as_bytes();
    let n = bytes.len().min(dst.len());
    dst[..n].copy_from_slice(&bytes[..n]);
}

fn sanitize_volume_label(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .to_ascii_uppercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(32)
        .collect();
    if cleaned.is_empty() {
        "PANADIGA_REC".to_string()
    } else {
        cleaned
    }
}

fn build_dir_record(extent_lba: u32, data_len: u32, flags: u8, ident: &[u8]) -> Vec<u8> {
    let pad = if ident.len().is_multiple_of(2) { 1 } else { 0 };
    let rec_len = 33 + ident.len() + pad;
    let mut rec = vec![0u8; rec_len];
    rec[0] = rec_len as u8;
    write_both_u32(&mut rec[2..10], extent_lba);
    write_both_u32(&mut rec[10..18], data_len);
    rec[18..25].copy_from_slice(&[126, 11, 20, 12, 0, 0, 0]);
    rec[25] = flags;
    write_both_u16(&mut rec[28..32], 1);
    rec[32] = ident.len() as u8;
    rec[33..33 + ident.len()].copy_from_slice(ident);
    rec
}

/// Synthesizes a clean ECMA-119 / ISO 9660 Level 1 image aligned to 32 KiB (16-sector ECC) blocks.
pub fn synthesize_iso9660_level1_image(
    volume_id: &str,
    files: &[Iso9660FileInput],
) -> Result<SynthesizedIsoImage> {
    if files.is_empty() {
        bail!("Cannot synthesize empty ISO 9660 image without recovery payloads");
    }
    let clean_vol = sanitize_volume_label(volume_id);
    let mut placed = Vec::with_capacity(files.len());
    let mut next_lba = ISO9660_DATA_START_LBA;

    for f in files {
        let _ = encode_dos_8_3_name(&f.name_8_3)?;
        let size_u32 = u32::try_from(f.bytes.len())?;
        let sectors = size_u32.div_ceil(ISO9660_SECTOR_SIZE as u32).max(1);
        let (_, sha) = calculate_hashes(&f.bytes);
        placed.push((
            Iso9660PlacedFileReport {
                name_8_3: f.name_8_3.trim().to_ascii_uppercase(),
                extent_lba: next_lba,
                size_bytes: size_u32,
                sector_count: sectors,
                sha256: sha,
            },
            &f.bytes,
        ));
        next_lba = next_lba
            .checked_add(sectors)
            .ok_or_else(|| anyhow::anyhow!("ISO 9660 extent LBA overflow"))?;
    }

    let rem = next_lba % ISO9660_ECC_ALIGN_SECTORS;
    let total_sectors = if rem == 0 {
        next_lba
    } else {
        next_lba + (ISO9660_ECC_ALIGN_SECTORS - rem)
    };
    let mut img = vec![0u8; (total_sectors as usize) * ISO9660_SECTOR_SIZE];

    // LBA 16: Primary Volume Descriptor
    let pvd = &mut img[ISO9660_PVD_LBA as usize * ISO9660_SECTOR_SIZE
        ..(ISO9660_PVD_LBA as usize + 1) * ISO9660_SECTOR_SIZE];
    pvd[0] = 0x01;
    pvd[1..6].copy_from_slice(b"CD001");
    pvd[6] = 0x01;
    write_ascii_padded(&mut pvd[8..40], "PANASONIC DIGA RECOVERY");
    write_ascii_padded(&mut pvd[40..72], &clean_vol);
    write_both_u32(&mut pvd[80..88], total_sectors);
    write_both_u16(&mut pvd[120..124], 1);
    write_both_u16(&mut pvd[124..128], 1);
    write_both_u16(&mut pvd[128..132], ISO9660_SECTOR_SIZE as u16);
    write_both_u32(&mut pvd[132..140], 10);
    pvd[140..144].copy_from_slice(&ISO9660_PATH_L_LBA.to_le_bytes());
    pvd[148..152].copy_from_slice(&ISO9660_PATH_M_LBA.to_be_bytes());
    let root_pvd_rec = build_dir_record(
        ISO9660_ROOT_DIR_LBA,
        ISO9660_SECTOR_SIZE as u32,
        0x02,
        &[0x00],
    );
    pvd[156..156 + root_pvd_rec.len()].copy_from_slice(&root_pvd_rec);
    write_ascii_padded(&mut pvd[190..318], &clean_vol);
    write_ascii_padded(&mut pvd[318..446], "PANADIGA STUDIO");
    write_ascii_padded(&mut pvd[574..702], "PANADIGA ISO9660 SYNTH");
    pvd[881] = 0x01;

    // LBA 17: Volume Descriptor Set Terminator
    let term = &mut img[ISO9660_TERMINATOR_LBA as usize * ISO9660_SECTOR_SIZE..];
    term[0] = 0xFF;
    term[1..6].copy_from_slice(b"CD001");
    term[6] = 0x01;

    // LBA 18 & 19: Type L & Type M Path Tables
    let pt_l = &mut img[ISO9660_PATH_L_LBA as usize * ISO9660_SECTOR_SIZE..];
    pt_l[0] = 1;
    pt_l[2..6].copy_from_slice(&ISO9660_ROOT_DIR_LBA.to_le_bytes());
    pt_l[6..8].copy_from_slice(&1u16.to_le_bytes());

    let pt_m = &mut img[ISO9660_PATH_M_LBA as usize * ISO9660_SECTOR_SIZE..];
    pt_m[0] = 1;
    pt_m[2..6].copy_from_slice(&ISO9660_ROOT_DIR_LBA.to_be_bytes());
    pt_m[6..8].copy_from_slice(&1u16.to_be_bytes());

    // LBA 20: Root Directory Sector
    let mut root_bytes = Vec::with_capacity(ISO9660_SECTOR_SIZE);
    root_bytes.extend_from_slice(&build_dir_record(
        ISO9660_ROOT_DIR_LBA,
        ISO9660_SECTOR_SIZE as u32,
        0x02,
        &[0x00],
    ));
    root_bytes.extend_from_slice(&build_dir_record(
        ISO9660_ROOT_DIR_LBA,
        ISO9660_SECTOR_SIZE as u32,
        0x02,
        &[0x01],
    ));
    for (rep, _) in &placed {
        let iso_ident = format!("{};1", rep.name_8_3);
        let rec = build_dir_record(rep.extent_lba, rep.size_bytes, 0x00, iso_ident.as_bytes());
        if root_bytes.len() + rec.len() > ISO9660_SECTOR_SIZE {
            bail!("Too many root files for single-sector ISO 9660 Level 1 root directory");
        }
        root_bytes.extend_from_slice(&rec);
    }
    let root_off = ISO9660_ROOT_DIR_LBA as usize * ISO9660_SECTOR_SIZE;
    img[root_off..root_off + root_bytes.len()].copy_from_slice(&root_bytes);

    let mut reports = Vec::with_capacity(placed.len());
    for (rep, data) in placed {
        let start = rep.extent_lba as usize * ISO9660_SECTOR_SIZE;
        img[start..start + data.len()].copy_from_slice(data);
        reports.push(rep);
    }

    let (_, image_sha256) = calculate_hashes(&img);
    Ok(SynthesizedIsoImage {
        volume_id: clean_vol,
        total_sectors,
        image_bytes: img,
        image_sha256,
        files: reports,
    })
}

/// Parses and validates an ISO 9660 Level 1 image buffer (`CD001` PVD, terminator, root entries, extents).
pub fn inspect_iso9660_image(bytes: &[u8]) -> Result<Iso9660InspectionReport> {
    let min_len = (ISO9660_ROOT_DIR_LBA as usize + 1) * ISO9660_SECTOR_SIZE;
    if bytes.len() < min_len {
        bail!("ISO 9660 image too short ({} B)", bytes.len());
    }
    let pvd = &bytes[ISO9660_PVD_LBA as usize * ISO9660_SECTOR_SIZE
        ..(ISO9660_PVD_LBA as usize + 1) * ISO9660_SECTOR_SIZE];
    let term = &bytes[ISO9660_TERMINATOR_LBA as usize * ISO9660_SECTOR_SIZE
        ..(ISO9660_TERMINATOR_LBA as usize + 1) * ISO9660_SECTOR_SIZE];
    let valid_pvd = pvd[0] == 0x01 && &pvd[1..6] == b"CD001" && pvd[6] == 0x01;
    let valid_terminator = term[0] == 0xFF && &term[1..6] == b"CD001" && term[6] == 0x01;
    let volume_id = String::from_utf8_lossy(&pvd[40..72]).trim_end().to_string();
    let logical_block_size = u16::from_le_bytes([pvd[128], pvd[129]]);
    let volume_space_sectors = u32::from_le_bytes([pvd[80], pvd[81], pvd[82], pvd[83]]);
    let root_lba = u32::from_le_bytes([pvd[158], pvd[159], pvd[160], pvd[161]]);

    let mut files = Vec::new();
    let root_off = root_lba as usize * ISO9660_SECTOR_SIZE;
    if root_off + ISO9660_SECTOR_SIZE <= bytes.len() {
        let root_sec = &bytes[root_off..root_off + ISO9660_SECTOR_SIZE];
        let mut cur = 0usize;
        while cur < root_sec.len() {
            let rec_len = root_sec[cur] as usize;
            if rec_len == 0 || cur + rec_len > root_sec.len() || rec_len < 34 {
                break;
            }
            let flags = root_sec[cur + 25];
            let id_len = root_sec[cur + 32] as usize;
            if flags & 0x02 == 0 && 33 + id_len <= rec_len {
                let raw_name = String::from_utf8_lossy(&root_sec[cur + 33..cur + 33 + id_len]);
                let name_8_3 = raw_name.trim_end_matches(";1").to_string();
                let extent_lba = u32::from_le_bytes([
                    root_sec[cur + 2],
                    root_sec[cur + 3],
                    root_sec[cur + 4],
                    root_sec[cur + 5],
                ]);
                let size_bytes = u32::from_le_bytes([
                    root_sec[cur + 10],
                    root_sec[cur + 11],
                    root_sec[cur + 12],
                    root_sec[cur + 13],
                ]);
                let start = extent_lba as usize * ISO9660_SECTOR_SIZE;
                let end = start.saturating_add(size_bytes as usize);
                if end <= bytes.len() {
                    let (_, sha) = calculate_hashes(&bytes[start..end]);
                    files.push(Iso9660PlacedFileReport {
                        name_8_3,
                        extent_lba,
                        size_bytes,
                        sector_count: size_bytes.div_ceil(ISO9660_SECTOR_SIZE as u32),
                        sha256: sha,
                    });
                }
            }
            cur += rec_len;
        }
    }

    let ok = valid_pvd
        && valid_terminator
        && logical_block_size == ISO9660_SECTOR_SIZE as u16
        && usize::try_from(volume_space_sectors).unwrap_or(0) * ISO9660_SECTOR_SIZE <= bytes.len()
        && !files.is_empty();
    Ok(Iso9660InspectionReport {
        valid_pvd,
        valid_terminator,
        volume_id,
        logical_block_size,
        volume_space_sectors,
        files,
        overall_status: if ok { "PASS" } else { "FAIL" }.to_string(),
    })
}
