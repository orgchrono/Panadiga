/**
 * @file p7501_uart_args.c
 * @brief Pure ISO C11 CLI/UART argument tokenization and boot/trigger spec parsers (DRY).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_codec.h"
#include "common/p7501_hex.h"

#include <stddef.h>
#include <string.h>

uint32_t p7501_parse_uint32_clamped(
    const char *arg,
    uint32_t default_val,
    uint32_t min_val,
    uint32_t max_val
) {
    uint64_t acc = 0u;
    bool saw_digit = false;
    const char *p = arg;

    if (!p) {
        return default_val;
    }
    while (*p == ' ' || *p == '\t') {
        ++p;
    }
    while (*p >= '0' && *p <= '9') {
        saw_digit = true;
        acc = (acc * 10ULL) + (uint64_t)(*p - '0');
        if (acc > (uint64_t)max_val) {
            return max_val;
        }
        ++p;
    }
    if (!saw_digit) {
        return default_val;
    }
    if (acc < (uint64_t)min_val) {
        return min_val;
    }
    return (uint32_t)acc;
}

const char *p7501_next_token(const char *cursor, char *out, size_t out_cap) {
    size_t idx = 0u;
    bool quoted = false;

    if (!cursor || !out || out_cap < 2u) {
        return NULL;
    }
    out[0] = '\0';
    while (*cursor == ' ' || *cursor == '\t') {
        ++cursor;
    }
    if (*cursor == '\0') {
        return NULL;
    }
    if (*cursor == '"') {
        quoted = true;
        ++cursor;
    }
    while (*cursor != '\0') {
        if (quoted) {
            if (*cursor == '"') {
                ++cursor;
                break;
            }
        } else if (*cursor == ' ' || *cursor == '\t') {
            break;
        }
        if ((idx + 1u) < out_cap) {
            out[idx++] = *cursor;
        }
        ++cursor;
    }
    out[idx] = '\0';
    return cursor;
}

p7501_boot_action_t p7501_parse_boot_spec(const char *token, uint32_t *out_jig_hold_ms) {
    if (out_jig_hold_ms) {
        *out_jig_hold_ms = 0u;
    }
    if (!token || token[0] == '\0') {
        return P7501_BOOT_ACTION_PASSIVE;
    }
    if (p7501_ascii_ieq(token, "NORMAL") || p7501_ascii_ieq(token, "BOOT")) {
        return P7501_BOOT_ACTION_NORMAL;
    }
    if (p7501_ascii_ieq(token, "JIG") || p7501_ascii_ieq(token, "SERIAL")) {
        return P7501_BOOT_ACTION_JIG;
    }
    /* Support `JIG:<hold_ms>` to hold SW_UART LOW across reset then release for Q7514 TX. */
    const char *colon = strchr(token, ':');
    if (colon != NULL) {
        char prefix[12];
        const size_t plen = (size_t)(colon - token);
        if (plen > 0u && plen < sizeof(prefix)) {
            memcpy(prefix, token, plen);
            prefix[plen] = '\0';
            if (p7501_ascii_ieq(prefix, "JIG") || p7501_ascii_ieq(prefix, "SERIAL")) {
                if (out_jig_hold_ms) {
                    *out_jig_hold_ms = p7501_parse_uint32_clamped(colon + 1, 350u, 50u, 30000u);
                }
                return P7501_BOOT_ACTION_JIG;
            }
        }
    }
    return P7501_BOOT_ACTION_PASSIVE;
}

p7501_boot_action_t p7501_parse_boot_action(const char *token) {
    return p7501_parse_boot_spec(token, NULL);
}

bool p7501_is_boot_spec_token(const char *token) {
    if (!token || token[0] == '\0') {
        return false;
    }
    return p7501_ascii_ieq(token, "NORMAL") || p7501_ascii_ieq(token, "BOOT") ||
           p7501_ascii_ieq(token, "NOPULL") || p7501_ascii_ieq(token, "PASSIVE") ||
           p7501_ascii_ieq(token, "JIG") || p7501_ascii_iprefix(token, "JIG:");
}

static bool is_sticky_token(const char *token) {
    return (token != NULL) &&
           (p7501_ascii_ieq(token, "STICKY") || p7501_ascii_ieq(token, "HOLD"));
}

static bool is_ctrl_token(const char *token) {
    return (token != NULL) &&
           (p7501_ascii_ieq(token, "CTRL") || p7501_ascii_ieq(token, "EDGES"));
}

