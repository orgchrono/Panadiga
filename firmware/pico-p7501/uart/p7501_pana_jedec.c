/**
 * @file p7501_pana_jedec.c
 * @brief MN103S SRAM 6-cycle Erase & 4-cycle Program Spansion `S29JL064H` JEDEC stub builders.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_jedec.h"
#include "uart/p7501_pana_rsp_internal.h"
#include "flash/p7501_flash.h"

#include <string.h>

typedef struct jedec_buf {
    uint8_t *out;
    size_t pos;
    size_t cap;
} jedec_buf_t;

static void j_u8(jedec_buf_t *jb, uint8_t v) {
    if (jb->pos < jb->cap) {
        jb->out[jb->pos] = v;
    }
    jb->pos += 1u;
}

static void j_arr(jedec_buf_t *jb, const uint8_t *src, size_t n) {
    if ((jb->pos + n) <= jb->cap) {
        memcpy(&jb->out[jb->pos], src, n);
    }
    jb->pos += n;
}

static size_t j_imm32(jedec_buf_t *jb, uint8_t sub_op, uint32_t imm) {
    const size_t lit_off = jb->pos + 2u;
    uint8_t le[4];
    p7501_rsp_encode_u32_le(imm, le);
    j_u8(jb, 0xFCu);
    j_u8(jb, sub_op);
    j_arr(jb, le, 4u);
    return lit_off;
}

static void j_patch_s8(jedec_buf_t *jb, size_t br_off, size_t target_off) {
    if ((br_off + 1u) < jb->cap) {
        jb->out[br_off + 1u] = (uint8_t)((int8_t)((ptrdiff_t)target_off - (ptrdiff_t)br_off));
    }
}

static void j_patch_s16(jedec_buf_t *jb, size_t insn_off, size_t target_off) {
    if ((insn_off + 2u) < jb->cap) {
        const int16_t d = (int16_t)((ptrdiff_t)target_off - (ptrdiff_t)insn_off);
        jb->out[insn_off + 1u] = (uint8_t)(d & 0xFF);
        jb->out[insn_off + 2u] = (uint8_t)((d >> 8) & 0xFF);
    }
}

static void j_patch_u32(jedec_buf_t *jb, size_t lit_off, uint32_t val) {
    if ((lit_off + 4u) <= jb->cap) {
        p7501_rsp_encode_u32_le(val, &jb->out[lit_off]);
    }
}

static void j_cmd_a0(jedec_buf_t *jb, uint32_t bus_va, uint8_t cmd_byte) {
    (void)j_imm32(jb, 0xDCu, bus_va);
    const uint8_t wr[5] = { 0x80u, cmd_byte, 0x14u, 0xF0u, 0x70u }; /* mov cmd,d0; extbu d0; movhu d0,(a0) */
    j_arr(jb, wr, sizeof(wr));
}

