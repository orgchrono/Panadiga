#![deny(unsafe_code)]

//! Clean-room `IC7501` Timer MCU `TBUS` keepalive packet builders and MN103S `M0` stub emitter.
//!
//! Targets `AM34` mailbox MMIO at `0xDC01_0000..0xDC01_0020` verified across `Loader ver0.61` / `BOOT.BIN`.

use byteorder::{ByteOrder, LittleEndian};

use crate::constants::{
    AM34_IC7501_TBUS_BASE, AM34_IC7501_TBUS_TX_OFF, IC7501_TBUS_ACK_STROBE_BIT,
    IC7501_TBUS_CHECKSUM_BIAS, IC7501_VFD_TEXT_CMD, IC7501_WDT_255S_CHECKSUM,
    IC7501_WDT_KEEPALIVE_CMD,
};
use crate::re::mn10300_emit::{emit_mov_imm32, emit_rets, emit_st_w_an_dm};

/// Computes the 8-bit `IC7501` `TBUS` checksum (`sum(frame[0..15]) - 0x5B` mod 256).
#[must_use]
pub fn compute_ic7501_tbus_checksum(frame15: &[u8; 15]) -> u8 {
    frame15
        .iter()
        .fold(0u8, |acc, &b| acc.wrapping_add(b))
        .wrapping_sub(IC7501_TBUS_CHECKSUM_BIAS)
}

fn finalize_tbus_frame(body: [u8; 15]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..15].copy_from_slice(&body);
    out[15] = compute_ic7501_tbus_checksum(&body);
    out
}

/// Builds the 16-byte `IC7501` 255-second hung-up watchdog frame (`FUN_44005cd0`: `0x8A, 0x02, 0x00, 0xFF`).
#[must_use]
pub fn build_ic7501_wdt_255s_frame() -> [u8; 16] {
    let mut body = [0u8; 15];
    body[0] = IC7501_WDT_KEEPALIVE_CMD;
    body[1] = 0x02;
    body[3] = 0xFF;
    finalize_tbus_frame(body)
}

/// Builds a 16-byte `IC7501` VFD 7-character ASCII display packet (`FUN_4400599a`: `0xA3`, `body[3..10]`).
#[must_use]
pub fn build_ic7501_vfd_text_frame(ascii7: &str) -> [u8; 16] {
    let mut body = [0u8; 15];
    body[0] = IC7501_VFD_TEXT_CMD;
    body[3..10].fill(b' ');
    for (slot, b) in body[3..10].iter_mut().zip(ascii7.bytes()) {
        *slot = if b.is_ascii_graphic() { b } else { b' ' };
    }
    finalize_tbus_frame(body)
}

/// Emits a position-independent `M0` MN103S stub for SDRAM (`PANAMON1>` `J` / `PROG061`)
/// that writes the 255s `IC7501` `TBUS` keepalive frame, strobes `0xDC01_0000` bit 4, and
/// returns checksum `0x30` in `D0`.
#[must_use]
pub fn emit_m0_tbus_wdt_keepalive_stub() -> Vec<u8> {
    let frame = build_ic7501_wdt_255s_frame();
    let w0 = LittleEndian::read_u32(&frame[0..4]);
    let w1 = LittleEndian::read_u32(&frame[4..8]);
    let w2 = LittleEndian::read_u32(&frame[8..12]);
    let w3 = LittleEndian::read_u32(&frame[12..16]);

    let mut buf = Vec::with_capacity(64);
    let _ = emit_mov_imm32(
        &mut buf,
        0xDC,
        AM34_IC7501_TBUS_BASE + AM34_IC7501_TBUS_TX_OFF,
    );
    let _ = emit_mov_imm32(&mut buf, 0xCC, w0);
    emit_st_w_an_dm(&mut buf, 0, 0);

    let _ = emit_mov_imm32(&mut buf, 0xCC, w1);
    buf.extend_from_slice(&[0xF8, 0x10, 0x04]);
    if w2 != w1 {
        let _ = emit_mov_imm32(&mut buf, 0xCC, w2);
    }
    buf.extend_from_slice(&[0xF8, 0x10, 0x08]);

    let _ = emit_mov_imm32(&mut buf, 0xCC, w3);
    buf.extend_from_slice(&[0xF8, 0x10, 0x0C]);

    let _ = emit_mov_imm32(&mut buf, 0xDD, AM34_IC7501_TBUS_BASE);
    buf.extend_from_slice(&[
        0x80,
        IC7501_TBUS_ACK_STROBE_BIT,
        0xF0,
        0x51,
        0x80,
        0x00,
        0xF0,
        0x51,
        0x80,
        IC7501_WDT_255S_CHECKSUM,
    ]);
    emit_rets(&mut buf);
    buf
}
