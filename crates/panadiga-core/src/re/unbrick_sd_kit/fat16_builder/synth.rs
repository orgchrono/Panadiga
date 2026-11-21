#![deny(unsafe_code)]

//! Pure in-memory synthesizer for deterministic, self-validated FAT16B (`0x06`) SD card images.

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::Fat16PlacedFileReport;
use super::layout::{
    FAT16_DOS_DATE_2006_03_06, FAT16_DOS_TIME_MIDNIGHT, FAT16_NUM_FATS, FAT16_PARTITION_TYPE_LBA,
    FAT16_ROOT_ENTRY_COUNT, FAT16_SECTOR_BYTES, FAT16_VOL_ID_DETERMINISTIC, Fat16SdGeometry,
    compute_fat16_sd_geometry, encode_dos_8_3_name, lba_to_mbr_chs,
};
use crate::nvram::calculate_hashes;
use crate::re::unbrick_sd_kit::inspect_sd::{SdDiskInspectionReport, inspect_sd_disk_bytes};

/// Single root-directory file to write contiguously into the synthesized FAT16B image.
#[derive(Debug, Clone)]
pub struct Fat16SdFileInput {
    pub name_8_3: String,
    pub bytes: Vec<u8>,
}

/// Complete synthesized SD card disk image with deterministic SHA-256 and self-audit report.
#[derive(Debug, Clone)]
pub struct SynthesizedSdImage {
    pub geometry: Fat16SdGeometry,
    pub image_bytes: Vec<u8>,
    pub image_sha256: String,
    pub files: Vec<Fat16PlacedFileReport>,
    pub inspection: SdDiskInspectionReport,
}

