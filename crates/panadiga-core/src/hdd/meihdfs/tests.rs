use super::*;

#[cfg(test)]
pub mod pure_meihdfs_tests {
    use super::*;

    #[test]
    fn test_decode_single_meihdfs_record_header() {
        // Raw chunk where inverted chunk has tag 0x5D (ExtentHeader)
        // 0x5D ^ 0xFF = 0xA2
        let mut raw = [0xFFu8; 16];
        raw[0] = 0x5D ^ 0xFF; // raw tag
        raw[1] = 0x01 ^ 0xFF; // subtag
        raw[2] = 0x34 ^ 0xFF; // extent ID low
        raw[3] = 0x12 ^ 0xFF; // extent ID high (0x1234)

        let record = decode_single_meihdfs_record(&raw, 42);
        assert_eq!(record.index, 42);
        assert_eq!(record.decoded_tag, MEIHDFS_TAG_EXTENT_HEADER);
        assert_eq!(record.tag_name, "0x5D (ExtentHeader)");
        assert_eq!(record.extent_id, Some(0x1234));
    }

    #[test]
    fn test_pure_meihdfs_slice_decoding() {
        let mut descriptor_table = vec![0xFFu8; ALLOCATION_DESCRIPTORS_SIZE];
        // Create one ExtentHeader record at index 0 (ID: 0x0001)
        descriptor_table[0] = 0x5D ^ 0xFF;
        descriptor_table[2] = 0x01 ^ 0xFF;
        descriptor_table[3] = 0xFF; // 0x00 inverted

        // Create one ExtentPointer record at index 1 (ID: 0x0001, 8 sectors, LBA 100)
        descriptor_table[16] = 0x55 ^ 0xFF;
        descriptor_table[18] = 0x01 ^ 0xFF;
        descriptor_table[19] = 0xFF; // 0x00 inverted
        descriptor_table[20] = 0x08 ^ 0xFF; // 8 sectors low
        descriptor_table[21] = 0xFF; // 0x00 inverted

        let report = decode_meihdfs_slice(&descriptor_table, false, "in-memory-test")
            .expect("pure decode should succeed");

        assert_eq!(report.total_records, 256);
        assert_eq!(
            report.tag_distribution.get("0x5D (ExtentHeader)").copied(),
            Some(1)
        );
        assert_eq!(
            report.tag_distribution.get("0x55 (ExtentPointer)").copied(),
            Some(1)
        );
        assert_eq!(report.extents.len(), 1);
        assert_eq!(report.extents[0].extent_id, 0x0001);
        assert_eq!(report.extents[0].total_sectors, 8);
        assert_eq!(report.extents[0].total_bytes, 8 * 512);
    }

    #[test]
    fn test_is_full_image_requires_mirror_descriptors_end() {
        // Length covers LBA_MIRROR_MBR_OFFSET + 4096 but stops 1 byte short of mirror descriptors end.
        let mirror_end =
            ALLOCATION_DESCRIPTORS_MIRROR_OFFSET as usize + ALLOCATION_DESCRIPTORS_SIZE;
        let short_buf = vec![0u8; mirror_end - 1];
        let rep_short = decode_meihdfs_slice(&short_buf, false, "short-buf").unwrap();
        assert!(!rep_short.is_full_hdd_image);
        assert!(!rep_short.is_mirror_verified);

        let full_buf = vec![0u8; mirror_end];
        let rep_full = decode_meihdfs_slice(&full_buf, false, "full-buf").unwrap();
        assert!(rep_full.is_full_hdd_image);
        assert!(rep_full.is_mirror_verified);
    }
}
