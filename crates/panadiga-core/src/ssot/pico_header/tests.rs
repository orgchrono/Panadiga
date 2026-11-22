use super::*;
use std::fs;
use std::path::Path;

fn assert_file_matches(rel: &str, expected: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    let on_disk = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Missing checked-in {}: {e}", path.display()));
    assert_eq!(
        on_disk.replace("\r\n", "\n"),
        expected,
        "{rel} is out of sync with Rust SSOT (run `panadiga ssot export-all`)"
    );
}

#[test]
fn generated_pico_ssot_files_match_checked_in_c_files() {
    let header = generate_c11_ssot_header();
    let tables = generate_c11_ssot_tables();
    let shim = generate_pana_rsp_ssot_header();

    assert_file_matches("firmware/pico-p7501/generated/p7501_ssot.h", &header);
    assert_file_matches("firmware/pico-p7501/generated/p7501_ssot_tables.c", &tables);
    assert_file_matches("firmware/pico-p7501/uart/p7501_pana_rsp_ssot.h", &shim);

    assert!(header.contains("0x80000000UL"));
    assert!(header.contains("0x8003FFFFUL"));
    assert!(header.contains("0x84000000UL"));
    assert!(header.contains("0x8403FFFFUL"));
    assert!(header.contains("0xC000100CUL"));
    assert!(header.contains("P7501_SSOT_DIGADIAG_LOAD_ADDR      0x50400000UL"));
    assert!(header.contains("P7501_SSOT_SDRAM_STUB_RAW_ADDR     0x51800000UL"));
    assert!(header.contains("0x00130000UL"));
    assert!(header.contains("45u"));
    assert!(header.contains("P7501_SSOT_VDD_NOMINAL_MV          3300u"));
    assert!(tables.contains("k_p7501_ssot_flash_chips"));
    assert!(tables.contains("k_p7501_ssot_target_profiles"));
    assert!(tables.contains("k_p7501_ssot_eeprom_chips"));
    assert!(!tables.contains("Loader ver0.63"));
    assert!(!tables.contains("VEP79142"));
    assert!(!tables.contains("VEP79179"));
    assert!(!tables.contains("VEP79115"));
    assert!(tables.contains("VEP79138"));
    assert!(tables.contains("VEP79104"));
}
