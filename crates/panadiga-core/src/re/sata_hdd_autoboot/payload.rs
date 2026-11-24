#![deny(unsafe_code)]

//! MN103S pre-FVU SATA HDD autoboot payload synthesis (escape hatches, mirror reader, bit-walk, teardown).

use anyhow::{Result, bail};

use super::super::eh55_bgo::{NOR_AUTOBOOT_CAPSULE_MAGIC_LE, NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET};
use super::super::mn10300_emit::{emit_and_imm32_d0, emit_ld_w_an_dm, emit_mov_d_to_a};
use super::apply::hdd_autoboot_spec_for_loader_profile;
use super::asm::{
    HddAutobootAsm, LOC_ALIGNED_SIZE, LOC_ATTEMPT, LOC_ENTRY_ADDR, LOC_LBA, LOC_MAX_LBA,
    LOC_MOUNT_ID, LOC_SIZE, LOC_SRC_ADDR,
};
use super::{HddAutobootTrainSpec, PROG_TRAILER_RESERVED_BYTES};

fn emit_escape_hatch_checks(a: &mut HddAutobootAsm, spec: &HddAutobootTrainSpec) {
    // 1. Pre-HDD UART0 escape hatch: poll SC0RXB for 'F'/'f'.
    a.printf0("s_banner");
    a.mov_imm8_d(2, spec.uart_abort_polls);
    a.label("uart_loop");
    a.call_abs(spec.uart0_getc_nb_va);
    a.cmp_imm8_d(0, b'F' as i8);
    a.jcc(0xC8, "uart_abort");
    a.cmp_imm8_d(0, b'f' as i8);
    a.jcc(0xC8, "uart_abort");
    a.mov_imm8_d(0, spec.uart_poll_ticks_10ms);
    a.call_abs(spec.sleep_10ms_va);
    a.add_imm8_d(2, -1);
    a.jcc(0xC9, "uart_loop");
    a.jmp("check_sd");

    a.label("uart_abort");
    a.printf0("s_uart_abort");
    a.jmp("fallback_fvu");

    // 2. Pre-HDD SD escape hatches: FORCEFVU.TXT always, or PANA__SD.FRM under Nor-FROM boot.
    a.label("check_sd");
    a.mov_imm32_d(0, spec.sd_info_bss_va);
    a.call_abs(spec.sd_init_va);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC9, "skip_sd");
    a.call_abs(spec.fat_mount_va);
    a.st_local(0, LOC_MOUNT_ID);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC0, "skip_sd"); // mount_id < 0 => mount failed (0 and >= 0 are valid mount IDs)

    a.mov_str_d(1, "s_force_file");
    a.call_abs(spec.fat_open_va);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC0, "check_norfrom"); // < 0 => not present
    a.call_abs(spec.fat_close_va);
    a.printf0("s_force_hit");
    a.jmp("fallback_fvu");

    a.label("check_norfrom");
    let ram_entry_va = spec
        .fvu_load_uncached_va
        .wrapping_add(spec.payload_offset as u32);
    let nor_entry_va = spec
        .norfrom_fvu_base_va
        .wrapping_add(spec.payload_offset as u32);
    a.ld_abs32_d(ram_entry_va, 0);
    a.ld_abs32_d(nor_entry_va, 1);
    a.cmp_d_to_d0(1);
    a.jcc(0xC8, "norfrom_matched");
    let nor_autoboot_magic_va = spec
        .norfrom_fvu_base_va
        .wrapping_add(NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET as u32);
    a.ld_abs32_d(nor_autoboot_magic_va, 0);
    a.cmp_imm32_d0(NOR_AUTOBOOT_CAPSULE_MAGIC_LE);
    a.jcc(0xC9, "skip_sd");
    a.label("norfrom_matched");
    a.ld_local(LOC_MOUNT_ID, 0);
    a.mov_str_d(1, "s_pana_sd");
    a.call_abs(spec.fat_open_va);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC0, "skip_sd");
    a.call_abs(spec.fat_close_va);
    a.printf0("s_norfrom_sd_hit");
    a.jmp("fallback_fvu");
    a.label("skip_sd");
}

