//! Interactive UART0 (`SC0`) pre-MAIN recovery monitor (`PANAMON1`) for `PROG061` / `DIGADIAG.FRM`.
//!
//! Placed in the verified `27,496`-byte `0xFF` slack region at file offset `0x039494`
//! (`VA = 0x50439494`) of `eh55_frm_PROG061_BOOT.bin`, invoked from `0x5040A2CD` when
//! `bgo_driver_init` fails on a corrupt BGO/NFROM flash partition.
//!
//! Supported single-character UART0 commands (`115200 8N1`):
//! - `'B'`: Bypass BGO checks and return `0` to `0x5040A306` (continue FVU/IDE boot).
//! - `'F'`: Invoke orphan `nfrom_format` (`0x50401F3C`) to format BGO and re-prompt.
//! - `'D'`: Stream 8 MB read-only uncached NOR dump (`0x84000000..0x847FFFFF`, `PANADMP1..PANADONE`).
//! - `'Q'`: Return `-1` to `0x5040A311` (factory error exit).

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use super::bgo_unbrick::{PROG061_BASE_VA, PROG061_NFROM_FORMAT_VA};
use super::nor_dumper::{NOR_MAGIC_FOOTER, NOR_MAGIC_HEADER, PUTC_D3_BYTES};

/// File offset of verified contiguous `0xFF` slack inside `eh55_frm_PROG061_BOOT.bin`.
pub const OFF_UART_MONITOR_SLACK: usize = 0x0003_9494;
/// Virtual load address of the `PANAMON1` UART0 monitor entrypoint in SDRAM.
pub const PROG061_UART_MONITOR_VA: u32 = PROG061_BASE_VA + OFF_UART_MONITOR_SLACK as u32;
/// Maximum permitted size of the `PANAMON1` payload inside `PROG061` slack (`0x039494..0x040000`).
pub const UART_MONITOR_MAX_SLACK_BYTES: usize = 27_496;

/// 20-byte branch/return dispatcher installed at `0x5040A2CD..0x5040A2E0` in `uart_monitor` mode.
///
/// ```text
/// 0x5040A2CD: DD C7 F1 02 00 00 00   call 0x50439494, [], 0
/// 0x5040A2D4: A0 00                  cmp  0, d0
/// 0x5040A2D6: C8 30                  beq  0x5040A306   ; bypass BGO -> mark init ok, return 0
/// 0x5040A2D8: A0 01                  cmp  1, d0
/// 0x5040A2DA: C8 07                  beq  0x5040A2E1   ; retry Control info init
/// 0x5040A2DC: 80 FF                  mov  -1, d0
/// 0x5040A2DE: CA 33                  bra  0x5040A311   ; factory error return
/// 0x5040A2E0: CB                     nop
/// ```
pub const PATCH_A2CD_UART_MONITOR: [u8; 20] = [
    0xDD, 0xC7, 0xF1, 0x02, 0x00, 0x00, 0x00, 0xA0, 0x00, 0xC8, 0x30, 0xA0, 0x01, 0xC8, 0x07, 0x80,
    0xFF, 0xCA, 0x33, 0xCB,
];

const GETC_D0_BYTES: [u8; 23] = [
    0xFC, 0xAC, 0x0C, 0x20, 0x00, 0xD4, // movhu (0xD400200C), d0 (SC0STR)
    0x1C, // exthu d0
    0xF8, 0xE0, 0x10, // and 0x10, d0 (RX ready bit)
    0xA0, 0x00, // cmp 0, d0
    0xC8, 0xF4, // beq -12
    0xFC, 0xA8, 0x09, 0x20, 0x00, 0xD4, // movbu (0xD4002009), d0 (SC0RXB)
    0xDF, 0x00, 0x00, // ret [], 0
];

struct PatchTable {
    str_refs: Vec<(usize, usize, usize)>, // (patch_a0, patch_a1, str_idx)
    calls16: Vec<(usize, usize)>,         // (call_off, target_off)
    j16s: Vec<(usize, usize)>,            // (jmp_off, target_off)
    beq8s: Vec<(usize, usize)>,           // (beq_off, target_off)
    bne8s: Vec<(usize, usize)>,           // (bne_off, target_off)
}

