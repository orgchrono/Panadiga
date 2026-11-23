#![deny(unsafe_code)]

//! Opt-in, fail-closed firmware-hosted Stage-2 HDD boot hook for SATA-to-PATA bridges
//! (e.g. StarTech `IDE2SAT2` / Marvell `88SA8052`).
//!
//! When the cold NOR bootloader hardcodes a short post-`RESET-` delay (`2 ms` in
//! `Loader ver0.61`), cold/warm boots time out against slow SATA-to-PATA bridge silicon
//! before loading `MAIN` from the HDD.
//!
//! When paired with [`super::sata_bridge_fix`] (`1000 ms` post-`RESET-` wait in RAM `PROG`),
//! this hook intercepts `call check_update_flag` inside the RAM updater payload to:
//! 1. Enforce three explicit pre-HDD recovery escape hatches (UART0 `'F'`/`'f'`, SD
//!    `FORCEFVU.TXT`, and `Nor-FROM` boot with SD `PANA__SD.FRM` present) so stock
//!    `Fvu` recovery remains reachable even if the HDD holds an incompatible image;
//! 2. Query `HDD_MAX_LBA` and attempt primary (`0x10000`) then outer mirror
//!    (`max_lba - 0x20000`) HDD `EXTRHEAD` load/Feistel decrypt/Adler-32 uncompress,
//!    rejecting null `EXTRHEAD+0x1C` (`src_addr == 0`);
//! 3. Validate the exact 32-word power-of-two bit-walk at `*(entry + 8)`;
//! 4. Purge D-cache, execute exact `Loader`-equivalent timer/IRQ teardown
//!    (with ZERO NOR/BGO writes), and jump to `MAIN` (`CALLS (A2)`).

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use super::bgo_unbrick::{PROG061_BASE_VA, PROG061_EXPECTED_SIZE};
use super::mn10300_emit::{
    emit_and_imm32_d0, emit_call32_abs, emit_cmp_imm, emit_ld_w_an_dm, emit_mov_d_to_a,
    emit_mov_imm32, patch_s16_rel,
};
use super::patch_spec::{
    PatchApplyAction, PatchSpec, apply_patch_specs_in_place, classify_patch_site,
};
use super::sata_bridge_fix::{
    SATA_BRIDGE_STATE_WAIT_1000MS, inspect_prog_sata_bridge_reset_site_for_profile,
    inspect_prog061_sata_bridge_reset_site,
};
use super::ssot_patches::stage2_boot_spec_for_loader;

/// File offset inside raw `PROG061` (`0x40000` B, `VA 0x5040_50D3`) hooked for Stage-2 HDD boot.
pub const OFF_PROG061_STAGE2_HOOK_SITE: usize = 0x0000_50D3;
/// Exact 7-byte factory instruction at `0x5040_50D3` (`CALL 0x5040_9FDB,[],0`).
pub const ORIG_PROG061_STAGE2_HOOK_BYTES: [u8; 7] = [0xDD, 0x08, 0x4F, 0x00, 0x00, 0x00, 0x00];
/// File offset inside `PROG061` zero-padded tail slack (`0x39494..0x3FFFF`) hosting Stage-2 code.
pub const PROG061_STAGE2_PAYLOAD_OFFSET: usize = 0x0003_9600;
/// MN103S virtual address of the embedded Stage-2 entry (`0x5043_9600`).
pub const PROG061_STAGE2_PAYLOAD_VA: u32 = PROG061_BASE_VA + PROG061_STAGE2_PAYLOAD_OFFSET as u32;

pub const PROG061_STAGE2_STATE_FACTORY: &str = "FACTORY";
pub const PROG061_STAGE2_STATE_ENABLED: &str = "STAGE2_HDD_BOOT";
pub const PROG061_STAGE2_STATE_MISMATCH: &str = "UNKNOWN_OR_MISMATCH";

const PROG_SLACK_FILL_BYTE: u8 = 0xFF;
const PROG_TRAILER_RESERVED_BYTES: usize = 4;

const LOC_MAX_LBA: i8 = -0x04;
const LOC_LBA: i8 = -0x08;
const LOC_SIZE: i8 = -0x0C;
const LOC_ALIGNED_SIZE: i8 = -0x10;
const LOC_ENTRY_ADDR: i8 = -0x14;
const LOC_SRC_ADDR: i8 = -0x18;
const LOC_ATTEMPT: i8 = -0x1C;
const LOC_MOUNT_ID: i8 = -0x20;

/// Declarative per-firmware-train parameters required to assemble and verify the Stage-2 HDD boot hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage2BootTrainSpec {
    pub train_id: &'static str,
    pub base_va: u32,
    pub expected_size: usize,
    pub hook_offset: usize,
    pub hook_expect: [u8; 7],
    pub payload_offset: usize,
    pub require_reset_fix_1000ms: bool,
    pub uart_abort_polls: i8,
    pub uart_poll_ticks_10ms: i8,
    pub hdd_primary_lba: u32,
    pub hdd_mirror_lba_delta: u32,
    pub hdd_probe_sectors: u32,
    pub feistel_crypt_span: u32,
    pub main_va_min: u32,
    pub main_va_max: u32,
    pub dcache_purge_base: u32,
    pub dcache_purge_len: u32,
    pub sd_info_bss_va: u32,
    pub hdd_dma_buf_va: u32,
    pub fvu_load_uncached_va: u32,
    pub norfrom_fvu_base_va: u32,
    pub dcache_purge_va: u32,
    pub uart0_getc_nb_va: u32,
    pub uart0_printf_va: u32,
    pub check_update_flag_va: u32,
    pub hdd_max_lba_va: u32,
    pub hdd_read_va: u32,
    pub sd_init_va: u32,
    pub stop_timer_irq_va: u32,
    pub feistel_decrypt_va: u32,
    pub extrhead_get_size_va: u32,
    pub extrhead_check_hdr_va: u32,
    pub extrhead_is_comp_va: u32,
    pub extrhead_get_src_va: u32,
    pub extrhead_get_entry_va: u32,
    pub extrhead_uncomp_va: u32,
    pub fat_mount_va: u32,
    pub fat_open_va: u32,
    pub fat_close_va: u32,
    pub disable_irq_va: u32,
    pub memcpy_va: u32,
    pub sleep_10ms_va: u32,
}

/// Verified `Loader ver0.61` (`PROG061 [05/11/01]` -> `MAIN 0x5700_0000`) Stage-2 descriptor.
pub const VER061_STAGE2_BOOT_SPEC: Stage2BootTrainSpec = Stage2BootTrainSpec {
    train_id: "loader_ver061_toc48_main57",
    base_va: PROG061_BASE_VA,
    expected_size: PROG061_EXPECTED_SIZE,
    hook_offset: OFF_PROG061_STAGE2_HOOK_SITE,
    hook_expect: ORIG_PROG061_STAGE2_HOOK_BYTES,
    payload_offset: PROG061_STAGE2_PAYLOAD_OFFSET,
    require_reset_fix_1000ms: true,
    uart_abort_polls: 50,
    uart_poll_ticks_10ms: 1,
    hdd_primary_lba: 0x0001_0000,
    hdd_mirror_lba_delta: 0x0002_0000,
    hdd_probe_sectors: 200,
    feistel_crypt_span: 0x0001_9000,
    main_va_min: 0x5000_0000,
    main_va_max: 0x57FF_FFFF,
    dcache_purge_base: 0x5700_0000,
    dcache_purge_len: 0x0100_0000,
    sd_info_bss_va: 0x5048_0EDC,
    hdd_dma_buf_va: 0x90C0_0000,
    fvu_load_uncached_va: 0x9040_0000,
    norfrom_fvu_base_va: 0x8404_0000,
    dcache_purge_va: 0x5040_08A6,
    uart0_getc_nb_va: 0x5040_4CEF,
    uart0_printf_va: 0x5040_4DCD,
    check_update_flag_va: 0x5040_9FDB,
    hdd_max_lba_va: 0x5040_CEAB,
    hdd_read_va: 0x5040_CFF5,
    sd_init_va: 0x5040_D24C,
    stop_timer_irq_va: 0x5040_D59F,
    feistel_decrypt_va: 0x5040_E9AE,
    extrhead_get_size_va: 0x5041_08A6,
    extrhead_check_hdr_va: 0x5041_091B,
    extrhead_is_comp_va: 0x5041_098E,
    extrhead_get_src_va: 0x5041_09B3,
    extrhead_get_entry_va: 0x5041_09E7,
    extrhead_uncomp_va: 0x5041_0A8E,
    fat_mount_va: 0x5041_174E,
    fat_open_va: 0x5041_1923,
    fat_close_va: 0x5041_1A27,
    disable_irq_va: 0x5041_6765,
    memcpy_va: 0x5042_D2D2,
    sleep_10ms_va: 0x5043_1303,
};

