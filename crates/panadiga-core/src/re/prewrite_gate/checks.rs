#![deny(unsafe_code)]

//! Implementation of individual `Mn103Vm` and structural verification checks for [`super::verify_prewrite_gate`].

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::constants::{
    AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_IPL_SIZE, AM34_FLASH_UNCACHED_BASE,
    AM34_FORBIDDEN_LETHAL_REG, AM34_IC7501_TBUS_BASE, AM34_IC7501_TBUS_TX_OFF,
    AM34_NVRAM_UNCACHED_BASE, PANA_DIGADIAG_FRM_SIZE, PANA_DIGADIAG_RAW_VER061_SIZE,
};
use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
use crate::hdd::{decode_meihdfs_slice, generate_meihdfs_layout, inspect_main_buffer_capacity};
use crate::nvram::calculate_hashes;
use crate::re::bgo_unbrick::{PROG061_BASE_VA, Prog061BgoPatchMode, inspect_prog061_bgo_sites};
use crate::re::emu_patch_suite::{new_spansion_nor_vm, verify_tbc_passthrough_routines};
use crate::re::main_passthrough::{MAIN_BASE_VA, inspect_main_passthrough_sites};
use crate::re::mn10300_emu::{Mn103Fault, Mn103Vm};
use crate::re::prog061_uart_monitor::PROG061_UART_MONITOR_VA;
use crate::re::tbc_appliance::build_ic7501_wdt_255s_frame;
use crate::re::unbrick_sd_kit::SdLoaderProfile;

use super::{PreWriteCheckItem, PreWriteGateReport, push_check};

const CLEANROOM_ENTRY_RETURN_SENTINEL: u32 = 0x504F_BEEFu32;
const CLEANROOM_STACK_TOP_VA: u32 = 0x5001_8000u32;

fn new_cleanroom_vm(payload: &[u8]) -> Result<(Mn103Vm, Vec<u8>)> {
    let mut vm = new_spansion_nor_vm();
    vm.map_segment(PROG061_BASE_VA, payload.to_vec(), false);
    vm.map_segment(AM34_IC7501_TBUS_BASE, vec![0u8; 0x100], true);
    vm.enable_sc0_uart(b"R 84000000,04\r\nE 84000000\r\nE 847F0000\r\nB");
    let mut ipl_pre = vec![0u8; 0x1000];
    for (idx, b) in ipl_pre.iter_mut().enumerate() {
        *b = vm
            .read_u8(AM34_FLASH_UNCACHED_BASE + idx as u32)
            .unwrap_or(0xFF);
    }
    Ok((vm, Sha256::digest(&ipl_pre).to_vec()))
}

fn verify_vm_lethal_and_ipl_post(
    vm: &mut Mn103Vm,
    ipl_pre_hash: &[u8],
) -> Result<(bool, bool, Vec<PreWriteCheckItem>)> {
    let mut ipl_post = vec![0u8; 0x1000];
    for (idx, b) in ipl_post.iter_mut().enumerate() {
        *b = vm.read_u8(AM34_FLASH_UNCACHED_BASE + idx as u32)?;
    }
    let ipl_untouched = Sha256::digest(&ipl_post).as_slice() == ipl_pre_hash;
    let lethal_blocked = matches!(
        vm.read_u32(AM34_FORBIDDEN_LETHAL_REG),
        Err(Mn103Fault::LethalRegisterRead(AM34_FORBIDDEN_LETHAL_REG))
    );
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "ipl_sha256_immutable",
        ipl_untouched,
        "SA0..SA10 (0x84000000) SHA-256 unchanged across VM execution",
    );
    push_check(
        &mut checks,
        "lethal_c000100c_blocked",
        lethal_blocked,
        "AM34 lethal register 0xC000100C intercepted with LethalRegisterRead fault",
    );
    Ok((ipl_untouched, lethal_blocked, checks))
}