fn emit_mov_imm32(buf: &mut Vec<u8>, sub_op: u8, imm: u32) -> usize {
    let off = buf.len() + 2;
    buf.extend_from_slice(&[0xFC, sub_op]);
    buf.extend_from_slice(&imm.to_le_bytes());
    off
}

fn emit_call16(buf: &mut Vec<u8>, pt: &mut PatchTable, target: usize) {
    let off = buf.len();
    buf.extend_from_slice(&[0xCD, 0x00, 0x00, 0x00, 0x00]);
    pt.calls16.push((off, target));
}

fn emit_jmp16_placeholder(buf: &mut Vec<u8>) -> usize {
    let off = buf.len();
    buf.extend_from_slice(&[0xCC, 0x00, 0x00]);
    off
}

fn emit_branch8_placeholder(buf: &mut Vec<u8>, opcode: u8) -> usize {
    let off = buf.len();
    buf.extend_from_slice(&[opcode, 0x00]);
    off
}

fn patch_s16_rel(buf: &mut [u8], insn_off: usize, target_off: usize, kind: &str) -> Result<()> {
    let disp = (target_off as isize) - (insn_off as isize);
    if !(-32768..=32767).contains(&disp) {
        bail!("MN103S {kind} displacement out of s16 range: {disp}");
    }
    LittleEndian::write_i16(&mut buf[insn_off + 1..insn_off + 3], disp as i16);
    Ok(())
}

fn patch_s8_rel(buf: &mut [u8], insn_off: usize, target_off: usize, kind: &str) -> Result<()> {
    let disp = (target_off as isize) - (insn_off as isize);
    if !(-128..=127).contains(&disp) {
        bail!("MN103S {kind} displacement out of s8 range: {disp}");
    }
    buf[insn_off + 1] = (disp as i8) as u8;
    Ok(())
}

fn emit_print_str(buf: &mut Vec<u8>, pt: &mut PatchTable, str_idx: usize, print_span_off: usize) {
    let a0 = emit_mov_imm32(buf, 0xDC, 0);
    let a1 = emit_mov_imm32(buf, 0xDD, 0);
    pt.str_refs.push((a0, a1, str_idx));
    emit_call16(buf, pt, print_span_off);
}

fn emit_nor_dumper_sub(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    print_span_off: usize,
    putc_off: usize,
    hdr_str_idx: usize,
    ftr_str_idx: usize,
) -> usize {
    let sub_off = buf.len();
    emit_print_str(buf, pt, hdr_str_idx, print_span_off);
    let _ = emit_mov_imm32(buf, 0xDC, 0x8400_0000);
    let _ = emit_mov_imm32(buf, 0xDD, 0x8480_0000);

    let blk_loop = buf.len();
    buf.extend_from_slice(&[0x8A, 0x00]); // mov 0, d2
    let _ = emit_mov_imm32(buf, 0xCD, 4096); // mov 4096, d1

    let byte_loop = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83, 0xE2]); // movbu (a0), d0; mov d0, d3; add d0, d2
    emit_call16(buf, pt, putc_off);
    buf.extend_from_slice(&[0x20, 0x01, 0x29, 0xFF, 0xA5, 0x00]); // add 1,a0; add -1,d1; cmp 0,d1
    pt.bne8s
        .push((emit_branch8_placeholder(buf, 0xC9), byte_loop));

    buf.extend_from_slice(&[0x85, 0x04]); // mov 4, d1
    let csum_loop = buf.len();
    buf.extend_from_slice(&[0x88, 0x83]); // mov d2, d0; mov d0, d3
    emit_call16(buf, pt, putc_off);
    buf.extend_from_slice(&[0xF8, 0xC6, 0x08, 0x29, 0xFF, 0xA5, 0x00]); // lsr 8,d2; add -1,d1; cmp 0,d1
    pt.bne8s
        .push((emit_branch8_placeholder(buf, 0xC9), csum_loop));

    buf.push(0xB4); // cmp a1, a0
    pt.bne8s
        .push((emit_branch8_placeholder(buf, 0xC9), blk_loop));

    emit_print_str(buf, pt, ftr_str_idx, print_span_off);
    buf.extend_from_slice(&[0xDF, 0x00, 0x00]); // ret [], 0
    sub_off
}