/// Computes the 7-byte MN103S `CALL d32rel, [], 0` instruction hooking `hook_offset` to `payload_offset`.
#[must_use]
pub const fn compute_stage2_hook_bytes(spec: &Stage2BootTrainSpec) -> [u8; 7] {
    let hook_va = spec.base_va.wrapping_add(spec.hook_offset as u32);
    let target_va = spec.base_va.wrapping_add(spec.payload_offset as u32);
    let rel = target_va.wrapping_sub(hook_va).to_le_bytes();
    [0xDD, rel[0], rel[1], rel[2], rel[3], 0x00, 0x00]
}

/// Inspection status for `PROG061` Stage-2 HDD boot integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prog061Stage2BootStatus {
    pub hook_va_hex: String,
    pub payload_va_hex: String,
    pub payload_size_bytes: usize,
    pub state: String,
    pub payload_present: bool,
    pub hook_bytes_hex: String,
}

const fn invert_cc(cc_op: u8) -> u8 {
    match cc_op {
        0xC0..=0xC7 => cc_op ^ 0x02,
        0xC8..=0xC9 => cc_op ^ 0x01,
        _ => cc_op ^ 0x01,
    }
}

struct Stage2Asm {
    base_va: u32,
    printf_va: u32,
    code: Vec<u8>,
    labels: BTreeMap<&'static str, usize>,
    jmp16_fixups: Vec<(usize, &'static str)>,
    imm32_fixups: Vec<(usize, &'static str)>,
    strings: Vec<(&'static str, &'static [u8])>,
}

impl Stage2Asm {
    fn new(base_va: u32, printf_va: u32) -> Self {
        Self {
            base_va,
            printf_va,
            code: Vec::with_capacity(768),
            labels: BTreeMap::new(),
            jmp16_fixups: Vec::new(),
            imm32_fixups: Vec::new(),
            strings: Vec::new(),
        }
    }

    fn label(&mut self, name: &'static str) {
        self.labels.insert(name, self.code.len());
    }

    fn intern_cstr(&mut self, name: &'static str, s: &'static [u8]) {
        self.strings.push((name, s));
    }

    fn clr_d(&mut self, d: u8) {
        self.code.push((d & 3) << 2);
    }

    fn mov_d_to_d(&mut self, src_d: u8, dst_d: u8) {
        self.code.push(0x80 | ((src_d & 3) << 2) | (dst_d & 3));
    }

    fn mov_imm8_d(&mut self, d: u8, imm8: i8) {
        let r = d & 3;
        self.code.push(0x80 | (r << 2) | r);
        self.code.push(imm8 as u8);
    }

    fn mov_imm32_d(&mut self, d: u8, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xCC + (d & 3), imm);
    }

    #[cfg(test)]
    fn mov_imm32_a(&mut self, a: u8, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xDC + (a & 3), imm);
    }

    fn mov_str_d(&mut self, d: u8, str_label: &'static str) {
        let imm_off = emit_mov_imm32(&mut self.code, 0xCC + (d & 3), 0);
        self.imm32_fixups.push((imm_off, str_label));
    }

    fn ld_local(&mut self, disp8: i8, d: u8) {
        self.code
            .extend_from_slice(&[0xF8, 0x03 | ((d & 3) << 2), disp8 as u8]);
    }

    fn st_local(&mut self, d: u8, disp8: i8) {
        self.code
            .extend_from_slice(&[0xF8, 0x13 | ((d & 3) << 2), disp8 as u8]);
    }

    fn lea_a3_disp_d(&mut self, disp8: i8, d: u8) {
        let r = d & 3;
        self.code
            .extend_from_slice(&[0xF1, 0xDC | r, 0x28 + r, disp8 as u8]);
    }

    fn st_d_sp(&mut self, d: u8, u8_off: u8) {
        self.code
            .extend_from_slice(&[0x42 | ((d & 3) << 2), u8_off]);
    }

    fn ld_abs32_d(&mut self, addr: u32, d: u8) {
        let _ = emit_mov_imm32(&mut self.code, 0xA4 + (d & 3), addr);
    }

    fn cmp_imm8_d(&mut self, d: u8, imm8: i8) {
        emit_cmp_imm(&mut self.code, d, imm8);
    }

    fn cmp_imm32_d0(&mut self, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xC8, imm);
    }

    fn cmp_d_to_d0(&mut self, src_d: u8) {
        self.code.push(0xA0 | ((src_d & 3) << 2));
    }

    fn add_imm8_d(&mut self, d: u8, imm8: i8) {
        self.code.extend_from_slice(&[0x28 + (d & 3), imm8 as u8]);
    }

    fn add_imm32_d(&mut self, d: u8, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xC0 + (d & 3), imm);
    }

    fn sub_imm32_d0(&mut self, imm: u32) {
        let _ = emit_mov_imm32(&mut self.code, 0xC4, imm);
    }

    fn asl_imm8_d(&mut self, d: u8, k: u8) {
        self.code.extend_from_slice(&[0xF8, 0xC0 | (d & 3), k]);
    }

    fn lsr_imm8_d0(&mut self, k: u8) {
        self.code.extend_from_slice(&[0xF8, 0xC4, k]);
    }

    fn call_abs(&mut self, target_va: u32) {
        emit_call32_abs(&mut self.code, self.base_va, target_va);
    }

    fn printf0(&mut self, str_label: &'static str) {
        self.mov_str_d(0, str_label);
        self.call_abs(self.printf_va);
    }

    fn jmp(&mut self, target: &'static str) {
        let off = self.code.len();
        self.code.extend_from_slice(&[0xCC, 0x00, 0x00]);
        self.jmp16_fixups.push((off, target));
    }

    /// Emits inverted short `B!cc +5` guarding a 3-byte `JMP16` so branches handle arbitrary spans.
    fn jcc(&mut self, cc_op: u8, target: &'static str) {
        self.code.extend_from_slice(&[invert_cc(cc_op), 0x05]);
        self.jmp(target);
    }

    fn finalize(mut self) -> Result<Vec<u8>> {
        for (name, bytes) in self.strings {
            let off = self.code.len();
            self.labels.insert(name, off);
            self.code.extend_from_slice(bytes);
            self.code.push(0x00);
        }
        while (self.code.len() & 3) != 0 {
            self.code.push(0x00);
        }
        for (insn_off, target) in self.jmp16_fixups {
            let tgt_off = *self
                .labels
                .get(target)
                .ok_or_else(|| anyhow::anyhow!("Unknown Stage-2 branch target '{target}'"))?;
            patch_s16_rel(&mut self.code, insn_off, tgt_off, target)?;
        }
        for (imm_off, target) in self.imm32_fixups {
            let tgt_off = *self
                .labels
                .get(target)
                .ok_or_else(|| anyhow::anyhow!("Unknown Stage-2 string label '{target}'"))?;
            let va = self.base_va + tgt_off as u32;
            LittleEndian::write_u32(&mut self.code[imm_off..imm_off + 4], va);
        }
        Ok(self.code)
    }
}

