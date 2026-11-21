#![deny(unsafe_code)]

//! Interactive UART0 (`SC0`) pre-MAIN recovery monitor (`PANAMON1`) for `PROG061` / `DIGADIAG.FRM`.
//!
//! Placed in the verified `27,496`-byte `0xFF` slack region at file offset `0x039494`
//! (`VA = 0x50439494`) of `eh55_frm_PROG061_BOOT.bin`, invoked from `0x5040A2CD` when
//! `bgo_driver_init` fails on a corrupt BGO/NFROM flash partition.
//!
//! Command `'F'` (`nfrom_format`) is permanently disabled fail-closed (`ERR:E05_IPL_RO`) because
//! `0x50401F3C` targets the `ver0.63` `0x84200000..0x843FFFFF` window with uninitialized RAM hooks.

mod cmd_handlers;
mod jedec_emit;
mod mn103_emit;
#[cfg(test)]
mod tests;

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::constants::{AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE, AM34_SDRAM_UNCACHED_BASE};

use super::bgo_unbrick::PROG061_BASE_VA;
use super::nor_dumper::{NOR_MAGIC_FOOTER, NOR_MAGIC_HEADER};
use cmd_handlers::{HandlerEmitCtx, emit_panamon1_cmd_handlers};
use jedec_emit::{
    emit_check_nor_access_a2_sub, emit_jedec_erase_a2_sub, emit_jedec_poll_dq7_reset_sub,
    emit_jedec_prog_a2_d2_sub,
};
use mn103_emit::{
    MONITOR_GETC_BYTES, MONITOR_PUTC_BYTES, PatchTable, emit_branch8_placeholder, emit_call16_leaf,
    emit_jmp16_placeholder, emit_nor_dumper_sub, emit_print_str, emit_put_hex_byte_sub,
    emit_read_hex_u32_sub, emit_uppercase_d0, patch_s8_rel, patch_s16_rel,
};

/// File offset of verified contiguous `0xFF` slack inside `eh55_frm_PROG061_BOOT.bin`.
pub const OFF_UART_MONITOR_SLACK: usize = 0x0003_9494;
/// Virtual load address of the `PANAMON1` UART0 monitor entrypoint in SDRAM.
pub const PROG061_UART_MONITOR_VA: u32 = PROG061_BASE_VA + OFF_UART_MONITOR_SLACK as u32;
/// Maximum permitted size of the `PANAMON1` payload inside `PROG061` slack (`0x039494..0x040000`).
pub const UART_MONITOR_MAX_SLACK_BYTES: usize = 27_496;

/// Dynamically computed SDRAM offsets for patching the embedded `PANAMON1` NOR dumper bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Panamon1NorDumperOffsets {
    pub entry_va: u32,
    pub start_imm_off: usize,
    pub end_imm_off: usize,
    pub block_size: u32,
}

const fn sdram_to_uncached(va: u32) -> u32 {
    if va < AM34_SDRAM_UNCACHED_BASE {
        (va & 0x0FFF_FFFF) | AM34_SDRAM_UNCACHED_BASE
    } else {
        va
    }
}

impl Panamon1NorDumperOffsets {
    /// Returns `(uncached_entry_va, uncached_start_imm_va, uncached_end_imm_va)` for live SDRAM patching.
    #[must_use]
    pub const fn uncached_patch_vas(&self, payload_base_va: u32) -> (u32, u32, u32) {
        let uncached_base = sdram_to_uncached(payload_base_va);
        (
            sdram_to_uncached(self.entry_va),
            uncached_base.wrapping_add(self.start_imm_off as u32),
            uncached_base.wrapping_add(self.end_imm_off as u32),
        )
    }
}

/// 20-byte branch/return dispatcher installed at `0x5040A2CD..0x5040A2E0` in `uart_monitor` mode.
pub const PATCH_A2CD_UART_MONITOR: [u8; 20] = [
    0xDD, 0xC7, 0xF1, 0x02, 0x00, 0x00, 0x00, 0xA0, 0x00, 0xC8, 0x30, 0xA0, 0x01, 0xC8, 0x07, 0x80,
    0xFF, 0xCA, 0x33, 0xCB,
];

/// Assembles the position-coupled `PANAMON1` interactive UART0 monitor for `PROG061` at `base_va`.
pub fn build_prog061_uart_monitor_payload(base_va: u32) -> Result<Vec<u8>> {
    build_prog061_uart_monitor_with_offsets(base_va).map(|(buf, _)| buf)
}

