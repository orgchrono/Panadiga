#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use super::*;

#[test]
fn generated_digaos_ssot_matches_checked_in_c_and_ts_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let h_path = root.join("firmware/digaos/generated/digaos_ssot.h");
    let c_path = root.join("firmware/digaos/generated/digaos_ssot_tables.c");
    let ts_path = root.join("apps/desktop/src/models/generated/digaos.ts");

    let expected_h = generate_digaos_ssot_header();
    let expected_c = generate_digaos_ssot_tables();
    let expected_ts = generate_ts_digaos_module();

    if h_path.is_file() {
        let disk_h = fs::read_to_string(&h_path).expect("read digaos_ssot.h");
        assert_eq!(disk_h.replace("\r\n", "\n"), expected_h);
    }
    if c_path.is_file() {
        let disk_c = fs::read_to_string(&c_path).expect("read digaos_ssot_tables.c");
        assert_eq!(disk_c.replace("\r\n", "\n"), expected_c);
    }
    if ts_path.is_file() {
        let disk_ts = fs::read_to_string(&ts_path).expect("read digaos.ts");
        assert_eq!(disk_ts.replace("\r\n", "\n"), expected_ts);
    }

    assert!(expected_h.contains("NOT LEGALLY CLEAN-ROOM"));
    assert!(expected_h.contains("DIGAOS_PROMPT \"DIGAOS>\""));
    assert!(expected_h.contains(&format!("0x{DIGAOS_SSOT_CONTRACT_HASH:08X}UL")));
    assert!(expected_ts.contains(&format!("0x{DIGAOS_SSOT_CONTRACT_HASH:08X}")));
    assert!(expected_c.contains("k_digaos_ssot_menu_items"));
}
