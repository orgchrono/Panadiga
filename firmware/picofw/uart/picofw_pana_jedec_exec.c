/**
 * @file picofw_pana_jedec_exec.c
 * @brief MN103S SRAM common JEDEC 6-cycle Erase & 4-cycle Program stub machine-code assembler.
 *
 * Split from `picofw_pana_jedec_stubs.c` (`picofw_pana_jedec_stubs.c` owns emitter primitives & policy checks).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_pana_jedec.h"
#include "uart/picofw_pana_jedec_internal.h"
#include "flash/picofw_flash.h"

static void j_emit_jmp16_fail(
    picofw_jedec_buf_t *jb,
    size_t *fail_jmps,
    size_t *fail_cnt,
    size_t fail_cap
) {
    static const uint8_t k_j16[3] = { 0xCCu, 0x00u, 0x00u };
    if ((fail_jmps != NULL) && (fail_cnt != NULL) && (*fail_cnt < fail_cap)) {
        fail_jmps[(*fail_cnt)++] = jb->pos;
    }
    picofw_j_arr(jb, k_j16, sizeof(k_j16));
}

size_t picofw_j_build_stub_common(
    bool is_erase,
    uint32_t norm_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    static const char k_ok_str[] = PICOFW_JEDEC_OK_TOKEN "\r\n";
    static const char k_to_str[] = PICOFW_JEDEC_TO_TOKEN "\r\n";
    static const uint8_t k_j16[3] = { 0xCCu, 0x00u, 0x00u };
    const uint32_t hw_base = PICOFW_SSOT_CS0_UNCACHED_BASE;

    picofw_jedec_buf_t jb = { out_buf, 0u, out_cap };
    size_t fail_jmps[4] = { 0u, 0u, 0u, 0u };
    size_t fail_cnt = 0u;
    size_t src_patch = 0u;
    size_t sector_loop = 0u;
    size_t br_to_ok = 0u;
    size_t br_dq5_ok = 0u;

    if (is_erase) {
        (void)picofw_j_imm32(&jb, 0xDEu, norm_addr & ~1UL);
        sector_loop = jb.pos;
        picofw_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xAAu);
        picofw_j_cmd_a0(&jb, hw_base + 0x0554UL, 0x55u);
        picofw_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0x80u);
        picofw_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xAAu);
        picofw_j_cmd_a0(&jb, hw_base + 0x0554UL, 0x55u);
        static const uint8_t k_sec30[4] = { 0x80u, 0x30u, 0xF0u, 0x72u };
        picofw_j_arr(&jb, k_sec30, sizeof(k_sec30));
        (void)picofw_j_imm32(&jb, 0xCDu, PICOFW_JEDEC_ERASE_POLL_ITERS);
        const size_t poll_off = jb.pos;
        static const uint8_t k_dq7_e[5] = { 0xF0u, 0x62u, 0xF8u, 0xECu, 0x80u };
        picofw_j_arr(&jb, k_dq7_e, sizeof(k_dq7_e));
        br_to_ok = jb.pos;
        picofw_j_u8(&jb, 0xC9u); picofw_j_u8(&jb, 0u);
        static const uint8_t k_dq5_e[3] = { 0xF8u, 0xECu, 0x20u };
        picofw_j_arr(&jb, k_dq5_e, sizeof(k_dq5_e));
        const size_t br_e_busy = jb.pos;
        picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0u);
        picofw_j_arr(&jb, k_dq7_e, sizeof(k_dq7_e));
        br_dq5_ok = jb.pos;
        picofw_j_u8(&jb, 0xC9u); picofw_j_u8(&jb, 0u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        picofw_j_patch_s8(&jb, br_e_busy, jb.pos);
        static const uint8_t k_dec[4] = { 0x29u, 0xFFu, 0xA5u, 0x00u };
        picofw_j_arr(&jb, k_dec, sizeof(k_dec));
        picofw_j_br(&jb, 0xC9u, poll_off);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
    } else {
        (void)picofw_j_imm32(&jb, 0xDEu, norm_addr);
        src_patch = picofw_j_imm32(&jb, 0xDFu, 0u);
        const size_t prog_loop = jb.pos;
        static const uint8_t k_rd_align[6] = { 0xF0u, 0x4Bu, 0x23u, 0x01u, 0xF1u, 0xD9u };
        picofw_j_arr(&jb, k_rd_align, sizeof(k_rd_align));
        (void)picofw_j_imm32(&jb, 0xCCu, 0xFFFFFFFEUL);
        static const uint8_t k_prep[11] = {
            0xF2u, 0x01u, 0xF1u, 0xE5u, 0xF0u, 0x61u, 0xF1u, 0xD9u, 0xF8u, 0xEDu, 0x01u
        };
        picofw_j_arr(&jb, k_prep, sizeof(k_prep));
        const size_t br_odd = jb.pos;
        picofw_j_u8(&jb, 0xC9u); picofw_j_u8(&jb, 0u);
        (void)picofw_j_imm32(&jb, 0xCDu, 0x0000FF00UL);
        static const uint8_t k_even_merge[4] = { 0xF2u, 0x04u, 0xF2u, 0x12u };
        picofw_j_arr(&jb, k_even_merge, sizeof(k_even_merge));
        const size_t br_go = jb.pos;
        picofw_j_u8(&jb, 0xCAu); picofw_j_u8(&jb, 0u);
        picofw_j_patch_s8(&jb, br_odd, jb.pos);
        static const uint8_t k_odd_merge[6] = { 0x14u, 0xF8u, 0xC2u, 0x08u, 0xF2u, 0x12u };
        picofw_j_arr(&jb, k_odd_merge, sizeof(k_odd_merge));
        picofw_j_patch_s8(&jb, br_go, jb.pos);

        picofw_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xAAu);
        picofw_j_cmd_a0(&jb, hw_base + 0x0554UL, 0x55u);
        picofw_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xA0u);
        static const uint8_t k_wr_poll[6] = { 0xF0u, 0x79u, 0x8Bu, 0xF8u, 0xE3u, 0x80u };
        picofw_j_arr(&jb, k_wr_poll, sizeof(k_wr_poll));
        (void)picofw_j_imm32(&jb, 0xCDu, PICOFW_JEDEC_PROG_POLL_ITERS);
        const size_t poll_p = jb.pos;
        static const uint8_t k_poll_p[8] = {
            0xF0u, 0x61u, 0xF2u, 0xF2u, 0xF8u, 0xE0u, 0x80u, 0xACu
        };
        picofw_j_arr(&jb, k_poll_p, sizeof(k_poll_p));
        const size_t br_pok = jb.pos;
        picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0u);
        static const uint8_t k_dq5_p[5] = { 0xF2u, 0xE0u, 0xF8u, 0xECu, 0x20u };
        picofw_j_arr(&jb, k_dq5_p, sizeof(k_dq5_p));
        const size_t br_p_busy = jb.pos;
        picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0u);
        static const uint8_t k_poll_p_reread[6] = { 0xF0u, 0x61u, 0xF8u, 0xE0u, 0x80u, 0xACu };
        picofw_j_arr(&jb, k_poll_p_reread, sizeof(k_poll_p_reread));
        const size_t br_dq5_pok = jb.pos;
        picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        picofw_j_patch_s8(&jb, br_p_busy, jb.pos);
        static const uint8_t k_pdec[4] = { 0x29u, 0xFFu, 0xA5u, 0x00u };
        picofw_j_arr(&jb, k_pdec, sizeof(k_pdec));
        picofw_j_br(&jb, 0xC9u, poll_p);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);

        picofw_j_patch_s8(&jb, br_pok, jb.pos);
        picofw_j_patch_s8(&jb, br_dq5_pok, jb.pos);
        picofw_j_cmd_a0(&jb, hw_base, 0xF0u);
        static const uint8_t k_verify_p[3] = { 0xF0u, 0x61u, 0xA8u };
        picofw_j_arr(&jb, k_verify_p, sizeof(k_verify_p));
        picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0x05u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        picofw_j_u8(&jb, 0x22u); picofw_j_u8(&jb, 0x01u);
        (void)picofw_j_imm32(&jb, 0xDAu, norm_addr + (uint32_t)len);
        const size_t br_done = jb.pos;
        picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0u);
        const size_t jmp_back = jb.pos;
        picofw_j_arr(&jb, k_j16, sizeof(k_j16));
        picofw_j_patch_s16(&jb, jmp_back, prog_loop);
        picofw_j_patch_s8(&jb, br_done, jb.pos);
    }

    if (is_erase) {
        picofw_j_patch_s8(&jb, br_to_ok, jb.pos);
        picofw_j_patch_s8(&jb, br_dq5_ok, jb.pos);
        picofw_j_cmd_a0(&jb, hw_base, 0xF0u);
        static const uint8_t k_rd_a2[2] = { 0xF0u, 0x62u };
        picofw_j_arr(&jb, k_rd_a2, sizeof(k_rd_a2));
        (void)picofw_j_imm32(&jb, 0xC8u, 0x0000FFFFUL);
        picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0x05u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        if (sector_count > 1u) {
            const uint32_t end_addr = (norm_addr & ~1UL) + (sector_step * sector_count);
            (void)picofw_j_imm32(&jb, 0xD2u, sector_step);
            (void)picofw_j_imm32(&jb, 0xDAu, end_addr);
            picofw_j_u8(&jb, 0xC8u); picofw_j_u8(&jb, 0x05u);
            const size_t jmp_sec = jb.pos;
            picofw_j_arr(&jb, k_j16, sizeof(k_j16));
            picofw_j_patch_s16(&jb, jmp_sec, sector_loop);
        }
    }
    size_t ok_s = 0u, ok_e = 0u, to_s = 0u, to_e = 0u;
    const size_t c_ok = picofw_j_emit_status_msg(&jb, &ok_s, &ok_e);
    const size_t br_park = jb.pos;
    picofw_j_u8(&jb, 0xCAu); picofw_j_u8(&jb, 0u);

    for (size_t fi = 0u; fi < fail_cnt; ++fi) {
        picofw_j_patch_s16(&jb, fail_jmps[fi], jb.pos);
    }
    picofw_j_cmd_a0(&jb, hw_base, 0xF0u);
    const size_t c_to = picofw_j_emit_status_msg(&jb, &to_s, &to_e);

    picofw_j_patch_s8(&jb, br_park, jb.pos);
    picofw_j_u8(&jb, 0xCAu); picofw_j_u8(&jb, 0x00u);

    const size_t span_off = jb.pos;
    static const uint8_t k_span_rd[3] = { 0xF0u, 0x40u, 0x83u };
    picofw_j_arr(&jb, k_span_rd, sizeof(k_span_rd));
    const size_t c_putc = jb.pos;
    static const uint8_t k_call_p[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    picofw_j_arr(&jb, k_call_p, sizeof(k_call_p));
    static const uint8_t k_span_nxt[3] = { 0x20u, 0x01u, 0xB4u };
    picofw_j_arr(&jb, k_span_nxt, sizeof(k_span_nxt));
    picofw_j_br(&jb, 0xC9u, span_off);
    static const uint8_t k_ret[3] = { 0xDFu, 0x00u, 0x00u };
    picofw_j_arr(&jb, k_ret, sizeof(k_ret));

    const size_t putc_off = jb.pos;
    picofw_j_arr(&jb, k_picofw_mn103_putc_d3, sizeof(k_picofw_mn103_putc_d3));

    const size_t ok_str_off = jb.pos;
    picofw_j_arr(&jb, (const uint8_t *)k_ok_str, sizeof(k_ok_str) - 1u);
    const size_t to_str_off = jb.pos;
    picofw_j_arr(&jb, (const uint8_t *)k_to_str, sizeof(k_to_str) - 1u);
    const size_t end_str_off = jb.pos;
    const size_t data_off = jb.pos;
    if (!is_erase && (len > 0u)) {
        picofw_j_arr(&jb, data, len);
    }

    if (jb.pos > out_cap) {
        return 0u;
    }
    picofw_j_patch_s16(&jb, c_ok, span_off);
    picofw_j_patch_s16(&jb, c_to, span_off);
    picofw_j_patch_s16(&jb, c_putc, putc_off);
    picofw_j_patch_u32(&jb, ok_s, load_addr + (uint32_t)ok_str_off);
    picofw_j_patch_u32(&jb, ok_e, load_addr + (uint32_t)to_str_off);
    picofw_j_patch_u32(&jb, to_s, load_addr + (uint32_t)to_str_off);
    picofw_j_patch_u32(&jb, to_e, load_addr + (uint32_t)end_str_off);
    if (!is_erase) {
        picofw_j_patch_u32(&jb, src_patch, load_addr + (uint32_t)data_off);
    }
    return jb.pos;
}
