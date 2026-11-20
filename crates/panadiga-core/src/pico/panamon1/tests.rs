#![deny(unsafe_code)]

use std::time::Duration;

use super::*;

#[test]
fn ping_uses_question_mark_cr_never_bare_cr_and_parses_read_hex() {
    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    mock.map_memory(0x847F_0010, vec![0xAA, 0x55, 0xFF, 0xFF]);

    let mut client = Panamon1Client::new(mock);
    client.ping().unwrap();
    assert_eq!(client.transport().tx_target_log, vec!["?"]);
    assert_eq!(client.mem_read_u32(0x847F_0010).unwrap(), 0xFFFF_55AA);
    assert!(parse_panamon1_read_bytes("R 847F0010,4\r\nAA55\r\n", 4).is_none());
}

#[test]
fn write_burst_retries_once_after_300us_on_syntax_err_and_settles_before_jump() {
    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    mock.simulate_first_write_syntax_err = true;
    mock.map_memory(0x9050_0000, vec![0u8; 64]);

    let mut client = Panamon1Client::new(mock);
    client
        .mem_write_bytes_burst(0x9050_0000, &[0xF0, 0xFC])
        .unwrap();
    client.jump(0x9050_0000).unwrap();

    let tr = client.transport();
    assert_eq!(
        tr.tx_target_log,
        vec![
            "W 90500000,F0",
            "W 90500000,F0",
            "W 90500001,FC",
            "J 90500000"
        ]
    );
    assert!(tr.recorded_sleeps.contains(&Duration::from_micros(300)));
    assert!(tr.recorded_sleeps.contains(&Duration::from_millis(3)));
}

#[test]
fn pincheck_failure_is_rejected_fail_closed() {
    let mut mock = MockPanamon1Transport::new();
    mock.pincheck_reply = "ERR:GP3_NSW5V_UNSAFE\r\nPANADIGA> ".to_string();
    let mut client = Panamon1Client::new(mock);
    let err = client.ensure_pico_cli().unwrap_err().to_string();
    assert!(err.contains("PINCHECK failed"), "err={err}");
    assert!(client.transport().tx_cli_log.contains(&"~~~".to_string()));
}

#[test]
fn upper_nor_guard_blocks_sa0_sa3_with_e05_and_drop_always_emits_u_lock() {
    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    mock.map_memory(0x8400_0000, vec![0x5Au8; 0x80_0000]);

    let mut client = Panamon1Client::new(mock);
    let sa0_err = client
        .unlock_upper_nor(&[(0x8003_FFFF, 1)])
        .err()
        .unwrap()
        .to_string();
    assert!(sa0_err.contains("E05"), "err={sa0_err}");
    assert!(
        !client
            .transport()
            .tx_target_log
            .contains(&"U CONFIRM_UPPER_NOR".to_string())
    );

    {
        let mut guard = client.unlock_upper_nor(&[(0x8420_0000, 1)]).unwrap();
        let err_erase = guard.erase_sector(0x8400_0000).unwrap_err().to_string();
        assert!(err_erase.contains("E05"), "err={err_erase}");
        let err_write = guard
            .write_nor_bytes(0x8000_0100, &[0x00])
            .unwrap_err()
            .to_string();
        assert!(err_write.contains("E05"), "err={err_write}");
    }

    let log = &client.transport().tx_target_log;
    assert_eq!(log.first().map(String::as_str), Some("U CONFIRM_UPPER_NOR"));
    assert_eq!(log.last().map(String::as_str), Some("U LOCK"));
    assert!(!client.transport().nor_unlocked);
}
