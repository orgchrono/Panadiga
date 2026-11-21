/**
 * @file p7501_pana_dumpnor.c
 * @brief Portable ISO C11 builder for read-only MN103S `PANADMP1` UART NOR dumper stub.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_dumpnor.h"

#include "common/p7501_hex.h"
#include "flash/p7501_flash.h"

#include <string.h>

const uint8_t k_p7501_mn103_putc_d3[24] = {
    0xFCu, 0xACu, 0x0Cu, 0x20u, 0x00u, 0xD4u, /* movhu (0xD400200C), d0 */
    0x1Cu,                                     /* exthu d0 */
    0xF8u, 0xE0u, 0xA0u,                       /* and 0xA0, d0 */
    0xA0u, 0x00u,                              /* cmp 0, d0 */
    0xC9u, 0xF4u,                              /* bne -12 */
    0x8Cu,                                     /* mov d3, d0 */
    0xFCu, 0x82u, 0x08u, 0x20u, 0x00u, 0xD4u, /* movbu d0, (0xD4002008) */
    0xDFu, 0x00u, 0x00u                        /* ret [], 0 */
};

typedef struct dumpnor_buf {
    uint8_t *ptr;
    size_t len;
    size_t cap;
} dumpnor_buf_t;

static void write_u16_le(uint8_t *dst, uint16_t v) {
    dst[0] = (uint8_t)(v & 0xFFu);
    dst[1] = (uint8_t)((v >> 8) & 0xFFu);
}

static void write_u32_le(uint8_t *dst, uint32_t v) {
    dst[0] = (uint8_t)(v & 0xFFu);
    dst[1] = (uint8_t)((v >> 8) & 0xFFu);
    dst[2] = (uint8_t)((v >> 16) & 0xFFu);
    dst[3] = (uint8_t)((v >> 24) & 0xFFu);
}

static void emit_bytes(dumpnor_buf_t *b, const uint8_t *src, size_t n) {
    if ((b->len + n) <= b->cap) {
        memcpy(&b->ptr[b->len], src, n);
    }
    b->len += n;
}

static void emit_byte(dumpnor_buf_t *b, uint8_t v) {
    if (b->len < b->cap) {
        b->ptr[b->len] = v;
    }
    b->len += 1u;
}

static size_t emit_mov_imm32_reg(dumpnor_buf_t *b, uint8_t sub_op, uint32_t imm) {
    const size_t patch_off = b->len + 2u;
    uint8_t enc[6] = { 0xFCu, sub_op, 0u, 0u, 0u, 0u };
    write_u32_le(&enc[2], imm);
    emit_bytes(b, enc, sizeof(enc));
    return patch_off;
}

