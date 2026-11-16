//! Unit tests for `pico::rsp_codec` guards, binary `$x` escaping, and register decoding.

use super::*;

#[test]
fn rsp_guard_binary_x_and_register_decoding_work() {
    assert!(is_ipl_readonly_address(0x8400_0000, 4));
    assert!(!is_ipl_readonly_address(0x847F_0000, 4));
    assert!(build_rsp_frame(b"M51800000,4:01020304", false).is_err());
    assert!(build_rsp_write_frame(0x8000_0000, &[0, 1], false).is_err());
    assert!(build_rsp_write_frame(0x847E_FFF8, &[0; 16], false).is_err());

    let x_frame = build_rsp_write_frame(0x847F_0000, b"}#$*", true).unwrap();
    let parsed = parse_rsp_reply(&x_frame, None).unwrap();
    assert!(parsed.checksum_ok);
    let decoded = hex::decode(&parsed.payload_hex).unwrap();
    assert!(decoded.starts_with(b"X847f0000,4:"));

    let mut raw_g = Vec::with_capacity(G_REGISTER_BYTES);
    for i in 0..28u32 {
        let v = match i {
            25 => 0xAA00_0064,
            26 => 0xBB00_0068,
            27 => 0xCC00_006C,
            _ => 0x1000 + i,
        };
        raw_g.extend_from_slice(&v.to_le_bytes());
    }
    let regs = decode_mn103s_g_registers(hex::encode(raw_g).as_bytes()).unwrap();
    assert_eq!(regs.get("w25"), Some(&0xAA00_0064));
    assert_eq!(regs.get("w27"), Some(&0xCC00_006C));
    assert!(decode_mn103s_g_registers(b"0000").is_err());
}

#[test]
fn nor_write_policy_unlocks_upper_app_while_keeping_critical_ipl_locked() {
    assert_eq!(NorWritePolicy::default(), NorWritePolicy::NvramOnly);
    assert_eq!(
        CRITICAL_IPL_BOOT_RANGES,
        [(0x8000_0000, 0x8004_0000), (0x8400_0000, 0x8404_0000)]
    );

    for &(addr, len) in &[
        (0x8000_0000u32, 4usize),
        (0x8003_FFFC, 4),
        (0x8003_FFFE, 4),
        (0x8400_0000, 4),
        (0x8403_FFFF, 1),
    ] {
        assert!(is_critical_ipl_boot_address(addr, len));
        assert!(is_nor_write_blocked_with_policy(
            addr,
            len,
            NorWritePolicy::NvramOnly
        ));
        assert!(is_nor_write_blocked_with_policy(
            addr,
            len,
            NorWritePolicy::UpperNorAppUnlocked
        ));
        assert!(
            build_rsp_write_frame_with_policy(
                addr,
                &[0x11, 0x22, 0x33, 0x44][..len],
                false,
                NorWritePolicy::UpperNorAppUnlocked
            )
            .is_err()
        );
    }

    for &upper_addr in &[0x8004_0000u32, 0x8420_0000, 0x8440_0000, 0x807E_FFFC] {
        assert!(!is_critical_ipl_boot_address(upper_addr, 4));
        assert!(is_nor_write_blocked_with_policy(
            upper_addr,
            4,
            NorWritePolicy::NvramOnly
        ));
        assert!(!is_nor_write_blocked_with_policy(
            upper_addr,
            4,
            NorWritePolicy::UpperNorAppUnlocked
        ));
        assert!(
            build_rsp_write_frame_with_policy(
                upper_addr,
                b"\xDE\xAD\xBE\xEF",
                false,
                NorWritePolicy::NvramOnly
            )
            .is_err()
        );
        let unlocked_frame = build_rsp_write_frame_with_policy(
            upper_addr,
            b"\xDE\xAD\xBE\xEF",
            false,
            NorWritePolicy::UpperNorAppUnlocked,
        )
        .unwrap();
        let parsed = parse_rsp_reply(&unlocked_frame, None).unwrap();
        assert!(parsed.checksum_ok);
    }

    let unlock = build_nor_unlock_frame();
    let unlock_parsed = parse_rsp_reply(&unlock, None).unwrap();
    assert!(unlock_parsed.checksum_ok);
    assert_eq!(
        hex::decode(&unlock_parsed.payload_hex).unwrap(),
        b"qPanadigaNorUnlock:CONFIRM_UPPER_NOR"
    );

    let lock = build_nor_lock_frame();
    let lock_parsed = parse_rsp_reply(&lock, None).unwrap();
    assert!(lock_parsed.checksum_ok);
    assert_eq!(
        hex::decode(&lock_parsed.payload_hex).unwrap(),
        b"qPanadigaNorLock"
    );
}
