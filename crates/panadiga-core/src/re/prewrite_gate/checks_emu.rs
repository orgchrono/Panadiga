#![deny(unsafe_code)]

//! Clean-room SD `DIGADIAG.FRM`, `PROG061` BGO/PANAMON1, and `3410PQ` MAIN firmware
//! emulator checks for [`super::verify_prewrite_gate`].

use anyhow::{Context, Result};

use crate::constants::{
    AM34_IC7501_TBUS_BASE, AM34_IC7501_TBUS_TX_OFF, AM34_NVRAM_UNCACHED_BASE,
    PANA_DIGADIAG_FRM_SIZE, PANA_DIGADIAG_RAW_VER061_SIZE,
};
use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
use crate::hdd::inspect_main_buffer_capacity;
use crate::re::bgo_unbrick::{PROG061_BASE_VA, Prog061BgoPatchMode, inspect_prog061_bgo_sites};
use crate::re::emu_patch_suite::verify_tbc_passthrough_routines;
use crate::re::main_passthrough::{MAIN_BASE_VA, inspect_main_passthrough_sites};
use crate::re::prog061_uart_monitor::PROG061_UART_MONITOR_VA;
use crate::re::tbc_appliance::build_ic7501_wdt_255s_frame;
use crate::re::unbrick_sd_kit::SdLoaderProfile;

use super::checks::{
    CLEANROOM_ENTRY_RETURN_SENTINEL, CLEANROOM_STACK_TOP_VA, build_report,
    finalize_with_cleanroom_vm_checks, new_cleanroom_vm, verify_vm_lethal_and_ipl_post,
};
use super::{PreWriteGateReport, push_check};

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
