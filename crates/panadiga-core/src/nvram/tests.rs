use super::*;

#[test]
fn test_pure_slice_cleaning_in_memory() {
    let size = 8 * 1024 * 1024;
    let mut mock_flash = vec![0x55u8; size];
    // Put corrupt data in NVRAM
    let nvram_offset = size - SPANSION_NVRAM_SIZE_BYTES;
    for i in 0..SPANSION_NVRAM_SIZE_BYTES {
        mock_flash[nvram_offset + i] = (i % 250) as u8;
    }

    let output = clean_flash_nvram_slice(
        &mock_flash,
        "memory://test.bin",
        "memory://clean.bin",
        "memory://backup.bin",
    )
    .expect("Pure slice cleaning must succeed");

    assert_eq!(output.cleaned_bytes.len(), size);
    assert_eq!(output.backup_nvram_bytes.len(), SPANSION_NVRAM_SIZE_BYTES);
    // Verify NVRAM area is completely 0xFF
    assert!(
        output.cleaned_bytes[nvram_offset..]
            .iter()
            .all(|&b| b == FLASH_ERASED_BYTE)
    );
    // Verify OS/Bootloader area is 100% identical
    assert_eq!(
        &output.cleaned_bytes[..nvram_offset],
        &mock_flash[..nvram_offset]
    );
    assert!(output.report.bootloader_immutable);
}

#[test]
fn test_patch_flash_nvram_slice_cow_and_ipl_guard() {
    let size = 8 * 1024 * 1024;
    let mock_flash = vec![0xA5u8; size];
    let nvram_offset = size - SPANSION_NVRAM_SIZE_BYTES;
    let patched = patch_flash_nvram_slice(
        &mock_flash,
        0x100,
        b"HITA",
        "memory://src.bin",
        "memory://patched.bin",
        "memory://bak.bin",
    )
    .expect("Guarded NVRAM patch within top 64 KB must succeed");
    assert_eq!(
        &patched.cleaned_bytes[nvram_offset + 0x100..nvram_offset + 0x104],
        b"HITA"
    );
    assert_eq!(
        &patched.cleaned_bytes[..nvram_offset],
        &mock_flash[..nvram_offset]
    );
    assert!(patched.report.bootloader_immutable);
    assert!(
        patch_flash_nvram_slice(
            &mock_flash,
            SPANSION_NVRAM_SIZE_BYTES - 2,
            b"HITA",
            "a",
            "b",
            "c"
        )
        .is_err()
    );
}

