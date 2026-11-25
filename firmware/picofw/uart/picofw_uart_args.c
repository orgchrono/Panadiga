/**
 * @file picofw_uart_args.c
 * @brief Pure ISO C11 CLI/UART argument tokenization and boot/trigger spec parsers (DRY).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_uart_codec.h"
#include "common/picofw_hex.h"
#include "common/picofw_wdt_logic.h"
#include "generated/picofw_ssot.h"

#include <stddef.h>
#include <string.h>

uint32_t picofw_parse_uint32_clamped(
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

const char *picofw_next_token(const char *cursor, char *out, size_t out_cap) {
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

picofw_boot_action_t picofw_parse_boot_spec_ex(
    const char *token,
    uint32_t *out_jig_hold_ms,
    bool *out_no_xreset
) {
    if (out_jig_hold_ms != NULL) {
        *out_jig_hold_ms = 0u;
    }
    if (out_no_xreset != NULL) {
        *out_no_xreset = false;
    }
    if (!token || token[0] == '\0') {
        return PICOFW_BOOT_ACTION_PASSIVE;
    }
    if (picofw_ascii_ieq(token, "NORMAL") || picofw_ascii_ieq(token, "BOOT")) {
        return PICOFW_BOOT_ACTION_NORMAL;
    }
    if (picofw_ascii_ieq(token, "JIG") || picofw_ascii_ieq(token, "SERIAL")) {
        if (out_jig_hold_ms != NULL) {
            *out_jig_hold_ms = PICOFW_SSOT_JIG_DEFAULT_HOLD_MS;
        }
        return PICOFW_BOOT_ACTION_JIG;
    }
    if (picofw_ascii_ieq(token, "PREJIG")) {
        if (out_jig_hold_ms != NULL) {
            *out_jig_hold_ms = 3000u;
        }
        if (out_no_xreset != NULL) {
            *out_no_xreset = true;
        }
        return PICOFW_BOOT_ACTION_JIG;
    }
    /* Support `JIG:<hold_ms>` and `PREJIG:<hold_ms>` (no XRESET pulse on Standby wake). */
    const char *colon = strchr(token, ':');
    if (colon != NULL) {
        char prefix[12];
        const size_t plen = (size_t)(colon - token);
        if (plen > 0u && plen < sizeof(prefix)) {
            memcpy(prefix, token, plen);
            prefix[plen] = '\0';
            const bool is_prejig = picofw_ascii_ieq(prefix, "PREJIG");
            if (is_prejig || picofw_ascii_ieq(prefix, "JIG") || picofw_ascii_ieq(prefix, "SERIAL")) {
                if (out_jig_hold_ms != NULL) {
                    *out_jig_hold_ms = picofw_parse_uint32_clamped(
                        colon + 1, is_prejig ? 3000u : 350u, 50u, 30000u
                    );
                }
                if (out_no_xreset != NULL) {
                    *out_no_xreset = is_prejig;
                }
                return PICOFW_BOOT_ACTION_JIG;
            }
        }
    }
    return PICOFW_BOOT_ACTION_PASSIVE;
}

picofw_boot_action_t picofw_parse_boot_spec(const char *token, uint32_t *out_jig_hold_ms) {
    return picofw_parse_boot_spec_ex(token, out_jig_hold_ms, NULL);
}

picofw_boot_action_t picofw_parse_boot_action(const char *token) {
    return picofw_parse_boot_spec(token, NULL);
}

bool picofw_is_boot_spec_token(const char *token) {
    if (!token || token[0] == '\0') {
        return false;
    }
    return picofw_ascii_ieq(token, "NORMAL") || picofw_ascii_ieq(token, "BOOT") ||
           picofw_ascii_ieq(token, "NOPULL") || picofw_ascii_ieq(token, "PASSIVE") ||
           picofw_ascii_ieq(token, "JIG") || picofw_ascii_iprefix(token, "JIG:") ||
           picofw_ascii_ieq(token, "PREJIG") || picofw_ascii_iprefix(token, "PREJIG:");
}

bool picofw_parse_wake_arg_token(const char *token, bool *out_wait, uint32_t *out_timeout_ms) {
    if ((token == NULL) || (out_wait == NULL) || (out_timeout_ms == NULL)) {
        return false;
    }
    if (picofw_ascii_ieq(token, "WAKE")) {
        *out_wait = true;
        *out_timeout_ms = (PICOFW_WDT_MAX_CMD_WINDOW_MS < 30000u)
            ? PICOFW_WDT_MAX_CMD_WINDOW_MS
            : 30000u;
        return true;
    }
    if (picofw_ascii_iprefix(token, "WAKE:")) {
        const char *ms_tok = token + 5u;
        if ((ms_tok[0] == '\0') || !picofw_ascii_is_digits(ms_tok)) {
            return false;
        }
        *out_wait = true;
        *out_timeout_ms = picofw_parse_uint32_clamped(
            ms_tok, 30000u, 1000u, PICOFW_WDT_MAX_CMD_WINDOW_MS
        );
        return true;
    }
    return false;
}

static bool is_sticky_token(const char *token) {
    return (token != NULL) &&
           (picofw_ascii_ieq(token, "STICKY") || picofw_ascii_ieq(token, "HOLD"));
}

static bool is_ctrl_token(const char *token) {
    return (token != NULL) &&
           (picofw_ascii_ieq(token, "CTRL") || picofw_ascii_ieq(token, "EDGES"));
}

