use super::*;

#[test]
fn embedded_firmware_error_dictionary_contains_all_431_drv_plus_core_records() {
    let all = all_firmware_errors();
    let drv_count = all
        .iter()
        .filter(|e| e.domain == FirmwareErrorDomain::DrvMecha)
        .count();
    assert_eq!(drv_count, 431, "Must embed all 431 DRV_decomp records");
    assert!(all.len() >= 447);

    let laser = lookup_firmware_errors("0x83A7");
    assert!(!laser.is_empty());
    assert!(laser[0].translation.contains("0.5mW"));

    let boot_apc = lookup_firmware_errors("80040904");
    assert!(boot_apc.iter().any(|e| e.code == "ODD_80040904"));

    let u99 = lookup_firmware_errors("U99");
    assert_eq!(u99[0].code, "U99");

    let fat = lookup_firmware_errors("-10025");
    assert_eq!(fat[0].code, "FAT_10025");
}

#[test]
fn correlates_boot_logs_and_standby_drop_with_electrical_cues() {
    let sample_uart =
        "Loader ver0.61 [06/03/06]\nBGO driver init failed[1].\nMain hung-up time changed 255[s].";
    let rep =
        correlate_firmware_occurrences(sample_uart, Some("U99"), Some(3.25), Some(1), Some(420));
    assert!(rep.matches.iter().any(|m| m.code == "BGO_INIT_FAIL_1"));
    assert!(rep.matches.iter().any(|m| m.code == "U99"));
    assert!(!rep.prioritized_actions.is_empty());

    let silent_drop = correlate_firmware_occurrences(
        "",
        Some("HELLO -> PLEASE WAIT (2s) -> U99"),
        Some(0.05),
        Some(2),
        Some(0),
    );
    assert!(
        silent_drop
            .matches
            .iter()
            .any(|m| m.code == "HELLO_PLEASE_WAIT_U99_2S")
    );
    assert!(
        silent_drop
            .prioritized_actions
            .iter()
            .any(|a| a.contains("0 edges"))
    );
}
