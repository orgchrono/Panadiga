//! End-to-End Patcher + Spansion NOR Flash + `Mn103Vm` Emulator Verification Suite.
//!
//! Validates the complete Panadiga patch lifecycle offline using Matsushita MN103S execution
//! (`Mn103Vm`), simulated 8 MiB Spansion `S29JL064H` NOR flash (CFI + IPL read-only protection +
//! 64 KiB NVRAM), and real `.FRM` pack/retarget/decrypt/unpack roundtrips across:
//! 1. Spansion NOR Flash CFI autoselect & IPL (`0x8000_0000..0x807E_FFFF`) hardware write lockout
//! 2. `PANA_DVD.FRM` (Feistel) & `PANA__SD.FRM` (plaintext 64B) pack -> retarget -> unpack -> flash
//! 3. `PROG061` BGO unbrick bypass + interactive `PANAMON1>` UART0 (`SC0`) command functions
//! 4. `MAIN` TBC (`E-E`) passthrough routines (`FPC` + `ODD` state branch/init)
//! 5. `MAIN` `<deviceinfo>` region/model unlock (`3410PQ` & `3060NP`) + `DEV.BIN` repacking
//! 6. `MAIN` `src/hdd_cmd.c` >200 GB LBA48 resize (500 GB drive clamped vs preserved)

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{
    AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_UNCACHED_BASE, PANA_DEV_BIN_MAGIC,
    PANA_DEV_BIN_SIG, PANA_DEV_BIN_SIZE, PANA_EXTRHEAD_DEFAULT_ADLER_STEP,
};
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, codec::write_padded_ascii, pack_extrhead,
    pack_frm, parse_sd_frm_toc_with_layout, unpack_dvd_frm_to_modules, unpack_extrhead,
    verify_packed_frm_roundtrip,
};
use crate::hdd::capacity_patch::specs::HDD_CMD_CLAMP_WINDOW_TEMPLATE;
use crate::hdd::{
    HddCapacityPatchMode, apply_capacity_patch_in_place, find_hdd_cmd_clamp_site,
    inspect_main_buffer_capacity,
};
use crate::nvram::calculate_hashes;
use crate::re::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, MainPassthroughMode, Mn103Fault, Mn103Vm,
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, OFF_FMU_DEVINFO_3060NP,
    OFF_FMU_DEVINFO_3410PQ, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
    ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_FMU_DEVINFO_3060NP, ORIG_FMU_DEVINFO_3410PQ,
    ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH, ORIG_SUM_BEQ,
    PROG061_BASE_VA, PROG061_EXPECTED_SIZE, Prog061BgoPatchMode, RegionCompatMode,
    apply_main_passthrough_in_place, build_prog061_uart_monitor_payload, patch_dev_bin_bytes,
    patch_main_region_compat_in_place, patch_prog061_bgo_in_place_with_policy,
    retarget_frm_toc_bytes,
};

const TEST_MAIN_HDD_CMD_OFFSET: usize = 0x004C_2600;
const TEST_500GB_BLOCKS_2KB: u32 = 0x0E8D_4A51;
const TEST_200GB_BLOCKS_2KB_M1: u32 = 0x05D2_1DB9;
const TEST_RAM_WORK_BASE: u32 = 0x5000_0000;
const TEST_RAM_WORK_SIZE: usize = 0x2_0000;
const TEST_BSS_BASE: u32 = 0x5311_B000;
const TEST_BSS_SIZE: usize = 0x1000;
const TEST_PHYS_BLOCKS_VA: u32 = 0x5311_BC80;
const TEST_RET_SENTINEL_VA: u32 = 0x577F_BEEF;

/// Individual verification stage outcome produced by [`run_nor_emu_patch_verification_suite`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchVerificationCheck {
    pub id: String,
    pub passed: bool,
    pub detail: String,
}

/// Complete report covering `.FRM` packing, Spansion NOR simulation, and MN103S routine execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NorEmuPatchSuiteReport {
    pub source_mode: String,
    pub cfi_manufacturer_id_hex: String,
    pub cfi_device_id_hex: String,
    pub frm_dvd_bytes: usize,
    pub frm_sd_bytes: usize,
    pub prog061_sha256: String,
    pub main_sha256: String,
    pub checks: Vec<PatchVerificationCheck>,
    pub all_passed: bool,
}

