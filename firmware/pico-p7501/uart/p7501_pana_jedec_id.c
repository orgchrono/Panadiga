/**
 * @file p7501_pana_jedec_id.c
 * @brief JEDEC Autoselect (`0x90`), CFI Query (`0x98`), address validators, and NOR bit-clear checks.
 *
 * Split from monolithic `p7501_pana_jedec.c` (`p7501_pana_jedec_stubs.c` owns Erase/Prog stubs).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_jedec.h"
#include "uart/p7501_pana_jedec_internal.h"
#include "flash/p7501_flash.h"

#include <string.h>

bool p7501_jedec_validate_erase_addr_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_even
) {
    if (!p7501_flash_is_erasable_nor_addr_for_flash(flash_size_bytes, addr, upper_nor_unlocked)) {
        return false;
    }
    if (out_uncached_even != NULL) {
        *out_uncached_even = p7501_flash_normalize_uncached_addr(addr) & ~1UL;
    }
    return true;
}

bool p7501_jedec_validate_erase_addr(
    uint32_t addr,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_even
) {
    return p7501_jedec_validate_erase_addr_for_flash(
        p7501_flash_get_active_size(), addr, upper_nor_unlocked, out_uncached_even
    );
}

bool p7501_jedec_validate_prog_range_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_start
) {
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    if ((len == 0u) || (len > P7501_JEDEC_MAX_DATA_LEN) ||
        !p7501_flash_is_readonly_nor_range(eff_size, addr, (uint32_t)len) ||
        p7501_flash_is_write_blocked_for_flash(eff_size, addr, len, upper_nor_unlocked)) {
        return false;
    }
    if (out_uncached_start != NULL) {
        *out_uncached_start = p7501_flash_normalize_uncached_addr(addr);
    }
    return true;
}

bool p7501_jedec_validate_prog_range(
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_start
) {
    return p7501_jedec_validate_prog_range_for_flash(
        p7501_flash_get_active_size(), addr, len, upper_nor_unlocked, out_uncached_start
    );
}

static size_t find_subbytes(const uint8_t *hay, size_t hlen, const char *needle, size_t nlen) {
    if ((hay == NULL) || (needle == NULL) || (hlen < nlen)) {
        return hlen;
    }
    for (size_t i = 0u; (i + nlen) <= hlen; ++i) {
        if (memcmp(&hay[i], needle, nlen) == 0) {
            return i;
        }
    }
    return hlen;
}

bool p7501_j_append_span_putc_msgs(
    p7501_jedec_buf_t *jb,
    uint32_t load_addr,
    const char hdr8[8],
    size_t c_hdr,
    size_t hdr_s,
    size_t hdr_e,
    size_t c_done,
    size_t done_s,
    size_t done_e,
    size_t *out_putc_off
) {
    static const uint8_t k_call_placeholder[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    static const uint8_t k_ret0[3] = { 0xDFu, 0x00u, 0x00u };
    static const uint8_t k_span_rd[3] = { 0xF0u, 0x40u, 0x83u };
    static const uint8_t k_span_nxt[3] = { 0x20u, 0x01u, 0xB4u };
    static const char k_done[] = "PANADONE";

    if ((jb == NULL) || (hdr8 == NULL) || (out_putc_off == NULL)) {
        return false;
    }

    const size_t span_off = jb->pos;
    p7501_j_arr(jb, k_span_rd, sizeof(k_span_rd));
    const size_t c_p_sp = jb->pos;
    p7501_j_arr(jb, k_call_placeholder, sizeof(k_call_placeholder));
    p7501_j_arr(jb, k_span_nxt, sizeof(k_span_nxt));
    p7501_j_br(jb, 0xC9u, span_off);
    p7501_j_arr(jb, k_ret0, sizeof(k_ret0));

    const size_t putc_off = jb->pos;
    p7501_j_arr(jb, k_p7501_mn103_putc_d3, sizeof(k_p7501_mn103_putc_d3));

    const size_t hdr_off = jb->pos;
    p7501_j_arr(jb, (const uint8_t *)hdr8, 8u);
    const size_t done_off = jb->pos;
    p7501_j_arr(jb, (const uint8_t *)k_done, 8u);
    const size_t end_off = jb->pos;

    if (jb->pos > jb->cap) {
        return false;
    }
    p7501_j_patch_s16(jb, c_hdr, span_off);
    p7501_j_patch_s16(jb, c_done, span_off);
    p7501_j_patch_s16(jb, c_p_sp, putc_off);
    p7501_j_patch_u32(jb, hdr_s, load_addr + (uint32_t)hdr_off);
    p7501_j_patch_u32(jb, hdr_e, load_addr + (uint32_t)done_off);
    p7501_j_patch_u32(jb, done_s, load_addr + (uint32_t)done_off);
    p7501_j_patch_u32(jb, done_e, load_addr + (uint32_t)end_off);
    *out_putc_off = putc_off;
    return true;
}

size_t p7501_jedec_build_autoselect_stub(
    bool is_cfi,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    if ((out_buf == NULL) || (load_addr > 0xFFF00000UL)) {
        return 0u;
    }
    const uint32_t hw_base = P7501_SSOT_CS0_UNCACHED_BASE;
    const uint32_t addrs[4] = {
        hw_base + (is_cfi ? 0x0020UL : 0x0000UL),
        hw_base + (is_cfi ? 0x0022UL : 0x0002UL),
        hw_base + (is_cfi ? 0x0024UL : 0x001CUL),
        hw_base + (is_cfi ? 0x004EUL : 0x001EUL),
    };
    const char *hdr = is_cfi ? P7501_CFI_QUERY_MAGIC : P7501_JEDEC_ID_MAGIC;
    static const uint8_t k_call_placeholder[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };

    p7501_jedec_buf_t jb = { out_buf, 0u, out_cap };
    size_t hdr_s = 0u, hdr_e = 0u, done_s = 0u, done_e = 0u;
    size_t rd_calls[4] = { 0u, 0u, 0u, 0u };

    const size_t c_hdr = p7501_j_emit_status_msg(&jb, &hdr_s, &hdr_e);
    if (is_cfi) {
        p7501_j_cmd_a0(&jb, hw_base + 0x00AAUL, 0x98u);
    } else {
        p7501_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0xAAu);
        p7501_j_cmd_a0(&jb, hw_base + 0x0554UL, 0x55u);
        p7501_j_cmd_a0(&jb, hw_base + 0x0AAAUL, 0x90u);
    }
    for (size_t i = 0u; i < 4u; ++i) {
        (void)p7501_j_imm32(&jb, 0xDCu, addrs[i]);
        rd_calls[i] = jb.pos;
        p7501_j_arr(&jb, k_call_placeholder, sizeof(k_call_placeholder));
    }
    p7501_j_cmd_a0(&jb, hw_base, 0xF0u);
    const size_t c_done = p7501_j_emit_status_msg(&jb, &done_s, &done_e);
    p7501_j_u8(&jb, 0xCAu); p7501_j_u8(&jb, 0x00u);

    /* Read halfword at (A0) -> emit LE low, high bytes via putc(d3) */
    const size_t emit_hw_off = jb.pos;
    static const uint8_t k_movhu_a0_d2[3] = { 0xF0u, 0x68u, 0x8Bu };
    p7501_j_arr(&jb, k_movhu_a0_d2, sizeof(k_movhu_a0_d2));
    const size_t c_p_lo = jb.pos;
    p7501_j_arr(&jb, k_call_placeholder, sizeof(k_call_placeholder));
    static const uint8_t k_lsr8_d2[4] = { 0xF8u, 0xC6u, 0x08u, 0x8Bu };
    p7501_j_arr(&jb, k_lsr8_d2, sizeof(k_lsr8_d2));
    const size_t c_p_hi = jb.pos;
    p7501_j_arr(&jb, k_call_placeholder, sizeof(k_call_placeholder));
    static const uint8_t k_ret0[3] = { 0xDFu, 0x00u, 0x00u };
    p7501_j_arr(&jb, k_ret0, sizeof(k_ret0));

    size_t putc_off = 0u;
    if (!p7501_j_append_span_putc_msgs(&jb, load_addr, hdr, c_hdr, hdr_s, hdr_e,
                                       c_done, done_s, done_e, &putc_off)) {
        return 0u;
    }
    for (size_t i = 0u; i < 4u; ++i) {
        p7501_j_patch_s16(&jb, rd_calls[i], emit_hw_off);
    }
    p7501_j_patch_s16(&jb, c_p_lo, putc_off);
    p7501_j_patch_s16(&jb, c_p_hi, putc_off);
    return jb.pos;
}