fn emit_escape_hatch_checks(a: &mut Stage2Asm, spec: &Stage2BootTrainSpec) {
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
    a.jcc(0xC9, "skip_sd"); // BNE => Nor-FROM does not host Stage-2 (Portao 3 SD boot or stock NOR fvu.exe)
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

fn emit_hdd_mirror_loader(a: &mut Stage2Asm, spec: &Stage2BootTrainSpec) {
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

/// Assembles the self-contained MN103S Stage-2 HDD bootloader for an explicit train spec.
pub fn assemble_stage2_payload_for_spec(spec: &Stage2BootTrainSpec) -> Result<Vec<u8>> {
    let payload_va = spec.base_va.wrapping_add(spec.payload_offset as u32);
    let slack_limit = spec
        .expected_size
        .saturating_sub(PROG_TRAILER_RESERVED_BYTES);
    let max_slack = slack_limit
        .checked_sub(spec.payload_offset)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Stage-2 payload_offset 0x{:X} exceeds usable slack end 0x{:X} for {}",
                spec.payload_offset,
                slack_limit,
                spec.train_id
            )
        })?;

    let mut a = Stage2Asm::new(payload_va, spec.uart0_printf_va);
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

    a.intern_cstr("s_banner", b"[STAGE2] SATA HDD boot ('F'->FVU)...\n");
    a.intern_cstr("s_uart_abort", b"[STAGE2] UART 'F' -> FVU\n");
    a.intern_cstr("s_force_file", b"FORCEFVU.TXT");
    a.intern_cstr("s_force_hit", b"[STAGE2] FORCEFVU.TXT -> FVU\n");
    a.intern_cstr("s_pana_sd", b"PANA__SD.FRM");
    a.intern_cstr("s_norfrom_sd_hit", b"[STAGE2] Nor-FROM+SD -> FVU\n");
    a.intern_cstr("s_hdd_fail", b"[STAGE2] HDD_MAX_LBA failed -> FVU\n");
    a.intern_cstr("s_hdd_lba", b"[STAGE2] HDD MAX LBA=0x%X\n");
    a.intern_cstr("s_main_bad", b"[STAGE2] HDD MAIN invalid -> FVU\n");
    a.intern_cstr("s_boot_main", b"[STAGE2] Boot MAIN @ 0x%08X\n");

    let payload = a.finalize()?;
    if payload.len() > max_slack {
        bail!(
            "Stage-2 payload ({} B) exceeds '{}' slack budget ({} B)",
            payload.len(),
            spec.train_id,
            max_slack
        );
    }
    Ok(payload)
}

/// Assembles the self-contained MN103S Stage-2 HDD bootloader for `Loader ver0.61`.
pub fn assemble_prog061_stage2_payload() -> Result<Vec<u8>> {
    assemble_stage2_payload_for_spec(&VER061_STAGE2_BOOT_SPEC)
}

fn build_hook_spec<'a>(spec: &'a Stage2BootTrainSpec, hook_patch: &'a [u8; 7]) -> PatchSpec<'a> {
    PatchSpec {
        name: "stage2_hdd_boot_hook",
        description: "Pre-FVU Stage-2 SATA HDD boot hook",
        va: spec.base_va + spec.hook_offset as u32,
        base_va: spec.base_va,
        expect: &spec.hook_expect,
        replace: hook_patch,
        required_len: Some(spec.expected_size),
        input_sha256: None,
    }
}

/// Inspects an updater RAM image against a `Stage2BootTrainSpec` (`fail-closed` bounds).
pub fn inspect_stage2_boot_site_for_spec(
    buf: &[u8],
    spec: &Stage2BootTrainSpec,
) -> Result<Prog061Stage2BootStatus> {
    let payload = assemble_stage2_payload_for_spec(spec)?;
    let hook_patch = compute_stage2_hook_bytes(spec);
    let hook_spec = build_hook_spec(spec, &hook_patch);
    let cls = classify_patch_site(buf, &hook_spec, PROG061_STAGE2_STATE_ENABLED)?;
    let end = spec.payload_offset + payload.len();
    let tail_slice = buf.get(spec.payload_offset..end).unwrap_or_default();
    let payload_present = tail_slice == payload.as_slice();
    let state = match cls.state {
        "FACTORY" if !payload_present => PROG061_STAGE2_STATE_FACTORY,
        PROG061_STAGE2_STATE_ENABLED if payload_present => PROG061_STAGE2_STATE_ENABLED,
        _ => PROG061_STAGE2_STATE_MISMATCH,
    };
    Ok(Prog061Stage2BootStatus {
        hook_va_hex: format!("0x{:08X}", cls.va),
        payload_va_hex: format!(
            "0x{:08X}",
            spec.base_va.wrapping_add(spec.payload_offset as u32)
        ),
        payload_size_bytes: payload.len(),
        state: state.to_string(),
        payload_present,
        hook_bytes_hex: hex::encode(cls.actual_bytes),
    })
}

/// Resolves a verified [`Stage2BootTrainSpec`] from SSOT `profiles/firmware_trains/*.json`.
pub fn stage2_spec_for_loader_profile(loader_profile_key: &str) -> Result<Stage2BootTrainSpec> {
    stage2_boot_spec_for_loader(loader_profile_key).ok_or_else(|| {
        anyhow::anyhow!(
            "No SSOT stage2_boot_spec defined for loader profile '{}'",
            loader_profile_key
        )
    })
}

/// Inspects `PROG061` (`Loader ver0.61`) for factory vs Stage-2 HDD boot hook.
pub fn inspect_prog061_stage2_boot_site(buf: &[u8]) -> Result<Prog061Stage2BootStatus> {
    let spec = stage2_spec_for_loader_profile("ver061").unwrap_or(VER061_STAGE2_BOOT_SPEC);
    inspect_stage2_boot_site_for_spec(buf, &spec)
}

/// Inspects an updater RAM binary against the SSOT Stage-2 spec for `loader_profile_key`.
pub fn inspect_stage2_boot_for_loader_profile(
    buf: &[u8],
    loader_profile_key: &str,
) -> Result<Prog061Stage2BootStatus> {
    let spec = stage2_spec_for_loader_profile(loader_profile_key)?;
    inspect_stage2_boot_site_for_spec(buf, &spec)
}

/// Applies or removes the Stage-2 HDD boot hook atomically for a train spec (`fail-closed`).
pub fn apply_stage2_boot_in_place_for_spec(
    buf: &mut [u8],
    spec: &Stage2BootTrainSpec,
    enable: bool,
) -> Result<()> {
    let payload = assemble_stage2_payload_for_spec(spec)?;
    let end = spec.payload_offset + payload.len();
    if buf.len() != spec.expected_size {
        bail!(
            "Stage-2 boot patch for '{}' requires {} B binary, got {} B",
            spec.train_id,
            spec.expected_size,
            buf.len()
        );
    }
    if enable && spec.require_reset_fix_1000ms {
        let reset_status = crate::ssot::registry::HardwareProfileRegistry::builtin()
            .bundle()
            .firmware_trains
            .get(spec.train_id)
            .map(|t| t.loader_profile_key.as_str())
            .and_then(|key| inspect_prog_sata_bridge_reset_site_for_profile(buf, key).ok());
        let reset_status = match reset_status {
            Some(st) => st,
            None => inspect_prog061_sata_bridge_reset_site(buf)?,
        };
        if reset_status.state != SATA_BRIDGE_STATE_WAIT_1000MS {
            bail!(
                "Refusing to enable Stage-2 HDD boot before SataBridgeResetFix ({} state={})",
                reset_status.va_hex,
                reset_status.state
            );
        }
    }

    let hook_patch = compute_stage2_hook_bytes(spec);
    let hook_spec = build_hook_spec(spec, &hook_patch);
    let hook_cls = classify_patch_site(buf, &hook_spec, PROG061_STAGE2_STATE_ENABLED)?;
    let tail_slice = &buf[spec.payload_offset..end];

    if enable {
        if hook_cls.state == PROG061_STAGE2_STATE_ENABLED && tail_slice == payload.as_slice() {
            return Ok(());
        }
        if hook_cls.state != "FACTORY" {
            bail!(
                "'{}' hook site 0x{:08X} unexpected state '{}' ({})",
                spec.train_id,
                hook_spec.va,
                hook_cls.state,
                hex::encode(&hook_cls.actual_bytes)
            );
        }
        if !tail_slice.iter().all(|&b| b == PROG_SLACK_FILL_BYTE) {
            bail!(
                "'{}' slack 0x{:05X}..0x{:05X} is not erased (expected 0x{:02X})",
                spec.train_id,
                spec.payload_offset,
                end,
                PROG_SLACK_FILL_BYTE
            );
        }
        buf[spec.payload_offset..end].copy_from_slice(&payload);
        apply_patch_specs_in_place(buf, &[(&hook_spec, PatchApplyAction::ApplyReplace)])?;
    } else {
        if hook_cls.state == "FACTORY" && tail_slice.iter().all(|&b| b == PROG_SLACK_FILL_BYTE) {
            return Ok(());
        }
        if hook_cls.state != PROG061_STAGE2_STATE_ENABLED || tail_slice != payload.as_slice() {
            bail!("Refusing to revert non-matching Stage-2 hook/payload");
        }
        apply_patch_specs_in_place(buf, &[(&hook_spec, PatchApplyAction::RestoreExpect)])?;
        buf[spec.payload_offset..end].fill(PROG_SLACK_FILL_BYTE);
    }
    Ok(())
}