fn build_synthetic_prog061() -> Vec<u8> {
    let mut img = vec![0xFFu8; PROG061_EXPECTED_SIZE];
    img[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + ORIG_SUM_BEQ.len()].copy_from_slice(&ORIG_SUM_BEQ);
    img[OFF_DEVINFO_CHECK_BEQ..OFF_DEVINFO_CHECK_BEQ + ORIG_DEVINFO_BEQ.len()]
        .copy_from_slice(&ORIG_DEVINFO_BEQ);
    img[OFF_BGO_INIT_FAIL_HOOK..OFF_BGO_INIT_FAIL_HOOK + ORIG_A2CD_HOOK.len()]
        .copy_from_slice(&ORIG_A2CD_HOOK);
    img
}

fn build_synthetic_main_3410pq() -> Vec<u8> {
    let mut img = vec![0xCBu8; EXPECTED_MAIN_3410PQ_SIZE];
    img[OFF_FPC_STATUS_CTRL..OFF_FPC_STATUS_CTRL + ORIG_FPC_STATUS_CTRL.len()]
        .copy_from_slice(&ORIG_FPC_STATUS_CTRL);
    img[OFF_ODD_STATE_BRANCH..OFF_ODD_STATE_BRANCH + ORIG_ODD_STATE_BRANCH.len()]
        .copy_from_slice(&ORIG_ODD_STATE_BRANCH);
    img[OFF_ODD_INIT_STATUS..OFF_ODD_INIT_STATUS + ORIG_ODD_INIT_STATUS.len()]
        .copy_from_slice(&ORIG_ODD_INIT_STATUS);
    img[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + ORIG_FMU_DEVINFO_3410PQ.len()]
        .copy_from_slice(&ORIG_FMU_DEVINFO_3410PQ);
    img[TEST_MAIN_HDD_CMD_OFFSET..TEST_MAIN_HDD_CMD_OFFSET + HDD_CMD_CLAMP_WINDOW_TEMPLATE.len()]
        .copy_from_slice(&HDD_CMD_CLAMP_WINDOW_TEMPLATE);
    img
}

fn new_spansion_nor_vm() -> Mn103Vm {
    let mut vm = Mn103Vm::new();
    let nor_image = vec![0xA5u8; AM34_FLASH_SIZE as usize];
    vm.map_segment(AM34_FLASH_UNCACHED_BASE, nor_image, true);
    vm.map_segment(TEST_RAM_WORK_BASE, vec![0u8; TEST_RAM_WORK_SIZE], true);
    vm.map_segment(TEST_BSS_BASE, vec![0u8; TEST_BSS_SIZE], true);
    vm
}

fn verify_nor_flash_and_cfi() -> Result<(u16, u16, PatchVerificationCheck)> {
    let mut vm = new_spansion_nor_vm();
    vm.write_u8(AM34_FLASH_UNCACHED_BASE + 0x0020_0554, 0xAA)?;
    vm.write_u8(AM34_FLASH_UNCACHED_BASE + 0x0020_0AAA, 0x55)?;
    vm.write_u8(AM34_FLASH_UNCACHED_BASE + 0x0020_0554, 0x90)?;
    let mfg = vm.read_u16(AM34_FLASH_UNCACHED_BASE + 0x0020_0000)?;
    let dev = vm.read_u16(AM34_FLASH_UNCACHED_BASE + 0x0020_0002)?;
    vm.write_u8(AM34_FLASH_UNCACHED_BASE, 0xF0)?;
    if mfg != 0x0001 || dev != 0x227E {
        bail!("Unexpected NOR CFI autoselect IDs: mfg=0x{mfg:04X}, dev=0x{dev:04X}");
    }

    if !matches!(
        vm.write_u8(AM34_FLASH_UNCACHED_BASE + 0x007E_FFFF, 0x00),
        Err(Mn103Fault::IplWriteProtected(_))
    ) {
        bail!("IPL boundary write was not blocked with IplWriteProtected");
    }
    vm.write_u8(AM34_NVRAM_UNCACHED_BASE + 0x10, 0x5A)?;
    if vm.read_u8(AM34_NVRAM_UNCACHED_BASE + 0x10)? != 0x5A {
        bail!("Top 64 KiB NVRAM sector failed readback after write");
    }

    Ok((
        mfg,
        dev,
        PatchVerificationCheck {
            id: "spansion_nor_cfi_and_ipl_airgap".to_string(),
            passed: true,
            detail: "CFI=0x0001/0x227E, IPL 0x84000000..0x847EFFFF RO, NVRAM 0x847F0000 RW"
                .to_string(),
        },
    ))
}