/// Assembles `PANAMON1` and returns `(payload_bytes, Panamon1NorDumperOffsets)`.
pub fn build_prog061_uart_monitor_with_offsets(
    base_va: u32,
) -> Result<(Vec<u8>, Panamon1NorDumperOffsets)> {
    let mut strings: Vec<&[u8]> = vec![
        b"PANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n", // 0: prompt
        b"OK:BYPASS\r\n",                    // 1: bypass msg
        b"OK:ABORT\r\n",                     // 2: abort msg
        b"OK:FORMAT\r\n",                    // 3: format start msg
        b"OK:DONE\r\n",                      // 4: done msg
        b"\r\n",                             // 5: newline
        b"?\r\n",                            // 6: unknown / syntax error msg
        b"OK:WRITE\r\n",                     // 7: guarded write ok
        b"OK:ERASE\r\n",                     // 8: JEDEC sector erase ok
        b"ERR:E05_IPL_RO\r\n",               // 9: hardware lockout error
        b"OK:NOR_UNLOCK\r\n",                // 10: runtime Upper NOR unlock
        b"OK:NOR_LOCK\r\n",                  // 11: runtime Upper NOR lock
        b"ONFIRM_UPPER_NOR",                 // 12: U unlock confirm tail
        b"OCK",                              // 13: U lock tail
    ];
    let mut nor_hdr = Vec::with_capacity(20);
    nor_hdr.extend_from_slice(NOR_MAGIC_HEADER);
    nor_hdr.extend_from_slice(&AM34_FLASH_UNCACHED_BASE.to_le_bytes());
    nor_hdr.extend_from_slice(&AM34_FLASH_SIZE.to_le_bytes());
    nor_hdr.extend_from_slice(&4096u32.to_le_bytes());

    let mut buf = Vec::with_capacity(1024);
    let mut pt = PatchTable {
        str_refs: Vec::new(),
        calls16: Vec::new(),
        j16s: Vec::new(),
        beq8s: Vec::new(),
        bne8s: Vec::new(),
    };

    buf.extend_from_slice(&[0xCF, 0xF0]); // movm [d2,d3,a2,a3], (sp)
    buf.extend_from_slice(&[0xF8, 0xFE, 0xF0, 0x00, 0x42, 0x0C]); // add -16,sp; clr d0; mov d0,(12,sp)
    let j_main = emit_jmp16_placeholder(&mut buf);

    let putc_off = buf.len();
    buf.extend_from_slice(&MONITOR_PUTC_BYTES);

    let getc_off = buf.len();
    buf.extend_from_slice(&MONITOR_GETC_BYTES);

    let print_span_off = buf.len();
    buf.extend_from_slice(&[0xF8, 0xFE, 0xFC]); // add -4, sp
    let print_span_loop = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83]); // movbu (a0), d0; mov d0, d3
    emit_call16_leaf(&mut buf, &mut pt, putc_off);
    buf.extend_from_slice(&[0x20, 0x01, 0xB4]); // add 1, a0; cmp a1, a0
    pt.bne8s
        .push((emit_branch8_placeholder(&mut buf, 0xC9), print_span_loop));
    buf.extend_from_slice(&[0xF8, 0xFE, 0x04, 0xDF, 0x00, 0x00]); // add +4, sp; ret [], 0

    let dumper_locs = emit_nor_dumper_sub(&mut buf, &mut pt, print_span_off, putc_off, 14, 15);
    let dump_nor_off = dumper_locs.sub_off;
    let put_hex_byte_off = emit_put_hex_byte_sub(&mut buf, &mut pt, putc_off)?;
    let read_hex_u32_off = emit_read_hex_u32_sub(&mut buf, &mut pt, getc_off, putc_off)?;
    let check_nor_access_off = emit_check_nor_access_a2_sub(&mut buf)?;
    let jedec_poll_reset_off = emit_jedec_poll_dq7_reset_sub(&mut buf)?;
    let jedec_erase_a2_off = emit_jedec_erase_a2_sub(&mut buf, &mut pt, jedec_poll_reset_off);
    let jedec_prog_a2_d2_off = emit_jedec_prog_a2_d2_sub(&mut buf, &mut pt, jedec_poll_reset_off)?;

    let prompt_loop = buf.len();
    pt.j16s.push((j_main, prompt_loop));
    emit_print_str(&mut buf, &mut pt, 0, print_span_off);

    let rx_loop = buf.len();
    emit_call16_leaf(&mut buf, &mut pt, getc_off);
    for ign in [0x0Du8, 0x0Au8, 0x20u8] {
        buf.extend_from_slice(&[0xA0, ign]);
        pt.beq8s
            .push((emit_branch8_placeholder(&mut buf, 0xC8), rx_loop));
    }
    buf.push(0x83);
    emit_call16_leaf(&mut buf, &mut pt, putc_off);
    buf.push(0x8C);

    emit_uppercase_d0(&mut buf, "CMD_UP")?;

    let mut handler_jmps = Vec::with_capacity(9);
    for (cmd_ch, handler_kind) in [
        (b'B', 0u8),
        (b'Q', 1u8),
        (b'F', 2u8),
        (b'D', 3u8),
        (b'R', 4u8),
        (b'W', 5u8),
        (b'J', 6u8),
        (b'E', 7u8),
        (b'U', 8u8),
    ] {
        buf.extend_from_slice(&[0xA0, cmd_ch]);
        let skip_jmp = emit_branch8_placeholder(&mut buf, 0xC9);
        let jmp_off = emit_jmp16_placeholder(&mut buf);
        let after_jmp = buf.len();
        patch_s8_rel(&mut buf, skip_jmp, after_jmp, "DISP_BNE")?;
        handler_jmps.push((handler_kind, jmp_off));
    }
    emit_print_str(&mut buf, &mut pt, 6, print_span_off);
    pt.j16s
        .push((emit_jmp16_placeholder(&mut buf), prompt_loop));

    let syntax_err_off = buf.len();
    emit_print_str(&mut buf, &mut pt, 5, print_span_off);
    emit_print_str(&mut buf, &mut pt, 6, print_span_off);
    pt.j16s
        .push((emit_jmp16_placeholder(&mut buf), prompt_loop));

    let e05_err_off = buf.len();
    emit_print_str(&mut buf, &mut pt, 5, print_span_off);
    emit_print_str(&mut buf, &mut pt, 9, print_span_off);
    pt.j16s
        .push((emit_jmp16_placeholder(&mut buf), prompt_loop));

    let ctx = HandlerEmitCtx {
        print_span_off,
        putc_off,
        getc_off,
        dump_nor_off,
        put_hex_byte_off,
        read_hex_u32_off,
        check_nor_access_off,
        jedec_erase_a2_off,
        jedec_prog_a2_d2_off,
        prompt_loop,
        syntax_err_off,
        e05_err_off,
    };
    emit_panamon1_cmd_handlers(&mut buf, &mut pt, &ctx, &handler_jmps)?;

    strings.push(&nor_hdr);
    strings.push(NOR_MAGIC_FOOTER);
    strings.push(b"ERR:ERASE\r\n"); // index 16
    strings.push(b"ERR:WRITE\r\n"); // index 17
    let mut str_spans = Vec::with_capacity(strings.len());
    for s in &strings {
        let s_off = buf.len();
        buf.extend_from_slice(s);
        str_spans.push((s_off, buf.len()));
    }

    for (a0_patch, a1_patch, idx) in pt.str_refs {
        let (s_off, e_off) = str_spans[idx];
        LittleEndian::write_u32(
            &mut buf[a0_patch..a0_patch + 4],
            base_va.wrapping_add(s_off as u32),
        );
        LittleEndian::write_u32(
            &mut buf[a1_patch..a1_patch + 4],
            base_va.wrapping_add(e_off as u32),
        );
    }
    for (call_off, tgt) in pt.calls16 {
        patch_s16_rel(&mut buf, call_off, tgt, "CALL16")?;
    }
    for (jmp_off, tgt) in pt.j16s {
        patch_s16_rel(&mut buf, jmp_off, tgt, "JMP16")?;
    }
    for (br_off, tgt) in pt.beq8s {
        patch_s8_rel(&mut buf, br_off, tgt, "BEQ8")?;
    }
    for (br_off, tgt) in pt.bne8s {
        patch_s8_rel(&mut buf, br_off, tgt, "BNE8")?;
    }

    if buf.len() > UART_MONITOR_MAX_SLACK_BYTES {
        bail!(
            "PANAMON1 payload ({} B) exceeds PROG061 slack ({UART_MONITOR_MAX_SLACK_BYTES} B)",
            buf.len()
        );
    }
    let offsets = Panamon1NorDumperOffsets {
        entry_va: base_va.wrapping_add(dumper_locs.sub_off as u32),
        start_imm_off: dumper_locs.start_imm_off,
        end_imm_off: dumper_locs.end_imm_off,
        block_size: dumper_locs.block_size,
    };
    Ok((buf, offsets))
}
