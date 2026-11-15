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
#include "flash/p7501_flash.h"
#include "common/p7501_hex.h"

#include <stdio.h>
#include <string.h>

static bool parse_mem_spec(const char *spec_text, char opcode, char *out_payload, size_t cap) {
    const char *comma = strchr(spec_text, ',');
    if (comma == NULL || comma == spec_text || comma[1] == '\0') {
        return false;
    }
    const char *addr_start = spec_text;
    if (addr_start[0] == '0' && (addr_start[1] == 'x' || addr_start[1] == 'X')) {
        addr_start += 2;
    }
    size_t addr_len = (size_t)(comma - addr_start);
    if (addr_len == 0u || addr_len > 8u) {
        return false;
    }
    for (size_t i = 0u; i < addr_len; ++i) {
        if (p7501_hex_char_to_nibble(addr_start[i]) < 0) {
            return false;
        }
    }

    const char *len_start = comma + 1;
    if (len_start[0] == '0' && (len_start[1] == 'x' || len_start[1] == 'X')) {
        len_start += 2;
    }
    size_t len_chars = strlen(len_start);
    if (len_chars == 0u || len_chars > 4u) {
        return false;
    }
    for (size_t i = 0u; i < len_chars; ++i) {
        if (p7501_hex_char_to_nibble(len_start[i]) < 0) {
            return false;
        }
    }

    char addr_buf[12];
    memcpy(addr_buf, addr_start, addr_len);
    addr_buf[addr_len] = '\0';
    int n = snprintf(out_payload, cap, "%c%s,%s", opcode, addr_buf, len_start);
    return n > 0 && (size_t)n < cap;
}

static bool assign_write_u32_payload(
    uint32_t dest_addr,
    uint32_t value_le,
    p7501_pana_rsp_spec_t *out
) {
    if (p7501_flash_is_protected_ipl_range(dest_addr, 4u)) {
        return false;
    }
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
    if (p7501_ascii_ieq(tok, "PROBESEQ") || p7501_ascii_ieq(tok, "PROBE") ||
        p7501_ascii_ieq(tok, "PING_STUB")) {
        out->action = PANA_RSP_ACT_SEQ;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_ieq(tok, "G")) {
        out->action = PANA_RSP_ACT_G;
        snprintf(out->payload, sizeof(out->payload), "g");
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_ieq(tok, "REGS")) {
        out->action = PANA_RSP_ACT_REGS;
        snprintf(out->payload, sizeof(out->payload), "g");
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_ieq(tok, "Z0")) {
        out->action = PANA_RSP_ACT_Z0;
        snprintf(out->payload, sizeof(out->payload), "Z0");
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_ieq(tok, "Z1")) {
        out->action = PANA_RSP_ACT_Z1;
        snprintf(out->payload, sizeof(out->payload), "Z1");
        *saw_action = true;
        return true;
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
    if (p7501_ascii_iprefix(tok, "WMEM:")) {
        const char *spec = tok + 5u;
        const char *comma = strchr(spec, ',');
        if (comma == NULL || comma == spec || comma[1] == '\0') {
            return false;
        }
        uint32_t addr = 0u;
        size_t data_len = 0u;
        if (!p7501_rsp_parse_hex_u32_span(spec, (size_t)(comma - spec), &addr) ||
            !p7501_decode_hex_bytes(comma + 1, out->data_bytes, sizeof(out->data_bytes), &data_len) ||
            data_len == 0u ||
            p7501_flash_is_protected_ipl_range(addr, data_len) ||
            !p7501_rsp_build_write_mem_payload(addr, out->data_bytes, data_len, out->payload, sizeof(out->payload))) {
            return false;
        }
        out->addr = addr;
        out->data_len = data_len;
        out->action = PANA_RSP_ACT_WMEM;
        *saw_action = true;
        return true;
    }
    if (p7501_ascii_iprefix(tok, "WRITE32:")) {
        const char *spec = tok + 8u;
        const char *comma = strchr(spec, ',');
        if (comma == NULL || comma == spec || comma[1] == '\0') {
            return false;
        }
        uint32_t addr = 0u;
        uint32_t val = 0u;
        if (!p7501_rsp_parse_hex_u32_span(spec, (size_t)(comma - spec), &addr) ||
            !p7501_rsp_parse_hex_u32(comma + 1, 10u, &val) ||
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
    uint8_t dumpnor_argc = 0u;

    while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
        if (tok[0] == '\0') {
            continue;
        }
        uint32_t dump_hex = 0u;
        if (saw_action && (out->action == PANA_RSP_ACT_DUMPNOR) && (dumpnor_argc < 2u) &&
            p7501_dumpnor_parse_hex_tok(tok, &dump_hex)) {
            if (dumpnor_argc == 0u) {
                out->addr = dump_hex;
            } else {
                out->value_u32 = dump_hex;
            }
            dumpnor_argc += 1u;
        } else if (p7501_ascii_is_digits(tok)) {
            out->timeout_ms = p7501_parse_uint32_clamped(tok, 4000u, 150u, 900000u);
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

    if (!saw_action) {
        if (explicit_nak_only) {
            out->action = PANA_RSP_ACT_NAK;
        } else if (explicit_magic_only && !out->send_nak_prefix) {
            out->action = PANA_RSP_ACT_MAGIC;
        }
    }
    if ((out->action == PANA_RSP_ACT_DUMPNOR) &&
        !p7501_dumpnor_validate_range(out->addr, out->value_u32)) {
        return false;
    }
    return true;
}
