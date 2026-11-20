#![deny(unsafe_code)]

use super::*;

#[test]
fn audits_all_142_spansion_sectors_entropy_and_nvram_health() {
    let mut nor = vec![0xFFu8; SPANSION_FLASH_SIZE_BYTES];
    nor[0x10..0x23].copy_from_slice(b"Loader ver0.61 test");
    nor[SPANSION_NVRAM_BASE_OFFSET..SPANSION_NVRAM_BASE_OFFSET + 4]
        .copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);

    let report = analyze_spansion_nor_sectors(&nor).unwrap();
    assert_eq!(report.total_sectors, 142);
    assert_eq!(report.populated_sectors, 2);
    assert_eq!(report.blank_sectors, 140);
    assert_eq!(report.sectors[0].size_bytes, 8192);
    assert_eq!(report.sectors[8].size_bytes, 65536);
    assert_eq!(report.sectors[134].size_bytes, 8192);
    assert_eq!(report.sectors[0].region, SpansionRegionKind::BootIpl);
    assert_eq!(report.sectors[26].region, SpansionRegionKind::BgoRegion);
    assert_eq!(report.sectors[134].region, SpansionRegionKind::EemulNvram);
    assert_eq!(
        report.sectors[0].printable_signature.as_deref(),
        Some("Loader ver0.61 test")
    );
    assert_eq!(report.sectors[8].shannon_entropy, 0.0);
    assert_eq!(report.nvram_health.non_ff_bytes, 4);
    assert_eq!(report.nvram_health.active_microsectors, 1);
    assert!(!report.nvram_health.is_factory_erased);
    assert!(report.loader_signatures.iter().any(|s| s.contains("ver0.")));
}