fn run_two_byte_branch_probe(branch_2b: &[u8], jump_disp: usize) -> Result<u32> {
    let mut vm = Mn103Vm::new();
    let code_va = TEST_RAM_WORK_BASE + 0x10_0000;
    let mut code = vec![0xCBu8; jump_disp + 16];
    code[0..4].copy_from_slice(&[0x80, 0x01, 0xA0, 0x00]); // mov 1, d0; cmp 0, d0 -> Z=0
    code[4..6].copy_from_slice(branch_2b);
    code[6..9].copy_from_slice(&[0x80, 0xFF, 0xCB]); // fallthrough: mov -1, d0; nop
    let target = 4 + jump_disp;
    code[target..target + 2].copy_from_slice(&[0x00, 0xCB]); // taken: clr d0; nop
    let stop_pass = code_va + target as u32 + 1;

    vm.map_segment(code_va, code, false);
    vm.pc = code_va;
    let mut steps = 0u64;
    while vm.pc != stop_pass && vm.pc != code_va + 8 && steps < 16 {
        vm.step()?;
        steps += 1;
    }
    Ok(vm.d[0])
}

fn verify_bgo_and_panamon_suite(
    prog061_factory: &[u8],
    prog061_patched: &[u8],
) -> Result<PatchVerificationCheck> {
    let sites = [(OFF_BGO_SUM_BEQ, 0x0Au32), (OFF_DEVINFO_CHECK_BEQ, 0x15u32)];
    for (off, disp) in sites {
        let d0_fac = run_two_byte_branch_probe(&prog061_factory[off..off + 2], disp as usize)?;
        let d0_pat = run_two_byte_branch_probe(&prog061_patched[off..off + 2], disp as usize)?;
        if d0_fac != 0xFFFF_FFFF || d0_pat != 0 {
            bail!(
                "BGO branch check failed at 0x{off:06X}: factory D0=0x{d0_fac:X}, patched D0=0x{d0_pat:X}"
            );
        }
    }

    // Boot patched PROG061 from SDRAM (0x50400000) and exercise PANAMON1 commands over SC0 UART.
    let mut vm = new_spansion_nor_vm();
    vm.enable_sc0_uart(&[]);
    let mon_va = crate::re::PROG061_UART_MONITOR_VA;
    let mon_off = crate::re::OFF_UART_MONITOR_SLACK;
    // Validate extracted PROG061 slack contains identical monitor bytes; map full PROG061 image.
    let built_mon = build_prog061_uart_monitor_payload(mon_va)?;
    if !prog061_patched[mon_off..].starts_with(&built_mon) {
        bail!("Unpacked PROG061 missing expected PANAMON1 payload at slack 0x{mon_off:X}");
    }
    vm.map_segment(PROG061_BASE_VA, prog061_patched.to_vec(), false);

    // Prepare callsite + jump target in work RAM.
    let call_va = TEST_RAM_WORK_BASE + 0x1000;
    let rel32 = (mon_va as i64 - call_va as i64) as i32;
    let mut ram_code = [0xCBu8; 16];
    ram_code[0] = 0xDD;
    ram_code[1..5].copy_from_slice(&rel32.to_le_bytes());
    ram_code[5] = 0x00;
    ram_code[6] = 0x00;
    let jump_stub: &[u8] = &[
        0xFC, 0xDE, 0x00, 0x02, 0x00, 0x50, // mov 0x50000200, a2
        0x8A, 0x33, // mov 0x33, d2
        0xF0, 0x5A, // movb d2, (a2)
        0xF0, 0xFC, // rets
    ];
    for (idx, &b) in ram_code.iter().enumerate() {
        vm.write_u8(call_va + idx as u32, b)?;
    }
    for (idx, &b) in jump_stub.iter().enumerate() {
        vm.write_u8(TEST_RAM_WORK_BASE + 0x1800 + idx as u32, b)?;
    }

    vm.sp = TEST_RAM_WORK_BASE + 0xF000;
    vm.pc = call_va;
    vm.queue_uart_rx(
        b"R 84000000,04\r\n\
          W 847EFFFF,11\r\n\
          E 847E0000\r\n\
          W 847F0020,7B\r\n\
          E 847FE000\r\n\
          J 50001800\r\n\
          B",
    );
    vm.run_until(call_va + 7, 250_000)?;
    let tx = vm.take_uart_tx_utf8();
    if vm.d[0] != 0
        || !tx.contains("PANAMON1>")
        || !tx.contains("A5A5A5A5")
        || tx.matches("ERR:E05_IPL_RO").count() != 2
        || !tx.contains("OK:WRITE")
        || !tx.contains("OK:ERASE")
        || !tx.contains("OK:BYPASS")
        || vm.read_u8(TEST_RAM_WORK_BASE + 0x0200)? != 0x33
        || vm.read_u8(AM34_NVRAM_UNCACHED_BASE + 0x20)? != 0x7B
        || vm.read_u8(AM34_NVRAM_UNCACHED_BASE + 0xE000)? != 0xFF
    {
        bail!("PANAMON1 end-to-end command session verification failed (tx={tx})");
    }

    // Exercise `D` (`PANADMP1`) binary NOR streaming header + initial NOR bytes over SC0 TX.
    vm.sp = TEST_RAM_WORK_BASE + 0xF000;
    vm.pc = call_va;
    vm.queue_uart_rx(b"D");
    while vm.uart_tx.len() < 54 && vm.steps < 500_000 {
        vm.step()?;
    }
    let dump_bytes = std::mem::take(&mut vm.uart_tx);
    if !dump_bytes.windows(8).any(|w| w == b"PANADMP1") {
        bail!("PANAMON1 `D` NOR streamer failed to emit PANADMP1 header");
    }

    Ok(PatchVerificationCheck {
        id: "bgo_bypass_and_panamon1_commands".to_string(),
        passed: true,
        detail: "BGO BEQ->BRA bypasses D0=0 + PANAMON1 R/W/E/J/D/B + E05 IPL block verified"
            .to_string(),
    })
}

