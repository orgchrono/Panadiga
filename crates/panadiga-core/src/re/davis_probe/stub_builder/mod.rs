#![deny(unsafe_code)]

//! MN103S SDRAM probe stub builders for Panasonic AM34 `DAVIS` IDE (`CH0`/`CH1`) diagnostics.
//!
//! Built directly on [`crate::re::mn10300_emit`] without opcode duplication.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::constants::AM34_SYS_STATUS_REG;
use crate::re::mn10300_emit::{
    emit_add_sp_imm8, emit_bne_back, emit_call32_abs, emit_ld_w_an_dm, emit_mov_imm32, emit_rets,
    emit_st_w_an_dm, emit_store_d0_abs32,
};

pub mod ide_stubs;

pub use ide_stubs::{
    build_ata_identify_ec_stub, build_ch0_probe_stub, build_ch1_probe_stub,
    build_ch1_scratchpad_probe_stub,
};

/// Default uncached SDRAM address where live `DAVIS` probes are uploaded (`0x9050_0000`).
pub const DAVIS_STUB_DEFAULT_VA: u32 = 0x9050_0000;
/// Default uncached SDRAM metadata address (`[sentinel, post_altstat]`, `8` bytes).
pub const DAVIS_IDENTIFY_META_DEFAULT_VA: u32 = 0x9050_13F4;
/// Default uncached SDRAM destination for `0xEC` `IDENTIFY DEVICE` 256-word PIO capture (`1024` B).
pub const DAVIS_IDENTIFY_BUF_DEFAULT_VA: u32 = 0x9050_2000;
/// `PROG061` `CH1` (`DAVIS` HDD) driver state struct VA (`0x30` bytes).
pub const DAVIS_CH1_STATE_STRUCT_VA: u32 = 0x504F_8234;
/// `PROG061` factory `DAVIS_Connect_IDE(ch)` entry VA (`FUN_5041CD4D`).
pub const DAVIS_CH1_CONNECT_IDE_VA: u32 = 0x5041_CD4D;
/// `DAVIS` `CH0` control/reset register VA.
pub const DAVIS_CH0_CONTROL_REG_VA: u32 = 0x8900_0420;
/// `DAVIS` `CH1` host block control register VA.
pub const DAVIS_CH1_CONTROL_REG_VA: u32 = 0x8900_0C00;
/// `DAVIS` `CH1` IDE hard-reset polarity control register VA.
pub const DAVIS_CH1_RESET_REG_VA: u32 = 0x8900_0C20;
/// Magic sentinel written to `meta_va` on successful `0xEC` completion.
pub const DAVIS_IDENTIFY_SENTINEL: u32 = 0xCAFE_00EC;

/// Verified `PROG061` timer, IRQ priority, and `rtsd_hw_init` (`0x504156AD`) entry VAs.
pub const DAVIS_FE_TIMER_IRQ_INIT_VAS: [u32; 6] = [
    0x5040_4B11,
    0x5040_C352,
    0x5040_B61B,
    0x5040_D516,
    0x5040_D587,
    0x5041_56AD,
];

/// Position-coupled MN103S probe stub artifact ready for SDRAM upload (`W`) & execution (`J`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledProbeStub {
    pub name: String,
    pub load_va: u32,
    pub code: Vec<u8>,
    pub output_va: Option<u32>,
    pub output_len: usize,
    pub meta_va: Option<u32>,
}

pub(super) fn emit_mmio_write32(buf: &mut Vec<u8>, addr: u32, val: u32) {
    let _ = emit_mov_imm32(buf, 0xCC, val);
    emit_store_d0_abs32(buf, addr);
}

pub(super) fn emit_fill_words(
    buf: &mut Vec<u8>,
    dst: u32,
    count: u32,
    fill_val: u32,
) -> Result<()> {
    let _ = emit_mov_imm32(buf, 0xDD, dst);
    let _ = emit_mov_imm32(buf, 0xCC, fill_val);
    let _ = emit_mov_imm32(buf, 0xCD, count);
    let loop_off = buf.len();
    emit_st_w_an_dm(buf, 0, 1);
    buf.extend_from_slice(&[0x51, 0x29, 0xFF]); // INC4 A1; ADD -1, D1
    emit_bne_back(buf, loop_off, "FILL_WORDS")
}

pub(super) fn emit_word_transfer_loop(
    buf: &mut Vec<u8>,
    src: u32,
    dst: u32,
    count: u32,
    increment_src: bool,
    tag: &str,
) -> Result<()> {
    let _ = emit_mov_imm32(buf, 0xDC, src);
    let _ = emit_mov_imm32(buf, 0xDD, dst);
    let _ = emit_mov_imm32(buf, 0xCD, count);
    let loop_off = buf.len();
    emit_ld_w_an_dm(buf, 0, 0);
    emit_st_w_an_dm(buf, 0, 1);
    if increment_src {
        buf.push(0x50); // INC4 A0
    }
    buf.extend_from_slice(&[0x51, 0x29, 0xFF]); // INC4 A1; ADD -1, D1
    emit_bne_back(buf, loop_off, tag)
}

