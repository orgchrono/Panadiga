use super::*;
use crate::ssot::{LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride};

fn plant(buf: &mut [u8], off: usize, bytes: &[u8]) {
    buf[off..off + bytes.len()].copy_from_slice(bytes);
}

fn eh55_seed(flash: &mut [u8]) {
    plant(flash, 0x0003_F000, b"Loader ver0.61 [06/03/06]");
    let nv = flash.len() - SPANSION_NVRAM_SIZE_BYTES;
    plant(flash, nv, &[0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0xF1, 0xFF, 0xFF]);
}

fn demo_override() -> LabOverride {
    LabOverride {
        disclaimer_ack: LAB_OVERRIDE_DISCLAIMER_ACK.to_string(),
        operator_reason: "Synthetic R&D unit test image verification".to_string(),
    }
}

#[test]
fn test_pure_slice_cleaning_in_memory() {
    let size = 8 * 1024 * 1024;
    let mut mock_flash = vec![0x55u8; size];
    eh55_seed(&mut mock_flash);
    let nvram_offset = size - SPANSION_NVRAM_SIZE_BYTES;
    for i in 16..SPANSION_NVRAM_SIZE_BYTES {
        mock_flash[nvram_offset + i] = (i % 250) as u8;
    }

    let output = clean_flash_nvram_slice(
        &mock_flash,
        "memory://test.bin",
        "memory://clean.bin",
        "memory://backup.bin",
    )
    .expect("Verified EH55 slice cleaning must succeed");

    assert_eq!(output.cleaned_bytes.len(), size);
    assert_eq!(output.backup_nvram_bytes.len(), SPANSION_NVRAM_SIZE_BYTES);
    assert_eq!(output.report.microsectors[0].sector_name, "SA134");
    assert_eq!(output.report.microsectors[7].sector_name, "SA141");
    assert!(
        output.cleaned_bytes[nvram_offset..]
            .iter()
            .all(|&b| b == FLASH_ERASED_BYTE)
    );
    assert_eq!(
        &output.cleaned_bytes[..nvram_offset],
        &mock_flash[..nvram_offset]
    );
    assert!(output.report.bootloader_immutable);
}

