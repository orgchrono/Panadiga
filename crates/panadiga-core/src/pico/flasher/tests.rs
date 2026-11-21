use std::fs;

use super::*;

#[test]
fn test_uf2_buffer_validation_valid() {
    let mut block = vec![0u8; 512];
    block[0..4].copy_from_slice(&UF2_MAGIC_START0.to_le_bytes());
    block[4..8].copy_from_slice(&UF2_MAGIC_START1.to_le_bytes());
    block[28..32].copy_from_slice(&RP2040_FAMILY_ID.to_le_bytes());
    block[508..512].copy_from_slice(&UF2_MAGIC_END.to_le_bytes());

    let res = validate_uf2_buffer(&block);
    assert!(res.is_ok());
    let (blocks, family) = res.unwrap();
    assert_eq!(blocks, 1);
    assert!(family.contains("RP2040"));

    block[28..32].copy_from_slice(&RP2040_SDK_FAMILY_ID.to_le_bytes());
    let res_sdk_2040 = validate_uf2_buffer(&block).unwrap();
    assert!(res_sdk_2040.1.contains("RP2040"));

    block[28..32].copy_from_slice(&RP2350_SDK_FAMILY_ID.to_le_bytes());
    let res_sdk_2350 = validate_uf2_buffer(&block).unwrap();
    assert!(res_sdk_2350.1.contains("RP2350"));
}

#[test]
fn test_uf2_buffer_validation_invalid_magic() {
    let block = vec![0u8; 512];
    let res = validate_uf2_buffer(&block);
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Corrupt UF2 block"));
}

#[test]
fn test_uf2_buffer_validation_invalid_length() {
    let block = vec![0u8; 400];
    let res = validate_uf2_buffer(&block);
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("multiple of 512"));
}

#[test]
fn test_embedded_uf2_containers_and_unified_layout() {
    let rp2040_slice = extract_uf2_family_slice(EMBEDDED_UNIFIED_UF2, Uf2TargetFamily::Rp2040)
        .expect("RP2040 slice extraction valid");
    let (b2040, f2040) = validate_uf2_buffer(&rp2040_slice).expect("RP2040 UF2 slice valid");
    assert!(b2040 > 0);
    assert!(f2040.contains("RP2040"));
    assert_eq!(
        u32::from_le_bytes(rp2040_slice[20..24].try_into().unwrap()),
        0
    );
    assert_eq!(
        u32::from_le_bytes(rp2040_slice[24..28].try_into().unwrap()),
        b2040 as u32
    );

    let rp2350_slice = extract_uf2_family_slice(EMBEDDED_UNIFIED_UF2, Uf2TargetFamily::Rp2350)
        .expect("RP2350 slice extraction valid");
    let (b2350, f2350) = validate_uf2_buffer(&rp2350_slice).expect("RP2350 UF2 slice valid");
    assert!(b2350 > 0);
    assert!(f2350.contains("RP2350"));
    assert_eq!(
        u32::from_le_bytes(rp2350_slice[20..24].try_into().unwrap()),
        0
    );
    assert_eq!(
        u32::from_le_bytes(rp2350_slice[24..28].try_into().unwrap()),
        b2350 as u32
    );

    assert_eq!(
        Uf2TargetFamily::detect("Model: Raspberry Pi Pico 2\n", None),
        Some(Uf2TargetFamily::Rp2350)
    );
    assert_eq!(
        Uf2TargetFamily::detect("Board-ID: RPI-RP2\n", None),
        Some(Uf2TargetFamily::Rp2040)
    );
    assert_eq!(
        Uf2TargetFamily::detect("Board-ID: RPI-RP2\n", Some("unified")),
        None
    );

    let (bunified, funified) =
        validate_uf2_buffer(EMBEDDED_UNIFIED_UF2).expect("Unified UF2 valid");
    assert_eq!(bunified, b2040 + b2350);
    assert!(funified.contains("Unified Multi-Family"));

    let blk0_family = u32::from_le_bytes(EMBEDDED_UNIFIED_UF2[28..32].try_into().unwrap());
    assert_eq!(blk0_family, RP2350_ABS_SDK_FAMILY_ID);

    let tail_start = (bunified - 2) * UF2_BLOCK_SIZE;
    let second_last_fid = u32::from_le_bytes(
        EMBEDDED_UNIFIED_UF2[tail_start + 28..tail_start + 32]
            .try_into()
            .unwrap(),
    );
    let last_fid = u32::from_le_bytes(
        EMBEDDED_UNIFIED_UF2[tail_start + UF2_BLOCK_SIZE + 28..tail_start + UF2_BLOCK_SIZE + 32]
            .try_into()
            .unwrap(),
    );
    assert_eq!(second_last_fid, RP2040_SDK_FAMILY_ID);
    assert_eq!(last_fid, RP2350_ARM_S_SDK_FAMILY_ID);
}