pub(super) fn emit_delay_iters(buf: &mut Vec<u8>, iters: u32) -> Result<()> {
    let _ = emit_mov_imm32(buf, 0xCD, iters);
    let loop_off = buf.len();
    buf.extend_from_slice(&[0x29, 0xFF]); // ADD -1, D1
    emit_bne_back(buf, loop_off, "DELAY_ITERS")
}

pub(super) fn emit_taskfile_snapshot(
    buf: &mut Vec<u8>,
    channel_mmio: u32,
    out_va: u32,
    sentinel: u32,
) -> Result<()> {
    emit_word_transfer_loop(
        buf,
        channel_mmio.wrapping_add(0x20),
        out_va,
        8,
        true,
        "COPY_TF",
    )?;
    emit_word_transfer_loop(
        buf,
        channel_mmio.wrapping_add(0x58),
        out_va.wrapping_add(0x20),
        1,
        true,
        "COPY_ALTSTAT",
    )?;
    emit_mmio_write32(buf, out_va.wrapping_add(0x24), sentinel);
    Ok(())
}

pub(super) fn emit_scratchpad_probe_seq(
    buf: &mut Vec<u8>,
    channel_mmio: u32,
    drive_sel: u8,
    out_va: u32,
    sentinel: u32,
) -> Result<()> {
    emit_mmio_write32(buf, channel_mmio.wrapping_add(0x38), u32::from(drive_sel));
    emit_delay_iters(buf, 0x1_0000)?;
    emit_mmio_write32(buf, channel_mmio.wrapping_add(0x28), 0x0000_0055);
    emit_mmio_write32(buf, channel_mmio.wrapping_add(0x2C), 0x0000_00AA);
    emit_delay_iters(buf, 0x1_0000)?;
    emit_taskfile_snapshot(buf, channel_mmio, out_va, sentinel)
}

/// Builds the F/E release + `DRV S211` decompression stub (`0xC000_1008 |= 0x807F`).
pub fn build_fe_release_stub(sdram_base: u32) -> CompiledProbeStub {
    let mut buf = Vec::with_capacity(96);
    emit_add_sp_imm8(&mut buf, -0x10);
    let _ = emit_mov_imm32(&mut buf, 0xCC, 0x8408_0000);
    emit_call32_abs(&mut buf, sdram_base, 0x4400_D016);
    let _ = emit_mov_imm32(&mut buf, 0xCC, 0x901F_3B60);
    buf.extend_from_slice(&[0x2D, 0x80, 0x00, 0x46, 0x0C, 0x04]);
    emit_call32_abs(&mut buf, sdram_base, 0x4402_A39B);
    emit_mmio_write32(&mut buf, 0xDC0F_0080, 0x0020_2000);
    buf.extend_from_slice(&[0xFC, 0xAC]);
    buf.extend_from_slice(&AM34_SYS_STATUS_REG.to_le_bytes());
    buf.extend_from_slice(&[0x81, 0xFA, 0xE5, 0x7F, 0x80, 0xFC, 0x87]);
    buf.extend_from_slice(&AM34_SYS_STATUS_REG.to_le_bytes());
    emit_add_sp_imm8(&mut buf, 0x10);
    emit_rets(&mut buf);
    CompiledProbeStub {
        name: "fe_release_stub".to_string(),
        load_va: sdram_base,
        code: buf,
        output_va: None,
        output_len: 0,
        meta_va: None,
    }
}

/// Builds a stub calling `fn_va(arg_u32)` (e.g., `FUN_5041CD4D(1)`) and storing `D0` return code.
pub fn build_ch1_call_stub(fn_va: u32, arg_u32: u32, sdram_base: u32) -> CompiledProbeStub {
    let rc_va = (sdram_base & 0xFFFF_0000).wrapping_add(0x12FC);
    let mut buf = Vec::with_capacity(48);
    emit_mmio_write32(&mut buf, rc_va, 0xA5A5_A5A5);
    emit_add_sp_imm8(&mut buf, -0x10);
    let _ = emit_mov_imm32(&mut buf, 0xCC, arg_u32);
    emit_call32_abs(&mut buf, sdram_base, fn_va);
    emit_store_d0_abs32(&mut buf, rc_va);
    emit_add_sp_imm8(&mut buf, 0x10);
    emit_rets(&mut buf);
    CompiledProbeStub {
        name: format!("call_{fn_va:08X}"),
        load_va: sdram_base,
        code: buf,
        output_va: Some(rc_va),
        output_len: 4,
        meta_va: Some(rc_va),
    }
}

/// Builds a compact stub writing `(addr, value)` pairs and returning cleanly with `RETS`.
pub fn build_mmio_write_stub(
    name: &str,
    sdram_base: u32,
    writes: &[(u32, u32)],
) -> CompiledProbeStub {
    let mut buf = Vec::with_capacity(writes.len() * 12 + 4);
    for &(addr, val) in writes {
        emit_mmio_write32(&mut buf, addr, val);
    }
    emit_rets(&mut buf);
    CompiledProbeStub {
        name: name.to_string(),
        load_va: sdram_base,
        code: buf,
        output_va: None,
        output_len: 0,
        meta_va: None,
    }
}
