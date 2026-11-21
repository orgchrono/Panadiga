/**
 * @file p7501_diag_watch_codec.c
 * @brief Portable ISO C11 argument parser and TX formatter for `DIAGWATCH`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_diag_watch_codec.h"
#include "uart/p7501_pana_rsp_internal.h"
#include "flash/p7501_flash.h"
#include "common/p7501_hex.h"
#include "common/p7501_wdt_logic.h"

#include <stdio.h>
#include <string.h>

static bool parse_diag_single_addr_param(
    const char *spec,
    char opcode,
    p7501_diag_watch_spec_t *out
) {
    uint32_t addr = 0u;
    if (!p7501_rsp_parse_hex_u32(spec, 10u, &addr) ||
        p7501_bus_is_lethal_mmio_range(addr, 4u)) {
        return false;
    }
    out->target_addr = addr;
    const int n = snprintf(out->tx_seq, sizeof(out->tx_seq), "%c%X\r", opcode, (unsigned)addr);
    return (n > 0) && ((size_t)n < sizeof(out->tx_seq));
}

static bool parse_diag_read_param(const char *spec, p7501_diag_watch_spec_t *out) {
    const char *comma = strchr(spec, ',');
    if (comma == NULL) {
        return parse_diag_single_addr_param(spec, 'R', out);
    }
    if ((comma == spec) || (comma[1] == '\0')) {
        return false;
    }
    uint32_t addr = 0u;
    uint32_t len = 0u;
    if (!p7501_rsp_parse_hex_u32_span(spec, (size_t)(comma - spec), &addr) ||
        !p7501_rsp_parse_hex_u32(comma + 1, 6u, &len) ||
        (len == 0u) || (len > 0xFFFFu) ||
        p7501_bus_is_lethal_mmio_range(addr, (size_t)len)) {
        return false;
    }
    out->target_addr = addr;
    const int n = snprintf(out->tx_seq, sizeof(out->tx_seq), "R%X,%X\r", (unsigned)addr, (unsigned)len);
    return (n > 0) && ((size_t)n < sizeof(out->tx_seq));
}

static bool parse_diag_write_param(const char *spec, p7501_diag_watch_spec_t *out) {
    const char *comma = strchr(spec, ',');
    if ((comma == NULL) || (comma == spec) || (comma[1] == '\0')) {
        return false;
    }
    uint32_t addr = 0u;
    uint32_t val = 0u;
    if (!p7501_rsp_parse_hex_u32_span(spec, (size_t)(comma - spec), &addr) ||
        !p7501_rsp_parse_hex_u32(comma + 1, 4u, &val) ||
        (val > 0xFFu) ||
        p7501_bus_is_lethal_mmio_range(addr, 1u)) {
        return false;
    }
    out->target_addr = addr;
    const int n = snprintf(out->tx_seq, sizeof(out->tx_seq), "W%X,%02X\r", (unsigned)addr, (unsigned)val);
    return (n > 0) && ((size_t)n < sizeof(out->tx_seq));
}

static bool parse_diag_action_token(
    const char *tok,
    p7501_diag_watch_spec_t *out,
    bool *saw_action
) {
    static const struct {
        const char *name;
        p7501_diag_action_t action;
    } k_simple_actions[] = {
        {"AUTO", P7501_DIAG_ACT_AUTO},
        {"BYPASS", P7501_DIAG_ACT_BYPASS},
        {"FORMAT", P7501_DIAG_ACT_FORMAT},
        {"DUMP", P7501_DIAG_ACT_DUMP},
        {"READ", P7501_DIAG_ACT_READ},
        {"WRITE", P7501_DIAG_ACT_WRITE},
        {"JUMP", P7501_DIAG_ACT_JUMP},
        {"ERASE", P7501_DIAG_ACT_ERASE},
        {"QUIT", P7501_DIAG_ACT_QUIT},
        {"NONE", P7501_DIAG_ACT_NONE},
    };
    if (*saw_action) {
        return false;
    }
    for (size_t i = 0u; i < (sizeof(k_simple_actions) / sizeof(k_simple_actions[0])); ++i) {
        if (p7501_ascii_ieq(tok, k_simple_actions[i].name)) {
            out->action = k_simple_actions[i].action;
            *saw_action = true;
            return true;
        }
    }
    if (p7501_ascii_iprefix(tok, "READ:") && (tok[5] != '\0')) {
        if (!parse_diag_read_param(tok + 5u, out)) return false;
        out->action = P7501_DIAG_ACT_READ;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "WRITE:") && (tok[6] != '\0')) {
        if (!parse_diag_write_param(tok + 6u, out)) return false;
        out->action = P7501_DIAG_ACT_WRITE;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "JUMP:") && (tok[5] != '\0')) {
        if (!parse_diag_single_addr_param(tok + 5u, 'J', out)) return false;
        out->action = P7501_DIAG_ACT_JUMP;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "ERASE:") && (tok[6] != '\0')) {
        if (!parse_diag_single_addr_param(tok + 6u, 'E', out)) return false;
        out->action = P7501_DIAG_ACT_ERASE;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "PANAMON_TXN:") && (tok[12] != '\0')) {
        const char *p = tok + 12u;
        out->txn_count = 0u;
        while (*p != '\0') {
            if (out->txn_count >= P7501_DIAG_TXN_MAX_STEPS) {
                return false;
            }
            const char *semi = strchr(p, ';');
            const char *end = (semi != NULL) ? semi : (p + strlen(p));
            while ((p < end) && ((*p == ' ') || (*p == '\t'))) {
                ++p;
            }
            size_t slen = (size_t)(end - p);
            while ((slen > 0u) && ((p[slen - 1u] == ' ') || (p[slen - 1u] == '\t'))) {
                --slen;
            }
            if ((slen == 0u) || (slen >= 30u)) {
                return false;
            }
            memcpy(out->txn_steps[out->txn_count], p, slen);
            out->txn_steps[out->txn_count][slen] = '\r';
            out->txn_steps[out->txn_count][slen + 1u] = '\0';
            out->txn_count += 1u;
            p = (semi != NULL) ? (semi + 1) : end;
        }
        if (out->txn_count == 0u) {
            return false;
        }
        out->action = P7501_DIAG_ACT_TXN;
        *saw_action = true;
        return true;
    }
    return false;
}

bool p7501_diag_watch_parse_args(const char *arg, p7501_diag_watch_spec_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->timeout_ms = 15000u;
    out->boot_action = P7501_BOOT_ACTION_JIG;
    out->jig_hold_ms = 2500u;
    out->action = P7501_DIAG_ACT_AUTO;
    out->release_on_ddr = true;

    if ((arg == NULL) || (arg[0] == '\0')) {
        return true;
    }

    const char *cur = arg;
    char tok[160];
    bool saw_boot = false;
    bool saw_action = false;
    int policy_override = -1;

    while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
        if (tok[0] == '\0') {
            continue;
        }
        bool bad_pol = false;
        if (p7501_flash_consume_policy_tok(tok, &policy_override, &bad_pol)) {
            if (bad_pol) return false;
            continue;
        }
        if (p7501_ascii_is_digits(tok)) {
            out->timeout_ms = p7501_parse_uint32_clamped(
                tok,
                15000u,
                1000u,
                P7501_WDT_MAX_CMD_WINDOW_MS
            );
        } else if (!saw_boot && p7501_is_boot_spec_token(tok)) {
            out->boot_action = p7501_parse_boot_spec_ex(
                tok, &out->jig_hold_ms, &out->no_xreset
            );
            saw_boot = true;
        } else if (parse_diag_action_token(tok, out, &saw_action)) {
            continue;
        } else if (p7501_ascii_ieq(tok, "RELEASE")) {
            out->release_on_ddr = true;
        } else if (p7501_ascii_ieq(tok, "HOLD")) {
            out->release_on_ddr = false;
        } else if (p7501_ascii_ieq(tok, "WAKE") || p7501_ascii_iprefix(tok, "WAKE:")) {
            if (!p7501_parse_wake_arg_token(tok, &out->wait_for_wake, &out->wake_timeout_ms)) {
                return false;
            }
        } else {
            return false;
        }
    }

    const bool eff_unlocked = p7501_flash_effective_policy(policy_override);
    const bool f14_ok = p7501_wireless_nor_write_allowed(p7501_wireless_global_state());
    out->policy_req = (policy_override == 1)
        ? P7501_POLICY_REQ_UNLOCK
        : ((policy_override == 0) ? P7501_POLICY_REQ_LOCK : P7501_POLICY_REQ_NONE);

    if (out->action == P7501_DIAG_ACT_FORMAT) {
        /* FORMAT erases BGO partition in Upper NOR -> requires explicit unlock + F14. */
        if (!eff_unlocked || !f14_ok) {
            return false;
        }
    }
    if ((out->action == P7501_DIAG_ACT_JUMP) && !f14_ok) {
        return false;
    }
    if (out->action == P7501_DIAG_ACT_WRITE) {
        if (out->tx_seq[0] == '\0') {
            if (!eff_unlocked || !f14_ok) {
                return false;
            }
        } else if (p7501_flash_is_write_blocked_with_policy(out->target_addr, 1u, eff_unlocked)) {
            return false;
        }
    }
    if (out->action == P7501_DIAG_ACT_ERASE) {
        if (out->tx_seq[0] == '\0') {
            if (!eff_unlocked || !f14_ok) {
                return false;
            }
        } else if (!p7501_flash_is_erasable_nor_addr(out->target_addr, eff_unlocked)) {
            return false;
        }
    }
    if (out->action == P7501_DIAG_ACT_TXN) {
        for (uint8_t i = 0u; i < out->txn_count; ++i) {
            const char *step = out->txn_steps[i];
            const char op = step[0];
            if ((op == 'F') || (op == 'f')) {
                if (!eff_unlocked || !f14_ok) {
                    return false;
                }
                out->nor_unlock = true;
            } else if ((op == 'W') || (op == 'w') || (op == 'E') || (op == 'e')) {
                const bool is_er = (op == 'E') || (op == 'e');
                const size_t slen_w = strlen(step);
                uint32_t saddr = 0u;
                const size_t sc = p7501_scan_hex_u32(step, slen_w, 1u, &saddr);
                if (sc == 0u) {
                    return false;
                }
                size_t wspan = 1u;
                if (!is_er && ((1u + sc) < slen_w) && (step[1u + sc] == ',')) {
                    uint32_t wval = 0u;
                    const size_t vc = p7501_scan_hex_u32(step, slen_w, 1u + sc + 1u, &wval);
                    if (vc == 0u) {
                        return false;
                    }
                    wspan = (vc > 2u) ? 4u : 1u;
                }
                if (p7501_bus_check_access(
                        p7501_session_default_ctx(),
                        p7501_flash_get_active_size(),
                        saddr,
                        wspan,
                        true,
                        is_er,
                        eff_unlocked
                    ) != P7501_BUS_ALLOW) {
                    return false;
                }
                if (p7501_flash_is_protected_ipl_range(saddr, wspan) && (policy_override == 1 || eff_unlocked)) {
                    out->nor_unlock = true;
                }
            } else if ((op == 'R') || (op == 'r') || (op == 'J') || (op == 'j')) {
                uint32_t saddr = 0u;
                const size_t slen_str = strlen(step);
                const size_t sc = p7501_scan_hex_u32(step, slen_str, 1u, &saddr);
                if (sc == 0u) {
                    return false;
                }
                uint32_t rspan = 4u;
                const size_t after = 1u + sc;
                if (((op == 'R') || (op == 'r')) && (after < slen_str) && (step[after] == ',')) {
                    uint32_t rlen = 0u;
                    if (p7501_scan_hex_u32(step, slen_str, after + 1u, &rlen) == 0u) {
                        return false;
                    }
                    if (rlen > 0u) {
                        rspan = rlen;
                    }
                }
                if (p7501_bus_is_lethal_mmio_range(saddr, (size_t)rspan)) {
                    return false;
                }
                if (((op == 'J') || (op == 'j')) && !f14_ok) {
                    return false;
                }
            }
        }
    } else {
        out->nor_unlock = (policy_override == 1) ||
            (eff_unlocked && (out->action == P7501_DIAG_ACT_FORMAT)) ||
            (eff_unlocked && (out->tx_seq[0] != '\0') &&
             ((out->action == P7501_DIAG_ACT_WRITE) || (out->action == P7501_DIAG_ACT_ERASE)) &&
             p7501_flash_is_protected_ipl_range(out->target_addr, 1u));
    }
    return true;
}

