use super::*;
use panadiga_core::constants::{AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_SIZE};
use panadiga_core::pico::panamon1::MockPanamon1Transport;

#[test]
fn panamon1_clean_nvram_backs_up_erases_sa134_sa141_only_and_verifies_ff() {
    let dir = std::env::temp_dir().join("panadiga_panamon1_clean_nvram_test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let mut mock = MockPanamon1Transport::new();
    mock.bridge_active = true;
    // Seed dirty U99 NVRAM bytes across 0x847F0000..+64KiB and protect IPL 0x84000000
    mock.map_memory(AM34_FLASH_UNCACHED_BASE, vec![0xAAu8; 0x10_000]);
    mock.map_memory(
        AM34_NVRAM_UNCACHED_BASE,
        vec![0x5Au8; AM34_NVRAM_SIZE as usize],
    );

    let req = PicoPanamon1CleanNvramRequestDto {
        port_name: "MOCK_COM10".to_string(),
        backup_dir: Some(dir.to_string_lossy().into_owned()),
        session_id: None,
    };

    let mut events = Vec::new();
    let rep = run_panamon1_clean_nvram_with_transport(mock, &req, |ev| events.push(ev)).unwrap();

    assert_eq!(rep.erased_sectors, 8);
    assert_eq!(rep.bytes_erased, 65_536);
    assert!(rep.verified_all_ff);
    assert!(rep.emu_gate.passed);
    assert!(rep.emu_gate.ipl_untouched);
    assert!(rep.emu_gate.lethal_mmio_blocked);
    assert_ne!(rep.sha256_before, rep.sha256_after);
    assert!(Path::new(&rep.backup_path).is_file());
    assert_eq!(fs::read(&rep.backup_path).unwrap().len(), 65_536);
    assert!(events.iter().any(|e| e.stage == "backup_verify"));
    assert!(events.iter().any(|e| e.stage == "erase_sector"));
    assert!(events.iter().any(|e| e.stage == "complete"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn panamon1_clean_nvram_rejects_empty_port() {
    let mock = MockPanamon1Transport::new();
    let req = PicoPanamon1CleanNvramRequestDto {
        port_name: "   ".to_string(),
        backup_dir: None,
        session_id: None,
    };
    assert!(run_panamon1_clean_nvram_with_transport(mock, &req, |_| {}).is_err());
}