fn make_synthetic_uf2_block(
    family_id: u32,
    block_no: u32,
    payload_tag: u8,
) -> [u8; UF2_BLOCK_SIZE] {
    let mut block = [0u8; UF2_BLOCK_SIZE];
    block[0..4].copy_from_slice(&UF2_MAGIC_START0.to_le_bytes());
    block[4..8].copy_from_slice(&UF2_MAGIC_START1.to_le_bytes());
    block[8..12].copy_from_slice(&UF2_FLAG_FAMILY_ID_PRESENT.to_le_bytes());
    block[20..24].copy_from_slice(&block_no.to_le_bytes());
    block[28..32].copy_from_slice(&family_id.to_le_bytes());
    block[32] = payload_tag;
    block[508..512].copy_from_slice(&UF2_MAGIC_END.to_le_bytes());
    block
}

#[test]
fn test_merge_unified_uf2_roundtrip() {
    let dir = std::env::temp_dir().join(format!("panadiga_uf2_test_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let rp2040_path = dir.join("rp2040.uf2");
    let rp2350_path = dir.join("rp2350.uf2");
    let out_path = dir.join("unified.uf2");

    let mut rp2040 = Vec::new();
    rp2040.extend_from_slice(&make_synthetic_uf2_block(RP2040_SDK_FAMILY_ID, 0, 0x11));
    rp2040.extend_from_slice(&make_synthetic_uf2_block(RP2040_SDK_FAMILY_ID, 1, 0x12));
    fs::write(&rp2040_path, &rp2040).unwrap();

    let mut rp2350 = Vec::new();
    rp2350.extend_from_slice(&make_synthetic_uf2_block(RP2350_ABS_SDK_FAMILY_ID, 0, 0xA0));
    rp2350.extend_from_slice(&make_synthetic_uf2_block(
        RP2350_ARM_S_SDK_FAMILY_ID,
        0,
        0x21,
    ));
    rp2350.extend_from_slice(&make_synthetic_uf2_block(
        RP2350_ARM_S_SDK_FAMILY_ID,
        1,
        0x22,
    ));
    fs::write(&rp2350_path, &rp2350).unwrap();

    let (count, sha) = merge_unified_uf2(&rp2040_path, &rp2350_path, &out_path).unwrap();
    assert_eq!(count, 5);
    assert_eq!(sha.len(), 64);

    let merged = fs::read(&out_path).unwrap();
    let (validated_count, label) = validate_uf2_buffer(&merged).unwrap();
    assert_eq!(validated_count, 5);
    assert!(label.contains("Unified Multi-Family"));

    let tags: Vec<u8> = merged
        .as_chunks::<UF2_BLOCK_SIZE>()
        .0
        .iter()
        .map(|b| b[32])
        .collect();
    assert_eq!(tags, vec![0xA0, 0x11, 0x21, 0x12, 0x22]);

    let _ = fs::remove_dir_all(&dir);
}