/// Assembles the position-coupled `PANAMON1` interactive UART0 monitor for `PROG061` at `base_va`.
pub fn build_prog061_uart_monitor_payload(base_va: u32) -> Result<Vec<u8>> {
    let mut strings: Vec<&[u8]> = vec![
        b"PANAMON1>[B/F/D/Q]\r\n", // 0: prompt
        b"OK:BYPASS\r\n",          // 1: bypass msg
        b"OK:ABORT\r\n",           // 2: abort msg
        b"OK:FORMAT\r\n",          // 3: format start msg
        b"OK:DONE\r\n",            // 4: format done msg
        b"\r\n",                   // 5: newline
        b"?\r\n",                  // 6: unknown cmd
    ];
    let mut nor_hdr = Vec::with_capacity(20);
    nor_hdr.extend_from_slice(NOR_MAGIC_HEADER);
    nor_hdr.extend_from_slice(&0x8400_0000u32.to_le_bytes());
    nor_hdr.extend_from_slice(&0x0080_0000u32.to_le_bytes());
    nor_hdr.extend_from_slice(&4096u32.to_le_bytes());

    let mut buf = Vec::with_capacity(512);
    let mut pt = PatchTable {
        str_refs: Vec::new(),
        calls16: Vec::new(),
        j16s: Vec::new(),
        beq8s: Vec::new(),
        bne8s: Vec::new(),
    };

    // Reserve jump over helper subroutines so entrypoint stays cleanly at offset 0 (+2 bytes after CF F0).
    buf.extend_from_slice(&[0xCF, 0xF0]); // movm [d2,d3,a2,a3], (sp)
    let j_main = emit_jmp16_placeholder(&mut buf);

    let putc_off = buf.len();
    buf.extend_from_slice(&PUTC_D3_BYTES);

    let getc_off = buf.len();
    buf.extend_from_slice(&GETC_D0_BYTES);

    let print_span_off = buf.len();
    buf.extend_from_slice(&[0xF0, 0x40, 0x83]); // movbu (a0), d0; mov d0, d3
    emit_call16(&mut buf, &mut pt, putc_off);
    buf.extend_from_slice(&[0x20, 0x01, 0xB4]); // add 1, a0; cmp a1, a0
    pt.bne8s
        .push((emit_branch8_placeholder(&mut buf, 0xC9), print_span_off));
    buf.extend_from_slice(&[0xDF, 0x00, 0x00]); // ret [], 0

    let dump_nor_off = emit_nor_dumper_sub(&mut buf, &mut pt, print_span_off, putc_off, 7, 8);

    // Main prompt loop
    let prompt_loop = buf.len();
    pt.j16s.push((j_main, prompt_loop));
    emit_print_str(&mut buf, &mut pt, 0, print_span_off);

    let rx_loop = buf.len();
    emit_call16(&mut buf, &mut pt, getc_off);
    for ign in [0x0Du8, 0x0Au8, 0x20u8] {
        buf.extend_from_slice(&[0xA0, ign]); // cmp ign, d0
        pt.beq8s
            .push((emit_branch8_placeholder(&mut buf, 0xC8), rx_loop));
    }
    buf.push(0x83); // mov d0, d3 (echo RX byte cleanly)
    emit_call16(&mut buf, &mut pt, putc_off);
    buf.push(0x8C); // mov d3, d0

    // Uppercase ASCII 'a'..'z' -> 'A'..'Z'
    buf.extend_from_slice(&[0xA0, b'a']);
    let blt_skip = emit_branch8_placeholder(&mut buf, 0xC0); // blt done_upper
    buf.extend_from_slice(&[0xA0, b'z']);
    let bgt_skip = emit_branch8_placeholder(&mut buf, 0xC1); // bgt done_upper
    buf.extend_from_slice(&[0x28, 0xE0]); // add -0x20, d0
    let done_upper = buf.len();
    patch_s8_rel(&mut buf, blt_skip, done_upper, "BLT")?;
    patch_s8_rel(&mut buf, bgt_skip, done_upper, "BGT")?;

    let mut next_cases = Vec::new();
    for (cmd_ch, handler_kind) in [(b'B', 0u8), (b'Q', 1u8), (b'F', 2u8), (b'D', 3u8)] {
        buf.extend_from_slice(&[0xA0, cmd_ch]); // cmp cmd_ch, d0
        let skip_case = emit_branch8_placeholder(&mut buf, 0xC9); // bne next_case
        match handler_kind {
            0 => {
                emit_print_str(&mut buf, &mut pt, 1, print_span_off);
                buf.extend_from_slice(&[0x00, 0xDF, 0xF0, 0x00]); // clr d0; ret [d2,d3,a2,a3], 0
            }
            1 => {
                emit_print_str(&mut buf, &mut pt, 2, print_span_off);
                buf.extend_from_slice(&[0x80, 0xFF, 0xDF, 0xF0, 0x00]); // mov -1, d0; ret
            }
            2 => {
                emit_print_str(&mut buf, &mut pt, 3, print_span_off);
                let call_fmt_off = buf.len();
                let call_fmt_va = (base_va as i64) + (call_fmt_off as i64);
                let rel32 = (PROG061_NFROM_FORMAT_VA as i64) - call_fmt_va;
                buf.push(0xDD); // call32
                buf.extend_from_slice(&(rel32 as i32).to_le_bytes());
                buf.extend_from_slice(&[0x00, 0x00]);
                emit_print_str(&mut buf, &mut pt, 4, print_span_off);
                pt.j16s
                    .push((emit_jmp16_placeholder(&mut buf), prompt_loop));
            }
            _ => {
                emit_print_str(&mut buf, &mut pt, 5, print_span_off);
                emit_call16(&mut buf, &mut pt, dump_nor_off);
                emit_print_str(&mut buf, &mut pt, 5, print_span_off);
                pt.j16s
                    .push((emit_jmp16_placeholder(&mut buf), prompt_loop));
            }
        }
        let next_off = buf.len();
        next_cases.push((skip_case, next_off));
    }
    emit_print_str(&mut buf, &mut pt, 6, print_span_off);
    pt.j16s
        .push((emit_jmp16_placeholder(&mut buf), prompt_loop));

    // Append constant strings and PANADMP1/PANADONE metadata
    strings.push(&nor_hdr);
    strings.push(NOR_MAGIC_FOOTER);
    let mut str_spans = Vec::with_capacity(strings.len());
    for s in &strings {
        let s_off = buf.len();
        buf.extend_from_slice(s);
        let e_off = buf.len();
        str_spans.push((s_off, e_off));
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
    for (br_off, tgt) in pt.bne8s.into_iter().chain(next_cases) {
        patch_s8_rel(&mut buf, br_off, tgt, "BNE8")?;
    }

    if buf.len() > UART_MONITOR_MAX_SLACK_BYTES {
        bail!(
            "PANAMON1 payload ({} B) exceeds PROG061 slack ({UART_MONITOR_MAX_SLACK_BYTES} B)",
            buf.len()
        );
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uart_monitor_payload_assembles_cleanly_within_slack() {
        let payload = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA).unwrap();
        assert!(payload.len() <= 512);
        assert_eq!(&payload[..2], &[0xCF, 0xF0]); // movm [d2,d3,a2,a3], (sp)
        assert!(payload.windows(8).any(|w| w == b"PANAMON1"));
        assert!(payload.windows(8).any(|w| w == b"PANADMP1"));
        assert!(payload.windows(8).any(|w| w == b"PANADONE"));
        // Verify CALL32 displacement from 0x5040A2CD -> 0x50439494
        let rel = i32::from_le_bytes(PATCH_A2CD_UART_MONITOR[1..5].try_into().unwrap());
        assert_eq!(
            (0x5040_A2CDi64 + rel as i64) as u32,
            PROG061_UART_MONITOR_VA
        );
    }
}