void p7501_parse_window_boot_args(
    const char *arg,
    uint32_t default_ms,
    uint32_t min_ms,
    uint32_t max_ms,
    p7501_window_boot_spec_t *out_spec
) {
    char tok[24] = {0};

    if (!out_spec) {
        return;
    }
    out_spec->window_ms = default_ms;
    out_spec->boot_action = P7501_BOOT_ACTION_NORMAL;
    out_spec->jig_hold_ms = 0u;
    out_spec->sticky_jig = false;

    const char *cur = p7501_next_token(arg, tok, sizeof(tok));
    if (cur != NULL) {
        out_spec->window_ms = p7501_parse_uint32_clamped(tok, default_ms, min_ms, max_ms);
        while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
            if (is_sticky_token(tok)) {
                out_spec->sticky_jig = true;
            } else if (p7501_is_boot_spec_token(tok)) {
                out_spec->boot_action = p7501_parse_boot_spec(tok, &out_spec->jig_hold_ms);
            }
        }
    }
}

void p7501_parse_bridge_args(
    const char *arg,
    uint32_t default_baud,
    p7501_bridge_spec_t *out_spec
) {
    char tok[24] = {0};

    if (!out_spec) {
        return;
    }
    out_spec->baud = default_baud;
    out_spec->boot_action = P7501_BOOT_ACTION_PASSIVE;
    out_spec->jig_hold_ms = 0u;
    out_spec->log_ctrl_edges = false;

    const char *cur = arg;
    while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
        uint32_t parsed_u32 = 0u;
        if (is_ctrl_token(tok)) {
            out_spec->log_ctrl_edges = true;
        } else if (p7501_is_boot_spec_token(tok)) {
            out_spec->boot_action = p7501_parse_boot_spec(tok, &out_spec->jig_hold_ms);
        } else if (p7501_parse_u32_strict(tok, &parsed_u32)) {
            out_spec->baud = p7501_parse_uint32_clamped(
                tok, default_baud, P7501_UART_MIN_BAUD, P7501_UART_MAX_BAUD
            );
        }
    }
}

bool p7501_parse_waittx_args(const char *arg, p7501_waittx_spec_t *out) {
    char tok_timeout[24];
    char tok_mode[24];
    char tok_tx[(P7501_UART_HEX_MAX_BYTES * 2u) + 8u];
    char tok_post[24];
    const char *cur = arg;

    if (!arg || !out) {
        return false;
    }
    memset(out, 0, sizeof(*out));

    cur = p7501_next_token(cur, tok_timeout, sizeof(tok_timeout));
    cur = p7501_next_token(cur, tok_mode, sizeof(tok_mode));
    cur = p7501_next_token(cur, out->trigger, sizeof(out->trigger));
    cur = p7501_next_token(cur, tok_tx, sizeof(tok_tx));
    if (!cur || tok_timeout[0] == '\0' || out->trigger[0] == '\0' || tok_tx[0] == '\0') {
        return false;
    }

    out->timeout_ms = p7501_parse_uint32_clamped(tok_timeout, 15000u, 500u, 120000u);
    out->boot_action = p7501_parse_boot_spec(tok_mode, &out->jig_hold_ms);
    if (p7501_ascii_ieq(tok_tx, "BRK") || p7501_ascii_ieq(tok_tx, "BREAK")) {
        out->tx_break = true;
    } else if (!p7501_decode_hex_bytes(
                   tok_tx, out->tx_bytes, sizeof(out->tx_bytes), &out->tx_len
               )) {
        return false;
    }

    if (p7501_next_token(cur, tok_post, sizeof(tok_post)) != NULL) {
        out->post_rx_ms = p7501_parse_uint32_clamped(tok_post, 4000u, 100u, 30000u);
    } else {
        out->post_rx_ms = 4000u;
    }
    return true;
}

bool p7501_parse_jigpin_arg(const char *arg, p7501_jigpin_mode_t *out_mode) {
    char tok[24] = {0};

    if (out_mode == NULL) {
        return false;
    }
    if (p7501_next_token(arg, tok, sizeof(tok)) == NULL || p7501_ascii_ieq(tok, "STATUS")) {
        *out_mode = P7501_JIGPIN_STATUS;
        return true;
    }
    if (p7501_ascii_ieq(tok, "LOW") || p7501_ascii_ieq(tok, "ON") ||
        p7501_ascii_ieq(tok, "GND") || p7501_ascii_ieq(tok, "1")) {
        *out_mode = P7501_JIGPIN_LOW;
        return true;
    }
    if (p7501_ascii_ieq(tok, "HIZ") || p7501_ascii_ieq(tok, "OFF") ||
        p7501_ascii_ieq(tok, "RELEASE") || p7501_ascii_ieq(tok, "0")) {
        *out_mode = P7501_JIGPIN_HIZ;
        return true;
    }
    return false;
}
