/**
 * @file p7501_hex.c
 * @brief Shared pure C11 strict hexadecimal and integer parsers & nibble codecs.
 *
 * Split companion `p7501_crc_csum.c` owns ASCII formatting, RSP checksums, CRC32, and LE32 helpers.
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_hex.h"

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

uint32_t p7501_format_hex_bitmap_csv(const uint8_t bitmap[32], char *out, size_t cap) {
    if ((bitmap == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    uint32_t count = 0u;
    size_t pos = 0u;
    out[0] = '\0';
    for (unsigned op = 0u; op < 256u; ++op) {
        if ((bitmap[op >> 3u] & (uint8_t)(1u << (op & 7u))) == 0u) {
            continue;
        }
        if ((count > 0u) && ((pos + 1u) < cap)) {
            out[pos++] = ',';
        }
        if ((pos + 2u) < cap) {
            static const char k_hex_upper[] = "0123456789ABCDEF";
            out[pos++] = k_hex_upper[(op >> 4u) & 0x0Fu];
            out[pos++] = k_hex_upper[op & 0x0Fu];
        }
        ++count;
    }
    if (count == 0u) {
        static const char k_none[] = "NONE";
        if (cap >= sizeof(k_none)) {
            for (size_t i = 0u; i < sizeof(k_none); ++i) {
                out[i] = k_none[i];
            }
        }
    } else {
        out[pos < cap ? pos : (cap - 1u)] = '\0';
    }
    return count;
}

size_t p7501_split_ws_tokens(char *buf, char *out_toks[], size_t max_toks) {
    if ((buf == NULL) || (out_toks == NULL) || (max_toks == 0u)) {
        return 0u;
    }
    size_t n = 0u;
    char *p = buf;
    while ((*p != '\0') && (n < max_toks)) {
        while ((*p == ' ') || (*p == '\t')) {
            ++p;
        }
        if (*p == '\0') {
            break;
        }
        out_toks[n++] = p;
        while ((*p != '\0') && (*p != ' ') && (*p != '\t')) {
            ++p;
        }
        if (*p != '\0') {
            *p++ = '\0';
        }
    }
    return n;
}

size_t p7501_copy_and_split_ws(
    const char *arg,
    char *scratch,
    size_t scratch_cap,
    char *out_toks[],
    size_t max_toks
) {
    if ((arg == NULL) || (*arg == '\0') || (scratch == NULL) || (scratch_cap == 0u)) {
        return 0u;
    }
    size_t i = 0u;
    while (((i + 1u) < scratch_cap) && (arg[i] != '\0')) {
        scratch[i] = arg[i];
        ++i;
    }
    scratch[i] = '\0';
    return p7501_split_ws_tokens(scratch, out_toks, max_toks);
}

bool p7501_line_accum_push_byte(
    char *buf,
    size_t *io_len,
    size_t cap,
    uint8_t byte
) {
    if ((buf == NULL) || (io_len == NULL) || (cap == 0u)) {
        return false;
    }
    if ((byte == '\r') || (byte == '\n')) {
        return true;
    }
    if (((*io_len + 1u) < cap) && (byte >= 0x20u) && (byte <= 0x7Eu)) {
        buf[(*io_len)++] = (char)byte;
    }
    return false;
}
