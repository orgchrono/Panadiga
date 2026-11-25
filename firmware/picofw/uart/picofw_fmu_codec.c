/**
 * @file picofw_fmu_codec.c
 * @brief Pure ISO C11 implementation of FMU trigger detection, hit gating, and argument parsing.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_fmu_codec.h"
#include "common/picofw_hex.h"
#include "uart/picofw_uart_codec.h"

#include <string.h>

void picofw_fmu_state_init(picofw_fmu_phase_state_t *st) {
    if (st != NULL) {
        memset(st, 0, sizeof(*st));
    }
}

void picofw_fmu_feed_byte(
    picofw_fmu_phase_state_t *st,
    uint32_t fmode,
    uint8_t rx_byte,
    uint32_t now_us,
    bool *out_should_release_sw_uart,
    bool *out_just_saw_serial_boot,
    bool *out_just_saw_wait1
) {
    if (out_should_release_sw_uart != NULL) *out_should_release_sw_uart = false;
    if (out_just_saw_serial_boot != NULL) *out_just_saw_serial_boot = false;
    if (out_just_saw_wait1 != NULL) *out_just_saw_wait1 = false;
    if (st == NULL) {
        return;
    }

    const size_t cap = sizeof(st->match_ring);
    if (st->match_ring_len < cap) {
        st->match_ring[st->match_ring_len++] = rx_byte;
    } else {
        memmove(&st->match_ring[0], &st->match_ring[1], cap - 1u);
        st->match_ring[cap - 1u] = rx_byte;
    }

    const char *tail = (const char *)st->match_ring;
    const size_t n = st->match_ring_len;

    const bool hit_serial = (n >= 16u) && (memcmp(&tail[n - 16u], "serial boot mode", 16u) == 0);
    const bool hit_ddr    = (n >= 9u)  && (memcmp(&tail[n - 9u],  "DDR CHECK", 9u) == 0);
    const bool hit_wait1  = (n >= 14u) && (memcmp(&tail[n - 14u], "wait 1 second.", 14u) == 0);
    const bool hit_sd     = (n >= 13u) && (memcmp(&tail[n - 13u], "SD init start", 13u) == 0);

    if (hit_serial && out_just_saw_serial_boot != NULL) {
        *out_just_saw_serial_boot = true;
    }
    if (!st->sw_uart_flipped && (((fmode == 2u) && hit_serial) || ((fmode == 3u) && hit_ddr))) {
        st->sw_uart_flipped = true;
        if (out_should_release_sw_uart != NULL) {
            *out_should_release_sw_uart = true;
        }
    }
    if (!st->wait1_seen && hit_wait1) {
        st->wait1_seen = true;
        st->wait1_us = now_us;
        if (out_just_saw_wait1 != NULL) {
            *out_just_saw_wait1 = true;
        }
    }
    if (hit_sd) {
        st->sd_started = true;
    }
}

bool picofw_fmu_is_echo_hit(
    uint16_t next_probe_bval,
    bool waiting_reply,
    bool sd_started,
    uint32_t dt_since_tx_us
) {
    return waiting_reply &&
           !sd_started &&
           (next_probe_bval > 0u) &&
           (next_probe_bval <= 256u) &&
           (dt_since_tx_us <= PICOFW_FMU_ECHO_WINDOW_US);
}

size_t picofw_fmu_decode_or_default_cr(const char *hex_str, uint8_t *out, size_t max_len) {
    if ((out == NULL) || (max_len == 0u)) {
        return 0u;
    }
    size_t out_len = 0u;
    if ((hex_str != NULL) && (*hex_str != '\0') &&
        picofw_decode_hex_bytes(hex_str, out, max_len, &out_len) && (out_len > 0u)) {
        return out_len;
    }
    out[0] = 0x0Du;
    return 1u;
}

void picofw_fmu_parse_test_args(
    const char *arg,
    uint32_t *out_fmode,
    bool *out_repeat_25ms,
    const char **out_hex_str
) {
    uint32_t raw_mode = 2u;
    const char *hex_ptr = "0D";

    if ((arg != NULL) && (*arg != '\0')) {
        char tok[24];
        const char *rest = picofw_next_token(arg, tok, sizeof(tok));
        if (rest != NULL) {
            if (picofw_parse_u32_strict(tok, &raw_mode)) {
                while ((*rest == ' ') || (*rest == '\t')) {
                    ++rest;
                }
                if (*rest != '\0') {
                    hex_ptr = rest;
                }
            } else {
                raw_mode = 0u;
                while ((*arg == ' ') || (*arg == '\t')) {
                    ++arg;
                }
                if (*arg != '\0') {
                    hex_ptr = arg;
                }
            }
        }
    }
    if (out_fmode != NULL) *out_fmode = raw_mode % 10u;
    if (out_repeat_25ms != NULL) *out_repeat_25ms = (raw_mode >= 10u);
    if (out_hex_str != NULL) *out_hex_str = hex_ptr;
}

void picofw_fmu_parse_batch_args(
    const char *arg,
    uint32_t *out_start,
    uint32_t *out_count,
    uint32_t *out_offset_ms
) {
    uint32_t start_b = 0u;
    uint32_t count_b = 16u;
    uint32_t offset_ms = 20u;

    if ((arg != NULL) && (*arg != '\0')) {
        char tok[24];
        const char *cur = picofw_next_token(arg, tok, sizeof(tok));
        if (cur != NULL && picofw_ascii_is_digits(tok)) {
            start_b = picofw_parse_uint32_clamped(tok, 0u, 0u, 255u);
            cur = picofw_next_token(cur, tok, sizeof(tok));
            if (cur != NULL && picofw_ascii_is_digits(tok)) {
                count_b = picofw_parse_uint32_clamped(tok, 16u, 1u, 256u);
                cur = picofw_next_token(cur, tok, sizeof(tok));
                if (cur != NULL && picofw_ascii_is_digits(tok)) {
                    offset_ms = picofw_parse_uint32_clamped(tok, 20u, 0u, 9000u);
                }
            }
        }
    }
    if ((start_b + count_b) > 256u) {
        count_b = 256u - start_b;
    }

    if (out_start != NULL) *out_start = start_b;
    if (out_count != NULL) *out_count = count_b;
    if (out_offset_ms != NULL) *out_offset_ms = offset_ms;
}
