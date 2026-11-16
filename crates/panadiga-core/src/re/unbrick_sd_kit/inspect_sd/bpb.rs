//! FAT16 VBR/BPB parser and cluster-count suitability checks for SD unbrick media.

use super::super::verify::{SdUnbrickCheck, check};
use super::mbr_bpb::{ascii_trimmed, u16_le, u32_le};
use super::types::FatBpbReport;

pub(crate) fn parse_bpb_from_vbr(vbr: &[u8]) -> FatBpbReport {
    let oem_name = ascii_trimmed(&vbr[3..11]);
    let bytes_per_sector = u16_le(vbr, 11);
    let sectors_per_cluster = vbr[13];
    let reserved_sectors = u16_le(vbr, 14);
    let num_fats = vbr[16];
    let root_entry_count = u16_le(vbr, 17);
    let total_sectors_16 = u32::from(u16_le(vbr, 19));
    let media_descriptor = vbr[21];
    let fat_size_16 = u32::from(u16_le(vbr, 22));
    let hidden_sectors = u32_le(vbr, 28);
    let total_sectors_32 = u32_le(vbr, 32);

    let total_sectors = if total_sectors_16 != 0 {
        total_sectors_16
    } else {
        total_sectors_32
    };
    let sectors_per_fat = if fat_size_16 != 0 {
        fat_size_16
    } else {
        u32_le(vbr, 36)
    };
    let (fat_type_label, volume_label) = if fat_size_16 != 0 {
        (ascii_trimmed(&vbr[54..62]), ascii_trimmed(&vbr[43..54]))
    } else {
        (ascii_trimmed(&vbr[82..90]), ascii_trimmed(&vbr[71..82]))
    };

    let root_dir_sectors = if bytes_per_sector != 0 {
        (u32::from(root_entry_count) * 32).div_ceil(u32::from(bytes_per_sector))
    } else {
        0
    };
    let data_start = u32::from(reserved_sectors)
        .saturating_add(u32::from(num_fats).saturating_mul(sectors_per_fat))
        .saturating_add(root_dir_sectors);
    let data_sectors = total_sectors.saturating_sub(data_start);
    let spc = u32::from(sectors_per_cluster.max(1));
    let data_clusters = if bytes_per_sector == 512 && sectors_per_cluster != 0 {
        data_sectors / spc
    } else {
        0
    };
    let computed_fat_kind = if bytes_per_sector != 512 || sectors_per_cluster == 0 {
        "INVALID"
    } else if data_clusters < 4085 {
        "FAT12"
    } else if data_clusters < 65525 {
        "FAT16"
    } else {
        "FAT32"
    }
    .to_string();

    FatBpbReport {
        oem_name,
        bytes_per_sector,
        sectors_per_cluster,
        cluster_size_bytes: u32::from(bytes_per_sector) * u32::from(sectors_per_cluster),
        reserved_sectors,
        num_fats,
        root_entry_count,
        total_sectors,
        media_descriptor,
        sectors_per_fat,
        hidden_sectors,
        fat_type_label,
        volume_label,
        data_clusters,
        computed_fat_kind,
    }
}

pub(crate) fn validate_fat16_bpb(
    part_lba: u64,
    vbr: &[u8],
    checks: &mut Vec<SdUnbrickCheck>,
) -> Option<(FatBpbReport, bool)> {
    if vbr[510] != 0x55 || vbr[511] != 0xAA {
        checks.push(check(
            "sd_vbr_sig",
            "fail",
            "Partition VBR missing 0x55AA signature",
            format!(
                "VBR at LBA {part_lba} ends in {:02X}{:02X}",
                vbr[510], vbr[511]
            ),
        ));
        return None;
    }

    let bpb = parse_bpb_from_vbr(vbr);
    let mut fat_ok = true;
    if bpb.bytes_per_sector != 512 {
        fat_ok = false;
        checks.push(check(
            "sd_bps",
            "fail",
            format!(
                "Unsupported BytesPerSector={} (expected 512)",
                bpb.bytes_per_sector
            ),
            "MN103S SD block driver reads 512-byte sectors.",
        ));
    }
    if bpb.computed_fat_kind != "FAT16" {
        fat_ok = false;
        checks.push(check(
            "sd_fat_kind",
            "fail",
            format!(
                "Filesystem is {} ({} data clusters; label='{}'), NOT FAT16!",
                bpb.computed_fat_kind, bpb.data_clusters, bpb.fat_type_label
            ),
            "FUN_4400c696 requires FAT16 (4085..=65524 clusters; rejects > 0xFFF5 clusters / FAT32).",
        ));
    } else if bpb.sectors_per_cluster > 64 {
        checks.push(check(
            "sd_cluster_size",
            "warn",
            format!(
                "Non-standard 64 KiB clusters ({} sectors/cluster) on FAT16",
                bpb.sectors_per_cluster
            ),
            "2006 embedded FAT16 drivers often cap at 32 KiB (64 sectors/cluster). Reformat with `/A:32K` if SD diag load failed[0] persists.",
        ));
    } else {
        checks.push(check(
            "sd_fat_kind",
            "pass",
            format!(
                "Valid FAT16 BPB: {} clusters, {} KB clusters, {} FATs, {} root entries",
                bpb.data_clusters,
                bpb.cluster_size_bytes / 1024,
                bpb.num_fats,
                bpb.root_entry_count
            ),
            format!(
                "OEM='{}', FSLabel='{}', HiddenSectors={}",
                bpb.oem_name, bpb.fat_type_label, bpb.hidden_sectors
            ),
        ));
    }

    if part_lba > 0 && u64::from(bpb.hidden_sectors) != part_lba {
        checks.push(check(
            "sd_hidden_sectors",
            "warn",
            format!(
                "BPB HiddenSectors ({}) != MBR lba_start ({part_lba})",
                bpb.hidden_sectors
            ),
            "Mismatch between MBR partition start and VBR HiddenSectors field.",
        ));
    }

    Some((bpb, fat_ok))
}
