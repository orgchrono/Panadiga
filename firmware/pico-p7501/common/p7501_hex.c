/**
 * @file p7501_hex.c
 * @brief Shared pure C11 hexadecimal and ASCII string helpers across CLI, UART, FMU, and GDB RSP.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_hex.h"

#include <stdio.h>

static char to_ascii_upper(char c) {
    return (c >= 'a' && c <= 'z') ? (char)(c - ('a' - 'A')) : c;
}

int p7501_hex_char_to_nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return (c - 'a') + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return (c - 'A') + 10;
    }
    return -1;
}

char p7501_nibble_to_hex_char(uint8_t nibble) {
    static const char k_hex[] = "0123456789abcdef";
    return k_hex[nibble & 0x0Fu];
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

static const char *skip_hspace(const char *p) {
    while ((p != NULL) && ((*p == ' ') || (*p == '\t'))) {
        ++p;
    }
    return p;
}

bool p7501_parse_u32_strict(const char *str, uint32_t *out_val) {
    const char *p = skip_hspace(str);
    if ((p == NULL) || (out_val == NULL) || (*p < '0') || (*p > '9')) {
        return false;
    }
    uint64_t acc = 0ULL;
    while ((*p >= '0') && (*p <= '9')) {
        acc = (acc * 10ULL) + (uint64_t)(*p - '0');
        if (acc > 0xFFFFFFFFULL) {
            return false;
        }
        ++p;
    }
    p = skip_hspace(p);
    if (*p != '\0') {
        return false;
    }
    *out_val = (uint32_t)acc;
    return true;
}

static bool strip_hex_prefix(const char **cursor) {
    const char *p = *cursor;
    if ((p != NULL) && (p[0] == '0') && ((p[1] == 'x') || (p[1] == 'X'))) {
        *cursor = p + 2;
        return true;
    }
    return false;
}

bool p7501_parse_hex_u32_prefix(const char *str, uint32_t *out_val, const char **out_rest) {
    const char *p = skip_hspace(str);
    if ((p == NULL) || (out_val == NULL)) {
        return false;
    }
    (void)strip_hex_prefix(&p);
    uint64_t acc = 0ULL;
    size_t digits = 0u;
    int nib = 0;
    while ((nib = p7501_hex_char_to_nibble(*p)) >= 0) {
        acc = (acc << 4u) | (uint64_t)(uint32_t)nib;
        if (acc > 0xFFFFFFFFULL) {
            return false;
        }
        ++digits;
        ++p;
    }
    if ((digits == 0u) || ((*p != '\0') && (*p != ' ') && (*p != '\t'))) {
        return false;
    }
    *out_val = (uint32_t)acc;
    if (out_rest != NULL) {
        *out_rest = skip_hspace(p);
    }
    return true;
}

bool p7501_parse_u32_auto(const char *str, uint32_t *out_val) {
    const char *p = skip_hspace(str);
    const char *after_prefix = p;
    if (strip_hex_prefix(&after_prefix)) {
        const char *rest = NULL;
        return p7501_parse_hex_u32_prefix(p, out_val, &rest) && (*rest == '\0');
    }
    return p7501_parse_u32_strict(p, out_val);
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

bool p7501_decode_hex_bytes(
    const char *hex,
    uint8_t *out_bytes,
    size_t max_bytes,
    size_t *out_len
) {
    if (hex == NULL || out_bytes == NULL || out_len == NULL || max_bytes == 0u) {
        return false;
    }

    size_t count = 0u;
    int pending_hi = -1;

    for (const char *p = hex; *p != '\0'; ++p) {
        const char ch = *p;
        if (ch == ' ' || ch == '\t' || ch == ',' || ch == ':' || ch == '-' || ch == '_') {
            if (pending_hi >= 0) {
                return false;
            }
            continue;
        }
        const int nib = p7501_hex_char_to_nibble(ch);
        if (nib < 0) {
            return false;
        }
        if (pending_hi < 0) {
            pending_hi = nib;
        } else {
            if (count >= max_bytes) {
                return false;
            }
            out_bytes[count++] = (uint8_t)((pending_hi << 4) | nib);
            pending_hi = -1;
        }
    }

    if (pending_hi >= 0 || count == 0u) {
        return false;
    }
    *out_len = count;
    return true;
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

bool p7501_parse_hex_u32_exact(const char *str, size_t max_chars, uint32_t *out_val) {
    if ((str == NULL) || (out_val == NULL) || (max_chars == 0u)) {
        return false;
    }
    if ((str[0] == '0') && ((str[1] == 'x') || (str[1] == 'X'))) {
        if (max_chars < 2u) {
            return false;
        }
        str += 2;
    }
    size_t count = 0u;
    uint32_t acc = 0u;
    while ((count < max_chars) && (str[count] != '\0')) {
        const int nib = p7501_hex_char_to_nibble(str[count]);
        if ((nib < 0) || (count >= 8u)) {
            return false;
        }
        acc = (acc << 4u) | (uint32_t)nib;
        ++count;
    }
    if ((count == 0u) || (str[count] != '\0')) {
        return false;
    }
    *out_val = acc;
    return true;
}

size_t p7501_scan_hex_u32(const char *buf, size_t len, size_t start, uint32_t *out_val) {
    if ((buf == NULL) || (out_val == NULL) || (start >= len)) {
        return 0u;
    }
    size_t pos = start;
    if (((pos + 1u) < len) && (buf[pos] == '0') &&
        ((buf[pos + 1u] == 'x') || (buf[pos + 1u] == 'X'))) {
        pos += 2u;
    }
    size_t digits = 0u;
    uint32_t acc = 0u;
    while ((pos < len) && (digits < 8u)) {
        const int nib = p7501_hex_char_to_nibble(buf[pos]);
        if (nib < 0) {
            break;
        }
        acc = (acc << 4u) | (uint32_t)nib;
        ++pos;
        ++digits;
    }
    if (digits == 0u) {
        return 0u;
    }
    /* Reject >8-digit hex numbers so `080000000` can never silently truncate to `0x08000000`. */
    if ((pos < len) && (p7501_hex_char_to_nibble(buf[pos]) >= 0)) {
        return 0u;
    }
    *out_val = acc;
    return pos - start;
}