fn emit_hdd_mirror_loader(a: &mut HddAutobootAsm, spec: &HddAutobootTrainSpec) {
    a.lea_a3_disp_d(LOC_MAX_LBA, 0);
    a.call_abs(spec.hdd_max_lba_va);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC8, "hdd_lba_ok");
    a.printf0("s_hdd_fail");
    a.jmp("fallback_fvu");

    a.label("hdd_lba_ok");
    a.ld_local(LOC_MAX_LBA, 0);
    emit_and_imm32_d0(&mut a.code, 0xFFFF_FFFC);
    a.st_local(0, LOC_MAX_LBA);
    a.mov_d_to_d(0, 1);
    a.printf0("s_hdd_lba");

    a.clr_d(0);
    a.st_local(0, LOC_ATTEMPT);
    a.mov_imm32_d(0, spec.hdd_primary_lba);
    a.st_local(0, LOC_LBA);

    a.label("try_mirror");
    a.mov_imm32_d(0, spec.hdd_probe_sectors);
    a.st_d_sp(0, 0x0C);
    a.clr_d(0);
    a.ld_local(LOC_LBA, 1);
    a.call_abs(spec.hdd_read_va);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC9, "next_mirror");

    a.mov_imm32_d(0, spec.hdd_dma_buf_va);
    a.mov_imm32_d(1, spec.feistel_crypt_span);
    a.call_abs(spec.feistel_decrypt_va);

    a.mov_imm32_d(0, spec.hdd_dma_buf_va);
    a.call_abs(spec.extrhead_check_hdr_va);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC8, "next_mirror");

    a.mov_imm32_d(0, spec.hdd_dma_buf_va);
    a.call_abs(spec.extrhead_get_entry_va);
    a.st_local(0, LOC_ENTRY_ADDR);
    a.cmp_imm32_d0(spec.main_va_min);
    a.jcc(0xC4, "next_mirror");
    a.cmp_imm32_d0(spec.main_va_max);
    a.jcc(0xC5, "next_mirror");

    a.mov_imm32_d(0, spec.hdd_dma_buf_va);
    a.call_abs(spec.extrhead_get_src_va);
    a.st_local(0, LOC_SRC_ADDR);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC8, "next_mirror"); // Reject EXTRHEAD+0x1C == 0 so Fvu can self-heal Boot 1

    a.mov_imm32_d(0, spec.hdd_dma_buf_va);
    a.call_abs(spec.extrhead_is_comp_va);
    a.cmp_imm8_d(0, 1);
    a.jcc(0xC9, "next_mirror");

    a.mov_imm32_d(0, spec.hdd_dma_buf_va);
    a.clr_d(1);
    a.call_abs(spec.extrhead_get_size_va);
    a.st_local(0, LOC_SIZE);
    a.add_imm8_d(0, 7);
    emit_and_imm32_d0(&mut a.code, 0xFFFF_FFF8);
    a.st_local(0, LOC_ALIGNED_SIZE);

    a.add_imm32_d(0, 511);
    a.lsr_imm8_d0(9);
    a.cmp_imm32_d0(spec.hdd_probe_sectors);
    a.jcc(0xC7, "next_mirror"); // <= probe sectors
    a.sub_imm32_d0(spec.hdd_probe_sectors);
    a.cmp_imm32_d0(0x0001_0000);
    a.jcc(0xC5, "next_mirror"); // > 0x10000 sectors

    a.st_d_sp(0, 0x0C);
    a.mov_imm32_d(0, spec.feistel_crypt_span);
    a.ld_local(LOC_LBA, 1);
    a.add_imm32_d(1, spec.hdd_probe_sectors);
    a.call_abs(spec.hdd_read_va);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC9, "next_mirror");

    // Feistel-decrypt trailing span: aligned_size + (hdd_dma_buf_va - feistel_crypt_span).
    a.ld_local(LOC_ALIGNED_SIZE, 0);
    a.add_imm32_d(0, spec.hdd_dma_buf_va.wrapping_sub(spec.feistel_crypt_span));
    a.mov_imm32_d(1, spec.feistel_crypt_span);
    a.call_abs(spec.feistel_decrypt_va);

    a.ld_local(LOC_SIZE, 0);
    a.st_d_sp(0, 0x0C);
    a.ld_local(LOC_SRC_ADDR, 0);
    a.mov_imm32_d(1, spec.hdd_dma_buf_va);
    a.call_abs(spec.memcpy_va);

    a.mov_imm32_d(0, spec.hdd_dma_buf_va);
    a.call_abs(spec.extrhead_uncomp_va);
    a.cmp_imm8_d(0, 1);
    a.jcc(0xC9, "next_mirror");

    // 32-word power-of-two bit-walk at *(entry_addr + 8), matching Loader validation.
    a.ld_local(LOC_ENTRY_ADDR, 0);
    a.add_imm8_d(0, 8);
    emit_mov_d_to_a(&mut a.code, 0, 0);
    emit_ld_w_an_dm(&mut a.code, 0, 0);
    a.cmp_imm32_d0(spec.main_va_min);
    a.jcc(0xC4, "next_mirror");
    a.cmp_imm32_d0(spec.main_va_max);
    a.jcc(0xC5, "next_mirror");
    emit_mov_d_to_a(&mut a.code, 0, 0);
    emit_and_imm32_d0(&mut a.code, 3);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC9, "next_mirror");

    a.mov_imm8_d(2, 1);
    a.mov_imm8_d(3, 32);
    a.label("bitwalk_loop");
    emit_ld_w_an_dm(&mut a.code, 0, 0);
    a.cmp_d_to_d0(2);
    a.jcc(0xC9, "next_mirror");
    a.code.push(0x50); // INC4 A0
    a.asl_imm8_d(2, 1);
    a.add_imm8_d(3, -1);
    a.jcc(0xC9, "bitwalk_loop");
    a.jmp("boot_main");

    a.label("next_mirror");
    a.ld_local(LOC_ATTEMPT, 0);
    a.cmp_imm8_d(0, 0);
    a.jcc(0xC9, "all_failed");
    a.mov_imm8_d(0, 1);
    a.st_local(0, LOC_ATTEMPT);
    a.ld_local(LOC_MAX_LBA, 0);
    a.cmp_imm32_d0(spec.hdd_mirror_lba_delta - 1);
    a.jcc(0xC7, "all_failed");
    a.sub_imm32_d0(spec.hdd_mirror_lba_delta);
    a.st_local(0, LOC_LBA);
    a.jmp("try_mirror");

    a.label("all_failed");
    a.printf0("s_main_bad");
    a.jmp("fallback_fvu");
}

