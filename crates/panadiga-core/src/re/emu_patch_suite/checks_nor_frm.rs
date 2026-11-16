//! Spansion NOR CFI/IPL lockout, `.FRM` pack/retarget/unpack, and `PROG061`/`PANAMON1` checks.

use anyhow::{Result, bail};

use crate::constants::{
    AM34_FLASH_UNCACHED_BASE, AM34_NVRAM_UNCACHED_BASE, PANA_EXTRHEAD_DEFAULT_ADLER_STEP,
};
use crate::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, pack_extrhead, pack_frm,
    parse_sd_frm_toc_with_layout, unpack_dvd_frm_to_modules, unpack_extrhead,
    verify_packed_frm_roundtrip,
};
use crate::re::{
    Mn103Fault, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, PROG061_BASE_VA,
    build_prog061_uart_monitor_payload, retarget_frm_toc_bytes,
};

use super::fixtures::{TEST_RAM_WORK_BASE, new_spansion_nor_vm, run_two_byte_branch_probe};
use super::types::{FlashedFrmArtifacts, PatchVerificationCheck};

pub(super) fn verify_frm_pack_retarget_and_flash(
    prog061_patched: &[u8],
    main_patched: &[u8],
) -> Result<FlashedFrmArtifacts> {
    let prog_eh = pack_extrhead(
        prog061_patched,
        "FIRMUP",
        0,
        "PROG061",
        ExtrheadCryptMode::Plain,
        PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
        1,
    )?;
    let main_eh = pack_extrhead(
        main_patched,
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

    let check = PatchVerificationCheck {
        id: "frm_pack_retarget_and_flash".to_string(),
        passed: true,
        detail: format!(
            "EXTRHEAD + PANA_DVD.FRM ({} B) & PANA__SD.FRM ({} B) retargeted & unpacked bit-exact",
            dvd_retargeted.len(),
            sd_retargeted.len()
        ),
    };
    Ok(FlashedFrmArtifacts {
        prog061: flashed_prog061_unpacked.decompressed,
        main: flashed_main_unpacked.decompressed,
        dvd_bytes: dvd_retargeted.len(),
        sd_bytes: sd_retargeted.len(),
        check,
    })
}

pub(super) fn verify_nor_flash_and_cfi() -> Result<(u16, u16, PatchVerificationCheck)> {
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

pub(super) fn verify_bgo_and_panamon_suite(
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
    let built_mon = build_prog061_uart_monitor_payload(mon_va)?;
    if !prog061_patched[mon_off..].starts_with(&built_mon) {
        bail!("Unpacked PROG061 missing expected PANAMON1 payload at slack 0x{mon_off:X}");
    }
    vm.map_segment(PROG061_BASE_VA, prog061_patched.to_vec(), false);

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
