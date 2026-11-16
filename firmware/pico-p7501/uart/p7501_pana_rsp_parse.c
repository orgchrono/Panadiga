/**
 * @file p7501_pana_rsp_parse.c
 * @brief Pure ISO C11 `PANARSP` CLI argument tokenizer and action specification parser.
 *
 * Compiled identically on Pico ARM/RISC-V and host C11 test targets (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_rsp_internal.h"
#include "uart/p7501_pana_dumpnor.h"
#include "uart/p7501_pana_jedec.h"
#include "flash/p7501_flash.h"
#include "common/p7501_hex.h"
#include "common/p7501_wdt_logic.h"

#include <stdio.h>
#include <string.h>

static bool parse_addr_comma_tail(const char *spec, uint32_t *out_addr, const char **out_tail) {
    const char *comma = strchr(spec, ',');
    if (comma == NULL || comma == spec || comma[1] == '\0') {
        return false;
    }
    if (!p7501_rsp_parse_hex_u32_span(spec, (size_t)(comma - spec), out_addr)) {
        return false;
    }
    *out_tail = comma + 1;
    return true;
}

static bool parse_mem_spec(const char *spec_text, char opcode, char *out_payload, size_t cap) {
    uint32_t addr = 0u;
    const char *tail = NULL;
    uint32_t len = 0u;
    if (!parse_addr_comma_tail(spec_text, &addr, &tail) ||
        !p7501_rsp_parse_hex_u32(tail, 6u, &len) || len == 0u || len > 0xFFFFu) {
        return false;
    }
    const char *len_start = (tail[0] == '0' && (tail[1] == 'x' || tail[1] == 'X')) ? (tail + 2) : tail;
    int n = snprintf(out_payload, cap, "%c%X,%s", opcode, (unsigned)addr, len_start);
    return n > 0 && (size_t)n < cap;
}

static bool assign_write_u32_payload(
    uint32_t dest_addr,
    uint32_t value_le,
    p7501_pana_rsp_spec_t *out
) {
    uint8_t le[4];
    p7501_rsp_encode_u32_le(value_le, le);
    if (!p7501_rsp_build_write_mem_payload(dest_addr, le, sizeof(le), out->payload, sizeof(out->payload))) {
        return false;
    }
    out->value_u32 = value_le;
    memcpy(out->data_bytes, le, sizeof(le));
    out->data_len = sizeof(le);
    return true;
}

static bool apply_panarsp_action_token(const char *tok, p7501_pana_rsp_spec_t *out, bool *saw_action) {
    static const struct {
        const char *name;
        p7501_pana_rsp_action_t act;
        const char *payload;
    } k_simple[] = {
        { "PROBESEQ", PANA_RSP_ACT_SEQ, "" },
        { "PROBE", PANA_RSP_ACT_SEQ, "" },
        { "PING_STUB", PANA_RSP_ACT_SEQ, "" },
        { "G", PANA_RSP_ACT_G, "g" },
        { "REGS", PANA_RSP_ACT_REGS, "g" },
        { "Z0", PANA_RSP_ACT_Z0, "Z0" },
        { "Z1", PANA_RSP_ACT_Z1, "Z1" },
    };
    for (size_t i = 0u; i < (sizeof(k_simple) / sizeof(k_simple[0])); ++i) {
        if (p7501_ascii_ieq(tok, k_simple[i].name)) {
            out->action = k_simple[i].act;
            if (k_simple[i].payload[0] != '\0') {
                snprintf(out->payload, sizeof(out->payload), "%s", k_simple[i].payload);
            }
            *saw_action = true;
            return true;
        }
    }
    if (p7501_ascii_iprefix(tok, "MEM:") || p7501_ascii_iprefix(tok, "XMEM:")) {
        const bool is_x = (tok[0] == 'X' || tok[0] == 'x');
        const size_t prefix_len = is_x ? 5u : 4u;
        if (!parse_mem_spec(tok + prefix_len, is_x ? 'x' : 'm', out->payload, sizeof(out->payload))) {
            return false;
        }
        out->action = is_x ? PANA_RSP_ACT_XMEM : PANA_RSP_ACT_MEM;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_ieq(tok, "PASSTHROUGH")) {
        out->action = PANA_RSP_ACT_PASSTHROUGH;
        out->send_magic_prefix = true;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "WMEM:") || p7501_ascii_iprefix(tok, "JEDEC_PROG:")) {
        const bool is_jedec = (tok[0] == 'J' || tok[0] == 'j');
        const char *spec = tok + (is_jedec ? 11u : 5u);
        const char *tail = NULL;
        uint32_t addr = 0u;
        size_t data_len = 0u;
        if (!parse_addr_comma_tail(spec, &addr, &tail) ||
            !p7501_decode_hex_bytes(tail, out->data_bytes, sizeof(out->data_bytes), &data_len) ||
            data_len == 0u) {
            return false;
        }
        if (!is_jedec &&
            !p7501_rsp_build_write_mem_payload(addr, out->data_bytes, data_len, out->payload, sizeof(out->payload))) {
            return false;
        }
        out->addr = addr;
        out->data_len = data_len;
        out->send_magic_prefix = is_jedec;
        out->action = is_jedec ? PANA_RSP_ACT_JEDEC_PROG : PANA_RSP_ACT_WMEM;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "JEDEC_ERASE:") && tok[12] != '\0') {
        uint32_t addr = 0u;
        if (!p7501_rsp_parse_hex_u32(tok + 12u, 10u, &addr)) {
            return false;
        }
        out->addr = addr;
        out->data_len = 1u;
        out->send_magic_prefix = true;
        out->action = PANA_RSP_ACT_JEDEC_ERASE;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "WRITE32:")) {
        const char *tail = NULL;
        uint32_t addr = 0u;
        uint32_t val = 0u;
        if (!parse_addr_comma_tail(tok + 8u, &addr, &tail) ||
            !p7501_rsp_parse_hex_u32(tail, 10u, &val) ||
            !assign_write_u32_payload(addr, val, out)) {
            return false;
        }
        out->addr = addr;
        out->action = PANA_RSP_ACT_WRITE32;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "CALL:") && tok[5] != '\0') {
        uint32_t target = 0u;
        if (!p7501_rsp_parse_hex_u32(tok + 5u, 10u, &target) ||
            !assign_write_u32_payload((uint32_t)P7501_AM34_SERIAL_CB_PTR_ADDR, target, out)) {
            return false;
        }
        out->addr = target;
        out->send_magic_prefix = true;
        out->action = PANA_RSP_ACT_CALL;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_ieq(tok, "DUMPNOR")) {
        out->action = PANA_RSP_ACT_DUMPNOR;
        out->send_magic_prefix = true;
        out->addr = P7501_DUMPNOR_DEFAULT_START;
        out->value_u32 = P7501_DUMPNOR_DEFAULT_LEN;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "DUMPNOR:")) {
        if (!p7501_dumpnor_parse_colon_spec(tok + 8u, &out->addr, &out->value_u32)) {
            return false;
        }
        out->action = PANA_RSP_ACT_DUMPNOR;
        out->send_magic_prefix = true;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "RAW:") && tok[4] != '\0') {
        out->action = PANA_RSP_ACT_RAW;
        snprintf(out->payload, sizeof(out->payload), "%s", tok + 4);
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "HEX:") && tok[4] != '\0') {
        if (!p7501_decode_hex_bytes(tok + 4, out->raw_bytes, sizeof(out->raw_bytes), &out->raw_len)) {
            return false;
        }
        out->action = PANA_RSP_ACT_HEX;
        *saw_action = true;
        return true;
    }
    return false;
}

bool p7501_panarsp_parse_args(const char *arg, p7501_pana_rsp_spec_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->timeout_ms = 4000u;
    out->boot_action = P7501_BOOT_ACTION_PASSIVE;
    out->send_nak_prefix = true;
    out->action = PANA_RSP_ACT_SEQ;

    if (arg == NULL || arg[0] == '\0') {
        return true;
    }

    const char *cur = arg;
    char tok[144];
    bool saw_boot = false;
    bool saw_action = false;
    bool explicit_nak_only = false;
    bool explicit_magic_only = false;
    int policy_override = -1;
    uint8_t dumpnor_argc = 0u;

    while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
        if (tok[0] == '\0') {
            continue;
        }
        uint32_t dump_hex = 0u;
        bool bad_pol = false;
        if (saw_action && (out->action == PANA_RSP_ACT_DUMPNOR) && (dumpnor_argc < 2u) &&
            p7501_dumpnor_parse_hex_tok(tok, &dump_hex)) {
            if (dumpnor_argc == 0u) {
                out->addr = dump_hex;
            } else {
                out->value_u32 = dump_hex;
            }
            dumpnor_argc += 1u;
        } else if (p7501_flash_consume_policy_tok(tok, &policy_override, &bad_pol)) {
            if (bad_pol) return false;
        } else if (p7501_ascii_is_digits(tok)) {
            out->timeout_ms = p7501_parse_uint32_clamped(
                tok,
                4000u,
                150u,
                P7501_WDT_MAX_CMD_WINDOW_MS
            );
        } else if (p7501_ascii_ieq(tok, "MAGIC")) {
            out->send_magic_prefix = true;
            explicit_magic_only = true;
        } else if (p7501_ascii_ieq(tok, "NONAK")) {
            out->send_nak_prefix = false;
        } else if (p7501_ascii_ieq(tok, "NAK")) {
            out->send_nak_prefix = true;
            explicit_nak_only = true;
        } else if (p7501_ascii_iprefix(tok, "WAIT:") && tok[5] != '\0') {
            out->wait_trigger_enabled = true;
            snprintf(out->trigger, sizeof(out->trigger), "%s", tok + 5);
        } else if (!saw_boot && p7501_is_boot_spec_token(tok)) {
            out->boot_action = p7501_parse_boot_spec(tok, &out->jig_hold_ms);
            saw_boot = true;
        } else if (!apply_panarsp_action_token(tok, out, &saw_action)) {
            return false;
        }
    }

    const bool eff_unlocked = p7501_flash_effective_policy(policy_override);
    out->nor_unlocked_effective = eff_unlocked;

    if (!saw_action) {
        if (policy_override != -1) {
            out->action = PANA_RSP_ACT_NOR_POLICY;
            out->send_nak_prefix = false;
        } else if (explicit_nak_only) {
            out->action = PANA_RSP_ACT_NAK;
        } else if (explicit_magic_only && !out->send_nak_prefix) {
            out->action = PANA_RSP_ACT_MAGIC;
        }
    }
    if ((out->action == PANA_RSP_ACT_WMEM || out->action == PANA_RSP_ACT_WRITE32) &&
        p7501_flash_is_write_blocked_with_policy(out->addr, out->data_len, eff_unlocked)) {
        return false;
    }
    if ((out->action == PANA_RSP_ACT_JEDEC_ERASE) &&
        !p7501_jedec_validate_erase_addr(out->addr, eff_unlocked, NULL)) {
        return false;
    }
    if ((out->action == PANA_RSP_ACT_JEDEC_PROG) &&
        !p7501_jedec_validate_prog_range(out->addr, out->data_len, eff_unlocked, NULL)) {
        return false;
    }
    if ((out->action == PANA_RSP_ACT_DUMPNOR) &&
        !p7501_dumpnor_validate_range(out->addr, out->value_u32)) {
        return false;
    }
    if (policy_override != -1) {
        p7501_flash_set_upper_nor_unlocked(policy_override == 1);
    }
    return true;
}
