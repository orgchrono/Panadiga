/**
 * @file p7501_crc_csum.c
 * @brief Shared pure C11 ASCII case/formatting helpers, GDB RSP checksums, CRC32 IEEE, and LE32 encoders.
 *
 * Split from `p7501_hex.c` (`p7501_hex.c` retains strict hex/u32 parsers & nibble codecs).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_hex.h"

#include <stdio.h>

static char to_ascii_upper(char c) {
    return (c >= 'a' && c <= 'z') ? (char)(c - ('a' - 'A')) : c;
}

bool p7501_ascii_ieq(const char *a, const char *b) {
    if (a == NULL || b == NULL) {
        return false;
    }
    while (*a != '\0' && *b != '\0') {
        if (to_ascii_upper(*a) != to_ascii_upper(*b)) {
            return false;
        }
        ++a;
        ++b;
    }
    return *a == '\0' && *b == '\0';
}

bool p7501_ascii_iprefix(const char *str, const char *prefix) {
    if ((str == NULL) || (prefix == NULL)) {
        return false;
    }
    while (*prefix != '\0') {
        if ((*str == '\0') || (to_ascii_upper(*str) != to_ascii_upper(*prefix))) {
            return false;
        }
        ++str;
        ++prefix;
    }
    return true;
}

bool p7501_ascii_is_digits(const char *str) {
    if ((str == NULL) || (*str == '\0')) {
        return false;
    }
    for (const char *p = str; *p != '\0'; ++p) {
        if ((*p < '0') || (*p > '9')) {
            return false;
        }
    }
    return true;
}

void p7501_ascii_upper_copy(char *dst, size_t dst_cap, const char *src) {
    if (dst == NULL || dst_cap == 0u) {
        return;
    }
    if (src == NULL) {
        dst[0] = '\0';
        return;
    }
    size_t i = 0u;
    while (src[i] != '\0' && (i + 1u) < dst_cap) {
        dst[i] = to_ascii_upper(src[i]);
        ++i;
    }
    dst[i] = '\0';
}

uint8_t p7501_rsp_checksum(const uint8_t *data, size_t len) {
    uint8_t sum = 0u;
    if (data == NULL) {
        return 0u;
    }
    for (size_t i = 0u; i < len; ++i) {
        sum = (uint8_t)(sum + data[i]);
    }
    return sum;
}

uint8_t p7501_rsp_checksum_str(const char *str) {
    uint8_t sum = 0u;
    if (str == NULL) {
        return 0u;
    }
    while (*str != '\0') {
        sum = (uint8_t)(sum + (uint8_t)(*str));
        ++str;
    }
    return sum;
}

size_t p7501_format_ascii_byte(
    uint8_t b,
    char left_delim,
    char right_delim,
    char *out_buf,
    size_t cap
) {
    if ((out_buf == NULL) || (cap < 5u)) {
        return 0u;
    }
    if (b == (uint8_t)'\r') {
        out_buf[0] = '\0';
        return 0u;
    }
    if ((b == (uint8_t)'\n') || ((b >= 0x20u) && (b <= 0x7Eu))) {
        out_buf[0] = (char)b;
        out_buf[1] = '\0';
        return 1u;
    }
    out_buf[0] = left_delim;
    out_buf[1] = (char)p7501_nibble_to_hex_char((uint8_t)(b >> 4u));
    out_buf[2] = (char)p7501_nibble_to_hex_char((uint8_t)(b & 0x0Fu));
    /* Uppercase escapes preserve existing `[%02X]` / `<%02X>` log conventions */
    out_buf[1] = to_ascii_upper(out_buf[1]);
    out_buf[2] = to_ascii_upper(out_buf[2]);
    out_buf[3] = right_delim;
    out_buf[4] = '\0';
    return 4u;
}

void p7501_print_ascii_byte(uint8_t b, char left_delim, char right_delim) {
    char tmp[8];
    const size_t n = p7501_format_ascii_byte(b, left_delim, right_delim, tmp, sizeof(tmp));
    if (n > 0u) {
        (void)fwrite(tmp, 1u, n, stdout);
    }
}