#[test]
fn test_patch_flash_nvram_slice_cow_and_ipl_guard() {
    let size = 8 * 1024 * 1024;
    let mut mock_flash = vec![0xA5u8; size];
    eh55_seed(&mut mock_flash);
    let nvram_offset = size - SPANSION_NVRAM_SIZE_BYTES;
    let patched = patch_flash_nvram_slice(
        &mock_flash,
        0x100,
        b"HITA",
        "memory://src.bin",
        "memory://patched.bin",
        "memory://bak.bin",
    )
    .expect("Guarded NVRAM patch within verified EH55 top 64 KB must succeed");
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
    eh55_seed(&mut mock_flash);
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

    // Layer 2 Upper NOR App/BGO (0x040000..0x7EFFFF) allowed with 64 KiB sector backup on EH55
    let out = patch_flash_upper_nor_slice(
        &mock_flash,
        0x0020_0100,
        b"BGO!",
        false,
        "memory://in.bin",
        "memory://out.bin",
        "memory://bak.bin",
    )
    .expect("Upper NOR BGO patch on verified EH55 must succeed");
    assert_eq!(&out.patched_bytes[0x0020_0100..0x0020_0104], b"BGO!");
    assert_eq!(
        out.backup_sector_bytes.len(),
        SPANSION_MAIN_SECTOR_SIZE_BYTES
    );
    assert_eq!(&out.backup_sector_bytes[0x100..0x104], b"ORIG");
    assert_eq!(out.report.first_sector_name, "SA39");
    assert_eq!(out.report.last_sector_name, "SA39");
    assert!(!out.report.critical_ipl_override_used);
    assert!(out.report.critical_ipl_untouched);
    assert!(out.report.nvram_untouched);
    assert_ne!(out.report.original_sha256, out.report.patched_sha256);
    assert_ne!(
        out.report.patched_range_before_sha256,
        out.report.patched_range_after_sha256
    );

    // Explicit offline override allows patching SA0..SA10 while keeping NVRAM intact
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

    // Fail-closed: unidentified 4 MiB dump without LabOverride is rejected
    assert!(
        patch_flash_upper_nor_file(
            &in_path,
            &out_path,
            Some(&bak_path),
            0x0006_0020,
            b"ABCD",
            false
        )
        .is_err()
    );

    // Reject in-place overwrite of input even with LabOverride
    let ov = demo_override();
    let ov_policy = NvramPolicyOptions {
        model_id: None,
        lab_override: Some(&ov),
    };
    assert!(
        patch_flash_upper_nor_file_with_policy(
            &in_path,
            &in_path,
            Some(&bak_path),
            0x0006_0020,
            b"ABCD",
            false,
            &ov_policy,
        )
        .is_err()
    );

    // Explicit LabOverride allows offline CoW Upper NOR experiment using real S29JL032H SA6 geometry
    let report = patch_flash_upper_nor_file_with_policy(
        &in_path,
        &out_path,
        Some(&bak_path),
        0x0006_0020,
        b"ABCD",
        false,
        &ov_policy,
    )
    .expect("Upper NOR CoW file patch with LabOverride must succeed");
    assert_eq!(report.total_flash_bytes, 4 * 1024 * 1024);
    assert_eq!(report.first_sector_name, "SA6");
    assert!(report.critical_ipl_untouched);
    assert!(report.nvram_untouched);

    let out_bytes = std::fs::read(&out_path).unwrap();
    assert_eq!(&out_bytes[0x0006_0020..0x0006_0024], b"ABCD");
    let bak_bytes = std::fs::read(&bak_path).unwrap();
    assert_eq!(bak_bytes.len(), SPANSION_MAIN_SECTOR_SIZE_BYTES);
    assert_eq!(&bak_bytes[0x20..0x24], &[0x10, 0x20, 0x30, 0x40]);
    assert_eq!(std::fs::read(&in_path).unwrap(), img);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn policy_denies_size_only_ez485v_es10_and_es35v_upper_nor_but_allows_lab_override_on_synthetic() {
    let ov = demo_override();

    // 1. Size-only 4 MiB / 8 MiB dumps are DENIED without LabOverride
    for size in [4 * 1024 * 1024, 8 * 1024 * 1024] {
        let blank = vec![0x55u8; size];
        assert!(clean_flash_nvram_slice(&blank, "i", "o", "b").is_err());
    }

    // 2. DMR-EZ485V (VxWorks / non-EEMUL) is DENIED even WITH LabOverride
    let mut ez485 = vec![0xFFu8; 4 * 1024 * 1024];
    ez485[0x0000_40D6..0x0000_40DE].copy_from_slice(b"ZEBRA 64");
    assert!(
        clean_flash_nvram_slice_with_policy(
            &ez485,
            &NvramPolicyOptions {
                model_id: Some("DMR-EZ485V"),
                lab_override: Some(&ov),
            },
            SliceIoLabels::new("i", "o", "b"),
        )
        .is_err()
    );

    // 3. DMR-ES10 (1 MiB OTP PROM) is DENIED even WITH LabOverride
    let es10 = vec![0xFFu8; 1024 * 1024];
    assert!(
        clean_flash_nvram_slice_with_policy(
            &es10,
            &NvramPolicyOptions {
                model_id: Some("DMR-ES10"),
                lab_override: Some(&ov),
            },
            SliceIoLabels::new("i", "o", "b"),
        )
        .is_err()
    );

    // 4. DMR-ES35V allows C1 NVRAM CoW when 0xF1 is present, but DENIES Upper NOR CoW (MAIN_XIP!)
    let mut es35 = vec![0xFFu8; 8 * 1024 * 1024];
    plant(&mut es35, 0x0003_F000, b"Loader ver0.18 [06/03/13]");
    plant(
        &mut es35,
        0x007F_2000,
        &[0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0xF1, 0xFF, 0xFF],
    );
    assert!(clean_flash_nvram_slice(&es35, "i", "o", "b").is_ok());
    assert!(
        patch_flash_upper_nor_slice(&es35, 0x0020_0000, b"TEST", false, "i", "o", "b").is_err(),
        "Upper NOR write into ES35V MAIN_XIP must fail closed"
    );

    // 5. Partial NVRAM capture indices use exact derived sector numbers (`SA31..SA34`, `SA63..SA70`, `SA127`)
    let r2m = inspect_partial_nvram_capture(&vec![0xFFu8; 64 * 1024], 0x801F_0000, "2m").unwrap();
    assert_eq!(r2m.microsectors.len(), 4);
    assert_eq!(r2m.microsectors[0].sector_name, "SA31");
    assert_eq!(r2m.microsectors[3].sector_name, "SA34");

    let r4m = inspect_partial_nvram_capture(&vec![0xFFu8; 64 * 1024], 0x803F_0000, "4m").unwrap();
    assert_eq!(r4m.microsectors.len(), 8);
    assert_eq!(r4m.microsectors[0].sector_name, "SA63");
    assert_eq!(r4m.microsectors[7].sector_name, "SA70");

    let r16m =
        inspect_partial_nvram_capture(&vec![0xFFu8; 128 * 1024], 0x80FE_0000, "16m").unwrap();
    assert_eq!(r16m.microsectors.len(), 1);
    assert_eq!(r16m.microsectors[0].sector_name, "SA127");
}
