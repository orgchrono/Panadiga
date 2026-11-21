/**
 * @file p7501_pana_jedec_stubs.c
 * @brief MN103S SRAM 6-cycle Erase & 4-cycle Program Spansion JEDEC stub builders + emitter primitives.
 *
 * Split from monolithic `p7501_pana_jedec.c` (`p7501_pana_jedec_id.c` owns Autoselect/CFI & validators).
 * Enforces `p7501_bus_check_access()` defense-in-depth inside `build_jedec_checked()`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_jedec.h"
#include "uart/p7501_pana_jedec_internal.h"
#include "uart/p7501_pana_rsp_internal.h"
#include "common/p7501_hex.h"
#include "flash/p7501_flash.h"

#include <string.h>

void p7501_j_u8(p7501_jedec_buf_t *jb, uint8_t v) {
    if (jb->pos < jb->cap) {
        jb->out[jb->pos] = v;
    }
    jb->pos += 1u;
}

void p7501_j_arr(p7501_jedec_buf_t *jb, const uint8_t *src, size_t n) {
    if ((jb->pos + n) <= jb->cap) {
        memcpy(&jb->out[jb->pos], src, n);
    }
    jb->pos += n;
}

size_t p7501_j_imm32(p7501_jedec_buf_t *jb, uint8_t sub_op, uint32_t imm) {
    const size_t lit_off = jb->pos + 2u;
    uint8_t le[4];
    p7501_rsp_encode_u32_le(imm, le);
    p7501_j_u8(jb, 0xFCu);
    p7501_j_u8(jb, sub_op);
    p7501_j_arr(jb, le, 4u);
    return lit_off;
}

void p7501_j_patch_s8(p7501_jedec_buf_t *jb, size_t br_off, size_t target_off) {
    if ((br_off + 1u) < jb->cap) {
        jb->out[br_off + 1u] = (uint8_t)((int8_t)((ptrdiff_t)target_off - (ptrdiff_t)br_off));
    }
}

void p7501_j_patch_s16(p7501_jedec_buf_t *jb, size_t insn_off, size_t target_off) {
    if ((insn_off + 2u) < jb->cap) {
        const int16_t d = (int16_t)((ptrdiff_t)target_off - (ptrdiff_t)insn_off);
        jb->out[insn_off + 1u] = (uint8_t)(d & 0xFF);
        jb->out[insn_off + 2u] = (uint8_t)((d >> 8) & 0xFF);
    }
}

void p7501_j_patch_u32(p7501_jedec_buf_t *jb, size_t lit_off, uint32_t val) {
    if ((lit_off + 4u) <= jb->cap) {
        p7501_rsp_encode_u32_le(val, &jb->out[lit_off]);
    }
}

void p7501_j_cmd_a0(p7501_jedec_buf_t *jb, uint32_t bus_va, uint8_t cmd_byte) {
    (void)p7501_j_imm32(jb, 0xDCu, bus_va);
    const uint8_t wr[5] = { 0x80u, cmd_byte, 0x14u, 0xF0u, 0x70u }; /* mov cmd,d0; extbu d0; movhu d0,(a0) */
    p7501_j_arr(jb, wr, sizeof(wr));
}