void picofw_parse_window_boot_args(
    const char *arg,
    uint32_t default_ms,
    uint32_t min_ms,
    uint32_t max_ms,
    picofw_window_boot_spec_t *out_spec
) {
    char tok[24] = {0};

    if (!out_spec) {
        return;
    }
    out_spec->window_ms = default_ms;
    out_spec->boot_action = PICOFW_BOOT_ACTION_NORMAL;
    out_spec->jig_hold_ms = 0u;
    out_spec->sticky_jig = false;
    out_spec->no_xreset = false;
    out_spec->wait_for_wake = false;
    out_spec->wake_timeout_ms = 0u;

    const char *cur = picofw_next_token(arg, tok, sizeof(tok));
    if (cur != NULL) {
        out_spec->window_ms = picofw_parse_uint32_clamped(tok, default_ms, min_ms, max_ms);
        while ((cur = picofw_next_token(cur, tok, sizeof(tok))) != NULL) {
            if (is_sticky_token(tok)) {
                out_spec->sticky_jig = true;
            } else if (picofw_is_boot_spec_token(tok)) {
                out_spec->boot_action = picofw_parse_boot_spec_ex(
                    tok, &out_spec->jig_hold_ms, &out_spec->no_xreset
                );
            } else {
                (void)picofw_parse_wake_arg_token(
                    tok, &out_spec->wait_for_wake, &out_spec->wake_timeout_ms
                );
            }
        }
    }
}

void picofw_parse_bridge_args(
    const char *arg,
    uint32_t default_baud,
    picofw_bridge_spec_t *out_spec
) {
    char tok[24] = {0};

    if (!out_spec) {
        return;
    }
    out_spec->baud = default_baud;
    out_spec->boot_action = PICOFW_BOOT_ACTION_PASSIVE;
    out_spec->jig_hold_ms = 0u;
    out_spec->log_ctrl_edges = false;

    const char *cur = arg;
    while ((cur = picofw_next_token(cur, tok, sizeof(tok))) != NULL) {
        uint32_t parsed_u32 = 0u;
        if (is_ctrl_token(tok)) {
            out_spec->log_ctrl_edges = true;
        } else if (picofw_is_boot_spec_token(tok)) {
            out_spec->boot_action = picofw_parse_boot_spec(tok, &out_spec->jig_hold_ms);
        } else if (picofw_parse_u32_strict(tok, &parsed_u32)) {
            out_spec->baud = picofw_parse_uint32_clamped(
                tok, default_baud, PICOFW_UART_MIN_BAUD, PICOFW_UART_MAX_BAUD
            );
        }
    }
}

bool picofw_parse_waittx_args(const char *arg, picofw_waittx_spec_t *out) {
    char tok_timeout[24];
    char tok_mode[24];
    char tok_tx[(PICOFW_UART_HEX_MAX_BYTES * 2u) + 8u];
    char tok_post[24];
    const char *cur = arg;

    if (!arg || !out) {
        return false;
    }
    memset(out, 0, sizeof(*out));

    cur = picofw_next_token(cur, tok_timeout, sizeof(tok_timeout));
    cur = picofw_next_token(cur, tok_mode, sizeof(tok_mode));
    cur = picofw_next_token(cur, out->trigger, sizeof(out->trigger));
    cur = picofw_next_token(cur, tok_tx, sizeof(tok_tx));
    if (!cur || tok_timeout[0] == '\0' || out->trigger[0] == '\0' || tok_tx[0] == '\0') {
        return false;
    }

    out->timeout_ms = picofw_parse_uint32_clamped(tok_timeout, 15000u, 500u, 120000u);
    out->boot_action = picofw_parse_boot_spec(tok_mode, &out->jig_hold_ms);
    if (picofw_ascii_ieq(tok_tx, "BRK") || picofw_ascii_ieq(tok_tx, "BREAK")) {
        out->tx_break = true;
    } else if (!picofw_decode_hex_bytes(
                   tok_tx, out->tx_bytes, sizeof(out->tx_bytes), &out->tx_len
               )) {
        return false;
    }

    if (picofw_next_token(cur, tok_post, sizeof(tok_post)) != NULL) {
        out->post_rx_ms = picofw_parse_uint32_clamped(tok_post, 4000u, 100u, 60000u);
    } else {
        out->post_rx_ms = 4000u;
    }
    return true;
}

bool picofw_parse_jigpin_arg(const char *arg, picofw_jigpin_mode_t *out_mode) {
    char tok[24] = {0};

    if (out_mode == NULL) {
        return false;
    }
    if (picofw_next_token(arg, tok, sizeof(tok)) == NULL || picofw_ascii_ieq(tok, "STATUS")) {
        *out_mode = PICOFW_JIGPIN_STATUS;
        return true;
    }
    if (picofw_ascii_ieq(tok, "LOW") || picofw_ascii_ieq(tok, "ON") ||
        picofw_ascii_ieq(tok, "GND") || picofw_ascii_ieq(tok, "1")) {
        *out_mode = PICOFW_JIGPIN_LOW;
        return true;
    }
    if (picofw_ascii_ieq(tok, "HIZ") || picofw_ascii_ieq(tok, "OFF") ||
        picofw_ascii_ieq(tok, "RELEASE") || picofw_ascii_ieq(tok, "0")) {
        *out_mode = PICOFW_JIGPIN_HIZ;
        return true;
    }
    return false;
}
