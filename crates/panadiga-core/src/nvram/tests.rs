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