size_t p7501_j_emit_status_msg(p7501_jedec_buf_t *jb, size_t *s_patch, size_t *e_patch) {
    *s_patch = p7501_j_imm32(jb, 0xDCu, 0u);
    *e_patch = p7501_j_imm32(jb, 0xDDu, 0u);
    const size_t c_off = jb->pos;
    static const uint8_t k_call[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    p7501_j_arr(jb, k_call, sizeof(k_call));
    return c_off;
}

void p7501_j_br(p7501_jedec_buf_t *jb, uint8_t op, size_t target_off) {
    const size_t br_off = jb->pos;
    p7501_j_u8(jb, op);
    p7501_j_u8(jb, 0u);
    p7501_j_patch_s8(jb, br_off, target_off);
}

static void j_emit_jmp16_fail(
    p7501_jedec_buf_t *jb,
    size_t *fail_jmps,
    size_t *fail_cnt,
    size_t fail_cap
) {
    static const uint8_t k_j16[3] = { 0xCCu, 0x00u, 0x00u };
    if ((fail_jmps != NULL) && (fail_cnt != NULL) && (*fail_cnt < fail_cap)) {
        fail_jmps[(*fail_cnt)++] = jb->pos;
    }
    p7501_j_arr(jb, k_j16, sizeof(k_j16));
}

static size_t build_jedec_stub_common(
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
    static const char k_ok_str[] = P7501_JEDEC_OK_TOKEN "\r\n";
    static const char k_to_str[] = P7501_JEDEC_TO_TOKEN "\r\n";
    static const uint8_t k_j16[3] = { 0xCCu, 0x00u, 0x00u };
    const uint32_t hw_base = P7501_SSOT_CS0_UNCACHED_BASE;

    p7501_jedec_buf_t jb = { out_buf, 0u, out_cap };
    size_t fail_jmps[4] = { 0u, 0u, 0u, 0u };
    size_t fail_cnt = 0u;
    size_t src_patch = 0u;
    size_t sector_loop = 0u;
    size_t br_to_ok = 0u;
    size_t br_dq5_ok = 0u;

    if (is_erase) {
        (void)p7501_j_imm32(&jb, 0xDEu, norm_addr & ~1UL);
        sector_loop = jb.pos;
        p7501_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xAAu);
        p7501_j_cmd_a0(&jb, hw_base + 0x0554UL, 0x55u);
        p7501_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0x80u);
        p7501_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xAAu);
        p7501_j_cmd_a0(&jb, hw_base + 0x0554UL, 0x55u);
        static const uint8_t k_sec30[4] = { 0x80u, 0x30u, 0xF0u, 0x72u };
        p7501_j_arr(&jb, k_sec30, sizeof(k_sec30));
        (void)p7501_j_imm32(&jb, 0xCDu, P7501_JEDEC_ERASE_POLL_ITERS);
        const size_t poll_off = jb.pos;
        static const uint8_t k_dq7_e[5] = { 0xF0u, 0x62u, 0xF8u, 0xECu, 0x80u };
        p7501_j_arr(&jb, k_dq7_e, sizeof(k_dq7_e));
        br_to_ok = jb.pos;
        p7501_j_u8(&jb, 0xC9u); p7501_j_u8(&jb, 0u);
        static const uint8_t k_dq5_e[3] = { 0xF8u, 0xECu, 0x20u };
        p7501_j_arr(&jb, k_dq5_e, sizeof(k_dq5_e));
        const size_t br_e_busy = jb.pos;
        p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0u);
        p7501_j_arr(&jb, k_dq7_e, sizeof(k_dq7_e));
        br_dq5_ok = jb.pos;
        p7501_j_u8(&jb, 0xC9u); p7501_j_u8(&jb, 0u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        p7501_j_patch_s8(&jb, br_e_busy, jb.pos);
        static const uint8_t k_dec[4] = { 0x29u, 0xFFu, 0xA5u, 0x00u };
        p7501_j_arr(&jb, k_dec, sizeof(k_dec));
        p7501_j_br(&jb, 0xC9u, poll_off);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
    } else {
        (void)p7501_j_imm32(&jb, 0xDEu, norm_addr);
        src_patch = p7501_j_imm32(&jb, 0xDFu, 0u);
        const size_t prog_loop = jb.pos;
        static const uint8_t k_rd_align[6] = { 0xF0u, 0x4Bu, 0x23u, 0x01u, 0xF1u, 0xD9u };
        p7501_j_arr(&jb, k_rd_align, sizeof(k_rd_align));
        (void)p7501_j_imm32(&jb, 0xCCu, 0xFFFFFFFEUL);
        static const uint8_t k_prep[11] = {
            0xF2u, 0x01u, 0xF1u, 0xE5u, 0xF0u, 0x61u, 0xF1u, 0xD9u, 0xF8u, 0xEDu, 0x01u
        };
        p7501_j_arr(&jb, k_prep, sizeof(k_prep));
        const size_t br_odd = jb.pos;
        p7501_j_u8(&jb, 0xC9u); p7501_j_u8(&jb, 0u);
        (void)p7501_j_imm32(&jb, 0xCDu, 0x0000FF00UL);
        static const uint8_t k_even_merge[4] = { 0xF2u, 0x04u, 0xF2u, 0x12u };
        p7501_j_arr(&jb, k_even_merge, sizeof(k_even_merge));
        const size_t br_go = jb.pos;
        p7501_j_u8(&jb, 0xCAu); p7501_j_u8(&jb, 0u);
        p7501_j_patch_s8(&jb, br_odd, jb.pos);
        static const uint8_t k_odd_merge[6] = { 0x14u, 0xF8u, 0xC2u, 0x08u, 0xF2u, 0x12u };
        p7501_j_arr(&jb, k_odd_merge, sizeof(k_odd_merge));
        p7501_j_patch_s8(&jb, br_go, jb.pos);

        p7501_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xAAu);
        p7501_j_cmd_a0(&jb, hw_base + 0x0554UL, 0x55u);
        p7501_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xA0u);
        static const uint8_t k_wr_poll[6] = { 0xF0u, 0x79u, 0x8Bu, 0xF8u, 0xE3u, 0x80u };
        p7501_j_arr(&jb, k_wr_poll, sizeof(k_wr_poll));
        (void)p7501_j_imm32(&jb, 0xCDu, P7501_JEDEC_PROG_POLL_ITERS);
        const size_t poll_p = jb.pos;
        static const uint8_t k_poll_p[8] = {
            0xF0u, 0x61u, 0xF2u, 0xF2u, 0xF8u, 0xE0u, 0x80u, 0xACu
        };
        p7501_j_arr(&jb, k_poll_p, sizeof(k_poll_p));
        const size_t br_pok = jb.pos;
        p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0u);
        static const uint8_t k_dq5_p[5] = { 0xF2u, 0xE0u, 0xF8u, 0xECu, 0x20u };
        p7501_j_arr(&jb, k_dq5_p, sizeof(k_dq5_p));
        const size_t br_p_busy = jb.pos;
        p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0u);
        static const uint8_t k_poll_p_reread[6] = { 0xF0u, 0x61u, 0xF8u, 0xE0u, 0x80u, 0xACu };
        p7501_j_arr(&jb, k_poll_p_reread, sizeof(k_poll_p_reread));
        const size_t br_dq5_pok = jb.pos;
        p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        p7501_j_patch_s8(&jb, br_p_busy, jb.pos);
        static const uint8_t k_pdec[4] = { 0x29u, 0xFFu, 0xA5u, 0x00u };
        p7501_j_arr(&jb, k_pdec, sizeof(k_pdec));
        p7501_j_br(&jb, 0xC9u, poll_p);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);

        p7501_j_patch_s8(&jb, br_pok, jb.pos);
        p7501_j_patch_s8(&jb, br_dq5_pok, jb.pos);
        p7501_j_cmd_a0(&jb, hw_base, 0xF0u);
        static const uint8_t k_verify_p[3] = { 0xF0u, 0x61u, 0xA8u };
        p7501_j_arr(&jb, k_verify_p, sizeof(k_verify_p));
        p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0x05u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        p7501_j_u8(&jb, 0x22u); p7501_j_u8(&jb, 0x01u);
        (void)p7501_j_imm32(&jb, 0xDAu, norm_addr + (uint32_t)len);
        const size_t br_done = jb.pos;
        p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0u);
        const size_t jmp_back = jb.pos;
        p7501_j_arr(&jb, k_j16, sizeof(k_j16));
        p7501_j_patch_s16(&jb, jmp_back, prog_loop);
        p7501_j_patch_s8(&jb, br_done, jb.pos);
    }

    if (is_erase) {
        p7501_j_patch_s8(&jb, br_to_ok, jb.pos);
        p7501_j_patch_s8(&jb, br_dq5_ok, jb.pos);
        p7501_j_cmd_a0(&jb, hw_base, 0xF0u);
        static const uint8_t k_rd_a2[2] = { 0xF0u, 0x62u };
        p7501_j_arr(&jb, k_rd_a2, sizeof(k_rd_a2));
        (void)p7501_j_imm32(&jb, 0xC8u, 0x0000FFFFUL);
        p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0x05u);
        j_emit_jmp16_fail(&jb, fail_jmps, &fail_cnt, 4u);
        if (sector_count > 1u) {
            const uint32_t end_addr = (norm_addr & ~1UL) + (sector_step * sector_count);
            (void)p7501_j_imm32(&jb, 0xD2u, sector_step);
            (void)p7501_j_imm32(&jb, 0xDAu, end_addr);
            p7501_j_u8(&jb, 0xC8u); p7501_j_u8(&jb, 0x05u);
            const size_t jmp_sec = jb.pos;
            p7501_j_arr(&jb, k_j16, sizeof(k_j16));
            p7501_j_patch_s16(&jb, jmp_sec, sector_loop);
        }
    }
    size_t ok_s = 0u, ok_e = 0u, to_s = 0u, to_e = 0u;
    const size_t c_ok = p7501_j_emit_status_msg(&jb, &ok_s, &ok_e);
    const size_t br_park = jb.pos;
    p7501_j_u8(&jb, 0xCAu); p7501_j_u8(&jb, 0u);

    for (size_t fi = 0u; fi < fail_cnt; ++fi) {
        p7501_j_patch_s16(&jb, fail_jmps[fi], jb.pos);
    }
    p7501_j_cmd_a0(&jb, hw_base, 0xF0u);
    const size_t c_to = p7501_j_emit_status_msg(&jb, &to_s, &to_e);

    p7501_j_patch_s8(&jb, br_park, jb.pos);
    p7501_j_u8(&jb, 0xCAu); p7501_j_u8(&jb, 0x00u);

    const size_t span_off = jb.pos;
    static const uint8_t k_span_rd[3] = { 0xF0u, 0x40u, 0x83u };
    p7501_j_arr(&jb, k_span_rd, sizeof(k_span_rd));
    const size_t c_putc = jb.pos;
    static const uint8_t k_call_p[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    p7501_j_arr(&jb, k_call_p, sizeof(k_call_p));
    static const uint8_t k_span_nxt[3] = { 0x20u, 0x01u, 0xB4u };
    p7501_j_arr(&jb, k_span_nxt, sizeof(k_span_nxt));
    p7501_j_br(&jb, 0xC9u, span_off);
    static const uint8_t k_ret[3] = { 0xDFu, 0x00u, 0x00u };
    p7501_j_arr(&jb, k_ret, sizeof(k_ret));

    const size_t putc_off = jb.pos;
    p7501_j_arr(&jb, k_p7501_mn103_putc_d3, sizeof(k_p7501_mn103_putc_d3));

    const size_t ok_str_off = jb.pos;
    p7501_j_arr(&jb, (const uint8_t *)k_ok_str, sizeof(k_ok_str) - 1u);
    const size_t to_str_off = jb.pos;
    p7501_j_arr(&jb, (const uint8_t *)k_to_str, sizeof(k_to_str) - 1u);
    const size_t end_str_off = jb.pos;
    const size_t data_off = jb.pos;
    if (!is_erase && (len > 0u)) {
        p7501_j_arr(&jb, data, len);
    }

    if (jb.pos > out_cap) {
        return 0u;
    }
    p7501_j_patch_s16(&jb, c_ok, span_off);
    p7501_j_patch_s16(&jb, c_to, span_off);
    p7501_j_patch_s16(&jb, c_putc, putc_off);
    p7501_j_patch_u32(&jb, ok_s, load_addr + (uint32_t)ok_str_off);
    p7501_j_patch_u32(&jb, ok_e, load_addr + (uint32_t)to_str_off);
    p7501_j_patch_u32(&jb, to_s, load_addr + (uint32_t)to_str_off);
    p7501_j_patch_u32(&jb, to_e, load_addr + (uint32_t)end_str_off);
    if (!is_erase) {
        p7501_j_patch_u32(&jb, src_patch, load_addr + (uint32_t)data_off);
    }
    return jb.pos;
}

