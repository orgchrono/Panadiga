#![forbid(unsafe_code)]

use std::ops::Deref;

use panadiga_constants::forensic::{ImageBuffer, calculate_entropy, calculate_hashes};
use panadiga_constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE, AM34_LETHAL_MMIO_ADDR,
    AM34_NVRAM_BASE, AM34_NVRAM_SIZE, LBA28_MAX_BLOCKS_2KB, PANA_LZSS_WINDOW_INIT,
    PANA_LZSS_WINDOW_SIZE, SPANSION_S29JL064H_SECTOR_COUNT,
};

#[test]
fn hardware_and_storage_constants_match_silicon_contracts() {
    assert_eq!(AM34_FLASH_BASE, 0x8000_0000);
    assert_eq!(AM34_FLASH_SIZE, 0x0080_0000);
    assert_eq!(AM34_FLASH_CRITICAL_IPL_SIZE, 0x0004_0000);
    assert_eq!(AM34_NVRAM_BASE, 0x807F_0000);
    assert_eq!(AM34_NVRAM_SIZE, 0x0001_0000);
    assert_eq!(AM34_LETHAL_MMIO_ADDR, 0xC000_100C);
    assert_eq!(SPANSION_S29JL064H_SECTOR_COUNT, 142);
    assert_eq!(LBA28_MAX_BLOCKS_2KB, 0x03FF_FFFF);
    assert_eq!(PANA_LZSS_WINDOW_SIZE, 0x1000);
    assert_eq!(PANA_LZSS_WINDOW_INIT, 0x0FEE);
}

#[test]
fn forensic_hashes_entropy_and_image_buffer_ram_are_exact() {
    let (empty_md5, empty_sha) = calculate_hashes(b"");
    assert_eq!(empty_md5, "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(
        empty_sha,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    let (pana_md5, pana_sha) = calculate_hashes(b"PANADIGA");
    assert_eq!(pana_md5.len(), 32);
    assert_eq!(pana_sha.len(), 64);
    assert_ne!(pana_sha, empty_sha);

    assert_eq!(calculate_entropy(b""), 0.0);
    assert_eq!(calculate_entropy(&[0xA5; 512]), 0.0);
    let uniform: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    assert!(calculate_entropy(&uniform) > 7.99);

    let ram = ImageBuffer::Ram(vec![1, 2, 3, 4]);
    assert_eq!(ram.deref(), &[1, 2, 3, 4]);
}