/// Assembles the self-contained MN103S pre-FVU SATA HDD autoboot payload for an explicit train spec.
pub fn assemble_hdd_autoboot_payload_for_spec(spec: &HddAutobootTrainSpec) -> Result<Vec<u8>> {
    let payload_va = spec.base_va.wrapping_add(spec.payload_offset as u32);
    let slack_limit = spec
        .expected_size
        .saturating_sub(PROG_TRAILER_RESERVED_BYTES);
    let max_slack = slack_limit
        .checked_sub(spec.payload_offset)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "HDD autoboot payload_offset 0x{:X} exceeds usable slack end 0x{:X} for {}",
                spec.payload_offset,
                slack_limit,
                spec.train_id
            )
        })?;

    let mut a = HddAutobootAsm::new(payload_va, spec.uart0_printf_va);
    a.code
        .extend_from_slice(&[0xCF, 0xF0, 0x3F, 0xF8, 0xFE, 0xC0]);

    emit_escape_hatch_checks(&mut a, spec);
    emit_hdd_mirror_loader(&mut a, spec);

    a.label("boot_main");
    a.ld_local(LOC_ENTRY_ADDR, 1);
    a.printf0("s_boot_main");
    a.mov_imm32_d(0, spec.dcache_purge_base);
    a.mov_imm32_d(1, spec.dcache_purge_len);
    a.call_abs(spec.dcache_purge_va);
    a.call_abs(spec.stop_timer_irq_va);
    a.call_abs(spec.disable_irq_va);
    a.ld_local(LOC_ENTRY_ADDR, 0);
    emit_mov_d_to_a(&mut a.code, 0, 2);
    a.code.extend_from_slice(&[0xF0, 0xF2]); // CALLS (A2)

    a.label("fallback_fvu");
    a.mov_imm8_d(0, 0x10);
    a.call_abs(spec.check_update_flag_va);
    a.code.extend_from_slice(&[0xF2, 0xFC, 0xDF, 0xF0, 0x10]);

    a.intern_cstr("s_banner", b"[HDD_AUTOBOOT] SATA HDD boot ('F'->FVU)...\n");
    a.intern_cstr("s_uart_abort", b"[HDD_AUTOBOOT] UART 'F' -> FVU\n");
    a.intern_cstr("s_force_file", b"FORCEFVU.TXT");
    a.intern_cstr("s_force_hit", b"[HDD_AUTOBOOT] FORCEFVU.TXT -> FVU\n");
    a.intern_cstr("s_pana_sd", b"PANA__SD.FRM");
    a.intern_cstr("s_norfrom_sd_hit", b"[HDD_AUTOBOOT] Nor-FROM+SD -> FVU\n");
    a.intern_cstr("s_hdd_fail", b"[HDD_AUTOBOOT] HDD_MAX_LBA failed -> FVU\n");
    a.intern_cstr("s_hdd_lba", b"[HDD_AUTOBOOT] HDD MAX LBA=0x%X\n");
    a.intern_cstr("s_main_bad", b"[HDD_AUTOBOOT] HDD MAIN invalid -> FVU\n");
    a.intern_cstr("s_boot_main", b"[HDD_AUTOBOOT] Boot MAIN @ 0x%08X\n");

    let payload = a.finalize()?;
    if payload.len() > max_slack {
        bail!(
            "HDD autoboot payload ({} B) exceeds '{}' slack budget ({} B)",
            payload.len(),
            spec.train_id,
            max_slack
        );
    }
    Ok(payload)
}

/// Assembles the self-contained MN103S pre-FVU SATA HDD autoboot payload for `Loader ver0.61` from SSOT.
pub fn assemble_prog061_hdd_autoboot_payload() -> Result<Vec<u8>> {
    let spec = hdd_autoboot_spec_for_loader_profile("ver061")?;
    assemble_hdd_autoboot_payload_for_spec(&spec)
}