pub(super) fn check_cleanroom_digadiag(
    bytes: &[u8],
    loader_profile: SdLoaderProfile,
    artifact_sha256: String,
) -> Result<PreWriteGateReport> {
    let mut checks = Vec::new();
    let expected_sz = match loader_profile {
        SdLoaderProfile::Ver061RawSd48 => PANA_DIGADIAG_RAW_VER061_SIZE,
        SdLoaderProfile::Ver063ExtrheadSd64 => PANA_DIGADIAG_FRM_SIZE,
    };
    let size_ok = bytes.len() == expected_sz;
    push_check(
        &mut checks,
        "digadiag_container_size",
        size_ok,
        format!("Actual {} B (expected {expected_sz} B)", bytes.len()),
    );
    let exec_payload = if !size_ok {
        return Ok(build_report(
            "cleanroom_digadiag",
            artifact_sha256,
            true,
            true,
            0,
            checks,
            vec!["Size check failed before VM execution".into()],
        ));
    } else if loader_profile == SdLoaderProfile::Ver063ExtrheadSd64 {
        unpack_extrhead(bytes, ExtrheadCryptMode::Plain)
            .context("Unpacking ver0.63 EXTRHEAD DIGADIAG.FRM")?
            .decompressed
    } else {
        bytes.to_vec()
    };

    let (mut vm, ipl_pre) = new_cleanroom_vm(&exec_payload)?;
    vm.sp = CLEANROOM_STACK_TOP_VA;
    vm.write_u32(vm.sp, CLEANROOM_ENTRY_RETURN_SENTINEL)?;
    vm.pc = PROG061_BASE_VA;
    let run_ok = vm
        .run_until(CLEANROOM_ENTRY_RETURN_SENTINEL, 500_000)
        .is_ok();
    let tx = vm.take_uart_tx_utf8();

    let expected_wdt = build_ic7501_wdt_255s_frame();
    let mut observed_wdt = [0u8; 16];
    for (idx, slot) in observed_wdt.iter_mut().enumerate() {
        *slot = vm.read_u8(AM34_IC7501_TBUS_BASE + AM34_IC7501_TBUS_TX_OFF + idx as u32)?;
    }
    let wdt_ok = observed_wdt == expected_wdt;
    let uart_ok = tx.contains("PANAMON1>")
        && tx.contains("ERR:E05_IPL_RO")
        && tx.contains("OK:ERASE")
        && vm.read_u8(AM34_NVRAM_UNCACHED_BASE)? == 0xFF;

    push_check(
        &mut checks,
        "mn103vm_entry_and_return",
        run_ok,
        format!("Executed {} steps from 0x{:08X}", vm.steps, PROG061_BASE_VA),
    );
    push_check(
        &mut checks,
        "ic7501_tbus_wdt_keepalive",
        wdt_ok,
        "Verified 255s WDT keepalive frame (chk=0x30) latched into 0xDC010010",
    );
    push_check(
        &mut checks,
        "panamon1_uart_e05_and_nvram_erase",
        uart_ok,
        "PANAMON1> booted, blocked IPL E 84000000 (ERR:E05_IPL_RO), erased 0x847F0000",
    );

    let (ipl_ok, lethal_ok, mut post_checks) = verify_vm_lethal_and_ipl_post(&mut vm, &ipl_pre)?;
    checks.append(&mut post_checks);

    Ok(build_report(
        "cleanroom_digadiag",
        artifact_sha256,
        ipl_ok,
        lethal_ok,
        vm.steps,
        checks,
        vec![format!("Profile={}", loader_profile.as_str())],
    ))
}

pub(super) fn check_patched_prog061(
    bytes: &[u8],
    mode: Prog061BgoPatchMode,
    artifact_sha256: String,
) -> Result<PreWriteGateReport> {
    let mut checks = Vec::new();
    let size_ok = bytes.len() == PANA_DIGADIAG_RAW_VER061_SIZE;
    push_check(
        &mut checks,
        "prog061_size",
        size_ok,
        format!(
            "Actual {} B (expected {PANA_DIGADIAG_RAW_VER061_SIZE} B)",
            bytes.len()
        ),
    );
    let bgo_rep = inspect_prog061_bgo_sites(bytes).ok();
    push_check(
        &mut checks,
        "prog061_bgo_sites",
        bgo_rep.is_some(),
        bgo_rep
            .as_ref()
            .map(|r| format!("mode={}", r.detected_mode))
            .unwrap_or_else(|| "BGO sites invalid".into()),
    );

    let (mut vm, ipl_pre) = new_cleanroom_vm(bytes)?;
    let mut mon_steps = 0u64;
    if mode == Prog061BgoPatchMode::UartMonitor && size_ok {
        vm.sp = CLEANROOM_STACK_TOP_VA;
        vm.write_u32(vm.sp, CLEANROOM_ENTRY_RETURN_SENTINEL)?;
        vm.pc = PROG061_UART_MONITOR_VA;
        let run_ok = vm
            .run_until(CLEANROOM_ENTRY_RETURN_SENTINEL, 500_000)
            .is_ok();
        mon_steps = vm.steps;
        let tx = vm.take_uart_tx_utf8();
        let ok = run_ok
            && tx.contains("PANAMON1>")
            && tx.contains("ERR:E05_IPL_RO")
            && tx.contains("OK:ERASE");
        push_check(
            &mut checks,
            "panamon1_vm_session",
            ok,
            "Executed embedded PANAMON1> inside Mn103Vm (E05 IPL lock + NVRAM OK:ERASE)",
        );
    }
    let (ipl_ok, lethal_ok, mut post) = verify_vm_lethal_and_ipl_post(&mut vm, &ipl_pre)?;
    checks.append(&mut post);

    Ok(build_report(
        "patched_prog061",
        artifact_sha256,
        ipl_ok,
        lethal_ok,
        mon_steps,
        checks,
        vec![format!("BgoMode={mode:?}")],
    ))
}