static size_t build_jedec_checked(
    uint32_t flash_size_bytes,
    bool is_erase,
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    const uint32_t span = is_erase ? 1UL : (uint32_t)len;
    if ((out_buf == NULL) || (load_addr > 0xFFF00000UL) ||
        (!is_erase && ((data == NULL) || (len == 0u) || (len > P7501_JEDEC_MAX_DATA_LEN))) ||
        !p7501_flash_is_readonly_nor_range(eff_size, target_addr, span)) {
        return 0u;
    }
    /* Defense-in-depth: enforce authoritative AM34 bus & 3-layer NOR write policy. */
    if (p7501_bus_check_access(
            p7501_session_default_ctx(),
            eff_size,
            target_addr,
            (size_t)span,
            true,
            is_erase,
            p7501_flash_is_upper_nor_unlocked()
        ) != P7501_BUS_ALLOW) {
        return 0u;
    }
    return build_jedec_stub_common(
        is_erase,
        p7501_flash_normalize_uncached_addr(target_addr),
        0u,
        1u,
        data,
        len,
        load_addr,
        out_buf,
        out_cap
    );
}

size_t p7501_jedec_build_erase_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t target_addr,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return build_jedec_checked(
        flash_size_bytes, true, target_addr, NULL, 0u, load_addr, out_buf, out_cap
    );
}