/// Applies or removes the `PROG061` (`Loader ver0.61`) Stage-2 HDD boot hook atomically (`fail-closed`).
pub fn apply_prog061_stage2_boot_in_place(buf: &mut [u8], enable: bool) -> Result<()> {
    let spec = stage2_spec_for_loader_profile("ver061").unwrap_or(VER061_STAGE2_BOOT_SPEC);
    apply_stage2_boot_in_place_for_spec(buf, &spec, enable)
}

/// Applies or removes the SSOT Stage-2 HDD boot hook for `loader_profile_key` atomically.
pub fn apply_stage2_boot_for_loader_profile_in_place(
    buf: &mut [u8],
    loader_profile_key: &str,
    enable: bool,
) -> Result<()> {
    let spec = stage2_spec_for_loader_profile(loader_profile_key)?;
    apply_stage2_boot_in_place_for_spec(buf, &spec, enable)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::nvram::calculate_hashes;
    use crate::re::SataBridgeResetFixMode;
    use crate::re::mn10300_emit::{emit_movbu_mem, emit_st_w_an_dm};
    use crate::re::mn10300_emu::Mn103Vm;
    use crate::re::sata_bridge_fix::{
        ORIG_PROG061_SATA_BRIDGE_RESET_WAIT, apply_prog061_sata_bridge_reset_fix_in_place,
    };

    use super::*;

    const VM_CALL_LOG_BASE: u32 = 0x5048_F000;
    const VM_CALL_COUNT_ADDR: u32 = 0x5048_EFFC;
    const VM_CFG_BASE: u32 = 0x5048_E000;
    const VM_CFG_SD_INIT_RC: u32 = VM_CFG_BASE;
    const VM_CFG_FAT_MOUNT_RC: u32 = VM_CFG_BASE + 0x04;
    const VM_CFG_FORCE_OPEN_RC: u32 = VM_CFG_BASE + 0x08;
    const VM_CFG_PANA_OPEN_RC: u32 = VM_CFG_BASE + 0x0C;
    const VM_CFG_MAX_LBA_RC: u32 = VM_CFG_BASE + 0x10;
    const VM_CFG_MAX_LBA_VAL: u32 = VM_CFG_BASE + 0x14;
    const VM_CFG_PRIMARY_FAIL: u32 = VM_CFG_BASE + 0x18;
    const VM_CFG_PRIMARY_SRC: u32 = VM_CFG_BASE + 0x1C;
    const VM_CFG_ENTRY_VA: u32 = VM_CFG_BASE + 0x20;
    const VM_CFG_BITWALK_FAIL: u32 = VM_CFG_BASE + 0x24;

    const TAG_PRINT: u32 = 1;
    const TAG_SLEEP: u32 = 2;
    const TAG_SD_INIT: u32 = 3;
    const TAG_FAT_MOUNT: u32 = 4;
    const TAG_FAT_OPEN_FORCE: u32 = 5;
    const TAG_FAT_OPEN_PANA: u32 = 6;
    const TAG_FAT_CLOSE: u32 = 7;
    const TAG_HDD_MAX_LBA: u32 = 8;
    const TAG_HDD_READ: u32 = 9;
    const TAG_FEISTEL: u32 = 10;
    const TAG_CHK_HDR: u32 = 11;
    const TAG_GET_ENTRY: u32 = 12;
    const TAG_GET_SRC: u32 = 13;
    const TAG_IS_COMP: u32 = 14;
    const TAG_GET_SIZE: u32 = 15;
    const TAG_MEMCPY: u32 = 16;
    const TAG_UNCOMP: u32 = 17;
    const TAG_DCACHE: u32 = 18;
    const TAG_STOP_TMR: u32 = 19;
    const TAG_DISABLE_IRQ: u32 = 20;
    const TAG_CHK_UPDATE: u32 = 21;

    const VA_MAIN_BITWALK_TABLE: u32 = 0x5700_0200;

    /// Logs `[tag, D3 (saved D1), D2 (saved D0)]` at `VM_CALL_LOG_BASE + slot * 16`, preserving `D2`/`D3`.
    fn emit_record_call(a: &mut Stage2Asm, tag: u32) {
        a.mov_imm32_a(0, VM_CALL_COUNT_ADDR);
        emit_ld_w_an_dm(&mut a.code, 0, 0); // D0 = slot_idx
        emit_mov_d_to_a(&mut a.code, 0, 1); // A1 = slot_idx
        a.add_imm8_d(0, 1);
        emit_st_w_an_dm(&mut a.code, 0, 0); // *(count_addr) = slot_idx + 1
        a.code.extend_from_slice(&[0xF1, 0xD4]); // MOV A1, D0
        a.asl_imm8_d(0, 4); // slot_idx * 16
        a.add_imm32_d(0, VM_CALL_LOG_BASE);
        emit_mov_d_to_a(&mut a.code, 0, 0); // A0 = entry ptr
        a.mov_imm32_d(0, tag);
        emit_st_w_an_dm(&mut a.code, 0, 0); // entry[0] = tag
        a.code.push(0x50); // INC4 A0
        emit_st_w_an_dm(&mut a.code, 3, 0); // entry[1] = saved D1
        a.code.push(0x50); // INC4 A0
        emit_st_w_an_dm(&mut a.code, 2, 0); // entry[2] = saved D0
    }

    const VM_STUB_POOL_BASE: u32 = 0x5080_0000;
    const VM_STUB_SLOT_SIZE: usize = 0x100;

    struct VmStubAllocator<'a> {
        prog_seg: &'a mut [u8],
        spec: &'a Stage2BootTrainSpec,
        stub_pool: Vec<u8>,
        slot_idx: usize,
    }

    impl<'a> VmStubAllocator<'a> {
        fn new(prog_seg: &'a mut [u8], spec: &'a Stage2BootTrainSpec) -> Self {
            Self {
                prog_seg,
                spec,
                stub_pool: vec![0xCB_u8; 0x1_0000],
                slot_idx: 0,
            }
        }

        fn install_custom(
            &mut self,
            entry_va: u32,
            build_fn: impl FnOnce(&mut Stage2Asm, &Stage2BootTrainSpec),
        ) {
            let slot_off = self.slot_idx * VM_STUB_SLOT_SIZE;
            let slot_va = VM_STUB_POOL_BASE + slot_off as u32;
            self.slot_idx += 1;

            let mut a = Stage2Asm::new(slot_va, self.spec.uart0_printf_va);
            a.code.extend_from_slice(&[0xCF, 0xC0]); // MOVM [D2, D3], (SP)
            a.mov_d_to_d(0, 2); // D2 = saved D0
            a.mov_d_to_d(1, 3); // D3 = saved D1
            build_fn(&mut a, self.spec);
            a.code.extend_from_slice(&[0xDF, 0xC0, 0x08]); // RET [D2, D3], 8
            let body = a.finalize().unwrap();
            assert!(
                body.len() <= VM_STUB_SLOT_SIZE && slot_off + body.len() <= self.stub_pool.len(),
                "VM stub at 0x{entry_va:08X} overflowed slot ({} B)",
                body.len()
            );
            self.stub_pool[slot_off..slot_off + body.len()].copy_from_slice(&body);

            let entry_off = (entry_va - self.spec.base_va) as usize;
            let rel = slot_va.wrapping_sub(entry_va).to_le_bytes();
            self.prog_seg[entry_off..entry_off + 5]
                .copy_from_slice(&[0xDC, rel[0], rel[1], rel[2], rel[3]]);
        }

        fn install_simple(
            &mut self,
            entry_va: u32,
            tag: u32,
            ret_d0: Option<u32>,
            cfg_addr: Option<u32>,
        ) {
            self.install_custom(entry_va, |a, _| {
                emit_record_call(a, tag);
                if let Some(addr) = cfg_addr {
                    a.ld_abs32_d(addr, 0);
                } else if let Some(val) = ret_d0 {
                    a.mov_imm32_d(0, val);
                } else {
                    a.clr_d(0);
                }
            });
        }
    }

    struct Stage2VmHarness {
        vm: Mn103Vm,
    }

    impl Stage2VmHarness {
        fn new(prog061: &[u8]) -> Self {
            Self::new_full(prog061, false, None)
        }

        fn new_with_nor(prog061: &[u8], nor_has_stage2: bool) -> Self {
            Self::new_full(prog061, nor_has_stage2, None)
        }

        fn new_full(prog061: &[u8], nor_has_stage2: bool, real_main: Option<Vec<u8>>) -> Self {
            let has_real_main = real_main.is_some();
            let s = &VER061_STAGE2_BOOT_SPEC;
            let mut vm = Mn103Vm::new();
            let mut prog_seg = prog061.to_vec();

            let real_getc_nb = [
                0xCF, 0x10, 0x3F, 0xFC, 0xAC, 0x0C, 0x20, 0x00, 0xD4, 0x81, 0xF8, 0xE1, 0x10, 0x84,
                0x1C, 0xA0, 0x00, 0xC8, 0x0D, 0xFC, 0xA8, 0x09, 0x20, 0x00, 0xD4, 0x81, 0x15, 0x84,
                0xCA, 0x05, 0x00, 0xCA, 0x02, 0xF2, 0xFC, 0xDF, 0x10, 0x04,
            ];
            let getc_off = (s.uart0_getc_nb_va - s.base_va) as usize;
            prog_seg[getc_off..getc_off + real_getc_nb.len()].copy_from_slice(&real_getc_nb);

            let stub_pool = {
                let mut alloc = VmStubAllocator::new(&mut prog_seg, s);
                alloc.install_simple(s.uart0_printf_va, TAG_PRINT, Some(0), None);
                alloc.install_simple(s.sleep_10ms_va, TAG_SLEEP, Some(0), None);
                alloc.install_simple(s.sd_init_va, TAG_SD_INIT, None, Some(VM_CFG_SD_INIT_RC));
                alloc.install_simple(
                    s.fat_mount_va,
                    TAG_FAT_MOUNT,
                    None,
                    Some(VM_CFG_FAT_MOUNT_RC),
                );
                alloc.install_simple(s.fat_close_va, TAG_FAT_CLOSE, Some(0), None);
                alloc.install_simple(s.feistel_decrypt_va, TAG_FEISTEL, Some(0), None);
                alloc.install_simple(s.extrhead_check_hdr_va, TAG_CHK_HDR, Some(1), None);
                alloc.install_simple(
                    s.extrhead_get_entry_va,
                    TAG_GET_ENTRY,
                    None,
                    Some(VM_CFG_ENTRY_VA),
                );
                alloc.install_simple(s.extrhead_is_comp_va, TAG_IS_COMP, Some(1), None);
                alloc.install_simple(
                    s.extrhead_get_size_va,
                    TAG_GET_SIZE,
                    Some(0x0002_0000),
                    None,
                );
                alloc.install_simple(s.memcpy_va, TAG_MEMCPY, Some(0x9400_0000), None);
                alloc.install_simple(s.dcache_purge_va, TAG_DCACHE, Some(0), None);
                alloc.install_simple(s.stop_timer_irq_va, TAG_STOP_TMR, Some(0), None);
                alloc.install_simple(s.disable_irq_va, TAG_DISABLE_IRQ, Some(0), None);
                alloc.install_simple(s.check_update_flag_va, TAG_CHK_UPDATE, Some(0), None);

                alloc.install_custom(s.fat_open_va, |a, _| {
                    emit_mov_d_to_a(&mut a.code, 3, 0);
                    emit_movbu_mem(&mut a.code, 0, 0);
                    a.cmp_imm8_d(0, b'F' as i8);
                    a.jcc(0xC8, "open_force");
                    emit_record_call(a, TAG_FAT_OPEN_PANA);
                    a.ld_abs32_d(VM_CFG_PANA_OPEN_RC, 0);
                    a.jmp("open_done");
                    a.label("open_force");
                    emit_record_call(a, TAG_FAT_OPEN_FORCE);
                    a.ld_abs32_d(VM_CFG_FORCE_OPEN_RC, 0);
                    a.label("open_done");
                });

                alloc.install_custom(s.hdd_max_lba_va, |a, _| {
                    emit_record_call(a, TAG_HDD_MAX_LBA);
                    emit_mov_d_to_a(&mut a.code, 2, 0);
                    a.ld_abs32_d(VM_CFG_MAX_LBA_VAL, 0);
                    emit_st_w_an_dm(&mut a.code, 0, 0);
                    a.ld_abs32_d(VM_CFG_MAX_LBA_RC, 0);
                });

                alloc.install_custom(s.hdd_read_va, |a, s| {
                    emit_record_call(a, TAG_HDD_READ);
                    a.cmp_imm8_d(2, 0);
                    a.jcc(0xC9, "read_done");
                    a.mov_imm32_a(0, s.hdd_dma_buf_va);
                    emit_st_w_an_dm(&mut a.code, 3, 0);
                    a.label("read_done");
                    a.clr_d(0);
                });

                alloc.install_custom(s.extrhead_get_src_va, |a, s| {
                    emit_record_call(a, TAG_GET_SRC);
                    a.ld_abs32_d(s.hdd_dma_buf_va, 0);
                    a.cmp_imm32_d0(s.hdd_primary_lba);
                    a.jcc(0xC9, "ret_primary_src");
                    a.ld_abs32_d(VM_CFG_PRIMARY_FAIL, 0);
                    a.cmp_imm8_d(0, 0);
                    a.jcc(0xC8, "ret_primary_src");
                    a.clr_d(0);
                    a.jmp("get_src_done");
                    a.label("ret_primary_src");
                    a.ld_abs32_d(VM_CFG_PRIMARY_SRC, 0);
                    a.label("get_src_done");
                });

                if has_real_main {
                    alloc.install_simple(s.extrhead_uncomp_va, TAG_UNCOMP, Some(1), None);
                } else {
                    alloc.install_custom(s.extrhead_uncomp_va, |a, _| {
                        emit_record_call(a, TAG_UNCOMP);
                        emit_mov_imm32(&mut a.code, 0xA0, VM_CFG_ENTRY_VA); // A0 = *(entry_va)
                        a.code.extend_from_slice(&[0x20, 0x08]); // ADD 8, A0
                        a.mov_imm32_d(0, VA_MAIN_BITWALK_TABLE);
                        emit_st_w_an_dm(&mut a.code, 0, 0); // *(entry + 8) = bitwalk_table
                        a.ld_abs32_d(VM_CFG_BITWALK_FAIL, 0);
                        a.cmp_imm8_d(0, 0);
                        a.jcc(0xC8, "uncomp_ok");
                        a.mov_imm32_a(0, VA_MAIN_BITWALK_TABLE);
                        a.clr_d(0);
                        emit_st_w_an_dm(&mut a.code, 0, 0); // corrupt word 0
                        a.label("uncomp_ok");
                        a.mov_imm8_d(0, 1);
                    });
                }

                alloc.stub_pool
            };

            let ram_entry_va = s.fvu_load_uncached_va.wrapping_add(s.payload_offset as u32);
            let nor_entry_va = s.norfrom_fvu_base_va.wrapping_add(s.payload_offset as u32);
            let ram_entry_bytes = prog061[s.payload_offset..s.payload_offset + 0x100].to_vec();
            let nor_entry_bytes = if nor_has_stage2 {
                ram_entry_bytes.clone()
            } else {
                vec![0xFFu8; 0x100]
            };
            vm.map_segment(s.base_va, prog_seg, false);
            vm.map_segment(VM_STUB_POOL_BASE, stub_pool, false);
            vm.map_segment(0x5048_0000, vec![0u8; 0x1_0000], true);
            vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
            vm.map_segment(ram_entry_va, ram_entry_bytes, false);
            vm.map_segment(nor_entry_va, nor_entry_bytes, false);
            vm.map_segment(s.hdd_dma_buf_va, vec![0u8; 0x200], true);
            if let Some(main_img) = real_main {
                vm.map_segment(0x5700_0000, main_img, false);
            } else {
                vm.map_segment(0x5700_0000, vec![0xCBu8; 0x1000], true);
                for bit in 0..32u32 {
                    vm.write_u32(VA_MAIN_BITWALK_TABLE + bit * 4, 1u32 << bit)
                        .unwrap();
                }
            }
            vm.sp = 0x50FF_FF00;
            vm.pc = s.base_va + s.hook_offset as u32;
            vm.enable_sc0_uart(&[]);
            vm.write_u32(VM_CFG_SD_INIT_RC, (-90001i32) as u32).unwrap();
            vm.write_u32(VM_CFG_FAT_MOUNT_RC, 1).unwrap();
            vm.write_u32(VM_CFG_FORCE_OPEN_RC, (-1i32) as u32).unwrap();
            vm.write_u32(VM_CFG_PANA_OPEN_RC, (-1i32) as u32).unwrap();
            vm.write_u32(VM_CFG_MAX_LBA_RC, 0).unwrap();
            vm.write_u32(VM_CFG_MAX_LBA_VAL, 0x1D1C_5970).unwrap();
            vm.write_u32(VM_CFG_PRIMARY_FAIL, 0).unwrap();
            vm.write_u32(VM_CFG_PRIMARY_SRC, 0x9400_0000).unwrap();
            vm.write_u32(VM_CFG_ENTRY_VA, 0x5700_0000).unwrap();
            vm.write_u32(VM_CFG_BITWALK_FAIL, 0).unwrap();
            Self { vm }
        }

        fn run(&mut self, stop_pc: u32) -> Result<(), crate::re::Mn103Fault> {
            self.vm.run_until(stop_pc, 250_000)
        }

        fn calls(&mut self) -> Vec<(u32, u32, u32)> {
            let n = self.vm.read_u32(VM_CALL_COUNT_ADDR).unwrap();
            (0..n)
                .map(|i| {
                    let base = VM_CALL_LOG_BASE + i * 16;
                    (
                        self.vm.read_u32(base).unwrap(),
                        self.vm.read_u32(base + 4).unwrap(),
                        self.vm.read_u32(base + 8).unwrap(),
                    )
                })
                .collect()
        }
    }

    fn build_stage2_prog_fixture() -> Vec<u8> {
        let mut buf = vec![0u8; PROG061_EXPECTED_SIZE];
        buf[PROG061_STAGE2_PAYLOAD_OFFSET..PROG061_EXPECTED_SIZE - PROG_TRAILER_RESERVED_BYTES]
            .fill(PROG_SLACK_FILL_BYTE);
        buf[0x01_D928..0x01_D928 + 11].copy_from_slice(&ORIG_PROG061_SATA_BRIDGE_RESET_WAIT);
        buf[OFF_PROG061_STAGE2_HOOK_SITE..OFF_PROG061_STAGE2_HOOK_SITE + 7]
            .copy_from_slice(&ORIG_PROG061_STAGE2_HOOK_BYTES);
        apply_prog061_sata_bridge_reset_fix_in_place(&mut buf, SataBridgeResetFixMode::Wait1000Ms)
            .unwrap();
        apply_prog061_stage2_boot_in_place(&mut buf, true).unwrap();
        buf
    }

    #[test]
    fn hook_bytes_match_ver061_call32_displacement() {
        assert_eq!(
            compute_stage2_hook_bytes(&VER061_STAGE2_BOOT_SPEC),
            [0xDD, 0x2D, 0x45, 0x03, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn mn103vm_executes_stage2_primary_and_mirror_hdd_boot_paths() {
        let prog = build_stage2_prog_fixture();

        // 1. Portao 3 happy path: SD + PANA__SD.FRM present while Nor-FROM (0x8407_9600) is virgin 0xFF
        //    -> must NOT trigger Nor-FROM+SD fallback, must boot MAIN @ 0x5700_0000 with Loader teardown.
        let mut h = Stage2VmHarness::new(&prog);
        h.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
        h.vm.write_u32(VM_CFG_FAT_MOUNT_RC, 1).unwrap();
        h.vm.write_u32(VM_CFG_PANA_OPEN_RC, 0xC8).unwrap();
        h.run(0x5700_0000).unwrap();
        let calls = h.calls();
        assert!(!calls.iter().any(|&(t, _, _)| t == TAG_FAT_OPEN_PANA));
        assert!(
            calls
                .iter()
                .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x10000 && off == 0)
        );
        assert!(calls.iter().any(|&(t, _, _)| t == TAG_UNCOMP));
        let idx_dc = calls.iter().position(|c| c.0 == TAG_DCACHE).unwrap();
        let idx_tmr = calls.iter().position(|c| c.0 == TAG_STOP_TMR).unwrap();
        let idx_irq = calls.iter().position(|c| c.0 == TAG_DISABLE_IRQ).unwrap();
        assert!(idx_dc < idx_tmr && idx_tmr < idx_irq);

        // 2. Primary src_addr == 0 rejected -> outer mirror fallback (0x1D1A_5970) succeeds.
        let mut h_mir = Stage2VmHarness::new(&prog);
        h_mir.vm.write_u32(VM_CFG_PRIMARY_FAIL, 1).unwrap();
        h_mir.run(0x5700_0000).unwrap();
        let mir_calls = h_mir.calls();
        assert!(
            mir_calls
                .iter()
                .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x1D1A_5970 && off == 0)
        );

        // 3. Portao 4 Nor-FROM happy path: *(0x8407_9600) == *(0x9043_9600) with no SD card -> boots HDD MAIN.
        let mut h_nf_ok = Stage2VmHarness::new_with_nor(&prog, true);
        h_nf_ok.run(0x5700_0000).unwrap();
        assert!(
            h_nf_ok
                .calls()
                .iter()
                .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x10000 && off == 0)
        );
    }

    #[test]
    fn mn103vm_executes_escape_hatches_and_fail_closed_fallbacks() {
        let prog = build_stage2_prog_fixture();
        let hook_ret_pc = PROG061_BASE_VA + OFF_PROG061_STAGE2_HOOK_SITE as u32 + 7;

        // 1. UART0 'F' abort via real SC0 RX code -> fallback FVU (no HDD reads or teardown).
        let mut h_uart = Stage2VmHarness::new(&prog);
        h_uart.vm.queue_uart_rx(b"F");
        h_uart.run(hook_ret_pc).unwrap();
        let u_calls = h_uart.calls();
        assert_eq!(h_uart.vm.sp, 0x50FF_FF00);
        assert!(
            u_calls
                .iter()
                .any(|&(t, _, d0)| t == TAG_CHK_UPDATE && d0 == 0x10)
        );
        assert!(
            !u_calls
                .iter()
                .any(|c| c.0 == TAG_HDD_MAX_LBA || c.0 == TAG_DISABLE_IRQ)
        );

        // 2. SD FORCEFVU.TXT with FAT_MOUNT == 0 (regression check: mount_id == 0 must succeed).
        let mut h_force = Stage2VmHarness::new(&prog);
        h_force.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
        h_force.vm.write_u32(VM_CFG_FAT_MOUNT_RC, 0).unwrap();
        h_force.vm.write_u32(VM_CFG_FORCE_OPEN_RC, 0xC8).unwrap();
        h_force.run(hook_ret_pc).unwrap();
        let f_calls = h_force.calls();
        assert!(
            f_calls
                .iter()
                .any(|&(t, _, fd)| t == TAG_FAT_CLOSE && fd == 0xC8)
        );
        assert!(!f_calls.iter().any(|c| c.0 == TAG_HDD_MAX_LBA));

        // 3. Nor-FROM boot (*(0x8407_9600) == *(0x9043_9600)) with SD PANA__SD.FRM present -> fallback FVU.
        let mut h_nf = Stage2VmHarness::new_with_nor(&prog, true);
        h_nf.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
        h_nf.vm.write_u32(VM_CFG_PANA_OPEN_RC, 0xC8).unwrap();
        h_nf.run(hook_ret_pc).unwrap();
        let nf_calls = h_nf.calls();
        assert!(nf_calls.iter().any(|c| c.0 == TAG_FAT_OPEN_PANA));
        assert!(
            nf_calls
                .iter()
                .any(|&(t, _, id)| t == TAG_FAT_CLOSE && id == 0xC8)
        );
        assert!(nf_calls.iter().any(|c| c.0 == TAG_CHK_UPDATE));
        assert!(!nf_calls.iter().any(|c| c.0 == TAG_HDD_MAX_LBA));

        // 4. Both mirrors invalid (src_addr == 0 everywhere) -> clean fallback to FVU.
        let mut h_bad = Stage2VmHarness::new(&prog);
        h_bad.vm.write_u32(VM_CFG_PRIMARY_SRC, 0).unwrap();
        h_bad.run(hook_ret_pc).unwrap();
        let bad_calls = h_bad.calls();
        assert_eq!(bad_calls.iter().filter(|c| c.0 == TAG_HDD_READ).count(), 2);
        assert!(bad_calls.iter().any(|c| c.0 == TAG_CHK_UPDATE));
        assert!(!bad_calls.iter().any(|c| c.0 == TAG_DISABLE_IRQ));

        // 5. Bit-walk failure at *(entry + 8) on both mirrors -> clean fallback to FVU.
        let mut h_bw = Stage2VmHarness::new(&prog);
        h_bw.vm.write_u32(VM_CFG_BITWALK_FAIL, 1).unwrap();
        h_bw.run(hook_ret_pc).unwrap();
        let bw_calls = h_bw.calls();
        assert_eq!(bw_calls.iter().filter(|c| c.0 == TAG_UNCOMP).count(), 2);
        assert!(bw_calls.iter().any(|c| c.0 == TAG_CHK_UPDATE));
        assert!(!bw_calls.iter().any(|c| c.0 == TAG_DISABLE_IRQ));
    }

    #[test]
    fn stage2_patch_enforces_fail_closed_preconditions_and_real_prog061_roundtrip() {
        let mut raw_prog = vec![0u8; PROG061_EXPECTED_SIZE];
        raw_prog
            [PROG061_STAGE2_PAYLOAD_OFFSET..PROG061_EXPECTED_SIZE - PROG_TRAILER_RESERVED_BYTES]
            .fill(PROG_SLACK_FILL_BYTE);
        raw_prog[0x01_D928..0x01_D928 + 11].copy_from_slice(&ORIG_PROG061_SATA_BRIDGE_RESET_WAIT);
        raw_prog[OFF_PROG061_STAGE2_HOOK_SITE..OFF_PROG061_STAGE2_HOOK_SITE + 7]
            .copy_from_slice(&ORIG_PROG061_STAGE2_HOOK_BYTES);
        assert!(apply_prog061_stage2_boot_in_place(&mut raw_prog, true).is_err());

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
        if path.is_file() {
            let orig = std::fs::read(&path).unwrap();
            let (_, orig_sha) = calculate_hashes(&orig);
            let mut work = orig;
            assert_eq!(
                inspect_prog061_stage2_boot_site(&work).unwrap().state,
                PROG061_STAGE2_STATE_FACTORY
            );
            apply_prog061_sata_bridge_reset_fix_in_place(
                &mut work,
                SataBridgeResetFixMode::Wait1000Ms,
            )
            .unwrap();
            apply_prog061_stage2_boot_in_place(&mut work, true).unwrap();
            let status = inspect_prog061_stage2_boot_site(&work).unwrap();
            assert_eq!(status.state, PROG061_STAGE2_STATE_ENABLED);
            assert!(status.payload_present);
            apply_prog061_stage2_boot_in_place(&mut work, false).unwrap();
            apply_prog061_sata_bridge_reset_fix_in_place(
                &mut work,
                SataBridgeResetFixMode::FactoryOriginal,
            )
            .unwrap();
            assert_eq!(calculate_hashes(&work).1, orig_sha);
        }
    }

    #[test]
    fn mn103vm_proves_portao4_loader_ver061_norfrom_boot_and_escape_hatch() {
        use crate::constants::{
            AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE,
        };
        use crate::pico::rsp_codec::policy::NorWritePolicy;
        use crate::re::bgo_unbrick::{Prog061BgoPatchMode, patch_prog061_bgo_in_place};

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let loader_path = root.join("firmware_and_images/eh55_loader_ver061_256k.bin");
        let nor_path = root.join("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
        let prog_path =
            root.join("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
        if !loader_path.is_file() || !nor_path.is_file() || !prog_path.is_file() {
            return;
        }

        let loader = std::fs::read(&loader_path).unwrap();
        let base_nor = std::fs::read(&nor_path).unwrap();
        assert_eq!(loader.len(), PROG061_EXPECTED_SIZE);
        assert_eq!(base_nor.len(), AM34_FLASH_SIZE as usize);
        // Confirm SA11..SA14 (0x40000..0x80000) is virgin 0xFF and stock BGO lacks "fvu.exe".
        assert!(base_nor[0x4_0000..0x8_0000].iter().all(|&b| b == 0xFF));
        assert!(
            !base_nor[0x13_0000..0x40_0000]
                .windows(7)
                .any(|w| w == b"fvu.exe")
        );

        let mut stage2_prog = std::fs::read(&prog_path).unwrap();
        patch_prog061_bgo_in_place(&mut stage2_prog, Prog061BgoPatchMode::BypassChecksOnly)
            .unwrap();
        apply_prog061_sata_bridge_reset_fix_in_place(
            &mut stage2_prog,
            SataBridgeResetFixMode::Wait1000Ms,
        )
        .unwrap();
        apply_prog061_stage2_boot_in_place(&mut stage2_prog, true).unwrap();
        let stage2_sha = calculate_hashes(&stage2_prog).1;

        let run_norfrom_stage = |fvu_exists_rc: i32| -> (u32, Vec<u8>, Vec<u8>, u32) {
            let mut loader_img = loader.clone();
            // Patch only FUN_44005DEE (uart0_printf) and FUN_4400A1F1 (NFD_FileExists) to pure MN103S RET/stub.
            // Keep real Loader memcpy (0x4402A34A), memcmp (0x4402A30A), and 0x4400894B..0x44008B98 intact!
            loader_img[0x5DEE..0x5DF1].copy_from_slice(&[0xDF, 0x00, 0x00]); // RET [], 0
            let stub_exists = if fvu_exists_rc == 0 {
                vec![0x00u8, 0xDF, 0x00, 0x00] // CLR D0; RET [], 0
            } else {
                vec![0x2C, 0xFF, 0xFF, 0xDF, 0x00, 0x00] // MOV 0xFFFF, D0; RET [], 0
            };
            loader_img[0xA1F1..0xA1F1 + stub_exists.len()].copy_from_slice(&stub_exists);

            let mut nor_cow = base_nor.clone();
            // Program Stage-2 PROG061 in SA11..SA14 (0x40000..0x80000) in CoW memory only.
            nor_cow[0x4_0000..0x8_0000].copy_from_slice(&stage2_prog);
            let ipl_sha_before =
                calculate_hashes(&nor_cow[..AM34_FLASH_CRITICAL_IPL_SIZE as usize]).1;

            let mut data_seg = vec![0u8; 0x2000];
            data_seg[..0xB38].copy_from_slice(&loader[0x3_52A0..0x3_5DD8]);
            assert_eq!(&data_seg[0xAF..0xB7], b"fvu.exe\0");

            let mut vm = Mn103Vm::new();
            vm.nor_write_policy = NorWritePolicy::NvramOnly;
            vm.map_segment(0x4400_0000, loader_img, false);
            vm.map_segment(AM34_FLASH_UNCACHED_BASE, nor_cow, false);
            vm.map_segment(0x5048_0000, data_seg, true);
            vm.map_segment(0x9040_0000, vec![0xAAu8; PROG061_EXPECTED_SIZE], true);
            vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
            let frame_a3 = 0x50FF_F800u32;
            vm.a[3] = frame_a3;
            vm.sp = 0x50FF_F700;
            vm.pc = 0x4400_894B;

            let stop_pc = if fvu_exists_rc == 0 {
                0x4400_8B98
            } else {
                0x4400_89F5
            };
            vm.run_until(stop_pc, 4_000_000)
                .unwrap_or_else(|e| panic!("Loader ver0.61 Nor-FROM run failed: {e}"));
            let loaded_flag = vm.read_u32(frame_a3 - 12).unwrap();
            let ram_copy = vm.segment_bytes(0x9040_0000).unwrap().to_vec();
            let ipl_sha_after = calculate_hashes(
                &vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()
                    [..AM34_FLASH_CRITICAL_IPL_SIZE as usize],
            )
            .1;
            assert_eq!(
                ipl_sha_before, ipl_sha_after,
                "IPL SA0..SA10 must remain bit-exact"
            );
            let nor_sa11_14 =
                vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()[0x4_0000..0x8_0000].to_vec();
            (vm.pc, ram_copy, nor_sa11_14, loaded_flag)
        };

        // 1. Negative control: when NFD finds no "fvu.exe" marker, Loader skips NOR copy and branches to Update-Disk.
        let (neg_pc, neg_ram, _, neg_flag) = run_norfrom_stage(-1);
        assert_eq!(neg_pc, 0x4400_89F5);
        assert_eq!(neg_flag, 0);
        assert!(neg_ram.iter().all(|&b| b == 0xAA));

        // 2. Positive Portao 4 proof: real Loader memcpy/memcmp copies SA11..SA14 -> 0x9040_0000 byte-for-byte.
        let (pos_pc, pos_ram, pos_nor, pos_flag) = run_norfrom_stage(0);
        assert_eq!(pos_pc, 0x4400_8B98);
        assert_eq!(pos_flag, 1);
        assert_eq!(calculate_hashes(&pos_ram).1, stage2_sha);
        assert_eq!(pos_ram, pos_nor);

        // 3. Chain into copied Stage-2 payload: without SD boots HDD MAIN; with PANA__SD.FRM hits Nor-FROM escape hatch.
        let mut h_boot = Stage2VmHarness::new_with_nor(&pos_ram, true);
        h_boot.run(0x5700_0000).unwrap();
        assert!(
            h_boot
                .calls()
                .iter()
                .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x10000 && off == 0)
        );
        let mut h_escape = Stage2VmHarness::new_with_nor(&pos_ram, true);
        h_escape.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
        h_escape.vm.write_u32(VM_CFG_PANA_OPEN_RC, 0xC8).unwrap();
        let hook_ret_pc = PROG061_BASE_VA + OFF_PROG061_STAGE2_HOOK_SITE as u32 + 7;
        h_escape.run(hook_ret_pc).unwrap();
        assert!(h_escape.calls().iter().any(|c| c.0 == TAG_FAT_OPEN_PANA));
        assert!(!h_escape.calls().iter().any(|c| c.0 == TAG_HDD_MAX_LBA));
    }

    #[test]
    fn ssot_stage2_spec_matches_ver061_contract_field_for_field() {
        let from_ssot = stage2_spec_for_loader_profile("ver061").unwrap();
        assert_eq!(from_ssot, VER061_STAGE2_BOOT_SPEC);
        assert!(stage2_spec_for_loader_profile("ver063").is_err());
    }

    #[test]
    fn mn103vm_proves_main3410pq_stage2_bitwalk_and_hdd_boot() {
        use crate::constants::AM34_MAIN_BASE_VA;
        use crate::crypto::{ExtrheadCryptMode, pack_extrhead, unpack_extrhead};
        use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
        use crate::re::main_bgo_retarget::{
            MAIN_BGO_STATE_8MB, inspect_or_verify_main_bgo_geometry,
        };
        use crate::re::main_passthrough::EXPECTED_MAIN_3410PQ_SIZE;
        use crate::re::sata_bridge_fix::{
            SATA_BRIDGE_STATE_WAIT_1000MS, apply_main_sata_bridge_reset_fix_in_place,
            inspect_main_sata_bridge_reset_site,
        };

        let part1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin");
        if !part1.is_file() {
            return;
        }
        let raw_ctr = std::fs::read(&part1).unwrap();
        let unpacked_meta = unpack_extrhead(&raw_ctr, ExtrheadCryptMode::Main).unwrap();
        let mut main_3410 = unpacked_meta.decompressed;
        assert_eq!(main_3410.len(), EXPECTED_MAIN_3410PQ_SIZE);

        // Verify real MAIN3410PQ *(0x5700_0008) pointer & 32-word power-of-two bit-walk table.
        let bitwalk_va = LittleEndian::read_u32(&main_3410[8..12]);
        assert!(
            bitwalk_va >= AM34_MAIN_BASE_VA && bitwalk_va & 3 == 0,
            "Invalid MAIN3410PQ bitwalk pointer 0x{bitwalk_va:08X}"
        );
        let bw_off = (bitwalk_va - AM34_MAIN_BASE_VA) as usize;
        for bit in 0..32usize {
            let word = LittleEndian::read_u32(&main_3410[bw_off + bit * 4..bw_off + bit * 4 + 4]);
            assert_eq!(
                word,
                1u32 << bit,
                "MAIN3410PQ bitwalk mismatch at word {bit} (off 0x{bw_off:X}): got 0x{word:08X}"
            );
        }

        // Apply universal 1000 ms SATA fix + 250 GB HDD capacity + verify native 8 MiB BGO + pack EXTRHEAD(0x94000000).
        apply_main_sata_bridge_reset_fix_in_place(
            &mut main_3410,
            SataBridgeResetFixMode::Wait1000Ms,
        )
        .unwrap();
        assert_eq!(
            inspect_main_sata_bridge_reset_site(&main_3410)
                .unwrap()
                .state,
            SATA_BRIDGE_STATE_WAIT_1000MS
        );
        apply_capacity_patch_in_place(
            &mut main_3410,
            HddCapacityPatchMode::FixedCapacity,
            Some(250),
        )
        .unwrap();
        assert_eq!(
            inspect_or_verify_main_bgo_geometry(&main_3410)
                .unwrap()
                .state,
            MAIN_BGO_STATE_8MB
        );
        let packed = pack_extrhead(
            &main_3410,
            "MAIN",
            AM34_MAIN_BASE_VA,
            "3410PQ",
            ExtrheadCryptMode::Main,
            unpacked_meta.header.adler_step.max(16),
            unpacked_meta.header.flag.max(1),
        )
        .unwrap();
        let repacked = unpack_extrhead(&packed, ExtrheadCryptMode::Main).unwrap();
        assert_eq!(repacked.header.src_addr, 0x9400_0000);

        // Execute Stage-2 assembly in Mn103Vm with REAL MAIN3410PQ mapped at 0x5700_0000
        // (no synthetic bit-walk stub — Stage-2 walks MAIN3410PQ's real *(0x5700_0008) table).
        let prog = build_stage2_prog_fixture();
        let mut h = Stage2VmHarness::new_full(&prog, false, Some(repacked.decompressed));
        h.vm.write_u32(VM_CFG_ENTRY_VA, repacked.header.dst_addr)
            .unwrap();
        h.vm.write_u32(VM_CFG_PRIMARY_SRC, repacked.header.src_addr)
            .unwrap();
        h.run(AM34_MAIN_BASE_VA).unwrap();
        assert_eq!(h.vm.pc, AM34_MAIN_BASE_VA);
        let calls = h.calls();
        assert!(calls.iter().any(|&(t, _, _)| t == TAG_UNCOMP));
        assert!(calls.iter().any(|&(t, _, _)| t == TAG_DCACHE));
        assert!(calls.iter().any(|&(t, _, _)| t == TAG_STOP_TMR));
        assert!(calls.iter().any(|&(t, _, _)| t == TAG_DISABLE_IRQ));
    }
}