static size_t j_emit_status_msg(jedec_buf_t *jb, size_t *s_patch, size_t *e_patch) {
    *s_patch = j_imm32(jb, 0xDCu, 0u);
    *e_patch = j_imm32(jb, 0xDDu, 0u);
    const size_t c_off = jb->pos;
    static const uint8_t k_call[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    j_arr(jb, k_call, sizeof(k_call));
    return c_off;
}

static void j_br(jedec_buf_t *jb, uint8_t op, size_t target_off) {
    const size_t br_off = jb->pos;
    j_u8(jb, op);
    j_u8(jb, 0u);
    j_patch_s8(jb, br_off, target_off);
}

static size_t build_jedec_stub_common(
    bool is_erase,
    uint32_t norm_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    static const char k_ok_str[] = P7501_JEDEC_OK_TOKEN "\r\n";
    static const char k_to_str[] = P7501_JEDEC_TO_TOKEN "\r\n";
    static const uint8_t k_j16[3] = { 0xCCu, 0x00u, 0x00u };

    jedec_buf_t jb = { out_buf, 0u, out_cap };
    size_t src_patch = 0u, jmp_to_off = 0u, br_to_ok = 0u;

    if (is_erase) {
        (void)j_imm32(&jb, 0xDEu, norm_addr & ~1UL);
        j_cmd_a0(&jb, 0x84000AAAUL, 0xAAu);
        j_cmd_a0(&jb, 0x84000554UL, 0x55u);
        j_cmd_a0(&jb, 0x84000AAAUL, 0x80u);
        j_cmd_a0(&jb, 0x84000AAAUL, 0xAAu);
        j_cmd_a0(&jb, 0x84000554UL, 0x55u);
        static const uint8_t k_sec30[4] = { 0x80u, 0x30u, 0xF0u, 0x72u };
        j_arr(&jb, k_sec30, sizeof(k_sec30));
        (void)j_imm32(&jb, 0xCDu, 0x00200000UL);
        const size_t poll_off = jb.pos;
        static const uint8_t k_dq7_e[5] = { 0xF0u, 0x62u, 0xF8u, 0xECu, 0x80u };
        j_arr(&jb, k_dq7_e, sizeof(k_dq7_e));
        br_to_ok = jb.pos;
        j_u8(&jb, 0xC9u); j_u8(&jb, 0u);
        static const uint8_t k_dec[4] = { 0x29u, 0xFFu, 0xA5u, 0x00u };
        j_arr(&jb, k_dec, sizeof(k_dec));
        j_br(&jb, 0xC9u, poll_off);
        jmp_to_off = jb.pos;
        j_arr(&jb, k_j16, sizeof(k_j16));
    } else {
        (void)j_imm32(&jb, 0xDEu, norm_addr);
        src_patch = j_imm32(&jb, 0xDFu, 0u);
        const size_t prog_loop = jb.pos;
        static const uint8_t k_rd_align[6] = { 0xF0u, 0x4Bu, 0x23u, 0x01u, 0xF1u, 0xD9u };
        j_arr(&jb, k_rd_align, sizeof(k_rd_align));
        (void)j_imm32(&jb, 0xCCu, 0xFFFFFFFEUL);
        static const uint8_t k_prep[11] = {
            0xF2u, 0x01u, 0xF1u, 0xE5u, 0xF0u, 0x61u, 0xF1u, 0xD9u, 0xF8u, 0xEDu, 0x01u
        };
        j_arr(&jb, k_prep, sizeof(k_prep));
        const size_t br_odd = jb.pos;
        j_u8(&jb, 0xC9u); j_u8(&jb, 0u);
        (void)j_imm32(&jb, 0xCDu, 0x0000FF00UL);
        static const uint8_t k_even_merge[4] = { 0xF2u, 0x04u, 0xF2u, 0x12u };
        j_arr(&jb, k_even_merge, sizeof(k_even_merge));
        const size_t br_go = jb.pos;
        j_u8(&jb, 0xCAu); j_u8(&jb, 0u);
        j_patch_s8(&jb, br_odd, jb.pos);
        static const uint8_t k_odd_merge[6] = { 0x14u, 0xF8u, 0xC2u, 0x08u, 0xF2u, 0x12u };
        j_arr(&jb, k_odd_merge, sizeof(k_odd_merge));
        j_patch_s8(&jb, br_go, jb.pos);

        j_cmd_a0(&jb, 0x84000AAAUL, 0xAAu);
        j_cmd_a0(&jb, 0x84000554UL, 0x55u);
        j_cmd_a0(&jb, 0x84000AAAUL, 0xA0u);
        static const uint8_t k_wr_poll[6] = { 0xF0u, 0x79u, 0x8Bu, 0xF8u, 0xE3u, 0x80u };
        j_arr(&jb, k_wr_poll, sizeof(k_wr_poll));
        (void)j_imm32(&jb, 0xCDu, 0x00100000UL);
        const size_t poll_p = jb.pos;
        static const uint8_t k_poll_p[6] = { 0xF0u, 0x61u, 0xF8u, 0xE0u, 0x80u, 0xACu };
        j_arr(&jb, k_poll_p, sizeof(k_poll_p));
        const size_t br_pok = jb.pos;
        j_u8(&jb, 0xC8u); j_u8(&jb, 0u);
        static const uint8_t k_pdec[4] = { 0x29u, 0xFFu, 0xA5u, 0x00u };
        j_arr(&jb, k_pdec, sizeof(k_pdec));
        j_br(&jb, 0xC9u, poll_p);
        jmp_to_off = jb.pos;
        j_arr(&jb, k_j16, sizeof(k_j16));

        j_patch_s8(&jb, br_pok, jb.pos);
        j_cmd_a0(&jb, 0x84000000UL, 0xF0u);
        j_u8(&jb, 0x22u); j_u8(&jb, 0x01u);
        (void)j_imm32(&jb, 0xDAu, norm_addr + (uint32_t)len);
        j_u8(&jb, 0xC8u); j_u8(&jb, 0x03u);
        const size_t jmp_back = jb.pos;
        j_arr(&jb, k_j16, sizeof(k_j16));
        j_patch_s16(&jb, jmp_back, prog_loop);
    }

    if (is_erase) {
        j_patch_s8(&jb, br_to_ok, jb.pos);
        j_cmd_a0(&jb, 0x84000000UL, 0xF0u);
    }
    size_t ok_s = 0u, ok_e = 0u, to_s = 0u, to_e = 0u;
    const size_t c_ok = j_emit_status_msg(&jb, &ok_s, &ok_e);
    const size_t br_park = jb.pos;
    j_u8(&jb, 0xCAu); j_u8(&jb, 0u);

    j_patch_s16(&jb, jmp_to_off, jb.pos);
    j_cmd_a0(&jb, 0x84000000UL, 0xF0u);
    const size_t c_to = j_emit_status_msg(&jb, &to_s, &to_e);

    j_patch_s8(&jb, br_park, jb.pos);
    j_u8(&jb, 0xCAu); j_u8(&jb, 0x00u);

    const size_t span_off = jb.pos;
    static const uint8_t k_span_rd[3] = { 0xF0u, 0x40u, 0x83u };
    j_arr(&jb, k_span_rd, sizeof(k_span_rd));
    const size_t c_putc = jb.pos;
    static const uint8_t k_call_p[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    j_arr(&jb, k_call_p, sizeof(k_call_p));
    static const uint8_t k_span_nxt[3] = { 0x20u, 0x01u, 0xB4u };
    j_arr(&jb, k_span_nxt, sizeof(k_span_nxt));
    j_br(&jb, 0xC9u, span_off);
    static const uint8_t k_ret[3] = { 0xDFu, 0x00u, 0x00u };
    j_arr(&jb, k_ret, sizeof(k_ret));

    const size_t putc_off = jb.pos;
    j_arr(&jb, k_p7501_mn103_putc_d3, sizeof(k_p7501_mn103_putc_d3));

    const size_t ok_str_off = jb.pos;
    j_arr(&jb, (const uint8_t *)k_ok_str, sizeof(k_ok_str) - 1u);
    const size_t to_str_off = jb.pos;
    j_arr(&jb, (const uint8_t *)k_to_str, sizeof(k_to_str) - 1u);
    const size_t end_str_off = jb.pos;
    const size_t data_off = jb.pos;
    if (!is_erase && (len > 0u)) {
        j_arr(&jb, data, len);
    }

    if (jb.pos > out_cap) {
        return 0u;
    }
    j_patch_s16(&jb, c_ok, span_off);
    j_patch_s16(&jb, c_to, span_off);
    j_patch_s16(&jb, c_putc, putc_off);
    j_patch_u32(&jb, ok_s, load_addr + (uint32_t)ok_str_off);
    j_patch_u32(&jb, ok_e, load_addr + (uint32_t)to_str_off);
    j_patch_u32(&jb, to_s, load_addr + (uint32_t)to_str_off);
    j_patch_u32(&jb, to_e, load_addr + (uint32_t)end_str_off);
    if (!is_erase) {
        j_patch_u32(&jb, src_patch, load_addr + (uint32_t)data_off);
    }
    return jb.pos;
}

bool p7501_jedec_validate_erase_addr(
    uint32_t addr,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_even
) {
    if (!p7501_flash_is_erasable_nor_addr(addr, upper_nor_unlocked)) {
        return false;
    }
    if (out_uncached_even != NULL) {
        *out_uncached_even = p7501_flash_normalize_uncached_addr(addr) & ~1UL;
    }
    return true;
}

bool p7501_jedec_validate_prog_range(
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_start
) {
    if ((len == 0u) || (len > P7501_JEDEC_MAX_DATA_LEN) ||
        !p7501_flash_is_readonly_nor_range(P7501_FLASH_SIZE_8MB, addr, (uint32_t)len) ||
        p7501_flash_is_write_blocked_with_policy(addr, len, upper_nor_unlocked)) {
        return false;
    }
    if (out_uncached_start != NULL) {
        *out_uncached_start = p7501_flash_normalize_uncached_addr(addr);
    }
    return true;
}

static size_t build_jedec_checked(
    bool is_erase,
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    const uint32_t span = is_erase ? 1UL : (uint32_t)len;
    if ((out_buf == NULL) || (load_addr > 0xFFF00000UL) ||
        (!is_erase && (data == NULL || len == 0u || len > P7501_JEDEC_MAX_DATA_LEN)) ||
        !p7501_flash_is_readonly_nor_range(P7501_FLASH_SIZE_8MB, target_addr, span)) {
        return 0u;
    }
    return build_jedec_stub_common(
        is_erase, p7501_flash_normalize_uncached_addr(target_addr), data, len, load_addr, out_buf, out_cap
    );
}

size_t p7501_jedec_build_erase_stub(uint32_t target_addr, uint32_t load_addr, uint8_t *out_buf, size_t out_cap) {
    return build_jedec_checked(true, target_addr, NULL, 0u, load_addr, out_buf, out_cap);
}

size_t p7501_jedec_build_prog_stub(
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return build_jedec_checked(false, target_addr, data, len, load_addr, out_buf, out_cap);
}