size_t p7501_jedec_build_erase_stub(
    uint32_t target_addr,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return p7501_jedec_build_erase_stub_for_flash(
        p7501_flash_get_active_size(), target_addr, load_addr, out_buf, out_cap
    );
}

size_t p7501_jedec_build_erase_range_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t start_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    if ((out_buf == NULL) || (load_addr > 0xFFF00000UL) ||
        (sector_count == 0u) || (sector_count > 32u)) {
        return 0u;
    }
    if ((sector_count > 1u) && (sector_step < 4096u)) {
        return 0u;
    }
    const uint64_t span = (uint64_t)sector_step * (uint64_t)sector_count;
    if (span > 0x01000000ULL) {
        return 0u;
    }
    uint32_t first_norm = 0u;
    for (uint32_t s = 0u; s < sector_count; ++s) {
        const uint64_t addr64 = (uint64_t)start_addr + ((uint64_t)s * (uint64_t)sector_step);
        if (addr64 > 0xFFFFFFFFULL) {
            return 0u;
        }
        uint32_t sec_norm = 0u;
        if (!p7501_jedec_validate_erase_addr_for_flash(eff_size, (uint32_t)addr64, false, &sec_norm)) {
            return 0u;
        }
        if (s == 0u) {
            first_norm = sec_norm;
        } else if (((uint64_t)first_norm + ((uint64_t)s * (uint64_t)sector_step)) != (uint64_t)sec_norm) {
            return 0u;
        }
    }
    if (((uint64_t)first_norm + span) > 0xFFFFFFFFULL) {
        return 0u;
    }
    return build_jedec_stub_common(
        true,
        first_norm,
        sector_step,
        sector_count,
        NULL,
        0u,
        load_addr,
        out_buf,
        out_cap
    );
}

size_t p7501_jedec_build_erase_range_stub(
    uint32_t start_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return p7501_jedec_build_erase_range_stub_for_flash(
        p7501_flash_get_active_size(),
        start_addr,
        sector_step,
        sector_count,
        load_addr,
        out_buf,
        out_cap
    );
}

size_t p7501_jedec_build_prog_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return build_jedec_checked(
        flash_size_bytes, false, target_addr, data, len, load_addr, out_buf, out_cap
    );
}

size_t p7501_jedec_build_prog_stub(
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return p7501_jedec_build_prog_stub_for_flash(
        p7501_flash_get_active_size(), target_addr, data, len, load_addr, out_buf, out_cap
    );
}
