//! Real-binary (`flash.bin` 8 MiB ZIP + `prog061_dec.bin` + `PANA__SD.FRM` / `main3060np`) emulation tests.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use flate2::read::DeflateDecoder;

use crate::constants::{AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE};
use crate::crypto::{
    ExtrheadCryptMode, parse_sd_frm_toc_with_layout, unpack_dvd_frm_to_modules, unpack_extrhead,
};
use crate::hdd::inspect_main_buffer_capacity;
use crate::nvram::calculate_hashes;
use crate::pico::rsp_codec::policy::NorWritePolicy;
use crate::re::{
    MAIN_BASE_VA, Mn103Fault, Mn103Vm, OFF_UART_MONITOR_SLACK, PROG061_BASE_VA,
    PROG061_UART_MONITOR_VA, Prog061BgoPatchMode, inspect_main_passthrough_sites,
    patch_prog061_bgo_in_place_with_policy,
};

use super::fixtures::TEST_RAM_WORK_BASE;
use super::{
    run_nor_emu_patch_verification_suite, run_nor_emu_patch_verification_suite_from_slices,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn extract_zip_entry_by_suffix(zip: &[u8], suffix: &str) -> Result<Vec<u8>> {
    let min_eocd = zip.len().saturating_sub(65_557);
    let eocd = (min_eocd..=zip.len().saturating_sub(22))
        .rev()
        .find(|&i| &zip[i..i + 4] == b"PK\x05\x06")
        .context("Missing ZIP EOCD signature")?;
    let total_entries = LittleEndian::read_u16(&zip[eocd + 10..eocd + 12]) as usize;
    let mut cd_pos = LittleEndian::read_u32(&zip[eocd + 16..eocd + 20]) as usize;

    for _ in 0..total_entries {
        if &zip.get(cd_pos..cd_pos + 4).unwrap_or(&[]) != b"PK\x01\x02" {
            bail!("Invalid ZIP central directory signature @ 0x{cd_pos:X}");
        }
        let method = LittleEndian::read_u16(&zip[cd_pos + 10..cd_pos + 12]);
        let comp_sz = LittleEndian::read_u32(&zip[cd_pos + 20..cd_pos + 24]) as usize;
        let uncomp_sz = LittleEndian::read_u32(&zip[cd_pos + 24..cd_pos + 28]) as usize;
        let name_len = LittleEndian::read_u16(&zip[cd_pos + 28..cd_pos + 30]) as usize;
        let extra_len = LittleEndian::read_u16(&zip[cd_pos + 30..cd_pos + 32]) as usize;
        let cmt_len = LittleEndian::read_u16(&zip[cd_pos + 32..cd_pos + 34]) as usize;
        let lfh_off = LittleEndian::read_u32(&zip[cd_pos + 42..cd_pos + 46]) as usize;
        let name = String::from_utf8_lossy(&zip[cd_pos + 46..cd_pos + 46 + name_len]);
        cd_pos += 46 + name_len + extra_len + cmt_len;

        if name.ends_with(suffix) {
            if &zip.get(lfh_off..lfh_off + 4).unwrap_or(&[]) != b"PK\x03\x04" {
                bail!("Invalid ZIP local header signature for {name}");
            }
            let lfh_name = LittleEndian::read_u16(&zip[lfh_off + 26..lfh_off + 28]) as usize;
            let lfh_extra = LittleEndian::read_u16(&zip[lfh_off + 28..lfh_off + 30]) as usize;
            let data_off = lfh_off + 30 + lfh_name + lfh_extra;
            let comp = &zip[data_off..data_off + comp_sz];
            return match method {
                0 => Ok(comp.to_vec()),
                8 => {
                    let mut out = Vec::with_capacity(uncomp_sz);
                    DeflateDecoder::new(comp).read_to_end(&mut out)?;
                    Ok(out)
                }
                other => bail!("Unsupported ZIP compression method {other} for {name}"),
            };
        }
    }
    bail!("Entry ending with '{suffix}' not found in ZIP")
}

/// Autonomous MN103S SRAM JEDEC 6-cycle Sector Erase + 4-cycle Word Program stub (`a2`=target, `d2`=byte).
fn build_sram_jedec_erase_prog_stub() -> Vec<u8> {
    let mut b = Vec::with_capacity(128);
    // Erase cycle: a0=0x84000AAA, a1=0x84000554
    b.extend_from_slice(&[0xFC, 0xDC, 0xAA, 0x0A, 0x00, 0x84]); // mov 0x84000AAA, a0
    b.extend_from_slice(&[0xFC, 0xDD, 0x54, 0x05, 0x00, 0x84]); // mov 0x84000554, a1
    for (cyc, op_reg) in [
        (0xAAu8, 0x70u8),
        (0x55, 0x71),
        (0x80, 0x70),
        (0xAA, 0x70),
        (0x55, 0x71),
    ] {
        b.extend_from_slice(&[0x80, cyc, 0x14, 0xF0, op_reg]); // mov cyc,d0; extbu d0; movhu d0,(a0/a1)
    }
    b.extend_from_slice(&[0x80, 0x30, 0xF0, 0x72]); // mov 0x30,d0; movhu d0,(a2) -> sector erase
    // Program cycle onto (a2 + 8) with byte value in d2: a3 = a2 + 8
    b.extend_from_slice(&[0xF1, 0xDB]); // mov a2, d3
    b.extend_from_slice(&[0x2B, 0x08]); // add 8, d3
    b.extend_from_slice(&[0xF1, 0xEF]); // mov d3, a3
    for (cyc, op_reg) in [(0xAAu8, 0x70u8), (0x55, 0x71), (0xA0, 0x70)] {
        b.extend_from_slice(&[0x80, cyc, 0x14, 0xF0, op_reg]);
    }
    b.extend_from_slice(&[0xF0, 0x5B]); // movbu d2, (a3) -> byte program (0xFFxx low byte in target)
    b.push(0xCB); // nop stop sentinel
    b
}

pub(super) fn verify_real_8mb_nor_and_sram_jedec_on_vm(
    real_flash_8mb: &[u8],
    prog061_factory: &[u8],
) -> Result<()> {
    assert_eq!(real_flash_8mb.len(), AM34_FLASH_SIZE as usize);
    let crit_len = AM34_FLASH_CRITICAL_IPL_SIZE as usize;
    let (_, orig_crit_sha) = calculate_hashes(&real_flash_8mb[..crit_len]);

    let mut prog_pat = prog061_factory.to_vec();
    patch_prog061_bgo_in_place_with_policy(&mut prog_pat, Prog061BgoPatchMode::UartMonitor, false)?;

    // 1. Run real PANAMON1 + real 8 MiB flash.bin in Mn103Vm
    let mut vm = Mn103Vm::new();
    vm.nor_write_policy = NorWritePolicy::UpperNorAppUnlocked;
    vm.enable_sc0_uart(&[]);
    vm.map_segment(AM34_FLASH_UNCACHED_BASE, real_flash_8mb.to_vec(), true);
    vm.map_segment(PROG061_BASE_VA, prog_pat.clone(), false);
    vm.map_segment(TEST_RAM_WORK_BASE, vec![0u8; 0x2_0000], true);

    let call_va = TEST_RAM_WORK_BASE + 0x1000;
    let rel32 = (PROG061_UART_MONITOR_VA as i64 - call_va as i64) as i32;
    vm.write_u8(call_va, 0xDD)?;
    for (i, &byte) in rel32.to_le_bytes().iter().enumerate() {
        vm.write_u8(call_va + 1 + i as u32, byte)?;
    }
    vm.write_u8(call_va + 5, 0x00)?;
    vm.write_u8(call_va + 6, 0x00)?;
    vm.write_u8(call_va + 7, 0xCB)?;
    vm.sp = TEST_RAM_WORK_BASE + 0xF000;
    vm.pc = call_va;

    vm.queue_uart_rx(
        b"R 84000000,04\r\n\
          W 84240010,3C\r\n\
          E 84240000\r\n\
          U CONFIRM_UPPER_NOR\r\n\
          W 84000100,AA\r\n\
          E 84030000\r\n\
          E 84240000\r\n\
          W 84240010,3C\r\n\
          W 84240011,C3\r\n\
          U LOCK\r\n\
          W 84240012,11\r\n\
          B",
    );
    vm.run_until(call_va + 7, 500_000)?;
    let tx = vm.take_uart_tx_utf8();
    assert_eq!(vm.d[0], 0);
    assert_eq!(tx.matches("ERR:E05_IPL_RO").count(), 5, "tx={tx}");
    assert!(tx.contains("OK:NOR_UNLOCK"), "tx={tx}");
    assert!(tx.contains("OK:NOR_LOCK"), "tx={tx}");
    assert_eq!(vm.read_u8(0x8423_FFFF)?, real_flash_8mb[0x23_FFFF]);
    assert_eq!(vm.read_u8(0x8424_0000)?, 0xFF);
    assert_eq!(vm.read_u8(0x8424_0010)?, 0x3C);
    assert_eq!(vm.read_u8(0x8424_0011)?, 0xC3);
    assert_eq!(vm.read_u8(0x8425_0000)?, real_flash_8mb[0x25_0000]);

    // 2. Execute SRAM JEDEC stub directly in Mn103Vm under NvramOnly vs UpperNorAppUnlocked
    let stub = build_sram_jedec_erase_prog_stub();
    let stub_va = TEST_RAM_WORK_BASE + 0x3_0000;
    let stop_va = stub_va + stub.len() as u32 - 1;
    vm.map_segment(stub_va, stub, false);

    vm.nor_write_policy = NorWritePolicy::NvramOnly;
    vm.a[2] = 0x8428_0000;
    vm.d[2] = 0x5C;
    vm.pc = stub_va;
    let locked_budget = vm.steps + 1000;
    let locked_res = vm.run_until(stop_va, locked_budget);
    assert!(
        matches!(locked_res, Err(Mn103Fault::IplWriteProtected(0x8428_0000))),
        "expected IplWriteProtected(0x84280000), got {locked_res:?} at pc={:#010x} stage={:?}",
        vm.pc,
        vm.cfi_stage
    );

    vm.nor_write_policy = NorWritePolicy::UpperNorAppUnlocked;
    vm.a[2] = 0x8428_0000;
    vm.d[2] = 0x5C;
    vm.pc = stub_va;
    let unlocked_budget = vm.steps + 1000;
    vm.run_until(stop_va, unlocked_budget)?;
    assert_eq!(vm.read_u8(0x8428_0000)?, 0xFF);
    assert_eq!(vm.read_u8(0x8428_0008)?, 0x5C);
    assert_eq!(vm.read_u8(0x8428_FFFF)?, 0xFF);

    vm.a[2] = 0x8400_0100;
    vm.pc = stub_va;
    let crit_budget = vm.steps + 1000;
    assert!(matches!(
        vm.run_until(stop_va, crit_budget),
        Err(Mn103Fault::IplWriteProtected(0x8400_0100))
    ));

    let mut post_crit = vec![0u8; crit_len];
    for (i, dst) in post_crit.iter_mut().enumerate() {
        *dst = vm.read_u8(AM34_FLASH_UNCACHED_BASE + i as u32)?;
    }
    let (_, post_crit_sha) = calculate_hashes(&post_crit);
    assert_eq!(post_crit_sha, orig_crit_sha);
    assert!(prog_pat.len() > OFF_UART_MONITOR_SLACK);
    Ok(())
}

pub(super) fn run_optional_real_binaries_integration() -> Result<()> {
    let root = repo_root();
    let zip_path = root
        .join("firmware_and_images/dmr-es35v/dmr-es35v-8mb-nor-flash-and-eeprom-triton-dump.zip");
    let prog_path =
        root.join("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
    let main3060_path =
        root.join("firmware_and_images/extracted_partitions/frm_unpacked/main3060np_unpacked.bin");
    let pana_dvd_path = root.join("firmware_and_images/PANA_DVD.FRM");
    let sd_frm_path = root.join("firmware_and_images/sd_unbrick_kit/PANA__SD.FRM");

    if zip_path.exists() && prog_path.exists() {
        let zip_bytes = fs::read(&zip_path)?;
        let flash_8mb = extract_zip_entry_by_suffix(&zip_bytes, "flash.bin")?;
        let prog_bytes = fs::read(&prog_path)?;
        verify_real_8mb_nor_and_sram_jedec_on_vm(&flash_8mb, &prog_bytes)?;

        if main3060_path.exists() {
            let rep_3060 =
                run_nor_emu_patch_verification_suite(Some(&prog_path), Some(&main3060_path))?;
            assert!(rep_3060.all_passed);
        }
        if pana_dvd_path.exists() {
            let dvd_bytes = fs::read(&pana_dvd_path)?;
            let modules = unpack_dvd_frm_to_modules(&dvd_bytes)?;
            if let Some(m) = modules.iter().find(|e| e.name.trim().starts_with("MAIN")) {
                let unpacked = unpack_extrhead(&m.payload, ExtrheadCryptMode::Main)?;
                let rep_3410 = run_nor_emu_patch_verification_suite_from_slices(
                    &prog_bytes,
                    &unpacked.decompressed,
                    "real_pana_dvd_3410pq",
                )?;
                assert!(rep_3410.all_passed);
            }
        }
        if sd_frm_path.exists() {
            let sd_bytes = fs::read(&sd_frm_path)?;
            let (_layout, entries) = parse_sd_frm_toc_with_layout(&sd_bytes);
            if let Some(m) = entries
                .iter()
                .find(|e| e.module_name.trim().starts_with("MAIN"))
            {
                let off = m.module_offset as usize;
                let end = off + m.module_size as usize;
                let unpacked = unpack_extrhead(&sd_bytes[off..end], ExtrheadCryptMode::Main)?;
                let cap = inspect_main_buffer_capacity(
                    &unpacked.decompressed,
                    MAIN_BASE_VA,
                    "EXTRHEAD_CONTAINER",
                    Some(unpacked.header.build_name.clone()),
                    "PANA__SD.FRM:MAIN",
                )?;
                assert!(
                    cap.is_dynamic_unclamped || cap.effective_default_limit_gb >= 249.0,
                    "Expected SD unbrick MAIN HDD limit >= 249 GB, got {} GB (0x{:08X})",
                    cap.effective_default_limit_gb,
                    cap.slot_200gb_blocks_2kb
                );
                assert!(inspect_main_passthrough_sites(&unpacked.decompressed).is_ok());
            }
        }
    }
    Ok(())
}