void p7501_uart_print_ascii_stream(const uint8_t *buf, uint32_t len) {
    if (buf == NULL) {
        return;
    }
    for (uint32_t i = 0u; i < len; ++i) {
        p7501_print_ascii_byte(buf[i], '<', '>');
    }
}

uint32_t p7501_crc32_ieee_update(uint32_t seed_crc, const uint8_t *data, size_t len) {
    if (data == NULL) {
        return seed_crc;
    }
    static const uint32_t k_crc32_nibble_lut[16] = {
        0x00000000UL, 0x1DB71064UL, 0x3B6E20C8UL, 0x26D930ACUL,
        0x76DC4190UL, 0x6B6B51F4UL, 0x4DB26158UL, 0x5005713CUL,
        0xEDB88320UL, 0xF00F9344UL, 0xD6D6A3E8UL, 0xCB61B38CUL,
        0x9B64C2B0UL, 0x86D3D2D4UL, 0xA00AE278UL, 0xBDBDF21CUL,
    };
    uint32_t crc = seed_crc ^ 0xFFFFFFFFUL;
    for (size_t i = 0u; i < len; ++i) {
        crc ^= (uint32_t)data[i];
        crc = (crc >> 4u) ^ k_crc32_nibble_lut[crc & 0x0Fu];
        crc = (crc >> 4u) ^ k_crc32_nibble_lut[crc & 0x0Fu];
    }
    return ~crc;
}

uint32_t p7501_crc32_ieee(const uint8_t *data, size_t len) {
    if (data == NULL) {
        return 0u;
    }
    return p7501_crc32_ieee_update(0u, data, len);
}

bool p7501_buf_is_all_ff(const uint8_t *buf, size_t len) {
    if ((buf == NULL) || (len == 0u)) {
        return false;
    }
    size_t i = 0u;
    while ((i < len) && ((((uintptr_t)(buf + i)) & 3u) != 0u)) {
        if (buf[i] != 0xFFu) {
            return false;
        }
        ++i;
    }
    while ((i + 4u) <= len) {
        const uint32_t w = *(const uint32_t *)(const void *)(buf + i);
        if (w != 0xFFFFFFFFUL) {
            return false;
        }
        i += 4u;
    }
    while (i < len) {
        if (buf[i] != 0xFFu) {
            return false;
        }
        ++i;
    }
    return true;
}

uint32_t p7501_buf_count_non_ff(const uint8_t *buf, size_t len) {
    if (buf == NULL) {
        return 0u;
    }
    uint32_t count = 0u;
    for (size_t i = 0u; i < len; ++i) {
        if (buf[i] != 0xFFu) {
            count += 1u;
        }
    }
    return count;
}

void p7501_write_u32_le(uint8_t out[4], uint32_t v) {
    out[0] = (uint8_t)(v & 0xFFu);
    out[1] = (uint8_t)((v >> 8u) & 0xFFu);
    out[2] = (uint8_t)((v >> 16u) & 0xFFu);
    out[3] = (uint8_t)((v >> 24u) & 0xFFu);
}

uint32_t p7501_read_u32_le(const uint8_t in[4]) {
    return ((uint32_t)in[0]) |
           (((uint32_t)in[1]) << 8u) |
           (((uint32_t)in[2]) << 16u) |
           (((uint32_t)in[3]) << 24u);
}

const char *p7501_fmt_mv_volts(uint32_t mv, char buf[8]) {
    if (buf == NULL) {
        return "0.00";
    }
    uint32_t cs = (mv + 5u) / 10u;
    if (cs > 99999u) {
        cs = 99999u;
    }
    const uint32_t whole = cs / 100u;
    const uint32_t frac = cs % 100u;
    (void)snprintf(buf, 8u, "%u.%02u", (unsigned)whole, (unsigned)frac);
    return buf;
}