#[test]
fn test_patch_flash_upper_nor_slice_layers_and_bounds() {
    let size = SPANSION_FLASH_SIZE_BYTES;
    let mut mock_flash = vec![0x33u8; size];
    mock_flash[0x0020_0100..0x0020_0104].copy_from_slice(b"ORIG");

    // Layer 1 Critical Boot IPL (0..0x040000) is blocked by default
    assert!(
        patch_flash_upper_nor_slice(&mock_flash, 0x0000_0000, b"TEST", false, "i", "o", "b")
            .is_err()
    );
    assert!(
        patch_flash_upper_nor_slice(&mock_flash, 0x0003_FFFE, b"TEST", false, "i", "o", "b")
            .is_err()
    );
    // Empty payload & invalid NOR size & NVRAM invasion & out-of-bounds rejected
    assert!(
        patch_flash_upper_nor_slice(&mock_flash, 0x0004_0000, b"", false, "i", "o", "b").is_err()
    );
    assert!(
        patch_flash_upper_nor_slice(&vec![0u8; 1024], 0x0004_0000, b"AB", false, "i", "o", "b")
            .is_err()
    );
    assert!(
        patch_flash_upper_nor_slice(&mock_flash, 0x007E_FFFE, b"ABCD", false, "i", "o", "b")
            .is_err()
    );
    assert!(
        patch_flash_upper_nor_slice(&mock_flash, 0x007F_0000, b"ABCD", false, "i", "o", "b")
            .is_err()
    );
    assert!(
        patch_flash_upper_nor_slice(&mock_flash, 0x0080_0000, b"ABCD", false, "i", "o", "b")
            .is_err()
    );

    // Layer 2 Upper NOR App/BGO (0x040000..0x7EFFFF) allowed with 64 KiB sector backup
    let out = patch_flash_upper_nor_slice(
        &mock_flash,
        0x0020_0100,
        b"BGO!",
        false,
        "memory://in.bin",
        "memory://out.bin",
        "memory://bak.bin",
    )
    .expect("Upper NOR BGO patch must succeed");
    assert_eq!(&out.patched_bytes[0x0020_0100..0x0020_0104], b"BGO!");
    assert_eq!(
        out.backup_sector_bytes.len(),
        SPANSION_MAIN_SECTOR_SIZE_BYTES
    );
    assert_eq!(&out.backup_sector_bytes[0x100..0x104], b"ORIG");
    assert_eq!(out.report.first_sector_name, "SA32");
    assert_eq!(out.report.last_sector_name, "SA32");
    assert!(!out.report.critical_ipl_override_used);
    assert!(out.report.critical_ipl_untouched);
    assert!(out.report.nvram_untouched);
    assert_ne!(out.report.original_sha256, out.report.patched_sha256);
    assert_ne!(
        out.report.patched_range_before_sha256,
        out.report.patched_range_after_sha256
    );

    // Explicit offline override allows patching SA0..SA3 while keeping NVRAM intact
    let crit = patch_flash_upper_nor_slice(
        &mock_flash,
        0x0000_0010,
        b"IPL!",
        true,
        "memory://in.bin",
        "memory://out_crit.bin",
        "memory://bak_crit.bin",
    )
    .expect("Explicit allow_critical_ipl must permit offline CoW IPL patch");
    assert_eq!(&crit.patched_bytes[0x10..0x14], b"IPL!");
    assert!(crit.report.critical_ipl_override_used);
    assert!(!crit.report.critical_ipl_untouched);
    assert!(crit.report.nvram_untouched);
    assert_eq!(crit.report.first_sector_name, "SA0");
}

#[test]
fn test_patch_flash_upper_nor_file_cow_and_backup() {
    let dir = std::env::temp_dir().join("panadiga_upper_nor_cow_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let in_path = dir.join("flash_4mb.bin");
    let out_path = dir.join("flash_4mb_patched.bin");
    let bak_path = dir.join("sector_sa6_bak.bin");
    let mut img = vec![0x77u8; 4 * 1024 * 1024];
    img[0x0006_0020..0x0006_0024].copy_from_slice(&[0x10, 0x20, 0x30, 0x40]);
    std::fs::write(&in_path, &img).unwrap();

    // Reject in-place overwrite of input
    assert!(
        patch_flash_upper_nor_file(
            &in_path,
            &in_path,
            Some(&bak_path),
            0x0006_0020,
            b"ABCD",
            false
        )
        .is_err()
    );

    let report = patch_flash_upper_nor_file(
        &in_path,
        &out_path,
        Some(&bak_path),
        0x0006_0020,
        b"ABCD",
        false,
    )
    .expect("Upper NOR CoW file patch must succeed");
    assert_eq!(report.total_flash_bytes, 4 * 1024 * 1024);
    assert_eq!(report.first_sector_name, "SA6");
    assert!(report.critical_ipl_untouched);
    assert!(report.nvram_untouched);

    let out_bytes = std::fs::read(&out_path).unwrap();
    assert_eq!(&out_bytes[0x0006_0020..0x0006_0024], b"ABCD");
    let bak_bytes = std::fs::read(&bak_path).unwrap();
    assert_eq!(bak_bytes.len(), SPANSION_MAIN_SECTOR_SIZE_BYTES);
    assert_eq!(&bak_bytes[0x20..0x24], &[0x10, 0x20, 0x30, 0x40]);
    // Original file must remain unmodified
    assert_eq!(std::fs::read(&in_path).unwrap(), img);

    let _ = std::fs::remove_dir_all(&dir);
}
