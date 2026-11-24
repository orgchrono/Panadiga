/**
 * @file p7501_bitwise_search.c
 * @brief Pure host-testable wildcard pattern matcher and UB-free 32-bit bitwise algebra engine.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_microsurgery.h"

#include "common/p7501_hex.h"

bool p7501_surg_parse_wildcard_hex(
    const char *str,
    uint8_t pat[P7501_SURG_MAX_PATTERN],
    uint8_t mask[P7501_SURG_MAX_PATTERN],
    size_t *out_len
) {
    if ((str == NULL) || (pat == NULL) || (mask == NULL) || (out_len == NULL)) {
        return false;
    }
    size_t n = 0u;
    const char *p = str;
    while (*p != '\0') {
        while ((*p == ' ') || (*p == '\t')) {
            ++p;
        }
        if (*p == '\0') {
            break;
        }
        if ((n >= P7501_SURG_MAX_PATTERN) || (p[1] == '\0')) {
            return false;
        }
        if ((p[0] == '?') && (p[1] == '?')) {
            pat[n] = 0x00u;
            mask[n] = 0x00u;
        } else {
            const int hi = p7501_hex_char_to_nibble(p[0]);
            const int lo = p7501_hex_char_to_nibble(p[1]);
            if ((hi < 0) || (lo < 0)) {
                return false;
            }
            pat[n] = (uint8_t)(((unsigned)hi << 4u) | (unsigned)lo);
            mask[n] = 0xFFu;
        }
        ++n;
        p += 2;
    }
    *out_len = n;
    return n > 0u;
}

size_t p7501_surg_search_buf(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    const uint8_t *pat,
    const uint8_t *mask,
    size_t plen,
    uint32_t *out_vas,
    size_t max_hits
) {
    if ((buf == NULL) || (pat == NULL) || (mask == NULL) || (plen == 0u) || (plen > len)) {
        return 0u;
    }
    size_t hits = 0u;
    for (size_t i = 0u; (i + plen) <= len; ++i) {
        bool ok = true;
        for (size_t j = 0u; j < plen; ++j) {
            if (((buf[i + j] ^ pat[j]) & mask[j]) != 0u) {
                ok = false;
                break;
            }
        }
        if (ok) {
            if ((out_vas != NULL) && (hits < max_hits)) {
                out_vas[hits] = base_va + (uint32_t)i;
            }
            hits += 1u;
        }
    }
    return hits;
}

bool p7501_bit_parse_op(const char *tok, p7501_bit_op_t *out_op) {
    static const char *const k_names[13] = {
        "INFO", "SET", "CLR", "TOG", "FIELD", "EXT",
        "POPCNT", "CLZ", "CTZ", "BSWAP16", "BSWAP32", "ROTL", "ROTR"
    };
    if ((tok == NULL) || (out_op == NULL)) {
        return false;
    }
    for (size_t i = 0u; i < 13u; ++i) {
        if (p7501_ascii_ieq(tok, k_names[i])) {
            *out_op = (p7501_bit_op_t)i;
            return true;
        }
    }
    return false;
}

bool p7501_bit_eval(
    p7501_bit_op_t op,
    uint32_t val,
    uint32_t a1,
    uint32_t a2,
    uint32_t a3,
    uint32_t *out_val
) {
    if (out_val == NULL) {
        return false;
    }
    switch (op) {
        case P7501_BIT_INFO:
            *out_val = val;
            return true;
        case P7501_BIT_SET:
        case P7501_BIT_CLR:
        case P7501_BIT_TOG:
            if (a1 >= 32u) {
                return false;
            }
            *out_val = (op == P7501_BIT_SET) ? (val | (1UL << a1))
                : ((op == P7501_BIT_CLR) ? (val & ~(1UL << a1)) : (val ^ (1UL << a1)));
            return true;
        case P7501_BIT_FIELD:
        case P7501_BIT_EXT: {
            if ((a1 >= 32u) || (a2 == 0u) || (a2 > 32u) || ((a1 + a2) > 32u)) {
                return false;
            }
            const uint32_t m = (a2 == 32u) ? 0xFFFFFFFFUL : ((1UL << a2) - 1UL);
            *out_val = (op == P7501_BIT_EXT)
                ? ((val >> a1) & m)
                : ((val & ~(m << a1)) | ((a3 & m) << a1));
            return true;
        }
        case P7501_BIT_POPCNT: {
            uint32_t c = 0u, v = val;
            for (; v != 0u; ++c) {
                v &= v - 1u;
            }
            *out_val = c;
            return true;
        }
        case P7501_BIT_CLZ:
        case P7501_BIT_CTZ: {
            if (val == 0u) {
                *out_val = 32u;
                return true;
            }
            uint32_t cnt = 0u;
            uint32_t bit = (op == P7501_BIT_CLZ) ? 0x80000000UL : 1UL;
            while ((val & bit) == 0u) {
                cnt += 1u;
                bit = (op == P7501_BIT_CLZ) ? (bit >> 1u) : (bit << 1u);
            }
            *out_val = cnt;
            return true;
        }
        case P7501_BIT_BSWAP16:
            *out_val = ((val & 0x00FF00FFUL) << 8u) | ((val & 0xFF00FF00UL) >> 8u);
            return true;
        case P7501_BIT_BSWAP32:
            *out_val = ((val & 0xFFu) << 24u) | ((val & 0xFF00u) << 8u) |
                       ((val >> 8u) & 0xFF00u) | ((val >> 24u) & 0xFFu);
            return true;
        case P7501_BIT_ROTL:
        case P7501_BIT_ROTR: {
            const uint32_t s = a1 & 31u;
            if (s == 0u) {
                *out_val = val;
                return true;
            }
            *out_val = (op == P7501_BIT_ROTL)
                ? ((val << s) | (val >> (32u - s)))
                : ((val >> s) | (val << (32u - s)));
            return true;
        }
        default:
            return false;
    }
}