bool p7501_parse_hex_u32_span(const char *start, size_t len, uint32_t *out_val) {
    if ((start == NULL) || (len == 0u) || (out_val == NULL)) {
        return false;
    }
    return p7501_scan_hex_u32(start, len, 0u, out_val) == len;
}

const char *p7501_parse_hex_delim(
    const char *str,
    uint32_t *out_val,
    char delim,
    const char **out_rest
) {
    uint32_t val = 0u;
    size_t digits = 0u;
    if ((str == NULL) || (out_val == NULL)) {
        return NULL;
    }
    while ((*str != '\0') && (*str != delim)) {
        const int nibble = p7501_hex_char_to_nibble(*str);
        if ((nibble < 0) || (digits >= 8u)) {
            return NULL;
        }
        val = (val << 4u) | (uint32_t)nibble;
        ++str;
        ++digits;
    }
    if ((digits == 0u) || (*str != delim)) {
        return NULL;
    }
    *out_val = val;
    if ((*str == delim) && (delim != '\0')) {
        ++str;
    }
    if (out_rest != NULL) {
        *out_rest = str;
    }
    return str;
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
    uint32_t crc = seed_crc ^ 0xFFFFFFFFUL;
    for (size_t i = 0u; i < len; ++i) {
        crc ^= (uint32_t)data[i];
        for (uint8_t bit = 0u; bit < 8u; ++bit) {
            const uint32_t mask = (uint32_t)(-(int32_t)(crc & 1UL));
            crc = (crc >> 1u) ^ (0xEDB88320UL & mask);
        }
    }
    return ~crc;
}

uint32_t p7501_crc32_ieee(const uint8_t *data, size_t len) {
    if (data == NULL) {
        return 0u;
    }
    return p7501_crc32_ieee_update(0u, data, len);
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
