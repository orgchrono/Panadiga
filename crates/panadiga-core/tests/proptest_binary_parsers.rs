#![forbid(unsafe_code)]

use proptest::prelude::*;

use panadiga_core::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_UNCACHED_BASE, MEIHDFS_RECORD_SIZE,
};
use panadiga_core::hdd::{
    MeihdfsError, decode_journal_blocks_slice, decode_meihdfs_slice, decode_single_meihdfs_record,
    try_decode_single_meihdfs_record,
};
use panadiga_core::pico::{
    NorWritePolicy, Uf2Error, Uf2TargetFamily, build_rsp_frame, decode_rsp_hex_memory,
    extract_uf2_family_slice, filter_host_rsp_packet, is_nor_write_blocked_with_policy,
    parse_rsp_reply, validate_uf2_buffer,
};
use panadiga_core::re::{
    audit_eh55_bgo_nor_8mb, eval_bgo_migration_index, eval_bgo_migration_target,
    is_active_bgo_p1_sector, parse_eh55_bgo_l2p,
};

fn cfg() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(cfg())]

    #[test]
    fn uf2_hostile_bytes_never_panic_and_reject_misaligned(
        raw in proptest::collection::vec(any::<u8>(), 0..=2048),
    ) {
        let validation = validate_uf2_buffer(&raw);
        let _ = extract_uf2_family_slice(&raw, Uf2TargetFamily::Rp2040);
        if raw.len() < 512 || !raw.len().is_multiple_of(512) {
            let err = validation.expect_err("misaligned or short UF2 must fail");
            prop_assert_eq!(
                err.downcast_ref::<Uf2Error>(),
                Some(&Uf2Error::InvalidLength { len: raw.len() })
            );
        }
    }

    #[test]
    fn meihdfs_hostile_bytes_never_panic(
        raw in proptest::collection::vec(any::<u8>(), 0..=4096),
        rec_index in 0usize..=255,
    ) {
        let _ = decode_single_meihdfs_record(&raw, rec_index);
        let try_rec = try_decode_single_meihdfs_record(&raw, rec_index);
        if raw.len() < MEIHDFS_RECORD_SIZE {
            prop_assert_eq!(
                try_rec.expect_err("short descriptor must error"),
                MeihdfsError::RecordTooShort {
                    actual: raw.len(),
                    expected: MEIHDFS_RECORD_SIZE,
                }
            );
        } else {
            prop_assert!(try_rec.is_ok());
        }
        let _ = decode_meihdfs_slice(&raw, true, "proptest");
        let _ = decode_journal_blocks_slice(&raw);
    }

    #[test]
    fn rsp_framing_roundtrip_and_ipl_lockout_invariants(
        safe_cmd in "[gqvm][a-zA-Z0-9,:=]{0,95}",
        hostile_bytes in proptest::collection::vec(any::<u8>(), 0..=512),
        ipl_offset in 0u32..AM34_FLASH_CRITICAL_IPL_SIZE,
        write_len in 1usize..=64,
    ) {
        let frame = build_rsp_frame(safe_cmd.as_bytes(), false).expect("safe RSP encode");
        let parsed = parse_rsp_reply(&frame, None).expect("encoded RSP frame must parse");
        prop_assert!(parsed.checksum_ok);
        prop_assert_eq!(
            hex::decode(&parsed.payload_hex).expect("valid hex"),
            safe_cmd.as_bytes()
        );

        let _ = parse_rsp_reply(&hostile_bytes, None);
        let _ = decode_rsp_hex_memory(&hostile_bytes);
        let mut policy = NorWritePolicy::NvramOnly;
        let _ = filter_host_rsp_packet(&hostile_bytes, &mut policy);

        for base in [AM34_FLASH_BASE, AM34_FLASH_UNCACHED_BASE] {
            prop_assert!(is_nor_write_blocked_with_policy(
                base + ipl_offset,
                write_len,
                NorWritePolicy::UpperNorAppUnlocked,
            ));
        }
    }

    #[test]
    fn bgo_hostile_slices_and_extreme_offsets_never_panic(
        buf in proptest::collection::vec(any::<u8>(), 0..=1024),
        off in any::<usize>(),
    ) {
        let _ = is_active_bgo_p1_sector(&buf, off);
        let _ = eval_bgo_migration_index(&buf, off);
        let _ = eval_bgo_migration_target(&buf, off);
        let _ = parse_eh55_bgo_l2p(&buf);
        let _ = audit_eh55_bgo_nor_8mb(&buf);
    }
}
