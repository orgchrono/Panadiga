#![deny(unsafe_code)]

//! MN103S command dispatch handlers (`B`/`Q`/`F`/`D`/`R`/`W`/`J`/`E`/`U`) for `PANAMON1`.

use anyhow::Result;

use super::jedec_emit::{emit_nor_unlock_handler, emit_write_or_erase_handler};
use super::mn103_emit::{
    PatchTable, emit_branch8_placeholder, emit_call16, emit_cond_jmp16, emit_jmp16_placeholder,
    emit_print_str, emit_read_hex_u32_or_syntax, patch_s8_rel,
};

pub(super) struct HandlerEmitCtx {
    pub print_span_off: usize,
    pub putc_off: usize,
    pub getc_off: usize,
    pub dump_nor_off: usize,
    pub put_hex_byte_off: usize,
    pub read_hex_u32_off: usize,
    pub check_nor_access_off: usize,
    pub jedec_erase_a2_off: usize,
    pub jedec_prog_a2_d2_off: usize,
    pub prompt_loop: usize,
    pub syntax_err_off: usize,
    pub e05_err_off: usize,
}

pub(super) fn emit_panamon1_cmd_handlers(
    buf: &mut Vec<u8>,
    pt: &mut PatchTable,
    ctx: &HandlerEmitCtx,
    handler_jmps: &[(u8, usize)],
) -> Result<()> {
    for &(handler_kind, jmp_off) in handler_jmps {
        let tgt_off = buf.len();
        pt.j16s.push((jmp_off, tgt_off));
        match handler_kind {
            0 => {
                emit_print_str(buf, pt, 1, ctx.print_span_off);
                buf.extend_from_slice(&[0x00, 0xDF, 0xF0, 0x20]); // clr d0; ret [d2..a3], 32
            }
            1 => {
                emit_print_str(buf, pt, 2, ctx.print_span_off);
                buf.extend_from_slice(&[0x80, 0xFF, 0xDF, 0xF0, 0x20]); // mov -1,d0; ret [d2..a3], 32
            }
            2 => {
                // Fail-closed: `'F'` never invokes `nfrom_format` (`0x50401F3C`).
                pt.j16s.push((emit_jmp16_placeholder(buf), ctx.e05_err_off));
            }
            3 => {
                emit_print_str(buf, pt, 5, ctx.print_span_off);
                emit_call16(buf, pt, ctx.dump_nor_off);
                emit_print_str(buf, pt, 5, ctx.print_span_off);
                pt.j16s.push((emit_jmp16_placeholder(buf), ctx.prompt_loop));
            }
            4 => emit_read_handler(buf, pt, ctx)?,
            5 | 7 => {
                let is_erase = handler_kind == 7;
                let op_sub = if is_erase {
                    ctx.jedec_erase_a2_off
                } else {
                    ctx.jedec_prog_a2_d2_off
                };
                emit_write_or_erase_handler(
                    buf,
                    pt,
                    (
                        ctx.read_hex_u32_off,
                        ctx.check_nor_access_off,
                        op_sub,
                        ctx.print_span_off,
                    ),
                    (ctx.prompt_loop, ctx.syntax_err_off, ctx.e05_err_off),
                    is_erase,
                )?;
            }
            6 => {
                emit_read_hex_u32_or_syntax(buf, pt, ctx.read_hex_u32_off, ctx.syntax_err_off)?;
                buf.extend_from_slice(&[0xF1, 0xEA]);
                emit_print_str(buf, pt, 5, ctx.print_span_off);
                buf.extend_from_slice(&[0xF0, 0xF2]); // calls (a2)
                emit_print_str(buf, pt, 4, ctx.print_span_off);
                pt.j16s.push((emit_jmp16_placeholder(buf), ctx.prompt_loop));
            }
            _ => emit_nor_unlock_handler(
                buf,
                pt,
                ctx.getc_off,
                ctx.putc_off,
                ctx.print_span_off,
                (ctx.prompt_loop, ctx.syntax_err_off),
                (12, 13, 5, 10, 11),
            )?,
        }
    }
    Ok(())
}

fn emit_read_handler(buf: &mut Vec<u8>, pt: &mut PatchTable, ctx: &HandlerEmitCtx) -> Result<()> {
    emit_read_hex_u32_or_syntax(buf, pt, ctx.read_hex_u32_off, ctx.syntax_err_off)?;
    buf.extend_from_slice(&[0xF1, 0xEA, 0xA0, b',']);
    let bne_default = emit_branch8_placeholder(buf, 0xC9);
    emit_read_hex_u32_or_syntax(buf, pt, ctx.read_hex_u32_off, ctx.syntax_err_off)?;
    let bra_len_ok = emit_branch8_placeholder(buf, 0xCA);
    let default_off = buf.len();
    patch_s8_rel(buf, bne_default, default_off, "R_DEF_BNE")?;
    buf.extend_from_slice(&[0x8A, 0x10]);
    let len_ok_off = buf.len();
    patch_s8_rel(buf, bra_len_ok, len_ok_off, "R_LEN_BRA")?;
    emit_print_str(buf, pt, 5, ctx.print_span_off);
    buf.extend_from_slice(&[0xAA, 0x00]);
    emit_cond_jmp16(buf, pt, 0xC9, ctx.prompt_loop)?;
    let r_loop = buf.len();
    buf.extend_from_slice(&[0xF0, 0x42]);
    emit_call16(buf, pt, ctx.put_hex_byte_off);
    buf.extend_from_slice(&[0x22, 0x01, 0x2A, 0xFF, 0xAA, 0x00]);
    pt.bne8s.push((emit_branch8_placeholder(buf, 0xC9), r_loop));
    emit_print_str(buf, pt, 5, ctx.print_span_off);
    pt.j16s.push((emit_jmp16_placeholder(buf), ctx.prompt_loop));
    Ok(())
}
