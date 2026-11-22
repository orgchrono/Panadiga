use super::*;

fn sample_regs() -> Am34RegisterState {
    Am34RegisterState {
        d: [0x11223344, 0, 0, 0],
        a: [0, 0, 0, 0x0003FFF0],
        mdr: 0,
        psw: 0x00000100,
        pc: 0x80000000,
        sp: 0x0003FFF0,
        is_halted: true,
    }
}

#[test]
fn test_verify_and_classify_rsp_packet() {
    let (cmd, valid) = verify_and_classify_rsp_packet("$g#67");
    assert_eq!(cmd, "ReadRegs");
    assert!(valid);

    let (cmd_halt, valid_halt) = verify_and_classify_rsp_packet("$?#3f");
    assert_eq!(cmd_halt, "HaltReason");
    assert!(valid_halt);

    let (cmd_invalid, valid_invalid) = verify_and_classify_rsp_packet("$g#00");
    assert_eq!(cmd_invalid, "ReadRegs");
    assert!(!valid_invalid);

    let (cmd_ack, valid_ack) = verify_and_classify_rsp_packet("+");
    assert_eq!(cmd_ack, "AckNak");
    assert!(valid_ack);
}

#[test]
fn test_encode_and_decode_registers_roundtrip() {
    let regs = sample_regs();
    let hex = encode_am34_registers_hex(&regs);
    assert_eq!(hex.len(), 96);
    let decoded = decode_am34_registers_hex(&hex, true).expect("should decode");
    assert_eq!(decoded.d[0], 0x11223344);
    assert_eq!(decoded.pc, 0x80000000);
    assert_eq!(decoded.sp, 0x0003FFF0);
}

#[test]
fn test_mock_rsp_memory_map_and_ipl_protection() {
    let regs = sample_regs();
    let map_resp = String::from_utf8(compute_mock_rsp_response_pure(
        "qXfer:memory-map:read::0,1ff",
        &regs,
    ))
    .unwrap();
    assert!(map_resp.starts_with("+$l<memory-map>"));
    assert!(map_resp.contains("start=\"0x80000000\""));
    let (_, valid) = verify_and_classify_rsp_packet(&map_resp);
    assert!(valid);

    let mem_resp = String::from_utf8(compute_mock_rsp_response_pure("m807f0000,4", &regs)).unwrap();
    assert!(mem_resp.starts_with("+$ffffffff#"));
    let (_, valid_mem) = verify_and_classify_rsp_packet(&mem_resp);
    assert!(valid_mem);

    let ipl_write = String::from_utf8(compute_mock_rsp_response_pure(
        "M80000000,4:deadbeef",
        &regs,
    ))
    .unwrap();
    assert!(ipl_write.starts_with("+$E05#"));

    let sram_write = String::from_utf8(compute_mock_rsp_response_pure(
        "M00004000,4:deadbeef",
        &regs,
    ))
    .unwrap();
    assert!(sram_write.starts_with("+$OK#"));

    let straddle_write = String::from_utf8(compute_mock_rsp_response_pure(
        "M7ffffffe,4:deadbeef",
        &regs,
    ))
    .unwrap();
    assert!(straddle_write.starts_with("+$E05#"));

    let lethal_read =
        String::from_utf8(compute_mock_rsp_response_pure("mc000100c,4", &regs)).unwrap();
    assert!(lethal_read.starts_with("+$E05#"));

    let uncached_ipl_read =
        String::from_utf8(compute_mock_rsp_response_pure("m84000000,4", &regs)).unwrap();
    let cached_ipl_read =
        String::from_utf8(compute_mock_rsp_response_pure("m80000000,4", &regs)).unwrap();
    assert_eq!(uncached_ipl_read, cached_ipl_read);

    let uncached_nvram_read =
        String::from_utf8(compute_mock_rsp_response_pure("m847f0000,4", &regs)).unwrap();
    assert!(uncached_nvram_read.starts_with("+$ffffffff#"));
}

#[test]
fn test_decode_224_char_dev_serial_registers_and_nor_unlock_policy() {
    let mut regs_hex_224 = encode_am34_registers_hex(&sample_regs());
    regs_hex_224.push_str(&"0".repeat(224 - 96));
    let decoded = decode_am34_registers_hex(&regs_hex_224, true).expect("224-char $g");
    assert_eq!(decoded.d[0], 0x11223344);
    assert_eq!(decoded.pc, 0x80000000);
    assert!(decode_am34_registers_hex(&"0".repeat(100), true).is_none());

    let regs = sample_regs();
    let mut policy = NorWritePolicy::NvramOnly;
    let locked_upper = String::from_utf8(compute_mock_rsp_response_with_policy(
        "X84040000,4:abcd",
        &regs,
        &mut policy,
    ))
    .unwrap();
    assert!(locked_upper.starts_with("+$E05#"));

    let bad_unlock = String::from_utf8(compute_mock_rsp_response_with_policy(
        "qPanadigaNorUnlock:NOPE",
        &regs,
        &mut policy,
    ))
    .unwrap();
    assert!(bad_unlock.starts_with("+$E01#"));

    let unlocked = String::from_utf8(compute_mock_rsp_response_with_policy(
        "qPanadigaNorUnlock:CONFIRM_UPPER_NOR",
        &regs,
        &mut policy,
    ))
    .unwrap();
    assert!(unlocked.starts_with("+$OK#"));

    let upper_ok = String::from_utf8(compute_mock_rsp_response_with_policy(
        "X84040000,4:abcd",
        &regs,
        &mut policy,
    ))
    .unwrap();
    assert!(upper_ok.starts_with("+$OK#"));

    let sa0_ro = String::from_utf8(compute_mock_rsp_response_with_policy(
        "M84000000,4:deadbeef",
        &regs,
        &mut policy,
    ))
    .unwrap();
    assert!(sa0_ro.starts_with("+$E05#"));

    let relocked = String::from_utf8(compute_mock_rsp_response_with_policy(
        "qPanadigaNorLock",
        &regs,
        &mut policy,
    ))
    .unwrap();
    assert!(relocked.starts_with("+$OK#"));

    let upper_ro_again = String::from_utf8(compute_mock_rsp_response_with_policy(
        "M84040000,4:deadbeef",
        &regs,
        &mut policy,
    ))
    .unwrap();
    assert!(upper_ro_again.starts_with("+$E05#"));
}
