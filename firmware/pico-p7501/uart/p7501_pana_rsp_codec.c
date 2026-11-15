/**
 * @file p7501_pana_rsp_codec.c
 * @brief Pure ISO C11 RSP client packet builder and RX frame verifier.
 *
 * Compiled identically on Pico ARM/RISC-V and host C11 test targets (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_rsp_internal.h"
#include "common/p7501_hex.h"

#include <stdio.h>
#include <string.h>

const uint8_t k_p7501_rsp_handshake[P7501_RSP_HANDSHAKE_LEN] = P7501_RSP_HANDSHAKE_INIT;

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

bool p7501_rsp_parse_hex_u32(const char *str, size_t max_chars, uint32_t *out_val) {
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

bool p7501_rsp_parse_hex_u32_span(const char *start, size_t len, uint32_t *out_val) {
    char tmp[12];
    if (start == NULL || len == 0u || len >= sizeof(tmp)) {
        return false;
    }
    memcpy(tmp, start, len);
    tmp[len] = '\0';
    return p7501_rsp_parse_hex_u32(tmp, len, out_val);
}

bool p7501_rsp_build_write_mem_payload(
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
    if (!p7501_rsp_build_write_mem_payload(addr, data, len, payload, sizeof(payload))) {
        return 0u;
    }
    return p7501_rsp_build_frame(payload, out, out_cap);
}

void p7501_rsp_encode_u32_le(uint32_t value_le, uint8_t out_le[4]) {
    out_le[0] = (uint8_t)(value_le & 0xFFu);
    out_le[1] = (uint8_t)((value_le >> 8) & 0xFFu);
    out_le[2] = (uint8_t)((value_le >> 16) & 0xFFu);
    out_le[3] = (uint8_t)((value_le >> 24) & 0xFFu);
}

size_t p7501_rsp_build_write_u32(uint32_t addr, uint32_t value_le, char *out, size_t out_cap) {
    uint8_t bytes[4];
    p7501_rsp_encode_u32_le(value_le, bytes);
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