void p7501_diag_watch_apply_policy(const p7501_diag_watch_spec_t *spec) {
    if (spec != NULL) {
        p7501_flash_apply_policy_req(spec->policy_req);
    }
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
        case P7501_DIAG_ACT_READ:
            return 'R';
        case P7501_DIAG_ACT_WRITE:
            return 'W';
        case P7501_DIAG_ACT_FORMAT:
            return 'F';
        case P7501_DIAG_ACT_DUMP:
            return 'D';
        case P7501_DIAG_ACT_JUMP:
            return 'J';
        case P7501_DIAG_ACT_ERASE:
            return 'E';
        case P7501_DIAG_ACT_QUIT:
            return 'Q';
        case P7501_DIAG_ACT_TXN:
            return 'T';
        case P7501_DIAG_ACT_NONE:
        default:
            return '\0';
    }
}

size_t p7501_diag_watch_format_tx(const p7501_diag_watch_spec_t *spec, char *out, size_t cap) {
    if ((spec == NULL) || (out == NULL) || (cap < 2u)) {
        return 0u;
    }
    if (spec->tx_seq[0] != '\0') {
        const size_t n = strlen(spec->tx_seq);
        if ((n + 1u) > cap) {
            return 0u;
        }
        memcpy(out, spec->tx_seq, n + 1u);
        return n;
    }
    const char ch = p7501_diag_watch_action_char(spec->action);
    if (ch == '\0') {
        out[0] = '\0';
        return 0u;
    }
    out[0] = ch;
    out[1] = '\0';
    return 1u;
}
