use super::*;

#[test]
fn test_inspect_hdd_slice_pure() {
    let total_size = (LBA_MIRROR_MBR_OFFSET as usize) + LBA_SECTOR_SIZE_BYTES;
    let mut mock_data = vec![0u8; total_size];

    // Format LBA 0
    mock_data[4] = 0x01; // Format version 1
    mock_data[6..8].copy_from_slice(b"SH");
    mock_data[8..16].copy_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);

    // Copy to mirror MBR
    let sector0 = mock_data[..LBA_SECTOR_SIZE_BYTES].to_vec();
    let mirror_offset = LBA_MIRROR_MBR_OFFSET as usize;
    mock_data[mirror_offset..mirror_offset + LBA_SECTOR_SIZE_BYTES].copy_from_slice(&sector0);

    let report = inspect_hdd_slice(&mock_data, "memory://mock_hdd.bin")
        .expect("inspect_hdd_slice should succeed on valid mock data");

    assert_eq!(report.mbr_tables.format_version, 1);
    assert_eq!(report.mbr_tables.magic_ascii, "SH");
    assert!(report.mbr_tables.mirror_is_bit_identical);
}
