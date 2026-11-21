#![deny(unsafe_code)]

use std::fs;

use super::super::codec::test_helpers::build_synthetic_hdd_cmd_main;
use super::super::specs::MAIN_DEFAULT_BASE_VA;
use super::*;
use crate::constants::PANA_EXTRHEAD_DEFAULT_ADLER_STEP;
use crate::crypto::{ExtrheadCryptMode, pack_extrhead};

#[test]
fn test_extrhead_container_roundtrip_capacity_patch() {
    let main = build_synthetic_hdd_cmd_main();
    let packed = pack_extrhead(
        &main,
        "MAIN",
        MAIN_DEFAULT_BASE_VA,
        "TEST3410PQ",
        ExtrheadCryptMode::Main,
        PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        0x0101,
    )
    .expect("Pack synthetic EXTRHEAD");

    let tmp = std::env::temp_dir().join("panadiga_cap_test");
    let _ = fs::create_dir_all(&tmp);
    let in_file = tmp.join("test_p1_in.bin");
    let out_file = tmp.join("test_p1_out.bin");
    fs::write(&in_file, &packed).expect("Write input");

    let report = patch_main_hdd_capacity(&in_file, &out_file, "dynamic", None)
        .expect("Patch EXTRHEAD container");
    assert_eq!(report.container_format, "EXTRHEAD_CONTAINER");
    assert!(report.inspection_after.is_dynamic_unclamped);

    let inspect_out = inspect_main_hdd_capacity(&out_file).expect("Inspect output EXTRHEAD");
    assert!(inspect_out.is_dynamic_unclamped);
    let _ = fs::remove_dir_all(&tmp);
}
