/**
 * @file p7501_pana_rsp_codec.c
 * @brief Pure ISO C11 RSP client packet builder, verifier, and `PANARSP` CLI parser.
 *
 * Compiled identically on Pico ARM/RISC-V and host C11 test targets (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_dumpnor.h"
#include "common/p7501_hex.h"

#include <stdio.h>
#include <string.h>

const uint8_t k_p7501_rsp_handshake[P7501_RSP_HANDSHAKE_LEN] = {
    0x0Eu, 0x02u, 0x02u, 0x04u
};

static bool ascii_iprefix(const char *str, const char *prefix) {
    if (str == NULL || prefix == NULL) {
        return false;
    }
    while (*prefix != '\0') {
        char a = *str++;
        char b = *prefix++;
        if (a >= 'a' && a <= 'z') a = (char)(a - ('a' - 'A'));
        if (b >= 'a' && b <= 'z') b = (char)(b - ('a' - 'A'));
        if (a != b) {
            return false;
        }
    }
    return true;
}

static bool is_digits_only(const char *tok) {
    if (tok == NULL || *tok == '\0') {
        return false;
    }
    for (const char *p = tok; *p != '\0'; ++p) {
        if (*p < '0' || *p > '9') {
            return false;
        }
    }
    return true;
}

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

size_t p7501_rsp_build_frame(const char *payload, char *out, size_t out_cap) {
    if (payload == NULL || out == NULL) {
        return 0u;
    }
    const size_t plen = strlen(payload);
    if ((plen + 5u) > out_cap) {
        return 0u;
    }
    const uint8_t csum = p7501_rsp_checksum_str(payload);
    out[0] = '$';
    if (plen > 0u) {
        memcpy(&out[1], payload, plen);
    }
    out[1u + plen] = '#';
    out[2u + plen] = p7501_nibble_to_hex_char((uint8_t)(csum >> 4));
    out[3u + plen] = p7501_nibble_to_hex_char((uint8_t)(csum & 0x0Fu));
    out[4u + plen] = '\0';
    return 4u + plen;
}

static bool parse_hex_u32(const char *str, size_t max_chars, uint32_t *out_val) {
    if (str == NULL || out_val == NULL || max_chars == 0u) {
        return false;
    }
    if (str[0] == '0' && (str[1] == 'x' || str[1] == 'X')) {
        str += 2;
        if (max_chars < 2u) {
            return false;
        }
    }
    size_t count = 0u;
    uint32_t acc = 0u;
    while (count < max_chars && str[count] != '\0') {
        const int nib = p7501_hex_char_to_nibble(str[count]);
        if (nib < 0) {
            return false;
        }
        acc = (acc << 4) | (uint32_t)nib;
        count++;
    }
    if (count == 0u || str[count] != '\0' || count > 8u) {
        return false;
    }
    *out_val = acc;
    return true;
}

static bool parse_hex_u32_span(const char *start, size_t len, uint32_t *out_val) {
    char tmp[12];
    if (start == NULL || len == 0u || len >= sizeof(tmp)) {
        return false;
    }
    memcpy(tmp, start, len);
    tmp[len] = '\0';
    return parse_hex_u32(tmp, len, out_val);
}

static bool build_write_mem_payload(
    uint32_t addr,
    const uint8_t *data,
    size_t len,
    char *out_payload,
    size_t cap
) {
    if (data == NULL || len == 0u || len > 16u || out_payload == NULL) {
        return false;
    }
    const int hdr = snprintf(out_payload, cap, "M%08X,%u:", (unsigned)addr, (unsigned)len);
    if (hdr <= 0 || ((size_t)hdr + (len * 2u) + 1u) > cap) {
        return false;
    }
    size_t pos = (size_t)hdr;
    for (size_t i = 0u; i < len; ++i) {
        out_payload[pos++] = p7501_nibble_to_hex_char((uint8_t)(data[i] >> 4));
        out_payload[pos++] = p7501_nibble_to_hex_char((uint8_t)(data[i] & 0x0Fu));
    }
    out_payload[pos] = '\0';
    return true;
}

size_t p7501_rsp_build_write_mem(
    uint32_t addr,
    const uint8_t *data,
    size_t len,
    char *out,
    size_t out_cap
) {
    char payload[64];
    if (!build_write_mem_payload(addr, data, len, payload, sizeof(payload))) {
        return 0u;
    }
    return p7501_rsp_build_frame(payload, out, out_cap);
}

static void encode_u32_le(uint32_t value_le, uint8_t out_le[4]) {
    out_le[0] = (uint8_t)(value_le & 0xFFu);
    out_le[1] = (uint8_t)((value_le >> 8) & 0xFFu);
    out_le[2] = (uint8_t)((value_le >> 16) & 0xFFu);
    out_le[3] = (uint8_t)((value_le >> 24) & 0xFFu);
}

size_t p7501_rsp_build_write_u32(uint32_t addr, uint32_t value_le, char *out, size_t out_cap) {
    uint8_t bytes[4];
    encode_u32_le(value_le, bytes);
    return p7501_rsp_build_write_mem(addr, bytes, sizeof(bytes), out, out_cap);
}

/** Return true when `ch` is an ASCII letter or decimal digit. */
static bool rsp_is_ascii_alnum(uint8_t ch) {
    return ((ch >= (uint8_t)'0') && (ch <= (uint8_t)'9')) ||
           ((ch >= (uint8_t)'A') && (ch <= (uint8_t)'Z')) ||
           ((ch >= (uint8_t)'a') && (ch <= (uint8_t)'z'));
}

