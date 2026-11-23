use super::*;
use crate::constants::{AM34_SC0_BASE, PANA_FEISTEL_SBOX_256};
use crate::crypto::{FrmContainerMode, FrmPackModule, pack_extrhead, pack_frm};
use crate::re::bgo_unbrick::{
    OFF_BGO_INIT_CALL, OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ,
    ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PROG061_BASE_VA,
};
use slot_unpack::unpack_frm_slot_bytes;
use stage_detect::infer_stage_kind_and_base;

#[test]
fn bench_runtime_decrypts_frm_discovers_anchors_and_interpolates_region() {
    let dir = std::env::temp_dir().join(format!("panadiga_bench_rt_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let mut prog = vec![0xFFu8; 0x4_0000];
    prog[..0x1000].fill(0xCB);
    prog[OFF_BGO_INIT_CALL..OFF_BGO_INIT_CALL + ORIG_A2BC_BGO_CALL.len()]
        .copy_from_slice(&ORIG_A2BC_BGO_CALL);
    prog[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + ORIG_A2CD_HOOK.len()]
        .copy_from_slice(&ORIG_A2CD_HOOK);
    prog[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + ORIG_SUM_BEQ.len()].copy_from_slice(&ORIG_SUM_BEQ);
    prog[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + ORIG_DEVINFO_BEQ.len()]
        .copy_from_slice(&ORIG_DEVINFO_BEQ);
    // Place BGO geometry marker (0x84130000 / 0x84400000) & SC0 MMIO ref
    prog[0x337A4 + 32..0x337A4 + 36].copy_from_slice(&0x10000u32.to_le_bytes());
    prog[0x337A4 + 44..0x337A4 + 48].copy_from_slice(&0x8413_0000u32.to_le_bytes());
    prog[0x337A4 + 48..0x337A4 + 52].copy_from_slice(&0x8440_0000u32.to_le_bytes());
    prog[0x337A4 + 52..0x337A4 + 56].copy_from_slice(&0x10000u32.to_le_bytes());
    prog[0x1200..0x1204].copy_from_slice(&AM34_SC0_BASE.to_le_bytes());

    let drv_payload = b"DRV SLOGON\0\0\0\0\0\0test".to_vec();
    let drv_ext = pack_extrhead(
        &drv_payload,
        "DRV ",
        0x4000_0000,
        "DRV_S211",
        ExtrheadCryptMode::Plain,
        16,
        1,
    )
    .unwrap();

    let (frm, _) = pack_frm(
        &[
            FrmPackModule::new("DRV S211", drv_ext),
            FrmPackModule::new("PROG061 ", prog),
        ],
        FrmContainerMode::Dvd,
        None,
    )
    .unwrap();
    let frm_path = dir.join("TEST_PANA.FRM");
    fs::write(&frm_path, &frm).unwrap();

    let out_dir = dir.join("unpacked");
    let plan =
        build_dynamic_bench_runtime_plan(&frm_path, Some("DMR-EH55LB-S"), None, Some(&out_dir))
            .expect("bench runtime plan succeeds");

    assert_eq!(plan.detected_container_format, "FRM_FEISTEL");
    assert_eq!(plan.detected_feistel_key.as_deref(), Some("old_df72"));
    assert_eq!(plan.resolved_model_id.as_deref(), Some("DMR-EH55"));
    let reg = plan.interpolated_region.expect("interpolated LB region");
    assert_eq!(reg.tv_standard, "PAL-M");
    assert_eq!(reg.aiof_vmodesel_hex, "0x0002");
    assert_eq!(plan.stages.len(), 2);
    let prog_stage = plan.stages.iter().find(|s| s.stage_kind == "PROG").unwrap();
    assert_eq!(
        prog_stage.bgo_partition_base_hex.as_deref(),
        Some("0x84130000")
    );
    assert_eq!(
        prog_stage.panamon1_slack_va_hex.as_deref(),
        Some("0x50439494")
    );
    assert!(out_dir.join("bench_runtime_manifest.json").is_file());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn stb_unaligned_feistel_tail_and_cached_va_normalization() {
    use crate::crypto::feistel_encrypt_buffer;
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;

    let payload = b"    c    ST20_OS20_MOCK_FIRMWARE_PAYLOAD_WITH_TAIL_ALIGNMENT_TEST";
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(payload).unwrap();
    let zbytes = enc.finish().unwrap();

    let mut hdr = vec![0xFFu8; 256];
    let hdr_str = format!(
        "CRC:1234\0size:{}\0Type:M4\0AppVer:1.03\0Model: M4EN\0zlib:{}\0",
        zbytes.len(),
        payload.len()
    );
    hdr[..hdr_str.len()].copy_from_slice(hdr_str.as_bytes());
    let mut slot = [hdr, zbytes].concat();
    while slot.len() % 8 != 4 {
        slot.push(0x41);
    }
    let orig_size = slot.len();
    let pad_len = (orig_size + 7) & !7;
    slot.resize(pad_len, 0xFF);
    feistel_encrypt_buffer(
        &mut slot,
        &PANA_FEISTEL_ROUND_KEY_32B,
        &PANA_FEISTEL_SBOX_256,
    );

    let unpacked = unpack_frm_slot_bytes(
        &slot,
        orig_size,
        "STB 103",
        true,
        Some(&PANA_FEISTEL_ROUND_KEY_32B),
    );
    assert_eq!(unpacked.source_kind, "FRM_FEISTEL+STB_ZLIB");
    assert_eq!(unpacked.algo.as_deref(), Some("zlib"));
    assert!(unpacked.bytes.starts_with(payload));

    assert_eq!(
        infer_stage_kind_and_base("DIGADIAG", &[0u8; 16], None, Some(0x9040_0000)),
        ("PROG", PROG061_BASE_VA)
    );
    assert_eq!(
        infer_stage_kind_and_base("DRV S514", &[0u8; 16], None, Some(0x9020_2000)),
        ("DRV", 0x4000_0000)
    );
}
