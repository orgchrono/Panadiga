//! Synthetic in-memory MBR / GPT / Superfloppy / FAT16 audit tests.

use super::inspect_sd_disk_bytes;
use super::mbr_bpb::SECTOR_SIZE;
use crate::constants::{PANA_DIGADIAG_DEFAULT_LOAD_ADDR, PANA_DIGADIAG_FRM_SIZE};
use crate::crypto::build_digadiag_frm;

fn make_synthetic_mbr_fat16_image(
    part_type: u8,
    spc: u8,
    total_sectors: u32,
    include_digadiag: bool,
    superfloppy: bool,
) -> Vec<u8> {
    let part_start: usize = if superfloppy { 0 } else { 63 };
    let vol_sectors = total_sectors - part_start as u32;
    let reserved: u16 = 1;
    let num_fats: u8 = 2;
    let root_entries: u16 = 512;
    let fat_sectors: u16 = 128;
    let mut img = vec![0u8; (total_sectors as usize) * SECTOR_SIZE];

    if !superfloppy {
        img[510] = 0x55;
        img[511] = 0xAA;
        img[0x1BE + 4] = part_type;
        img[0x1BE + 8..0x1BE + 12].copy_from_slice(&(part_start as u32).to_le_bytes());
        img[0x1BE + 12..0x1BE + 16].copy_from_slice(&vol_sectors.to_le_bytes());
    }

    let vbr_off = part_start * SECTOR_SIZE;
    img[vbr_off..vbr_off + 3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
    img[vbr_off + 3..vbr_off + 11].copy_from_slice(b"PANADIGA");
    img[vbr_off + 11..vbr_off + 13].copy_from_slice(&512u16.to_le_bytes());
    img[vbr_off + 13] = spc;
    img[vbr_off + 14..vbr_off + 16].copy_from_slice(&reserved.to_le_bytes());
    img[vbr_off + 16] = num_fats;
    img[vbr_off + 17..vbr_off + 19].copy_from_slice(&root_entries.to_le_bytes());
    img[vbr_off + 21] = 0xF8;
    img[vbr_off + 22..vbr_off + 24].copy_from_slice(&fat_sectors.to_le_bytes());
    img[vbr_off + 28..vbr_off + 32].copy_from_slice(&(part_start as u32).to_le_bytes());
    img[vbr_off + 32..vbr_off + 36].copy_from_slice(&vol_sectors.to_le_bytes());
    img[vbr_off + 43..vbr_off + 54].copy_from_slice(b"EH55SD     ");
    img[vbr_off + 54..vbr_off + 62].copy_from_slice(b"FAT16   ");
    img[vbr_off + 510] = 0x55;
    img[vbr_off + 511] = 0xAA;

    let fat1_off = (part_start + usize::from(reserved)) * SECTOR_SIZE;
    let root_off =
        (part_start + usize::from(reserved) + usize::from(num_fats) * usize::from(fat_sectors))
            * SECTOR_SIZE;
    let data_start_lba =
        part_start + usize::from(reserved) + usize::from(num_fats) * usize::from(fat_sectors) + 32;

    if include_digadiag {
        let prog = vec![0xCB; 0x4000];
        let (diag, _) = build_digadiag_frm(
            &prog,
            PANA_DIGADIAG_DEFAULT_LOAD_ADDR,
            "FVU ",
            "TESTSD",
            4,
            PANA_DIGADIAG_FRM_SIZE,
        )
        .unwrap();

        let cluster_bytes = usize::from(spc) * SECTOR_SIZE;
        let clusters_needed = diag.len().div_ceil(cluster_bytes);
        for i in 0..clusters_needed {
            let clus = (2 + i) as u16;
            let next = if i + 1 == clusters_needed {
                0xFFFFu16
            } else {
                clus + 1
            };
            let f_off = fat1_off + usize::from(clus) * 2;
            img[f_off..f_off + 2].copy_from_slice(&next.to_le_bytes());
        }
        img[root_off..root_off + 11].copy_from_slice(b"DIGADIAGFRM");
        img[root_off + 11] = 0x20;
        img[root_off + 26..root_off + 28].copy_from_slice(&2u16.to_le_bytes());
        img[root_off + 28..root_off + 32].copy_from_slice(&(diag.len() as u32).to_le_bytes());

        let dst_off = data_start_lba * SECTOR_SIZE;
        img[dst_off..dst_off + diag.len()].copy_from_slice(&diag);
    }

    img
}

#[test]
fn valid_mbr_fat16_with_digadiag_passes_full_audit() {
    let img = make_synthetic_mbr_fat16_image(0x06, 4, 65_536, true, false);
    let rep = inspect_sd_disk_bytes("mem:ok.img", &img).unwrap();
    assert_eq!(rep.partition_scheme, "MBR");
    assert_eq!(rep.overall_status, "PASS", "checks: {:?}", rep.checks);
    assert!(rep.digadiag_payload_verified);
    assert_eq!(rep.bpb.as_ref().unwrap().computed_fat_kind, "FAT16");
}

#[test]
fn detects_superfloppy_and_gpt_failures() {
    let sf = make_synthetic_mbr_fat16_image(0x06, 4, 65_536, false, true);
    let rep_sf = inspect_sd_disk_bytes("mem:sf.img", &sf).unwrap();
    assert_eq!(rep_sf.partition_scheme, "SUPERFLOPPY_NO_MBR");
    assert_eq!(rep_sf.overall_status, "FAIL");

    let mut gpt = vec![0u8; 4096];
    gpt[510] = 0x55;
    gpt[511] = 0xAA;
    gpt[0x1BE + 4] = 0xEE;
    gpt[0x1BE + 8..0x1BE + 12].copy_from_slice(&1u32.to_le_bytes());
    gpt[0x1BE + 12..0x1BE + 16].copy_from_slice(&1000u32.to_le_bytes());
    let rep_gpt = inspect_sd_disk_bytes("mem:gpt.img", &gpt).unwrap();
    assert_eq!(rep_gpt.partition_scheme, "GPT");
    assert_eq!(rep_gpt.overall_status, "FAIL");
}
