#![deny(unsafe_code)]

//! ATA `CH0`/`CH1` taskfile, scratchpad, and `IDENTIFY DEVICE (0xEC)` PIO capture stub builders.

use anyhow::Result;

use super::{
    CompiledProbeStub, DAVIS_CH0_CONTROL_REG_VA, DAVIS_IDENTIFY_BUF_DEFAULT_VA,
    DAVIS_IDENTIFY_META_DEFAULT_VA, DAVIS_IDENTIFY_SENTINEL, DAVIS_STUB_DEFAULT_VA,
    emit_delay_iters, emit_fill_words, emit_mmio_write32, emit_scratchpad_probe_seq,
    emit_taskfile_snapshot, emit_word_transfer_loop,
};
use crate::constants::AM34_DAVIS_PRI_MMIO_BASE;
use crate::re::mn10300_emit::{
    emit_and_imm32_d0, emit_bne_back, emit_branch8_placeholder, emit_cmp_imm, emit_ld_w_an_dm,
    emit_mov_imm32, emit_rets, emit_store_d0_abs32, patch_s8_rel,
};

/// Builds the Step 1C `DAVIS CH0` (ODD positive control) taskfile probe stub.
pub fn build_ch0_probe_stub(sdram_base: u32, out_va: u32) -> Result<CompiledProbeStub> {
    let mut buf = Vec::with_capacity(160);
    emit_fill_words(&mut buf, out_va, 10, 0xA5A5_A5A5)?;
    emit_mmio_write32(&mut buf, DAVIS_CH0_CONTROL_REG_VA, 0x0000_0002);
    emit_mmio_write32(&mut buf, AM34_DAVIS_PRI_MMIO_BASE + 0x58, 0x0000_0000);
    emit_mmio_write32(&mut buf, AM34_DAVIS_PRI_MMIO_BASE + 0x38, 0x0000_00A0);
    emit_delay_iters(&mut buf, 0x1_0000)?;
    emit_taskfile_snapshot(&mut buf, AM34_DAVIS_PRI_MMIO_BASE, out_va, 0xCAFE_0001)?;
    emit_rets(&mut buf);

    Ok(CompiledProbeStub {
        name: "ch0_positive_control".to_string(),
        load_va: sdram_base,
        code: buf,
        output_va: Some(out_va),
        output_len: 40,
        meta_va: None,
    })
}

/// Builds the Step 2A `DAVIS CH1` raw + Master (`0xA0`) + Slave (`0xB0`) scratchpad probe stub.
pub fn build_ch1_probe_stub(
    sdram_base: u32,
    channel_mmio: u32,
    out_raw_va: u32,
    out_master_va: u32,
    out_slave_va: u32,
) -> Result<CompiledProbeStub> {
    let reset_ctl_va = (channel_mmio & 0xFFFF_FF00).wrapping_sub(0xE0);
    let host_ctl_va = reset_ctl_va.wrapping_sub(0x20);
    let mut buf = Vec::with_capacity(512);
    for out_va in [out_raw_va, out_master_va, out_slave_va] {
        emit_fill_words(&mut buf, out_va, 10, 0xA5A5_A5A5)?;
    }
    emit_mmio_write32(&mut buf, host_ctl_va, 0x0000_0000);
    emit_delay_iters(&mut buf, 0x100)?;
    emit_mmio_write32(&mut buf, host_ctl_va, 0x0000_0003);
    emit_mmio_write32(&mut buf, reset_ctl_va, 0x0000_0000);
    emit_mmio_write32(&mut buf, channel_mmio.wrapping_add(0x58), 0x0000_0002);
    emit_mmio_write32(&mut buf, channel_mmio.wrapping_add(0x38), 0x0000_00A0);
    emit_delay_iters(&mut buf, 0x1_0000)?;
    emit_taskfile_snapshot(&mut buf, channel_mmio, out_raw_va, 0xCAFE_02A0)?;
    emit_scratchpad_probe_seq(&mut buf, channel_mmio, 0xA0, out_master_va, 0xCAFE_02A1)?;
    emit_scratchpad_probe_seq(&mut buf, channel_mmio, 0xB0, out_slave_va, 0xCAFE_02A2)?;
    emit_rets(&mut buf);

    Ok(CompiledProbeStub {
        name: "ch1_2a_raw_master_slave".to_string(),
        load_va: sdram_base,
        code: buf,
        output_va: Some(out_raw_va),
        output_len: 120,
        meta_va: None,
    })
}