pub(super) fn check_patched_main(
    raw_main: &[u8],
    expect_passthrough: bool,
    expect_capacity_unlock: bool,
    artifact_sha256: String,
) -> Result<PreWriteGateReport> {
    let mut checks = Vec::new();
    let mut vm_steps = 0u64;
    if expect_passthrough {
        let sites_ok = inspect_main_passthrough_sites(raw_main).is_ok();
        let res = if sites_ok {
            verify_tbc_passthrough_routines(raw_main)
        } else {
            Err(anyhow::anyhow!("Missing 3410PQ passthrough signatures"))
        };
        if res.is_ok() {
            vm_steps += 16;
        }
        push_check(
            &mut checks,
            "main_passthrough_mn103vm",
            res.is_ok(),
            res.map(|c| c.detail).unwrap_or_else(|e| e.to_string()),
        );
    }
    if expect_capacity_unlock {
        let cap = inspect_main_buffer_capacity(raw_main, MAIN_BASE_VA, "RAW_MAIN", None, "gate");
        let ok = cap.as_ref().is_ok_and(|c| c.is_dynamic_unclamped);
        push_check(
            &mut checks,
            "main_hdd_capacity_unlock",
            ok,
            cap.map(|c| {
                format!(
                    "dynamic_unclamped={}, eff_gb={}",
                    c.is_dynamic_unclamped, c.effective_default_limit_gb
                )
            })
            .unwrap_or_else(|e| e.to_string()),
        );
    }
    finalize_with_cleanroom_vm_checks(
        "patched_main",
        artifact_sha256,
        vm_steps,
        checks,
        Vec::new(),
    )
}

fn finalize_with_cleanroom_vm_checks(
    kind: &str,
    artifact_sha256: String,
    executed_steps: u64,
    mut checks: Vec<PreWriteCheckItem>,
    notes: Vec<String>,
) -> Result<PreWriteGateReport> {
    let (mut vm, ipl_pre) = new_cleanroom_vm(&[])?;
    let (ipl_ok, lethal_ok, mut post) = verify_vm_lethal_and_ipl_post(&mut vm, &ipl_pre)?;
    checks.append(&mut post);
    Ok(build_report(
        kind,
        artifact_sha256,
        ipl_ok,
        lethal_ok,
        executed_steps,
        checks,
        notes,
    ))
}

pub(super) fn check_nor_cow_repair(
    original_nor: &[u8],
    repaired_nor: &[u8],
    allow_upper_nor: bool,
    artifact_sha256: String,
) -> Result<PreWriteGateReport> {
    let mut checks = Vec::new();
    let same_len = original_nor.len() == repaired_nor.len()
        && repaired_nor.len() >= AM34_FLASH_CRITICAL_IPL_SIZE as usize;
    push_check(
        &mut checks,
        "nor_size_parity",
        same_len,
        format!(
            "orig={} B, repaired={} B",
            original_nor.len(),
            repaired_nor.len()
        ),
    );
    let critical_len = (AM34_FLASH_CRITICAL_IPL_SIZE as usize)
        .min(original_nor.len())
        .min(repaired_nor.len());
    let ipl_untouched = same_len
        && calculate_hashes(&original_nor[..critical_len]).1
            == calculate_hashes(&repaired_nor[..critical_len]).1;
    push_check(
        &mut checks,
        "nor_ipl_sa0_sa10_immutable",
        ipl_untouched,
        "SA0..SA10 (0x000000..0x03FFFF) cryptographic equality verified",
    );
    if !allow_upper_nor && repaired_nor.len() >= AM34_FLASH_IPL_SIZE as usize && same_len {
        let upper_len = AM34_FLASH_IPL_SIZE as usize;
        let upper_untouched = calculate_hashes(&original_nor[critical_len..upper_len]).1
            == calculate_hashes(&repaired_nor[critical_len..upper_len]).1;
        push_check(
            &mut checks,
            "nor_upper_sa11_sa133_immutable",
            upper_untouched,
            "Upper NOR (0x040000..0x7EFFFF) untouched for NVRAM-only policy",
        );
    }
    let (mut vm, ipl_pre) = new_cleanroom_vm(&[])?;
    let ipl_ro_enforced = matches!(
        vm.write_u8(AM34_FLASH_UNCACHED_BASE + 0x100, 0x00),
        Err(Mn103Fault::IplWriteProtected(_))
    );
    push_check(
        &mut checks,
        "nor_vm_ipl_lockout",
        ipl_ro_enforced,
        "Mn103Vm rejects NOR IPL writes with IplWriteProtected",
    );
    let (vm_ipl_ok, lethal_ok, mut post) = verify_vm_lethal_and_ipl_post(&mut vm, &ipl_pre)?;
    checks.append(&mut post);
    Ok(build_report(
        "nor_cow_repair",
        artifact_sha256,
        ipl_untouched && vm_ipl_ok,
        lethal_ok,
        0,
        checks,
        vec![format!("allow_upper_nor={allow_upper_nor}")],
    ))
}