fn verify_tbc_passthrough_routines(main_patched: &[u8]) -> Result<PatchVerificationCheck> {
    let mut vm = new_spansion_nor_vm();
    vm.map_segment(MAIN_BASE_VA, main_patched.to_vec(), false);

    // Routine 1: FPC status control -> mov 1, d0; rets
    vm.sp = TEST_RAM_WORK_BASE + 0xE000;
    vm.write_u32(vm.sp, TEST_RET_SENTINEL_VA)?;
    vm.pc = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32;
    vm.run_until(TEST_RET_SENTINEL_VA, 16)?;
    if vm.d[0] != 1 {
        bail!("FPC status ctrl returned D0={}, expected 1", vm.d[0]);
    }

    // Routine 2: ODD state branch -> jmp +0x00D0 -> ODD init status -> clr d0
    let odd_end = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32 + 7;
    vm.d[0] = 0xDEAD_BEEF;
    vm.pc = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32;
    vm.run_until(odd_end, 16)?;
    if vm.d[0] != 0 {
        bail!(
            "ODD state branch/init returned D0=0x{:X}, expected 0",
            vm.d[0]
        );
    }

    Ok(PatchVerificationCheck {
        id: "tbc_ee_passthrough_routines".to_string(),
        passed: true,
        detail: "FPC returns D0=1 and ODD branch skips drive init with D0=0 in MAIN SDRAM"
            .to_string(),
    })
}

