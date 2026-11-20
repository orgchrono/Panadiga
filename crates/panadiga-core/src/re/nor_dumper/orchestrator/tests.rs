#![deny(unsafe_code)]

use std::fs;

use super::*;
use crate::constants::AM34_SDRAM_UNCACHED_BASE;
use crate::pico::panamon1::MockPanamon1Transport;

fn setup_mock_with_dumper(nor_bytes: Vec<u8>) -> (NorCaptureConfig, MockPanamon1Transport) {
    let mut cfg = NorCaptureConfig::try_default().unwrap();
    cfg.chunk_size_bytes = 8192;
    let (mon_payload, offsets) =
        build_prog061_uart_monitor_with_offsets(PROG061_UART_MONITOR_VA).unwrap();
    let (entry_u, start_imm_u, end_imm_u) = offsets.uncached_patch_vas(PROG061_UART_MONITOR_VA);

    let mut mock = MockPanamon1Transport::new();
    let uncached_mon = (PROG061_UART_MONITOR_VA & 0x0FFF_FFFF) | AM34_SDRAM_UNCACHED_BASE;
    mock.map_memory(uncached_mon, mon_payload);
    mock.map_memory(AM34_FLASH_UNCACHED_BASE, nor_bytes);
    mock.enable_dynamic_nor_dumper(entry_u, start_imm_u, end_imm_u, offsets.block_size);
    (cfg, mock)
}

#[test]
fn sliced_nor_capture_assembles_and_resumes_checkpoints_offline() {
    let mut nor = vec![0xFFu8; AM34_FLASH_SIZE as usize];
    for (i, b) in nor[..16384].iter_mut().enumerate() {
        *b = (i & 0xFF) as u8;
    }
    let (mut cfg, mock) = setup_mock_with_dumper(nor.clone());
    let temp_dir = std::env::temp_dir().join(format!("panadiga_nor_orch_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    cfg.checkpoint_dir = Some(temp_dir.clone());

    let orch = NorCaptureOrchestrator::new(cfg).unwrap();
    let mut client = Panamon1Client::new(mock);
    let preset = NorSlicePreset::Custom {
        start_va: AM34_FLASH_UNCACHED_BASE,
        len: 16_384,
    };

    let first = orch
        .capture_preset(&mut client, preset, &mut |_, _, _| {})
        .unwrap();
    assert_eq!(first.data, nor[..16_384]);
    assert_eq!(first.slices.len(), 2);
    assert!(first.slices.iter().all(|s| !s.resumed_from_checkpoint));

    let prev_j_count = client
        .transport()
        .tx_target_log
        .iter()
        .filter(|c| c.starts_with("J "))
        .count();
    let second = orch
        .capture_preset(&mut client, preset, &mut |_, _, _| {})
        .unwrap();
    let after_j_count = client
        .transport()
        .tx_target_log
        .iter()
        .filter(|c| c.starts_with("J "))
        .count();
    assert_eq!(prev_j_count, after_j_count);
    assert_eq!(second.sha256, first.sha256);
    assert!(second.slices.iter().all(|s| s.resumed_from_checkpoint));
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn bridge_ctrl_edges_or_overflow_fails_capture_deterministically() {
    let nor = vec![0xAAu8; AM34_FLASH_SIZE as usize];
    let (cfg, mut mock) = setup_mock_with_dumper(nor);
    mock.bridge_ctrl_edges = 2;
    let orch = NorCaptureOrchestrator::new(cfg).unwrap();
    let mut client = Panamon1Client::new(mock);
    let err = orch
        .capture_preset(
            &mut client,
            NorSlicePreset::Custom {
                start_va: AM34_FLASH_UNCACHED_BASE,
                len: 4096,
            },
            &mut |_, _, _| {},
        )
        .unwrap_err()
        .to_string();
    assert!(err.contains("ctrl_edges=2"), "err={err}");
}
