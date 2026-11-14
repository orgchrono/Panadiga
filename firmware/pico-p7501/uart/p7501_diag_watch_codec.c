/**
 * @file p7501_diag_watch_codec.c
 * @brief Portable ISO C11 parser and signature classifier for `DIAGWATCH`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_diag_watch_codec.h"

#include "common/p7501_hex.h"

#include <string.h>

typedef struct diag_sig_rule {
    const char *needle;
    size_t len;
    uint32_t flag;
} diag_sig_rule_t;

static const diag_sig_rule_t k_diag_rules[] = {
    { "DDR CHECK: OK",          13u, P7501_DIAG_EVT_DDR_OK },
    { "serial boot mode",       16u, P7501_DIAG_EVT_SERIAL_BOOT },
    { "-91006",                  6u, P7501_DIAG_EVT_SD_ERR },
    { "-90002",                  6u, P7501_DIAG_EVT_SD_ERR },
    { "** open tray **",        15u, P7501_DIAG_EVT_ODD_OPEN },
    { "80040904",                8u, P7501_DIAG_EVT_CD_APC_ERR },
    { "83A7",                    4u, P7501_DIAG_EVT_CD_APC_ERR },
    { "BGO driver init failed", 22u, P7501_DIAG_EVT_BGO_FAIL },
    { "hung-up time changed",   20u, P7501_DIAG_EVT_HUNG_U99 },
    { "PANAMON1>",               9u, P7501_DIAG_EVT_PANAMON },
};

static bool is_digits_tok(const char *s) {
    if ((s == NULL) || (s[0] == '\0')) {
        return false;
    }
    for (size_t i = 0u; s[i] != '\0'; ++i) {
        if ((s[i] < '0') || (s[i] > '9')) {
            return false;
        }
    }
    return true;
}

static bool has_ci_prefix(const char *s, const char *p) {
    for (size_t i = 0u; p[i] != '\0'; ++i) {
        char a = s[i];
        char b = p[i];
        if ((a >= 'a') && (a <= 'z')) {
            a = (char)(a - ('a' - 'A'));
        }
        if ((b >= 'a') && (b <= 'z')) {
            b = (char)(b - ('a' - 'A'));
        }
        if ((a == '\0') || (a != b)) {
            return false;
        }
    }
    return true;
}

bool p7501_diag_watch_parse_args(const char *arg, p7501_diag_watch_spec_t *out) {
    if (out == NULL) {
        return false;
    }
    out->timeout_ms = 15000u;
    out->boot_action = P7501_BOOT_ACTION_JIG;
    out->jig_hold_ms = 2500u;
    out->action = P7501_DIAG_ACT_AUTO;
    out->release_on_ddr = true;

    if ((arg == NULL) || (arg[0] == '\0')) {
        return true;
    }

    const char *cur = arg;
    char tok[48];
    bool saw_boot = false;
    bool saw_action = false;

    while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
        if (tok[0] == '\0') {
            continue;
        }
        if (is_digits_tok(tok)) {
            out->timeout_ms = p7501_parse_uint32_clamped(tok, 15000u, 1000u, 900000u);
        } else if (!saw_boot && (p7501_ascii_ieq(tok, "NORMAL") || p7501_ascii_ieq(tok, "BOOT") ||
                                 p7501_ascii_ieq(tok, "NOPULL") || p7501_ascii_ieq(tok, "PASSIVE") ||
                                 p7501_ascii_ieq(tok, "JIG") || has_ci_prefix(tok, "JIG:"))) {
            out->boot_action = p7501_parse_boot_spec(tok, &out->jig_hold_ms);
            saw_boot = true;
        } else if (!saw_action && p7501_ascii_ieq(tok, "AUTO")) {
            out->action = P7501_DIAG_ACT_AUTO;
            saw_action = true;
        } else if (!saw_action && p7501_ascii_ieq(tok, "BYPASS")) {
            out->action = P7501_DIAG_ACT_BYPASS;
            saw_action = true;
        } else if (!saw_action && p7501_ascii_ieq(tok, "FORMAT")) {
            out->action = P7501_DIAG_ACT_FORMAT;
            saw_action = true;
        } else if (!saw_action && p7501_ascii_ieq(tok, "DUMP")) {
            out->action = P7501_DIAG_ACT_DUMP;
            saw_action = true;
        } else if (!saw_action && p7501_ascii_ieq(tok, "NONE")) {
            out->action = P7501_DIAG_ACT_NONE;
            saw_action = true;
        } else if (p7501_ascii_ieq(tok, "RELEASE")) {
            out->release_on_ddr = true;
        } else if (p7501_ascii_ieq(tok, "HOLD")) {
            out->release_on_ddr = false;
        } else {
            return false;
        }
    }
    return true;
}

void p7501_diag_watch_init(p7501_diag_watch_state_t *st) {
    if (st != NULL) {
        memset(st, 0, sizeof(*st));
    }
}

char p7501_diag_watch_action_char(p7501_diag_action_t action) {
    switch (action) {
        case P7501_DIAG_ACT_AUTO:
        case P7501_DIAG_ACT_BYPASS:
            return 'B';
        case P7501_DIAG_ACT_FORMAT:
            return 'F';
        case P7501_DIAG_ACT_DUMP:
            return 'D';
        case P7501_DIAG_ACT_NONE:
        default:
            return '\0';
    }
}

uint32_t p7501_diag_watch_feed_byte(
    p7501_diag_watch_state_t *st,
    const p7501_diag_watch_spec_t *spec,
    uint8_t byte,
    char *out_tx_char,
    bool *out_release_sw_uart
) {
    if (out_tx_char != NULL) {
        *out_tx_char = '\0';
    }
    if (out_release_sw_uart != NULL) {
        *out_release_sw_uart = false;
    }
    if ((st == NULL) || (spec == NULL)) {
        return 0u;
    }

    const size_t cap = sizeof(st->ring);
    if (st->ring_len < cap) {
        st->ring[st->ring_len++] = byte;
    } else {
        memmove(&st->ring[0], &st->ring[1], cap - 1u);
        st->ring[cap - 1u] = byte;
    }

    uint32_t newly_hit = 0u;
    for (size_t i = 0u; i < (sizeof(k_diag_rules) / sizeof(k_diag_rules[0])); ++i) {
        const size_t nlen = k_diag_rules[i].len;
        if (st->ring_len >= nlen &&
            memcmp(&st->ring[st->ring_len - nlen], k_diag_rules[i].needle, nlen) == 0) {
            const uint32_t flag = k_diag_rules[i].flag;
            if ((st->seen_events & flag) == 0u) {
                st->seen_events |= flag;
                newly_hit |= flag;
            }
        }
    }

    const uint32_t release_mask = (spec->boot_action == P7501_BOOT_ACTION_JIG)
        ? (P7501_DIAG_EVT_SERIAL_BOOT | P7501_DIAG_EVT_BGO_FAIL | P7501_DIAG_EVT_HUNG_U99)
        : P7501_DIAG_EVT_DDR_OK;

    if ((newly_hit & release_mask) != 0u &&
        spec->release_on_ddr && !st->sw_uart_released) {
        st->sw_uart_released = true;
        if (out_release_sw_uart != NULL) {
            *out_release_sw_uart = true;
        }
    }

    if ((newly_hit & P7501_DIAG_EVT_PANAMON) != 0u && !st->tx_dispatched) {
        const char ch = p7501_diag_watch_action_char(spec->action);
        if (ch != '\0') {
            st->tx_dispatched = true;
            if (out_tx_char != NULL) {
                *out_tx_char = ch;
            }
        }
    }
    return newly_hit;
}

const char *p7501_diag_watch_diagnosis(uint32_t events) {
    if ((events & P7501_DIAG_EVT_PANAMON) != 0u) {
        return "PANAMON1 active on SC0 (ready for B=bypass, F=nfrom_format, D=dump_nor)";
    }
    if ((events & P7501_DIAG_EVT_BGO_FAIL) != 0u && (events & P7501_DIAG_EVT_SERIAL_BOOT) == 0u) {
        if ((events & P7501_DIAG_EVT_CD_APC_ERR) != 0u) {
            return "BGO driver init failed[1] -> U99 hang (normal mode; ODD 80040904 also observed)";
        }
        return ((events & P7501_DIAG_EVT_HUNG_U99) != 0u)
            ? "BGO driver init failed[1] -> U99 hang (load patched DIGADIAG.FRM)"
            : "BGO driver init failed[1] (corrupt/erased BGO partition)";
    }
    if (((events & P7501_DIAG_EVT_SD_ERR) != 0u) && ((events & P7501_DIAG_EVT_CD_APC_ERR) != 0u)) {
        return "SDDIAG -91006 + CDDIAG 80040904 (SDSC 4-bit failed; ODD tray/laser APC failed)";
    }
    if ((events & P7501_DIAG_EVT_SD_ERR) != 0u) {
        return "SDDIAG -91006 (4-bit SD_STATUS mismatch)";
    }
    if ((events & P7501_DIAG_EVT_CD_APC_ERR) != 0u) {
        return "CDDIAG 80040904 / 0x83A7 (Laser APC < 0.5mW)";
    }
    if ((events & P7501_DIAG_EVT_BGO_FAIL) != 0u) {
        return ((events & P7501_DIAG_EVT_HUNG_U99) != 0u)
            ? "BGO driver init failed[1] -> U99 hang (load patched DIGADIAG.FRM)"
            : "BGO driver init failed[1] (corrupt/erased BGO partition)";
    }
    if ((events & P7501_DIAG_EVT_ODD_OPEN) != 0u) {
        return "Serial boot: ODD ** open tray ** state reported";
    }
    if ((events & P7501_DIAG_EVT_SERIAL_BOOT) != 0u) {
        return "Serial boot mode entered (waiting SD/CD media)";
    }
    if ((events & P7501_DIAG_EVT_DDR_OK) != 0u) {
        return "DDR CHECK: OK seen; normal boot progress";
    }
    return "No known Panasonic boot signatures observed";
}