/// Builds a single-drive post-reset `55/AA` scratchpad + taskfile capture stub (Steps 2B/2C).
pub fn build_ch1_scratchpad_probe_stub(
    name: &str,
    sdram_base: u32,
    channel_mmio: u32,
    drive_sel: u8,
    out_va: u32,
    sentinel: u32,
) -> Result<CompiledProbeStub> {
    let mut buf = Vec::with_capacity(224);
    emit_fill_words(&mut buf, out_va, 10, 0xA5A5_A5A5)?;
    emit_scratchpad_probe_seq(&mut buf, channel_mmio, drive_sel, out_va, sentinel)?;
    emit_rets(&mut buf);
    Ok(CompiledProbeStub {
        name: name.to_string(),
        load_va: sdram_base,
        code: buf,
        output_va: Some(out_va),
        output_len: 40,
        meta_va: None,
    })
}

/// Builds the ATA `IDENTIFY DEVICE` (`0xEC`) PIO capture stub writing 256 32-bit LE PIO words
/// (`1024` B) to `out_buf_va` (default `0x9050_2000`) and `[0xCAFE00EC, altstat]` to `meta_va`.
pub fn build_ata_identify_ec_stub(
    channel_mmio: u32,
    drive_sel: u8,
    out_buf_va: u32,
) -> Result<CompiledProbeStub> {
    let meta_va = if out_buf_va == DAVIS_IDENTIFY_BUF_DEFAULT_VA {
        DAVIS_IDENTIFY_META_DEFAULT_VA
    } else {
        out_buf_va.wrapping_add(1024)
    };
    let data_reg = channel_mmio.wrapping_add(0x20);
    let devsel_reg = channel_mmio.wrapping_add(0x38);
    let cmd_reg = channel_mmio.wrapping_add(0x3C);
    let altstat_reg = channel_mmio.wrapping_add(0x58);

    let mut buf = Vec::with_capacity(256);
    emit_fill_words(&mut buf, out_buf_va, 256, 0xA5A5_A5A5)?;
    emit_mmio_write32(&mut buf, meta_va, 0xA5A5_A5A5);
    emit_mmio_write32(&mut buf, meta_va.wrapping_add(4), 0xA5A5_A5A5);
    emit_mmio_write32(&mut buf, altstat_reg, 0x0000_0002);
    emit_mmio_write32(&mut buf, devsel_reg, u32::from(drive_sel));
    emit_delay_iters(&mut buf, 0x1_0000)?;
    emit_mmio_write32(&mut buf, cmd_reg, 0x0000_00EC);

    let _ = emit_mov_imm32(&mut buf, 0xDC, altstat_reg);
    let _ = emit_mov_imm32(&mut buf, 0xCD, 0x0010_0000);
    let poll_loop = buf.len();
    emit_ld_w_an_dm(&mut buf, 0, 0);
    emit_and_imm32_d0(&mut buf, 0x0000_0088);
    emit_cmp_imm(&mut buf, 0, 0x08);
    let br_drq = emit_branch8_placeholder(&mut buf, 0xC8);
    buf.extend_from_slice(&[0x29, 0xFF]); // ADD -1, D1
    emit_bne_back(&mut buf, poll_loop, "ATA_POLL_EC")?;
    let drq_ready = buf.len();
    patch_s8_rel(&mut buf, br_drq, drq_ready, "ATA_DRQ_RDY")?;

    emit_ld_w_an_dm(&mut buf, 0, 0);
    emit_store_d0_abs32(&mut buf, meta_va.wrapping_add(4));
    emit_word_transfer_loop(&mut buf, data_reg, out_buf_va, 256, false, "PIO_READ_EC")?;
    emit_mmio_write32(&mut buf, meta_va, DAVIS_IDENTIFY_SENTINEL);
    emit_rets(&mut buf);

    Ok(CompiledProbeStub {
        name: format!("ata_identify_ec_{drive_sel:02X}"),
        load_va: DAVIS_STUB_DEFAULT_VA,
        code: buf,
        output_va: Some(out_buf_va),
        output_len: 1024,
        meta_va: Some(meta_va),
    })
}