static size_t emit_call_rel16(dumpnor_buf_t *b) {
    const size_t insn_off = b->len;
    static const uint8_t k_call[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    emit_bytes(b, k_call, sizeof(k_call));
    return insn_off;
}

static void emit_bne_back(dumpnor_buf_t *b, size_t target_off) {
    const ptrdiff_t disp = (ptrdiff_t)target_off - (ptrdiff_t)b->len;
    emit_byte(b, 0xC9u);
    emit_byte(b, (uint8_t)((int8_t)disp));
}

static void patch_call_rel16(dumpnor_buf_t *b, size_t call_off, size_t target_off) {
    const int16_t disp = (int16_t)((ptrdiff_t)target_off - (ptrdiff_t)call_off);
    write_u16_le(&b->ptr[call_off + 1u], (uint16_t)disp);
}

uint32_t p7501_dumpnor_block_size(uint32_t total_len) {
    if (total_len == 0u) {
        return 0u;
    }
    if ((total_len % 4096u) == 0u) {
        return 4096u;
    }
    if ((total_len % 256u) == 0u) {
        return 256u;
    }
    return 0u;
}

bool p7501_dumpnor_validate_range(uint32_t start_addr, uint32_t total_len) {
    return (p7501_dumpnor_block_size(total_len) != 0u) &&
           p7501_flash_is_readonly_nor_range(p7501_flash_get_active_size(), start_addr, total_len);
}

bool p7501_dumpnor_parse_hex_tok(const char *tok, uint32_t *out_val) {
    return p7501_parse_hex_u32_exact(tok, 8u, out_val);
}

bool p7501_dumpnor_parse_colon_spec(const char *spec, uint32_t *out_start, uint32_t *out_len) {
    if ((spec == NULL) || (spec[0] == '\0') || (out_start == NULL) || (out_len == NULL)) {
        return false;
    }
    const char *comma = strchr(spec, ',');
    if (comma == NULL) {
        *out_len = P7501_DUMPNOR_DEFAULT_LEN;
        return p7501_dumpnor_parse_hex_tok(spec, out_start) &&
               p7501_dumpnor_validate_range(*out_start, *out_len);
    }
    if ((comma == spec) || (comma[1] == '\0') || ((size_t)(comma - spec) > 10u)) {
        return false;
    }
    char start_buf[12];
    const size_t slen = (size_t)(comma - spec);
    memcpy(start_buf, spec, slen);
    start_buf[slen] = '\0';
    return p7501_dumpnor_parse_hex_tok(start_buf, out_start) &&
           p7501_dumpnor_parse_hex_tok(comma + 1, out_len) &&
           p7501_dumpnor_validate_range(*out_start, *out_len);
}

static size_t build_dumpnor_stub_core(
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t load_addr,
    bool stream_raw_bytes,
    const char *hdr_magic,
    uint8_t *out_buf,
    size_t out_cap
) {
    const uint32_t block_size = p7501_dumpnor_block_size(total_len);
    const size_t min_cap = stream_raw_bytes ? P7501_DUMPNOR_STUB_SIZE : (P7501_DUMPNOR_STUB_SIZE - 5u);
    if (!p7501_dumpnor_validate_range(start_addr, total_len) ||
        (load_addr > 0xFFF00000UL) ||
        (out_buf == NULL) ||
        (hdr_magic == NULL) ||
        (out_cap < min_cap)) {
        return 0u;
    }

    /* Emit SC0 PUTC(D3) leaf subroutine + loop primitives */
    static const uint8_t k_loop_hdr[2]  = { 0x8Au, 0x00u };
    static const uint8_t k_rd_byte[4]   = { 0xF0u, 0x40u, 0x83u, 0xE2u };
    static const uint8_t k_next_byte[6] = { 0x20u, 0x01u, 0x29u, 0xFFu, 0xA5u, 0x00u };
    static const uint8_t k_csum_init[2] = { 0x85u, 0x04u };
    static const uint8_t k_csum_mov[2]  = { 0x88u, 0x83u };
    static const uint8_t k_csum_step[7] = { 0xF8u, 0xC6u, 0x08u, 0x29u, 0xFFu, 0xA5u, 0x00u };
    static const uint8_t k_park[2]      = { 0xCAu, 0x00u };
    static const uint8_t k_span_rd[3]   = { 0xF0u, 0x40u, 0x83u };
    static const uint8_t k_span_nxt[3]  = { 0x20u, 0x01u, 0xB4u };
    static const uint8_t k_ret0[3]      = { 0xDFu, 0x00u, 0x00u };

    dumpnor_buf_t b = { out_buf, 0u, out_cap };
    const size_t hdr_s_patch = emit_mov_imm32_reg(&b, 0xDCu, 0u);
    const size_t hdr_e_patch = emit_mov_imm32_reg(&b, 0xDDu, 0u);
    const size_t c_emit_hdr  = emit_call_rel16(&b);

    (void)emit_mov_imm32_reg(&b, 0xDCu, start_addr);
    (void)emit_mov_imm32_reg(&b, 0xDDu, start_addr + total_len);

    const size_t blk_loop = b.len;
    emit_bytes(&b, k_loop_hdr, sizeof(k_loop_hdr));
    (void)emit_mov_imm32_reg(&b, 0xCDu, block_size);

    const size_t byte_loop = b.len;
    emit_bytes(&b, k_rd_byte, sizeof(k_rd_byte));
    size_t c_putc_data = 0u;
    if (stream_raw_bytes) {
        c_putc_data = emit_call_rel16(&b);
    }
    emit_bytes(&b, k_next_byte, sizeof(k_next_byte));
    emit_bne_back(&b, byte_loop);

    emit_bytes(&b, k_csum_init, sizeof(k_csum_init));
    const size_t csum_loop = b.len;
    emit_bytes(&b, k_csum_mov, sizeof(k_csum_mov));
    const size_t c_putc_csum = emit_call_rel16(&b);
    emit_bytes(&b, k_csum_step, sizeof(k_csum_step));
    emit_bne_back(&b, csum_loop);

    emit_byte(&b, 0xB4u);
    emit_bne_back(&b, blk_loop);

    const size_t ftr_s_patch = emit_mov_imm32_reg(&b, 0xDCu, 0u);
    const size_t ftr_e_patch = emit_mov_imm32_reg(&b, 0xDDu, 0u);
    const size_t c_emit_ftr  = emit_call_rel16(&b);
    emit_bytes(&b, k_park, sizeof(k_park));

    const size_t emit_span_off = b.len;
    emit_bytes(&b, k_span_rd, sizeof(k_span_rd));
    const size_t c_putc_span = emit_call_rel16(&b);
    emit_bytes(&b, k_span_nxt, sizeof(k_span_nxt));
    emit_bne_back(&b, emit_span_off);
    emit_bytes(&b, k_ret0, sizeof(k_ret0));

    const size_t putc_off = b.len;
    emit_bytes(&b, k_p7501_mn103_putc_d3, sizeof(k_p7501_mn103_putc_d3));

    const size_t hdr_off = b.len;
    emit_bytes(&b, (const uint8_t *)hdr_magic, 8u);
    uint8_t meta[12];
    write_u32_le(&meta[0], start_addr);
    write_u32_le(&meta[4], total_len);
    write_u32_le(&meta[8], block_size);
    emit_bytes(&b, meta, sizeof(meta));

    const size_t ftr_off = b.len;
    emit_bytes(&b, (const uint8_t *)"PANADONE", 8u);
    const size_t end_off = b.len;

    if (stream_raw_bytes && (end_off != P7501_DUMPNOR_STUB_SIZE)) {
        return 0u;
    }

    patch_call_rel16(&b, c_emit_hdr, emit_span_off);
    patch_call_rel16(&b, c_emit_ftr, emit_span_off);
    if (stream_raw_bytes) {
        patch_call_rel16(&b, c_putc_data, putc_off);
    }
    patch_call_rel16(&b, c_putc_csum, putc_off);
    patch_call_rel16(&b, c_putc_span, putc_off);

    write_u32_le(&out_buf[hdr_s_patch], load_addr + (uint32_t)hdr_off);
    write_u32_le(&out_buf[hdr_e_patch], load_addr + (uint32_t)ftr_off);
    write_u32_le(&out_buf[ftr_s_patch], load_addr + (uint32_t)ftr_off);
    write_u32_le(&out_buf[ftr_e_patch], load_addr + (uint32_t)end_off);
    return end_off;
}

size_t p7501_dumpnor_build_stub(
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return build_dumpnor_stub_core(
        start_addr, total_len, load_addr, true, "PANADMP1", out_buf, out_cap
    );
}

size_t p7501_dumpnor_build_checksum_stub(
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return build_dumpnor_stub_core(
        start_addr, total_len, load_addr, false, P7501_DUMPNOR_SUM_MAGIC, out_buf, out_cap
    );
}

bool p7501_dumpnor_eval_sum_only_reply(
    const uint8_t *buf,
    size_t len,
    uint32_t block_size,
    uint32_t expected_blocks,
    p7501_nor_check_result_t *out
) {
    if ((buf == NULL) || (out == NULL) || (block_size == 0u) || (expected_blocks == 0u)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    const size_t body_len = (size_t)expected_blocks * 4u;
    size_t pos = len;
    for (size_t i = 0u; i + 20u + body_len + 8u <= len; ++i) {
        if ((memcmp(&buf[i], P7501_DUMPNOR_SUM_MAGIC, 8u) == 0) &&
            (memcmp(&buf[i + 20u + body_len], "PANADONE", 8u) == 0)) {
            pos = i;
            break;
        }
    }
    if (pos >= len) {
        return false;
    }
    const uint8_t *sums = &buf[pos + 20u];
    const uint32_t expected_ff_sum = block_size * 0xFFu;
    uint32_t blank_cnt = 0u;
    for (uint32_t b = 0u; b < expected_blocks; ++b) {
        const uint32_t s = p7501_read_u32_le(&sums[b * 4u]);
        if (s == expected_ff_sum) {
            blank_cnt += 1u;
        }
    }
    out->valid = true;
    out->blocks_checked = expected_blocks;
    out->blank_blocks = blank_cnt;
    out->all_blank = (blank_cnt == expected_blocks);
    out->digest_crc32 = p7501_crc32_ieee(sums, body_len);
    return true;
}

bool p7501_dumpnor_verify_block(
    const uint8_t *blk,
    size_t blk_len,
    const uint8_t sum_le[4],
    bool *out_all_ff
) {
    if ((blk == NULL) || (sum_le == NULL) || (blk_len == 0u)) {
        return false;
    }
    uint32_t sum = 0u;
    for (size_t i = 0u; i < blk_len; ++i) {
        sum += (uint32_t)blk[i];
    }
    const uint32_t expected = p7501_read_u32_le(sum_le);
    if (out_all_ff != NULL) {
        *out_all_ff = (sum == (uint32_t)blk_len * 0xFFu);
    }
    return sum == expected;
}

bool p7501_dumpnor_is_fast_baud(uint32_t baud) {
    return (baud == 115200u) || (baud == 230400u) || (baud == 460800u);
}

bool p7501_dumpnor_resume_begin(
    p7501_dumpnor_resume_state_t *st,
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t baud
) {
    if ((st == NULL) || !p7501_dumpnor_validate_range(start_addr, total_len) ||
        !p7501_dumpnor_is_fast_baud(baud)) {
        return false;
    }
    st->active = true;
    st->start_addr = start_addr;
    st->total_len = total_len;
    st->completed_bytes = 0u;
    st->block_size = p7501_dumpnor_block_size(total_len);
    st->baud = baud;
    return true;
}

bool p7501_dumpnor_resume_next_slice(
    const p7501_dumpnor_resume_state_t *st,
    uint32_t *out_addr,
    uint32_t *out_len
) {
    if ((st == NULL) || !st->active || (st->completed_bytes >= st->total_len) ||
        (out_addr == NULL) || (out_len == NULL)) {
        return false;
    }
    *out_addr = st->start_addr + st->completed_bytes;
    *out_len = st->total_len - st->completed_bytes;
    return true;
}

void p7501_dumpnor_resume_advance(p7501_dumpnor_resume_state_t *st, uint32_t verified_bytes) {
    if ((st == NULL) || !st->active) {
        return;
    }
    const uint32_t rem = st->total_len - st->completed_bytes;
    st->completed_bytes += (verified_bytes > rem) ? rem : verified_bytes;
    if (st->completed_bytes >= st->total_len) {
        st->active = false;
    }
}