fn write_mbr(mbr: &mut [u8], geom: Fat16SdGeometry) {
    mbr.fill(0);
    let slot0 = &mut mbr[0x1BE..0x1CE];
    slot0[0] = 0x00;
    slot0[1..4].copy_from_slice(&lba_to_mbr_chs(geom.lba_start));
    slot0[4] = FAT16_PARTITION_TYPE_LBA;
    slot0[5..8].copy_from_slice(&lba_to_mbr_chs(geom.lba_start + geom.volume_sectors - 1));
    slot0[8..12].copy_from_slice(&geom.lba_start.to_le_bytes());
    slot0[12..16].copy_from_slice(&geom.volume_sectors.to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xAA;
}

fn write_vbr(vbr: &mut [u8], geom: Fat16SdGeometry) {
    vbr.fill(0);
    vbr[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
    vbr[3..11].copy_from_slice(b"MSDOS5.0");
    vbr[11..13].copy_from_slice(&(FAT16_SECTOR_BYTES as u16).to_le_bytes());
    vbr[13] = geom.sectors_per_cluster;
    vbr[14..16].copy_from_slice(&geom.reserved_sectors.to_le_bytes());
    vbr[16] = FAT16_NUM_FATS;
    vbr[17..19].copy_from_slice(&FAT16_ROOT_ENTRY_COUNT.to_le_bytes());
    vbr[19..21].copy_from_slice(&0u16.to_le_bytes());
    vbr[21] = 0xF8;
    vbr[22..24].copy_from_slice(&geom.sectors_per_fat.to_le_bytes());
    vbr[24..26].copy_from_slice(&63u16.to_le_bytes());
    vbr[26..28].copy_from_slice(&255u16.to_le_bytes());
    vbr[28..32].copy_from_slice(&geom.lba_start.to_le_bytes());
    vbr[32..36].copy_from_slice(&geom.volume_sectors.to_le_bytes());
    vbr[36] = 0x80;
    vbr[37] = 0x00;
    vbr[38] = 0x29;
    vbr[39..43].copy_from_slice(&FAT16_VOL_ID_DETERMINISTIC.to_le_bytes());
    vbr[43..54].copy_from_slice(b"SDCARD     ");
    vbr[54..62].copy_from_slice(b"FAT16   ");
    vbr[510] = 0x55;
    vbr[511] = 0xAA;
}

fn write_root_entry(entry: &mut [u8], raw_8_3: &[u8; 11], first_cluster: u16, file_size: u32) {
    entry.fill(0);
    entry[0..11].copy_from_slice(raw_8_3);
    entry[11] = 0x20;
    let time_bytes = FAT16_DOS_TIME_MIDNIGHT.to_le_bytes();
    let date_bytes = FAT16_DOS_DATE_2006_03_06.to_le_bytes();
    entry[14..16].copy_from_slice(&time_bytes);
    entry[16..18].copy_from_slice(&date_bytes);
    entry[18..20].copy_from_slice(&date_bytes);
    entry[22..24].copy_from_slice(&time_bytes);
    entry[24..26].copy_from_slice(&date_bytes);
    entry[26..28].copy_from_slice(&first_cluster.to_le_bytes());
    entry[28..32].copy_from_slice(&file_size.to_le_bytes());
}

/// Synthesizes a byte-deterministic FAT16B (`0x06`) SD card image and validates it via [`inspect_sd_disk_bytes`].
pub fn synthesize_fat16_sd_image(
    volume_mb: u32,
    inputs: &[Fat16SdFileInput],
) -> Result<SynthesizedSdImage> {
    let geom = compute_fat16_sd_geometry(volume_mb)?;
    if inputs.is_empty() || inputs.len() > usize::from(geom.root_entry_count) {
        bail!(
            "Invalid FAT16 root file count {} (must be 1..={})",
            inputs.len(),
            geom.root_entry_count
        );
    }

    let mut seen_names = BTreeSet::new();
    let mut encoded_inputs = Vec::with_capacity(inputs.len());
    for file in inputs {
        let raw_name = encode_dos_8_3_name(&file.name_8_3)?;
        if !seen_names.insert(raw_name) {
            bail!("Duplicate 8.3 filename in FAT16 root: '{}'", file.name_8_3);
        }
        encoded_inputs.push((raw_name, file));
    }

    let mut image = vec![0u8; geom.total_image_bytes()];
    write_mbr(&mut image[0..FAT16_SECTOR_BYTES], geom);
    let vbr_off = geom.lba_start as usize * FAT16_SECTOR_BYTES;
    write_vbr(&mut image[vbr_off..vbr_off + FAT16_SECTOR_BYTES], geom);

    let fat_bytes_len = usize::from(geom.sectors_per_fat) * FAT16_SECTOR_BYTES;
    let mut fat = vec![0u8; fat_bytes_len];
    fat[0..2].copy_from_slice(&0xFFF8u16.to_le_bytes());
    fat[2..4].copy_from_slice(&0xFFFFu16.to_le_bytes());

    let root_off = geom.root_dir_lba() as usize * FAT16_SECTOR_BYTES;
    let data_off = geom.data_start_lba() as usize * FAT16_SECTOR_BYTES;
    let cluster_bytes = geom.cluster_size_bytes();

    let mut next_cluster = 2u32;
    let mut placed_files = Vec::with_capacity(encoded_inputs.len());

    for (idx, (raw_name, file)) in encoded_inputs.into_iter().enumerate() {
        let clus_count = if file.bytes.is_empty() {
            0usize
        } else {
            file.bytes.len().div_ceil(cluster_bytes)
        };
        let first_cluster = if clus_count == 0 {
            0
        } else {
            next_cluster as u16
        };
        let last_cluster = next_cluster + clus_count as u32;
        if last_cluster > geom.data_clusters + 2 {
            bail!(
                "FAT16 image ({volume_mb} MiB) out of clusters placing '{}'",
                file.name_8_3
            );
        }

        if clus_count > 0 {
            for c in next_cluster..(last_cluster - 1) {
                let off = c as usize * 2;
                fat[off..off + 2].copy_from_slice(&((c + 1) as u16).to_le_bytes());
            }
            let end_off = (last_cluster - 1) as usize * 2;
            fat[end_off..end_off + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());

            let byte_off = data_off + (usize::from(first_cluster) - 2) * cluster_bytes;
            image[byte_off..byte_off + file.bytes.len()].copy_from_slice(&file.bytes);
            next_cluster = last_cluster;
        }

        let r_off = root_off + idx * 32;
        write_root_entry(
            &mut image[r_off..r_off + 32],
            &raw_name,
            first_cluster,
            file.bytes.len() as u32,
        );

        let (_, sha256) = calculate_hashes(&file.bytes);
        placed_files.push(Fat16PlacedFileReport {
            name_8_3: file.name_8_3.trim().to_string(),
            size_bytes: file.bytes.len(),
            first_cluster,
            cluster_count: clus_count,
            sha256,
        });
    }

    let fat1_off = geom.fat1_lba() as usize * FAT16_SECTOR_BYTES;
    let fat2_off = geom.fat2_lba() as usize * FAT16_SECTOR_BYTES;
    image[fat1_off..fat1_off + fat_bytes_len].copy_from_slice(&fat);
    image[fat2_off..fat2_off + fat_bytes_len].copy_from_slice(&fat);

    let inspection = inspect_sd_disk_bytes("synth:fat16b.img", &image)?;
    if inspection.overall_status != "PASS" {
        bail!(
            "Synthesized FAT16B SD image failed mandatory self-audit ({}): {:?}",
            inspection.overall_status,
            inspection.checks
        );
    }

    let (_, image_sha256) = calculate_hashes(&image);
    Ok(SynthesizedSdImage {
        geometry: geom,
        image_bytes: image,
        image_sha256,
        files: placed_files,
        inspection,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PANA_FEISTEL_ROUND_KEY_32B;
    use crate::crypto::{FrmContainerMode, FrmPackModule, pack_frm_with_key};
    use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};
    use crate::re::unbrick_sd_kit::synthetic_prog061_fixture;

    fn make_valid_ver061_digadiag() -> Vec<u8> {
        let mut raw = synthetic_prog061_fixture();
        patch_prog061_bgo_in_place(&mut raw, Prog061BgoPatchMode::UartMonitor).unwrap();
        raw
    }

    #[test]
    fn test_synthesize_32mb_fat16b_matches_exact_layout_and_determinism() {
        let diag = make_valid_ver061_digadiag();
        let pana_sd = pack_frm_with_key(
            &[FrmPackModule::new("PROG061", diag.clone())],
            FrmContainerMode::Sd48,
            Some(0x1800),
            &PANA_FEISTEL_ROUND_KEY_32B,
        )
        .unwrap()
        .0;

        let files = vec![
            Fat16SdFileInput {
                name_8_3: "DIGADIAG.FRM".into(),
                bytes: diag,
            },
            Fat16SdFileInput {
                name_8_3: "PANA__SD.FRM".into(),
                bytes: pana_sd,
            },
        ];
        let a = synthesize_fat16_sd_image(32, &files).unwrap();
        let b = synthesize_fat16_sd_image(32, &files).unwrap();
        assert_eq!(a.image_sha256, b.image_sha256);
        assert_eq!(a.image_bytes.len(), 65_599 * 512);
        assert_eq!(a.geometry.sectors_per_fat, 32);
        assert_eq!(a.geometry.data_clusters, 8_179);
        assert_eq!(a.geometry.root_dir_lba(), 128);
        assert_eq!(a.image_bytes[0x1C2], 0x06);
        assert_eq!(
            u16::from_le_bytes([a.image_bytes[63 * 512 + 14], a.image_bytes[63 * 512 + 15]]),
            1
        );
        assert_eq!(a.inspection.overall_status, "PASS");
        assert!(a.inspection.pana_sd_entry.as_ref().unwrap().chain_complete);

        // Inject LFN entry right after existing root entries -> inspect_sd must flag warn!
        let mut with_lfn = a.image_bytes.clone();
        let lfn_off = 128 * 512 + 2 * 32;
        with_lfn[lfn_off] = 0x41;
        with_lfn[lfn_off + 11] = 0x0F;
        let rep_lfn = inspect_sd_disk_bytes("mem:lfn.img", &with_lfn).unwrap();
        assert_eq!(rep_lfn.overall_status, "WARN");
        assert!(
            rep_lfn
                .checks
                .iter()
                .any(|c| c.id == "sd_root_lfn_or_junk" && c.severity == "warn")
        );
    }
}
