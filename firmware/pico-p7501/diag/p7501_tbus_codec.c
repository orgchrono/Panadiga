/**
 * @file p7501_tbus_codec.c
 * @brief Pure C11 Panasonic Timer-MCU T-BUS frame encoder/decoder (`TBUSSNIFF` / `M0KEEPALIVE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_tbus_codec.h"
#include "common/p7501_hex.h"
#include "uart/p7501_uart_codec.h"
#include <stdio.h>
#include <string.h>

uint8_t p7501_tbus_checksum(uint8_t opcode, const uint8_t *payload, uint8_t len) {
    uint8_t x = (uint8_t)(P7501_TBUS_SYNC_BYTE ^ opcode ^ len);
    if (payload != NULL) {
        for (uint8_t i = 0u; i < len; ++i) {
            x ^= payload[i];
        }
    }
    return x;
}

size_t p7501_tbus_build_keepalive(uint8_t seq, uint8_t *out_buf, size_t cap) {
    if ((out_buf == NULL) || (cap < 6u)) {
        return 0u;
    }
    const uint8_t payload[2] = { 0x01u, seq };
    out_buf[0] = P7501_TBUS_SYNC_BYTE;
    out_buf[1] = 0x10u; /* M0_KEEPALIVE opcode */
    out_buf[2] = sizeof(payload);
    out_buf[3] = payload[0];
    out_buf[4] = payload[1];
    out_buf[5] = p7501_tbus_checksum(0x10u, payload, sizeof(payload));
    return 6u;
}

bool p7501_tbus_decode_frame(const uint8_t *raw, size_t raw_len, p7501_tbus_frame_t *out) {
    if ((raw == NULL) || (out == NULL) || (raw_len < 4u) || (raw[0] != P7501_TBUS_SYNC_BYTE)) {
        return false;
    }
    const uint8_t plen = raw[2];
    if ((plen > P7501_TBUS_MAX_PAYLOAD) || (raw_len < (size_t)(4u + plen))) {
        return false;
    }
    const uint8_t exp_csum = p7501_tbus_checksum(raw[1], &raw[3], plen);
    if (raw[3u + plen] != exp_csum) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->sync = raw[0];
    out->opcode = raw[1];
    out->len = plen;
    if (plen > 0u) {
        memcpy(out->payload, &raw[3], plen);
    }
    out->checksum = exp_csum;
    return true;
}

bool p7501_tbus_parse_keepalive_args(
    const char *arg,
    p7501_m0_keepalive_action_t *out_action,
    uint32_t *out_interval_ms
) {
    if ((out_action == NULL) || (out_interval_ms == NULL)) {
        return false;
    }
    *out_action = P7501_M0_KA_ON;
    *out_interval_ms = P7501_M0_KEEPALIVE_DEFAULT_MS;

    char tok1[24] = {0};
    const char *rest = p7501_next_token(arg, tok1, sizeof(tok1));
    if (tok1[0] == '\0') {
        return true;
    }
    if (p7501_ascii_ieq(tok1, "OFF")) {
        *out_action = P7501_M0_KA_OFF;
        return true;
    }
    if (p7501_ascii_ieq(tok1, "STATUS")) {
        *out_action = P7501_M0_KA_STATUS;
        return true;
    }
    if (p7501_ascii_ieq(tok1, "ON")) {
        char tok2[24] = {0};
        if ((rest != NULL) && (p7501_next_token(rest, tok2, sizeof(tok2)) != NULL) && (tok2[0] != '\0')) {
            if (!p7501_ascii_is_digits(tok2)) {
                return false;
            }
            *out_interval_ms = p7501_parse_uint32_clamped(
                tok2, P7501_M0_KEEPALIVE_DEFAULT_MS, P7501_M0_KEEPALIVE_MIN_MS, P7501_M0_KEEPALIVE_MAX_MS
            );
        }
        return true;
    }
    if (p7501_ascii_is_digits(tok1)) {
        *out_interval_ms = p7501_parse_uint32_clamped(
            tok1, P7501_M0_KEEPALIVE_DEFAULT_MS, P7501_M0_KEEPALIVE_MIN_MS, P7501_M0_KEEPALIVE_MAX_MS
        );
        return true;
    }
    return false;
}

bool p7501_tbus_opcode_rejected_by_br_ic7501(uint8_t opcode) {
    static const uint8_t k_rejected[13] = {
        0x85u, 0x89u, 0x9Bu, 0xA9u, 0xAAu, 0xC9u, 0xDAu,
        0xE3u, 0xE4u, 0xECu, 0xF0u, 0xF9u, 0xFAu
    };
    for (size_t i = 0u; i < sizeof(k_rejected); ++i) {
        if (k_rejected[i] == opcode) {
            return true;
        }
    }
    return false;
}

void p7501_tbus_audit_init(p7501_tbus_audit_t *st) {
    if (st == NULL) {
        return;
    }
    memset(st, 0, sizeof(*st));
}

void p7501_tbus_audit_record_frame(p7501_tbus_audit_t *st, const p7501_tbus_frame_t *frame) {
    if ((st == NULL) || (frame == NULL)) {
        return;
    }
    st->frames_valid += 1u;
    if ((frame->opcode == 0x7Fu) && (frame->len >= 1u)) {
        st->nak_7f_seen += 1u;
        const uint8_t orig_op = frame->payload[0];
        st->rejected_bitmap[orig_op >> 3u] |= (uint8_t)(1u << (orig_op & 7u));
        if (p7501_tbus_opcode_rejected_by_br_ic7501(orig_op)) {
            st->frames_rejected_br += 1u;
        }
        return;
    }
    if (p7501_tbus_opcode_rejected_by_br_ic7501(frame->opcode)) {
        st->frames_rejected_br += 1u;
        st->rejected_bitmap[frame->opcode >> 3u] |= (uint8_t)(1u << (frame->opcode & 7u));
    }
}

size_t p7501_tbus_audit_format_summary(const p7501_tbus_audit_t *st, char *out, size_t cap) {
    if ((st == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    char ops_csv[96];
    (void)p7501_format_hex_bitmap_csv(st->rejected_bitmap, ops_csv, sizeof(ops_csv));
    const int n = snprintf(
        out,
        cap,
        "@P7501_TBUS_AUDIT frames=%lu br_rejected=%lu nak7f=%lu ops=%s",
        (unsigned long)st->frames_valid,
        (unsigned long)st->frames_rejected_br,
        (unsigned long)st->nak_7f_seen,
        ops_csv
    );
    if (n < 0) {
        out[0] = '\0';
        return 0u;
    }
    return ((size_t)n < cap) ? (size_t)n : (cap - 1u);
}
