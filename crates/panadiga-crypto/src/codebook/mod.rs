//! 64-bit ECB Known-Plaintext Attack (KPA) codebook and HDD correlation engine.

mod analyze;
mod render;

pub use analyze::{analyze_codebook_slices, correlate_hdd_partitions};
pub(crate) use analyze::{load_optional_hdd_image, open_resolved_frm};
pub use render::print_codebook;

#[cfg(test)]
mod pure_codebook_tests {
    use super::*;
    use crate::constants::{ECB_NULL_BLOCK_8B, ECB_VIRGIN_FLASH_8B};

    #[test]
    fn test_pure_codebook_analysis_in_memory() {
        let mut mock_frm = vec![0u8; 64];
        mock_frm[0..8].copy_from_slice(&ECB_VIRGIN_FLASH_8B);
        mock_frm[8..16].copy_from_slice(&ECB_VIRGIN_FLASH_8B);
        mock_frm[16..24].copy_from_slice(&ECB_NULL_BLOCK_8B);

        let mut mock_hdd = vec![0u8; 32];
        mock_hdd[0..8].copy_from_slice(&ECB_VIRGIN_FLASH_8B);

        let report = analyze_codebook_slices(
            &mock_frm,
            Some(&mock_hdd),
            "memory://mock.frm",
            Some("memory://mock.hdd".to_string()),
        );

        assert_eq!(report.total_64bit_blocks_analyzed, 8);
        assert!(report.cross_medium_key_identity_proven);

        let virgin = report
            .dictionary
            .iter()
            .find(|e| e.block_hex == "035065C87BB8E090")
            .unwrap();
        assert_eq!(virgin.frm_occurrences, 2);
        assert_eq!(virgin.hdd_occurrences, 1);
    }
}