fn verify_region_compat_and_devbin(
    main_factory: &[u8],
    main_patched: &[u8],
) -> Result<PatchVerificationCheck> {
    // 3410PQ uses main_factory/main_patched; also verify 3060NP `<deviceinfo>` bypass branch.
    for (label, off, fac_pair, pat_pair) in [
        (
            "3410PQ",
            OFF_FMU_DEVINFO_3410PQ,
            &main_factory[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 4],
            &main_patched[OFF_FMU_DEVINFO_3410PQ..OFF_FMU_DEVINFO_3410PQ + 4],
        ),
        (
            "3060NP",
            OFF_FMU_DEVINFO_3060NP,
            &ORIG_FMU_DEVINFO_3060NP[..4],
            &super::PATCH_FMU_DEVINFO_3060NP[..4],
        ),
    ] {
        // Skip first 2 bytes (`cmp 0, d0` = `A0 00`)? Both 3410PQ & 3060NP start with `A0 00 [C8|CA] 05`!
        if fac_pair[..2] != [0xA0, 0x00] || pat_pair[..2] != [0xA0, 0x00] {
            bail!("Unexpected <deviceinfo> compare prefix for {label} @ 0x{off:06X}");
        }
        let d0_fac = run_two_byte_branch_probe(&fac_pair[2..4], 5)?;
        let d0_pat = run_two_byte_branch_probe(&pat_pair[2..4], 5)?;
        if d0_fac != 0xFFFF_FFFF || d0_pat != 0 {
            bail!("Region/model guard {label} mismatch: factory={d0_fac:X} patched={d0_pat:X}");
        }
    }

    // DEV.BIN descriptor repacking validation
    let mut dev = vec![0xFFu8; PANA_DEV_BIN_SIZE];
    dev[0..4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
    write_padded_ascii(&mut dev[0x04..0x0C], "EH58EE", 0);
    dev[0x0C] = b'M';
    dev[0x20..0x2C].copy_from_slice(PANA_DEV_BIN_SIG);
    let (dev_out, dev_rep) = patch_dev_bin_bytes(&dev, "EH55EG", Some('E'))?;
    if dev_out.len() != PANA_DEV_BIN_SIZE || dev_rep.model != "EH55EG" || dev_rep.oem != "E" {
        bail!("DEV.BIN retarget check failed");
    }

    Ok(PatchVerificationCheck {
        id: "region_model_compat_unlock".to_string(),
        passed: true,
        detail: "3410PQ & 3060NP <deviceinfo> guard bypassed (D0=0 on mismatch) + DEV.BIN repacked"
            .to_string(),
    })
}

fn exec_hdd_cmd_block(main_buf: &[u8], site_off: usize, phys_blocks: u32) -> Result<u32> {
    let mut vm = new_spansion_nor_vm();
    vm.map_segment(MAIN_BASE_VA, main_buf.to_vec(), false);
    vm.write_u32(TEST_PHYS_BLOCKS_VA, phys_blocks)?;

    let site_va = MAIN_BASE_VA + site_off as u32;
    // 1. Run `mov slot_200gb, d0; mov d0, (MaxProductLBA)` at `site_va + 0x27..0x33`
    vm.pc = site_va + 0x27;
    vm.run_until(site_va + 0x33, 16)?;
    // 2. Run comparator & clamp at `site_va + 0x4E..0x6F`
    vm.pc = site_va + 0x4E;
    vm.run_until(site_va + 0x6F, 32)?;
    Ok(vm.read_u32(TEST_PHYS_BLOCKS_VA)?)
}

fn verify_hdd_capacity_unlock(
    main_factory: &[u8],
    main_patched: &[u8],
) -> Result<PatchVerificationCheck> {
    let site_off = find_hdd_cmd_clamp_site(main_factory)
        .context("Missing src/hdd_cmd.c clamp site in factory MAIN binary")?;
    let rep_pat = inspect_main_buffer_capacity(
        main_patched,
        MAIN_BASE_VA,
        "EXTRHEAD_UNPACKED",
        None,
        "patched",
    )?;
    if !rep_pat.is_dynamic_unclamped {
        bail!("Patched MAIN binary does not report DynamicUnclamped mode");
    }

    let factory_result = exec_hdd_cmd_block(main_factory, site_off, TEST_500GB_BLOCKS_2KB)?;
    let patched_result = exec_hdd_cmd_block(main_patched, site_off, TEST_500GB_BLOCKS_2KB)?;
    if factory_result != TEST_200GB_BLOCKS_2KB_M1 {
        bail!(
            "Unpatched MAIN failed to clamp 500GB disk to 200GB limit: got 0x{factory_result:08X}"
        );
    }
    if patched_result != TEST_500GB_BLOCKS_2KB - 1 {
        bail!("Patched MAIN failed to preserve 500GB (>200GB) drive: got 0x{patched_result:08X}");
    }

    Ok(PatchVerificationCheck {
        id: "hdd_resize_over_200gb_lba48".to_string(),
        passed: true,
        detail: format!(
            "500GB input (0x{TEST_500GB_BLOCKS_2KB:08X} blks): factory clamped=0x{factory_result:08X} (200GB), patched preserved=0x{patched_result:08X}"
        ),
    })
}

/// Runs the full offline end-to-end Patcher + `.FRM` Pack/Unpack + Spansion NOR + `Mn103Vm` suite.
///
/// When `prog061_path` / `main_path` are omitted, audited real MN103S byte windows are embedded into
/// synthetic `PROG061`/`MAIN3410PQ` layouts so CI verifies deterministically without proprietary ROMs.
pub fn run_nor_emu_patch_verification_suite(
    prog061_path: Option<&Path>,
    main_path: Option<&Path>,
) -> Result<NorEmuPatchSuiteReport> {
    let (prog061_factory, source_prog) = match prog061_path {
        Some(p) => (
            fs::read(p).with_context(|| format!("Reading PROG061 binary {}", p.display()))?,
            p.display().to_string(),
        ),
        None => (
            build_synthetic_prog061(),
            "synthetic_audited_windows".to_string(),
        ),
    };
    let (main_factory, source_main) = match main_path {
        Some(p) => (
            fs::read(p).with_context(|| format!("Reading MAIN binary {}", p.display()))?,
            p.display().to_string(),
        ),
        None => (
            build_synthetic_main_3410pq(),
            "synthetic_audited_windows".to_string(),
        ),
    };
    let source_mode = if prog061_path.is_some() || main_path.is_some() {
        format!("real_binaries ({source_prog} + {source_main})")
    } else {
        source_main
    };

    // 1. Apply all patches cleanly in memory to working copies (CoW invariant preserved).
    let mut prog061_patched = prog061_factory.clone();
    patch_prog061_bgo_in_place_with_policy(
        &mut prog061_patched,
        Prog061BgoPatchMode::UartMonitor,
        false,
    )?;

    let mut main_patched = main_factory.clone();
    apply_main_passthrough_in_place(&mut main_patched, MainPassthroughMode::AvPassthrough)?;
    patch_main_region_compat_in_place(&mut main_patched, RegionCompatMode::BypassModelAndOem)?;
    apply_capacity_patch_in_place(
        &mut main_patched,
        HddCapacityPatchMode::DynamicUnclamped,
        None,
    )?;

    // 2. Wrap into EXTRHEAD, pack into PANA_DVD.FRM (Feistel) & PANA__SD.FRM (64B plaintext), retarget & verify!
    let prog_eh = pack_extrhead(
        &prog061_patched,
        "FIRMUP",
        0,
        "PROG061",
        ExtrheadCryptMode::Plain,
        PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        1,
    )?;
    let main_eh = pack_extrhead(
        &main_patched,
        "FVU",
        0,
        "3410PQ",
        ExtrheadCryptMode::Plain,
        PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        1,
    )?;
    let mod_prog = FrmPackModule::new("PROG061", prog_eh);
    let mod_main = FrmPackModule::new("MAIN3410PQ", main_eh);

    let (dvd_frm, _) = pack_frm(
        &[mod_prog.clone(), mod_main.clone()],
        FrmContainerMode::Dvd,
        None,
    )?;
    let (dvd_retargeted, dvd_ret_rep) =
        retarget_frm_toc_bytes(&dvd_frm, Some("M6931"), Some("EU-UNV"), Some('E'), Some(1))?;
    if dvd_ret_rep.updated_entries != 2 || !dvd_ret_rep.entries.iter().all(|e| e.list_sum_valid) {
        bail!("PANA_DVD.FRM TOC retarget checksum verification failed");
    }
    let _ = verify_packed_frm_roundtrip(&dvd_retargeted, FrmContainerMode::Dvd, 2)?;
    let dvd_modules = unpack_dvd_frm_to_modules(&dvd_retargeted)?;
    let flashed_prog061_unpacked =
        unpack_extrhead(&dvd_modules[0].payload, ExtrheadCryptMode::Plain)?;
    let flashed_main_unpacked = unpack_extrhead(&dvd_modules[1].payload, ExtrheadCryptMode::Plain)?;
    if !flashed_prog061_unpacked.checksum_valid
        || !flashed_main_unpacked.checksum_valid
        || flashed_prog061_unpacked.decompressed != prog061_patched
        || flashed_main_unpacked.decompressed != main_patched
    {
        bail!("PANA_DVD.FRM EXTRHEAD decrypt/unpack mismatch vs patched sources");
    }

    let (sd_frm, _) = pack_frm(&[mod_prog, mod_main], FrmContainerMode::Sd, None)?;
    let (sd_retargeted, sd_ret_rep) =
        retarget_frm_toc_bytes(&sd_frm, Some("M6931"), Some("EG-UNIV"), None, Some(1))?;
    let sd_entries = verify_packed_frm_roundtrip(&sd_retargeted, FrmContainerMode::Sd, 2)?;
    if sd_ret_rep.updated_entries != 2 || parse_sd_frm_toc_with_layout(&sd_retargeted).0.is_none() {
        bail!("PANA__SD.FRM 64B plaintext TOC retarget verification failed");
    }
    let sd_prog = unpack_extrhead(
        &sd_retargeted[sd_entries[0].module_offset as usize
            ..(sd_entries[0].module_offset + sd_entries[0].module_size) as usize],
        ExtrheadCryptMode::Plain,
    )?;
    if !sd_prog.checksum_valid || sd_prog.decompressed != prog061_patched {
        bail!("PANA__SD.FRM slot 0 unpack mismatch vs patched PROG061");
    }

    let frm_check = PatchVerificationCheck {
        id: "frm_pack_retarget_and_flash".to_string(),
        passed: true,
        detail: format!(
            "EXTRHEAD + PANA_DVD.FRM ({} B) & PANA__SD.FRM ({} B) retargeted & unpacked bit-exact",
            dvd_retargeted.len(),
            sd_retargeted.len()
        ),
    };

    // 3. Flash unpacked binaries into NOR/SDRAM inside Mn103Vm & run every patched routine!
    let flashed_prog061 = &flashed_prog061_unpacked.decompressed;
    let flashed_main = &flashed_main_unpacked.decompressed;
    let (mfg, dev, nor_check) = verify_nor_flash_and_cfi()?;
    let bgo_check = verify_bgo_and_panamon_suite(&prog061_factory, flashed_prog061)?;
    let tbc_check = verify_tbc_passthrough_routines(flashed_main)?;
    let region_check = verify_region_compat_and_devbin(&main_factory, flashed_main)?;
    let hdd_check = verify_hdd_capacity_unlock(&main_factory, flashed_main)?;

    let checks = vec![
        nor_check,
        frm_check,
        bgo_check,
        tbc_check,
        region_check,
        hdd_check,
    ];
    let all_passed = checks.iter().all(|c| c.passed);
    let (_, prog_sha) = calculate_hashes(flashed_prog061);
    let (_, main_sha) = calculate_hashes(flashed_main);

    Ok(NorEmuPatchSuiteReport {
        source_mode,
        cfi_manufacturer_id_hex: format!("0x{mfg:04X}"),
        cfi_device_id_hex: format!("0x{dev:04X}"),
        frm_dvd_bytes: dvd_retargeted.len(),
        frm_sd_bytes: sd_retargeted.len(),
        prog061_sha256: prog_sha,
        main_sha256: main_sha,
        checks,
        all_passed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nor_emu_end_to_end_patcher_suite_passes_deterministically() {
        let rep = run_nor_emu_patch_verification_suite(None, None)
            .expect("Offline NOR + MN103Vm patch verification suite should pass");
        assert!(rep.all_passed);
        assert_eq!(rep.checks.len(), 6);
        assert_eq!(rep.cfi_manufacturer_id_hex, "0x0001");
        assert_eq!(rep.cfi_device_id_hex, "0x227E");
    }
}
