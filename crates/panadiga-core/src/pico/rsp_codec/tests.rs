//! Unit tests for `pico::rsp_codec` guards, binary `$x` escaping, and register decoding.

use super::*;
use crate::constants::{AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_UNCACHED_BASE};
use policy::is_nor_write_blocked_for_flash;

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

#[test]
fn policy_boundaries_match_c11_bus_gate_across_all_capacities() {
    let capacities = [
        2u64 * 1024 * 1024,
        4 * 1024 * 1024,
        8 * 1024 * 1024,
        16 * 1024 * 1024,
    ];
    for flash_bytes in capacities {
        let nvram_sz: u32 = if flash_bytes >= 16 * 1024 * 1024 {
            128 * 1024
        } else {
            64 * 1024
        };
        let flash_u32 = flash_bytes as u32;
        let nvram_off = flash_u32 - nvram_sz;
        for base in [AM34_FLASH_BASE, AM34_FLASH_UNCACHED_BASE] {
            // Permanent IPL Layer 1 (256 KiB) always blocked regardless of policy
            assert!(is_nor_write_blocked_for_flash(
                base,
                4,
                flash_bytes,
                NorWritePolicy::UpperNorAppUnlocked
            ));
            assert!(is_nor_write_blocked_for_flash(
                base + AM34_FLASH_CRITICAL_IPL_SIZE - 2,
                4,
                flash_bytes,
                NorWritePolicy::UpperNorAppUnlocked
            ));
            // Upper NOR boundary blocked in NvramOnly, allowed when UpperNorAppUnlocked
            assert!(is_nor_write_blocked_for_flash(
                base + AM34_FLASH_CRITICAL_IPL_SIZE,
                4,
                flash_bytes,
                NorWritePolicy::NvramOnly
            ));
            assert!(!is_nor_write_blocked_for_flash(
                base + AM34_FLASH_CRITICAL_IPL_SIZE,
                4,
                flash_bytes,
                NorWritePolicy::UpperNorAppUnlocked
            ));
            // Exact byte before NVRAM straddling into NVRAM must be blocked under NvramOnly
            assert!(is_nor_write_blocked_for_flash(
                base + nvram_off - 1,
                2,
                flash_bytes,
                NorWritePolicy::NvramOnly
            ));
            // NVRAM window allowed even under NvramOnly
            assert!(!is_nor_write_blocked_for_flash(
                base + nvram_off,
                4,
                flash_bytes,
                NorWritePolicy::NvramOnly
            ));
            // End-of-flash straddle into CS0 alias mirror MUST be blocked even unlocked
            assert!(is_nor_write_blocked_for_flash(
                base + flash_u32 - 2,
                4,
                flash_bytes,
                NorWritePolicy::UpperNorAppUnlocked
            ));
            // CS0 alias mirror beyond flash_bytes MUST be blocked
            assert!(is_nor_write_blocked_for_flash(
                base + flash_u32,
                4,
                flash_bytes,
                NorWritePolicy::UpperNorAppUnlocked
            ));
        }
        // Lethal MMIO 0xC000100C..0xC000100F always blocked even with UpperNorAppUnlocked
        for addr in [0xC000_100Au32, 0xC000_100C, 0xC000_100F] {
            assert!(is_nor_write_blocked_for_flash(
                addr,
                4,
                flash_bytes,
                NorWritePolicy::UpperNorAppUnlocked
            ));
        }
        // SDRAM target RAM outside CS0 / lethal MMIO allowed
        assert!(!is_nor_write_blocked_for_flash(
            0x9040_0000,
            4,
            flash_bytes,
            NorWritePolicy::NvramOnly
        ));
        assert!(!is_nor_write_blocked_for_flash(
            0x0000_0000,
            4,
            flash_bytes,
            NorWritePolicy::NvramOnly
        ));
    }
}

#[test]
fn loopback_socket_validation_and_host_rsp_filter_enforce_barriers() {
    assert!(parse_loopback_socket_addr("127.0.0.1:3333").is_ok());
    assert!(parse_loopback_socket_addr("[::1]:3333").is_ok());
    assert!(parse_loopback_socket_addr("0.0.0.0:3333").is_err());
    assert!(parse_loopback_socket_addr("192.168.1.10:3333").is_err());
    assert!(parse_loopback_socket_addr("[::]:3333").is_err());

    let mut pol = NorWritePolicy::NvramOnly;
    // Lethal MMIO read/write blocked
    assert!(filter_host_rsp_packet(b"mC000100C,4", &mut pol).is_some());
    assert!(filter_host_rsp_packet(b"MC000100C,4:00000000", &mut pol).is_some());
    // Layer 1 IPL blocked permanently; Upper NOR blocked until explicit unlock
    assert!(filter_host_rsp_packet(b"M84000000,4:aaaaaaaa", &mut pol).is_some());
    assert!(filter_host_rsp_packet(b"M84130000,4:aaaaaaaa", &mut pol).is_some());
    assert!(filter_host_rsp_packet(b"vFlashErase:84000000,10000", &mut pol).is_some());
    // NVRAM allowed in default policy
    assert!(filter_host_rsp_packet(b"M847F0000,4:ffffffff", &mut pol).is_none());

    // Unlock permits Upper NOR but keeps SA0..SA10 and lethal MMIO blocked
    assert!(filter_host_rsp_packet(b"qPanadigaNorUnlock:CONFIRM_UPPER_NOR", &mut pol).is_none());
    assert_eq!(pol, NorWritePolicy::UpperNorAppUnlocked);
    assert!(filter_host_rsp_packet(b"M84130000,4:aaaaaaaa", &mut pol).is_none());
    assert!(filter_host_rsp_packet(b"M84000000,4:aaaaaaaa", &mut pol).is_some());
    assert!(filter_host_rsp_packet(b"qPanadigaNorLock", &mut pol).is_none());
    assert_eq!(pol, NorWritePolicy::NvramOnly);

    // Split chunk streaming holds incomplete frame and strips blocked frame cleanly
    let mut acc = Vec::new();
    let r1 = filter_host_rsp_tcp_stream(&mut acc, b"+$mC000100C,", &mut pol);
    assert_eq!(r1.forward_to_serial, b"+");
    assert!(r1.blocked_host_replies.is_empty());
    let r2 = filter_host_rsp_tcp_stream(&mut acc, b"4#aa$g#67", &mut pol);
    assert_eq!(r2.forward_to_serial, b"$g#67");
    assert_eq!(r2.blocked_host_replies, b"+$E05#aa");
}