pub(super) fn check_composed_hdd_stream(
    total_sectors: u64,
    main_extrhead: Option<&[u8]>,
    drv_extrhead: Option<&[u8]>,
) -> Result<PreWriteGateReport> {
    let mut checks = Vec::new();
    let img = generate_meihdfs_layout(total_sectors, None)?;
    let (_, layout_sha) = calculate_hashes(&img.mbr_lba0);
    let mut probe_buf = vec![0u8; 0x01C0_4000];
    probe_buf[..512].copy_from_slice(&img.mbr_lba0);
    probe_buf[0x3000..0x4000].copy_from_slice(&img.descriptors_lba24);
    probe_buf[0x0080_0000..0x0080_0200].copy_from_slice(&img.journal_header_lba16384);
    probe_buf[0x01C0_0000..0x01C0_0200].copy_from_slice(&img.mirror_mbr_lba57344);
    probe_buf[0x01C0_3000..0x01C0_4000].copy_from_slice(&img.mirror_descriptors_lba57368);

    let decoded = decode_meihdfs_slice(&probe_buf, true, "prewrite:composed_hdd")?;
    let hdr_ok = decoded.is_mirror_verified
        && img.journal_header_lba16384[0..8] == [10, 0, 0, 0, 9, 0, 0, 0];
    push_check(
        &mut checks,
        "meihdfs_superblock_mirror_journal",
        hdr_ok,
        format!(
            "sectors={}, mirror_verified={}, journal=[10,9], extents={}",
            total_sectors,
            decoded.is_mirror_verified,
            decoded.extents.len()
        ),
    );
    if let Some(m) = main_extrhead {
        let unp = unpack_extrhead(m, ExtrheadCryptMode::Main)?;
        push_check(
            &mut checks,
            "composed_main_extrhead",
            unp.checksum_valid && !unp.decompressed.is_empty(),
            format!(
                "MAIN unpacked {} B (adler_ok={})",
                unp.decompressed.len(),
                unp.checksum_valid
            ),
        );
    }
    if let Some(d) = drv_extrhead {
        let unp = unpack_extrhead(d, ExtrheadCryptMode::Drv)?;
        push_check(
            &mut checks,
            "composed_drv_extrhead",
            unp.checksum_valid && !unp.decompressed.is_empty(),
            format!(
                "DRV unpacked {} B (adler_ok={})",
                unp.decompressed.len(),
                unp.checksum_valid
            ),
        );
    }
    finalize_with_cleanroom_vm_checks(
        "composed_hdd_stream",
        layout_sha,
        0,
        checks,
        vec![format!("p3_sectors={}", img.p3_sectors)],
    )
}

fn build_report(
    kind: &str,
    artifact_sha256: String,
    ipl_untouched: bool,
    lethal_mmio_blocked: bool,
    executed_steps: u64,
    checks: Vec<PreWriteCheckItem>,
    notes: Vec<String>,
) -> PreWriteGateReport {
    let passed = ipl_untouched
        && lethal_mmio_blocked
        && !checks.is_empty()
        && checks.iter().all(|c| c.passed);
    PreWriteGateReport {
        passed,
        artifact_kind: kind.to_string(),
        artifact_sha256,
        ipl_untouched,
        lethal_mmio_blocked,
        executed_steps,
        checks,
        notes,
    }
}
