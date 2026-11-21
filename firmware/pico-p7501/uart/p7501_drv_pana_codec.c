/**
 * @file p7501_drv_pana_codec.c
 * @brief Pure ISO C11 parser and formatter for Matshita R6-MV4 Drive Factory CLI (`DRVPANA`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_drv_pana_codec.h"
#include "common/p7501_hex.h"

#include <stdio.h>
#include <string.h>

bool p7501_drv_pana_parse_args(const char *arg, p7501_drv_pana_spec_t *out) {
    if ((arg == NULL) || (out == NULL)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->timeout_ms = 3000u;
    out->boot_action = P7501_BOOT_ACTION_PASSIVE;
    out->send_pass = true;

    bool explicit_nopass = false;
    bool saw_boot = false;
    const char *cur = arg;

    while (*cur != '\0') {
        while ((*cur == ' ') || (*cur == '\t') || (*cur == '\r') || (*cur == '\n')) {
            ++cur;
        }
        if (*cur == '\0') {
            break;
        }
        char tok[48];
        const char *after = p7501_next_token(cur, tok, sizeof(tok));
        if ((after == NULL) || (tok[0] == '\0')) {
            break;
        }
        if (p7501_ascii_is_digits(tok)) {
            out->timeout_ms = p7501_parse_uint32_clamped(tok, 3000u, 150u, 120000u);
            cur = after;
        } else if (!saw_boot && p7501_is_boot_spec_token(tok)) {
            out->boot_action = p7501_parse_boot_spec(tok, &out->jig_hold_ms);
            saw_boot = true;
            cur = after;
        } else if (p7501_ascii_ieq(tok, "NOPASS")) {
            explicit_nopass = true;
            cur = after;
        } else {
            break;
        }
    }

    size_t clen = strlen(cur);
    while ((clen > 0u) && ((cur[clen - 1u] == ' ') || (cur[clen - 1u] == '\t') ||
                           (cur[clen - 1u] == '\r') || (cur[clen - 1u] == '\n'))) {
        --clen;
    }
    if ((clen >= 2u) && (cur[0] == '"') && (cur[clen - 1u] == '"')) {
        ++cur;
        clen -= 2u;
    }
    if ((clen == 0u) || (clen >= sizeof(out->cmd))) {
        return false;
    }
    memcpy(out->cmd, cur, clen);
    out->cmd[clen] = '\0';
    out->diag_macro = p7501_ascii_ieq(out->cmd, "DIAG");

    const bool starts_with_pass = p7501_ascii_ieq(out->cmd, "PASS") ||
                                  p7501_ascii_iprefix(out->cmd, "PASS ");
    out->send_pass = (!explicit_nopass) && (!starts_with_pass);
    return true;
}

size_t p7501_drv_pana_format_cmd(const p7501_drv_pana_spec_t *spec, char *out, size_t cap) {
    if ((spec == NULL) || (out == NULL) || (spec->cmd[0] == '\0') || (cap < 4u)) {
        return 0u;
    }
    const int n = snprintf(out, cap, "%s\r\n", spec->cmd);
    if ((n <= 0) || ((size_t)n >= cap)) {
        return 0u;
    }
    return (size_t)n;
}
