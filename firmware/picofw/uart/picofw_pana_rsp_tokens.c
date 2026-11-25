/**
 * @file picofw_pana_rsp_tokens.c
 * @brief Pure ISO C11 `PANARSP` per-action token decoder (`MEM`, `WMEM`, `JEDEC_*`, `DUMPNOR`, `RAW`, `HEX`).
 *
 * Split from `picofw_pana_rsp_parse.c` (`picofw_pana_rsp_parse.c` retains option loop & policy checks).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_pana_rsp.h"
#include "uart/picofw_pana_rsp_internal.h"
#include "uart/picofw_pana_dumpnor.h"
#include "flash/picofw_flash.h"
#include "common/picofw_hex.h"

#include <stdio.h>
#include <string.h>

static bool parse_addr_comma_tail(const char *spec, uint32_t *out_addr, const char **out_tail) {
    const char *comma = strchr(spec, ',');
    if ((comma == NULL) || (comma == spec) || (comma[1] == '\0')) {
        return false;
    }
    if (!picofw_rsp_parse_hex_u32_span(spec, (size_t)(comma - spec), out_addr)) {
        return false;
    }
    *out_tail = comma + 1;
    return true;
}

static bool parse_mem_spec(
    const char *spec_text, char opcode, char *out_payload, size_t cap,
    uint32_t *out_addr, uint32_t *out_len
) {
    uint32_t addr = 0u, len = 0u;
    const char *tail = NULL;
    if (!parse_addr_comma_tail(spec_text, &addr, &tail) ||
        !picofw_rsp_parse_hex_u32(tail, 6u, &len) || (len == 0u) || (len > 0xFFFFu)) {
        return false;
    }
    const char *len_start = ((tail[0] == '0') && ((tail[1] == 'x') || (tail[1] == 'X')))
        ? (tail + 2)
        : tail;
    const int n = snprintf(out_payload, cap, "%c%X,%s", opcode, (unsigned)addr, len_start);
    if ((n <= 0) || ((size_t)n >= cap)) {
        return false;
    }
    if (out_addr != NULL) {
        *out_addr = addr;
    }
    if (out_len != NULL) {
        *out_len = len;
    }
    return true;
}

static bool assign_write_u32_payload(uint32_t dest_addr, uint32_t value_le, picofw_pana_rsp_spec_t *out) {
    uint8_t le[4];
    picofw_rsp_encode_u32_le(value_le, le);
    if (!picofw_rsp_build_write_mem_payload(dest_addr, le, sizeof(le), out->payload, sizeof(out->payload))) {
        return false;
    }
    out->value_u32 = value_le;
    memcpy(out->data_bytes, le, sizeof(le));
    out->data_len = sizeof(le);
    return true;
}

bool picofw_rsp_apply_action_token(const char *tok, picofw_pana_rsp_spec_t *out, bool *saw_action) {
    static const struct {
        const char *name;
        picofw_pana_rsp_action_t act;
        const char *payload;
    } k_simple[] = {
        { "PROBESEQ", PANA_RSP_ACT_SEQ, "" },
        { "PROBE", PANA_RSP_ACT_SEQ, "" },
        { "PING_STUB", PANA_RSP_ACT_SEQ, "" },
        { "G", PANA_RSP_ACT_G, "g" },
        { "REGS", PANA_RSP_ACT_REGS, "g" },
        { "Z0", PANA_RSP_ACT_Z0, "Z0" },
        { "Z1", PANA_RSP_ACT_Z1, "Z1" },
        { "JEDEC_ID", PANA_RSP_ACT_JEDEC_ID, "" },
        { "CFI_QUERY", PANA_RSP_ACT_CFI_QUERY, "" },
        { "DUMPNOR_RESUME", PANA_RSP_ACT_DUMPNOR_RESUME, "" },
    };
    for (size_t i = 0u; i < (sizeof(k_simple) / sizeof(k_simple[0])); ++i) {
        if (picofw_ascii_ieq(tok, k_simple[i].name)) {
            out->action = k_simple[i].act;
            if (k_simple[i].payload[0] != '\0') {
                snprintf(out->payload, sizeof(out->payload), "%s", k_simple[i].payload);
            }
            *saw_action = true;
            return true;
        }
    }
    if (picofw_ascii_iprefix(tok, "MEM:") || picofw_ascii_iprefix(tok, "XMEM:")) {
        const bool is_x = (tok[0] == 'X' || tok[0] == 'x');
        const size_t prefix_len = is_x ? 5u : 4u;
        uint32_t m_addr = 0u, m_len = 0u;
        if (!parse_mem_spec(tok + prefix_len, is_x ? 'x' : 'm', out->payload, sizeof(out->payload), &m_addr, &m_len)) {
            return false;
        }
        out->addr = m_addr;
        out->value_u32 = m_len;
        out->action = is_x ? PANA_RSP_ACT_XMEM : PANA_RSP_ACT_MEM;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_ieq(tok, "PASSTHROUGH")) {
        out->action = PANA_RSP_ACT_PASSTHROUGH;
        out->send_magic_prefix = true;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "WMEM:") || picofw_ascii_iprefix(tok, "JEDEC_PROG:") ||
        picofw_ascii_iprefix(tok, "BGO_BITCLEAR:")) {
        const bool is_jedec = picofw_ascii_iprefix(tok, "JEDEC_PROG:");
        const bool is_bitclear = picofw_ascii_iprefix(tok, "BGO_BITCLEAR:");
        const char *spec = tok + (is_bitclear ? 13u : (is_jedec ? 11u : 5u));
        const char *tail = NULL;
        uint32_t addr = 0u;
        size_t data_len = 0u;
        if (!parse_addr_comma_tail(spec, &addr, &tail) ||
            !picofw_decode_hex_bytes(tail, out->data_bytes, sizeof(out->data_bytes), &data_len) ||
            data_len == 0u) {
            return false;
        }
        if (!is_jedec && !is_bitclear &&
            !picofw_rsp_build_write_mem_payload(addr, out->data_bytes, data_len, out->payload, sizeof(out->payload))) {
            return false;
        }
        out->addr = addr;
        out->data_len = data_len;
        out->send_magic_prefix = is_jedec || is_bitclear;
        out->action = is_bitclear
            ? PANA_RSP_ACT_BGO_BITCLEAR
            : (is_jedec ? PANA_RSP_ACT_JEDEC_PROG : PANA_RSP_ACT_WMEM);
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "CRC32_NOR:") || picofw_ascii_iprefix(tok, "BLANK_CHECK:")) {
        const bool is_blank = picofw_ascii_iprefix(tok, "BLANK_CHECK:");
        const char *spec = tok + (is_blank ? 12u : 10u);
        if (!picofw_dumpnor_parse_colon_spec(spec, &out->addr, &out->value_u32)) {
            return false;
        }
        out->send_magic_prefix = true;
        out->action = is_blank ? PANA_RSP_ACT_BLANK_CHECK : PANA_RSP_ACT_CRC32_NOR;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "FAST_DUMPNOR:")) {
        const char *spec = tok + 13u;
        uint32_t req_baud = 0u;
        const char *comma = strchr(spec, ',');
        char bbuf[16];
        const size_t blen = (comma != NULL) ? (size_t)(comma - spec) : strlen(spec);
        if ((blen == 0u) || (blen >= sizeof(bbuf))) {
            return false;
        }
        memcpy(bbuf, spec, blen);
        bbuf[blen] = '\0';
        if (!picofw_parse_u32_strict(bbuf, &req_baud) || !picofw_dumpnor_is_fast_baud(req_baud)) {
            return false;
        }
        out->baud = req_baud;
        if (comma != NULL) {
            if (!picofw_dumpnor_parse_colon_spec(comma + 1, &out->addr, &out->value_u32)) {
                return false;
            }
        } else {
            out->addr = PICOFW_DUMPNOR_DEFAULT_START;
            out->value_u32 = picofw_flash_get_active_size();
        }
        out->send_magic_prefix = true;
        out->action = PANA_RSP_ACT_FAST_DUMPNOR;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "JEDEC_ERASE:") && tok[12] != '\0') {
        uint32_t addr = 0u;
        if (!picofw_rsp_parse_hex_u32(tok + 12u, 10u, &addr)) {
            return false;
        }
        out->addr = addr;
        out->data_len = 1u;
        out->send_magic_prefix = true;
        out->action = PANA_RSP_ACT_JEDEC_ERASE;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "WRITE32:")) {
        const char *tail = NULL;
        uint32_t addr = 0u, val = 0u;
        if (!parse_addr_comma_tail(tok + 8u, &addr, &tail) ||
            !picofw_rsp_parse_hex_u32(tail, 10u, &val) ||
            !assign_write_u32_payload(addr, val, out)) {
            return false;
        }
        out->addr = addr;
        out->action = PANA_RSP_ACT_WRITE32;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "CALL:") && tok[5] != '\0') {
        uint32_t target = 0u;
        if (!picofw_rsp_parse_hex_u32(tok + 5u, 10u, &target) ||
            !assign_write_u32_payload((uint32_t)PICOFW_AM34_SERIAL_CB_PTR_ADDR, target, out)) {
            return false;
        }
        out->addr = target;
        out->send_magic_prefix = true;
        out->action = PANA_RSP_ACT_CALL;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_ieq(tok, "DUMPNOR")) {
        out->action = PANA_RSP_ACT_DUMPNOR;
        out->send_magic_prefix = true;
        out->addr = PICOFW_DUMPNOR_DEFAULT_START;
        out->value_u32 = PICOFW_DUMPNOR_DEFAULT_LEN;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "DUMPNOR:")) {
        if (!picofw_dumpnor_parse_colon_spec(tok + 8u, &out->addr, &out->value_u32)) {
            return false;
        }
        out->action = PANA_RSP_ACT_DUMPNOR;
        out->send_magic_prefix = true;
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "RAW:") && tok[4] != '\0') {
        out->action = PANA_RSP_ACT_RAW;
        snprintf(out->payload, sizeof(out->payload), "%s", tok + 4);
        *saw_action = true;
        return true;
    }
    if (picofw_ascii_iprefix(tok, "HEX:") && tok[4] != '\0') {
        if (!picofw_decode_hex_bytes(tok + 4, out->raw_bytes, sizeof(out->raw_bytes), &out->raw_len)) {
            return false;
        }
        out->action = PANA_RSP_ACT_HEX;
        *saw_action = true;
        return true;
    }
    return false;
}