/** Distinguish GDB RSP ACK (`+`) / NAK (`-`) from ASCII hyphens/signs inside boot prose (`hung-up`, `-91006`). */
static bool rsp_is_standalone_ack_or_nak(
    const uint8_t *buf,
    size_t start_offset,
    size_t len,
    size_t idx
) {
    if (((idx + 1u) < len) && rsp_is_ascii_alnum(buf[idx + 1u])) {
        return false;
    }
    if ((idx > start_offset) && rsp_is_ascii_alnum(buf[idx - 1u])) {
        const bool after_checksum = (idx >= (start_offset + 3u)) && (buf[idx - 3u] == (uint8_t)'#');
        if (!after_checksum) {
            return false;
        }
    }
    return true;
}

bool p7501_rsp_inspect_frame_from(
    const uint8_t *buf,
    size_t len,
    size_t start_offset,
    p7501_rsp_frame_info_t *out
) {
    if (buf == NULL || out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    if (start_offset >= len) {
        return false;
    }

    p7501_rsp_frame_info_t fallback;
    memset(&fallback, 0, sizeof(fallback));
    bool ack_plus = false;
    bool nak_minus = false;
    bool raw_nak_0x15 = false;

    for (size_t i = start_offset; i < len; ++i) {
        if (buf[i] == 0x15u) {
            raw_nak_0x15 = true;
        } else if ((buf[i] == (uint8_t)'+') && rsp_is_standalone_ack_or_nak(buf, start_offset, len, i)) {
            ack_plus = true;
        } else if ((buf[i] == (uint8_t)'-') && rsp_is_standalone_ack_or_nak(buf, start_offset, len, i)) {
            nak_minus = true;
        }
        if (buf[i] != (uint8_t)'$') {
            continue;
        }
        const size_t start = i + 1u;
        for (size_t j = start; (j + 2u) < len; ++j) {
            if (buf[j] != (uint8_t)'#') {
                continue;
            }
            const int hi = p7501_hex_char_to_nibble((char)buf[j + 1u]);
            const int lo = p7501_hex_char_to_nibble((char)buf[j + 2u]);
            if (hi < 0 || lo < 0) {
                continue;
            }
            const size_t plen = j - start;
            const uint8_t expected = p7501_rsp_checksum(&buf[start], plen);
            const uint8_t actual = (uint8_t)((hi << 4) | lo);
            const p7501_rsp_frame_info_t cand = {
                .found = true,
                .ack_plus = ack_plus,
                .nak_minus = nak_minus,
                .raw_nak_0x15 = raw_nak_0x15,
                .checksum_ok = (expected == actual),
                .expected_csum = expected,
                .actual_csum = actual,
                .payload_offset = start,
                .payload_len = plen,
            };
            if (cand.checksum_ok) {
                *out = cand;
                return true;
            }
            if (!fallback.found) {
                fallback = cand;
            }
        }
    }

    if (fallback.found) {
        *out = fallback;
        return true;
    }
    out->ack_plus = ack_plus;
    out->nak_minus = nak_minus;
    out->raw_nak_0x15 = raw_nak_0x15;
    return false;
}

bool p7501_rsp_inspect_frame(const uint8_t *buf, size_t len, p7501_rsp_frame_info_t *out) {
    return p7501_rsp_inspect_frame_from(buf, len, 0u, out);
}

static bool assign_write_u32_payload(
    uint32_t dest_addr,
    uint32_t value_le,
    p7501_pana_rsp_spec_t *out
) {
    uint8_t le[4];
    encode_u32_le(value_le, le);
    if (!build_write_mem_payload(dest_addr, le, sizeof(le), out->payload, sizeof(out->payload))) {
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
    if (ascii_iprefix(tok, "MEM:") || ascii_iprefix(tok, "XMEM:")) {
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
    if (ascii_iprefix(tok, "WMEM:")) {
        const char *spec = tok + 5u;
        const char *comma = strchr(spec, ',');
        if (comma == NULL || comma == spec || comma[1] == '\0') {
            return false;
        }
        uint32_t addr = 0u;
        size_t data_len = 0u;
        if (!parse_hex_u32_span(spec, (size_t)(comma - spec), &addr) ||
            !p7501_decode_hex_bytes(comma + 1, out->data_bytes, sizeof(out->data_bytes), &data_len) ||
            data_len == 0u ||
            !build_write_mem_payload(addr, out->data_bytes, data_len, out->payload, sizeof(out->payload))) {
            return false;
        }
        out->addr = addr;
        out->data_len = data_len;
        out->action = PANA_RSP_ACT_WMEM;
        *saw_action = true;
        return true;
    }
    if (ascii_iprefix(tok, "WRITE32:")) {
        const char *spec = tok + 8u;
        const char *comma = strchr(spec, ',');
        if (comma == NULL || comma == spec || comma[1] == '\0') {
            return false;
        }
        uint32_t addr = 0u;
        uint32_t val = 0u;
        if (!parse_hex_u32_span(spec, (size_t)(comma - spec), &addr) ||
            !parse_hex_u32(comma + 1, 10u, &val) ||
            !assign_write_u32_payload(addr, val, out)) {
            return false;
        }
        out->addr = addr;
        out->action = PANA_RSP_ACT_WRITE32;
        *saw_action = true;
        return true;
    }
    if (ascii_iprefix(tok, "CALL:") && tok[5] != '\0') {
        uint32_t target = 0u;
        if (!parse_hex_u32(tok + 5u, 10u, &target) ||
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
    if (ascii_iprefix(tok, "DUMPNOR:")) {
        if (!p7501_dumpnor_parse_colon_spec(tok + 8u, &out->addr, &out->value_u32)) {
            return false;
        }
        out->action = PANA_RSP_ACT_DUMPNOR;
        out->send_magic_prefix = true;
        *saw_action = true;
        return true;
    }
    if (ascii_iprefix(tok, "RAW:") && tok[4] != '\0') {
        out->action = PANA_RSP_ACT_RAW;
        snprintf(out->payload, sizeof(out->payload), "%s", tok + 4);
        *saw_action = true;
        return true;
    }
    if (ascii_iprefix(tok, "HEX:") && tok[4] != '\0') {
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
        } else if (is_digits_only(tok)) {
            out->timeout_ms = p7501_parse_uint32_clamped(tok, 4000u, 150u, 900000u);
        } else if (p7501_ascii_ieq(tok, "MAGIC")) {
            out->send_magic_prefix = true;
            explicit_magic_only = true;
        } else if (p7501_ascii_ieq(tok, "NONAK")) {
            out->send_nak_prefix = false;
        } else if (p7501_ascii_ieq(tok, "NAK")) {
            out->send_nak_prefix = true;
            explicit_nak_only = true;
        } else if (ascii_iprefix(tok, "WAIT:") && tok[5] != '\0') {
            out->wait_trigger_enabled = true;
            snprintf(out->trigger, sizeof(out->trigger), "%s", tok + 5);
        } else if (!saw_boot && (p7501_ascii_ieq(tok, "NORMAL") || p7501_ascii_ieq(tok, "BOOT") ||
                                 p7501_ascii_ieq(tok, "NOPULL") || p7501_ascii_ieq(tok, "PASSIVE") ||
                                 p7501_ascii_ieq(tok, "JIG") || ascii_iprefix(tok, "JIG:"))) {
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
