/**
 * @file picofw_pana_dumpnor_verify.c
 * @brief Portable ISO C11 verification, resume state machine, and `0xFF` blank tracker for `PANADMP1`/`PANASUM1`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_pana_dumpnor.h"

#include "common/picofw_hex.h"

#include <string.h>

bool picofw_dumpnor_eval_sum_only_reply(
    const uint8_t *buf,
    size_t len,
    uint32_t block_size,
    uint32_t expected_blocks,
    picofw_nor_check_result_t *out
) {
    if ((buf == NULL) || (out == NULL) || (block_size == 0u) || (expected_blocks == 0u)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    const size_t body_len = (size_t)expected_blocks * 4u;
    size_t pos = len;
    for (size_t i = 0u; i + 20u + body_len + 8u <= len; ++i) {
        if ((memcmp(&buf[i], PICOFW_DUMPNOR_SUM_MAGIC, 8u) == 0) &&
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
        const uint32_t s = picofw_read_u32_le(&sums[b * 4u]);
        if (s == expected_ff_sum) {
            blank_cnt += 1u;
        }
    }
    out->valid = true;
    out->blocks_checked = expected_blocks;
    out->blank_blocks = blank_cnt;
    out->all_blank = (blank_cnt == expected_blocks);
    out->digest_crc32 = picofw_crc32_ieee(sums, body_len);
    return true;
}

bool picofw_dumpnor_verify_block(
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
    const uint32_t expected = picofw_read_u32_le(sum_le);
    if (out_all_ff != NULL) {
        *out_all_ff = (sum == (uint32_t)blk_len * 0xFFu);
    }
    return sum == expected;
}

bool picofw_dumpnor_is_fast_baud(uint32_t baud) {
    return (baud == 115200u) || (baud == 230400u) || (baud == 460800u);
}

bool picofw_dumpnor_resume_begin(
    picofw_dumpnor_resume_state_t *st,
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t baud
) {
    if ((st == NULL) || !picofw_dumpnor_validate_range(start_addr, total_len) ||
        !picofw_dumpnor_is_fast_baud(baud)) {
        return false;
    }
    st->active = true;
    st->start_addr = start_addr;
    st->total_len = total_len;
    st->completed_bytes = 0u;
    st->block_size = picofw_dumpnor_block_size(total_len);
    st->baud = baud;
    return true;
}

bool picofw_dumpnor_resume_next_slice(
    const picofw_dumpnor_resume_state_t *st,
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

void picofw_dumpnor_resume_advance(picofw_dumpnor_resume_state_t *st, uint32_t verified_bytes) {
    if ((st == NULL) || !st->active) {
        return;
    }
    const uint32_t rem = st->total_len - st->completed_bytes;
    st->completed_bytes += (verified_bytes > rem) ? rem : verified_bytes;
    if (st->completed_bytes >= st->total_len) {
        st->active = false;
    }
}

void picofw_dumpnor_ff_tracker_init(picofw_dumpnor_ff_tracker_t *t, uint32_t block_size) {
    if (t == NULL) {
        return;
    }
    memset(t, 0, sizeof(*t));
    t->block_size = block_size;
    t->cur_all_ff = true;
}

void picofw_dumpnor_ff_tracker_feed(picofw_dumpnor_ff_tracker_t *t, uint8_t b) {
    if ((t == NULL) || (t->block_size == 0u)) {
        return;
    }
    if (t->hdr_skip < 12u) {
        t->hdr_skip += 1u;
        return;
    }
    if (t->blk_pos < t->block_size) {
        if (b != 0xFFu) {
            t->cur_all_ff = false;
        }
    }
    t->blk_pos += 1u;
    if (t->blk_pos >= (t->block_size + 4u)) {
        t->total_blocks += 1u;
        if (t->cur_all_ff) {
            t->ff_blocks += 1u;
        }
        t->blk_pos = 0u;
        t->cur_all_ff = true;
    }
}