bool p7501_jedec_parse_autoselect_reply(
    const uint8_t *buf,
    size_t len,
    bool is_cfi,
    p7501_jedec_id_reply_t *out
) {
    if ((buf == NULL) || (out == NULL)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->is_cfi = is_cfi;
    const char *magic = is_cfi ? P7501_CFI_QUERY_MAGIC : P7501_JEDEC_ID_MAGIC;
    const size_t h = find_subbytes(buf, len, magic, 8u);
    if (((h + 24u) > len) || (memcmp(&buf[h + 16u], "PANADONE", 8u) != 0)) {
        return false;
    }
    const uint8_t *p = &buf[h + 8u];
    const uint16_t hw0 = (uint16_t)p[0] | ((uint16_t)p[1] << 8u);
    const uint16_t hw1 = (uint16_t)p[2] | ((uint16_t)p[3] << 8u);
    const uint16_t hw2 = (uint16_t)p[4] | ((uint16_t)p[5] << 8u);
    const uint16_t hw3 = (uint16_t)p[6] | ((uint16_t)p[7] << 8u);
    if (is_cfi) {
        out->qry[0] = (uint8_t)(hw0 & 0xFFu);
        out->qry[1] = (uint8_t)(hw1 & 0xFFu);
        out->qry[2] = (uint8_t)(hw2 & 0xFFu);
        const uint8_t exp = (uint8_t)(hw3 & 0xFFu);
        out->valid = (out->qry[0] == 'Q') && (out->qry[1] == 'R') && (out->qry[2] == 'Y') &&
                     (exp >= 20u) && (exp <= 25u);
        out->size_bytes = out->valid ? (1UL << exp) : 0UL;
    } else {
        out->mfg = hw0;
        out->dev[0] = hw1;
        out->dev[1] = hw2;
        out->dev[2] = hw3;
        const p7501_ssot_flash_chip_t *chip = p7501_flash_find_chip_by_jedec(hw0, out->dev, 3u);
        out->size_bytes = (chip != NULL) ? chip->total_bytes : 0UL;
        out->valid = (hw0 != 0xFFFFu) && (hw0 != 0x0000u);
    }
    return out->valid;
}

bool p7501_jedec_verify_chip_match(uint16_t mfg, const uint16_t dev[3], uint32_t flash_size_bytes) {
    const p7501_ssot_flash_chip_t *chip = p7501_flash_find_chip_by_jedec(mfg, dev, 3u);
    if (chip == NULL) {
        return false;
    }
    return chip->total_bytes == p7501_flash_normalize_size(flash_size_bytes);
}

bool p7501_jedec_validate_bitclear(
    const uint8_t *expected_old,
    const uint8_t *desired_new,
    size_t len
) {
    if ((expected_old == NULL) || (desired_new == NULL) || (len == 0u)) {
        return false;
    }
    for (size_t i = 0u; i < len; ++i) {
        /* Spansion NOR Flash bit-clear cannot flip 0 -> 1 without Sector Erase */
        if ((desired_new[i] & (uint8_t)(~expected_old[i])) != 0u) {
            return false;
        }
    }
    return true;
}
