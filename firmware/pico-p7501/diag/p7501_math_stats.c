/**
 * @file p7501_math_stats.c
 * @brief Fixed-point Q10 entropy, Q8 chi^2 goodness-of-fit, ECB-64, pin-mirror & S-box detectors.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_math_gps.h"

#include "common/p7501_hex.h"
#include "crypto/p7501_feistel.h"

#include <string.h>

/* round(1024 * log2(1 + i/32)) for i = 0..32 (integer Q10 mantissa LUT) */
static const uint16_t k_log2_q10_lut[33] = {
       0u,   45u,   90u,  132u,  174u,  214u,  254u,  292u,
     330u,  366u,  402u,  436u,  470u,  504u,  536u,  568u,
     599u,  629u,  659u,  689u,  717u,  745u,  773u,  800u,
     827u,  853u,  879u,  904u,  929u,  953u,  977u, 1001u,
    1024u
};

uint32_t p7501_math_log2_q10(uint32_t x) {
    if (x <= 1u) {
        return 0u;
    }
    uint32_t k = 0u;
    uint32_t t = x;
    while (t > 1u) {
        t >>= 1u;
        ++k;
    }
    uint32_t idx = 0u;
    uint32_t frac = 0u;
    if (k >= 5u) {
        const uint32_t sh = k - 5u;
        idx = (x >> sh) & 0x1Fu;
        if (sh > 0u) {
            const uint32_t rem_mask = (1UL << sh) - 1UL;
            const uint32_t step = (uint32_t)(k_log2_q10_lut[idx + 1u] - k_log2_q10_lut[idx]);
            frac = (step * (x & rem_mask)) >> sh;
        }
    } else {
        idx = (x << (5u - k)) & 0x1Fu;
    }
    return (k << 10u) + (uint32_t)k_log2_q10_lut[idx] + frac;
}

const char *p7501_block_kind_name(p7501_block_kind_t kind) {
    static const char *const k_names[8] = {
        "ZERO_FILL",
        "ERASED_FF",
        "ASCII_TEXT",
        "MN103S_CODE",
        "COMPRESSED",
        "ENCRYPTED_FEISTEL",
        "HIGH_ENTROPY",
        "STRUCTURED_DATA",
    };
    return ((unsigned)kind < 8u) ? k_names[(unsigned)kind] : "UNKNOWN";
}

static bool has_16bit_byteswap_magic(const uint8_t *buf, size_t len) {
    if (len < 8u) {
        return false;
    }
    for (size_t i = 0u; (i + 8u) <= len; i += 2u) {
        if (memcmp(&buf[i], "XERTDAEH", 8u) == 0) {
            return true;
        }
    }
    return false;
}

bool p7501_math_analyze_block(
    const uint8_t *buf,
    size_t len,
    uint32_t hist256[256],
    p7501_math_block_stats_t *out
) {
    if ((buf == NULL) || (len == 0u) || (hist256 == NULL) || (out == NULL)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    memset(hist256, 0, 256u * sizeof(uint32_t));
    out->feistel_key_idx = -1;
    out->len = (uint32_t)len;
    out->crc32 = p7501_crc32_ieee(buf, len);

    uint32_t ascii_cnt = 0u;
    for (size_t i = 0u; i < len; ++i) {
        const uint8_t b = buf[i];
        hist256[b] += 1u;
        if (((b >= 0x20u) && (b <= 0x7Eu)) || (b == '\r') || (b == '\n') || (b == '\t')) {
            ascii_cnt += 1u;
        }
    }

    const uint32_t n_u32 = (uint32_t)len;
    uint64_t weighted_log = 0ULL;
    for (size_t b = 0u; b < 256u; ++b) {
        const uint32_t c = hist256[b];
        if (c > 0u) {
            weighted_log += (uint64_t)c * (uint64_t)p7501_math_log2_q10(c);
        }
    }
    const uint32_t total_q10 = p7501_math_log2_q10(n_u32);
    const uint32_t avg_log_q10 = (uint32_t)(weighted_log / (uint64_t)n_u32);
    const uint32_t h_q10 = (total_q10 > avg_log_q10) ? (total_q10 - avg_log_q10) : 0u;
    out->entropy_mbit = (h_q10 * 1000u + 512u) >> 10u;

    out->ff_pct = (uint8_t)((hist256[0xFFu] * 100u) / n_u32);
    out->zero_pct = (uint8_t)((hist256[0x00u] * 100u) / n_u32);
    out->ascii_pct = (uint8_t)((ascii_cnt * 100u) / n_u32);
    out->byte_swapped_16 = has_16bit_byteswap_magic(buf, len);

    if (len >= 2u) {
        const uint16_t hdr16 = ((uint16_t)buf[0] << 8u) | (uint16_t)buf[1];
        out->zlib_header = ((buf[0] & 0x0Fu) == 8u) && ((buf[0] >> 4u) <= 7u) &&
                           ((hdr16 % 31u) == 0u);
    }
    if (len >= 8u) {
        uint8_t plain8[8];
        size_t kidx = 0u;
        if (p7501_feistel_autodetect_key(buf, plain8, &kidx)) {
            out->feistel_key_idx = (int8_t)kidx;
        }
    }

    const size_t edges = p7501_mn103_scan_edges(buf, len, 0u, NULL, 0u);
    out->mn103_hits = (edges > 0xFFFFu) ? 0xFFFFu : (uint16_t)edges;

    if (hist256[0xFFu] == n_u32) {
        out->kind = P7501_BLK_ERASED_FF;
    } else if (hist256[0x00u] == n_u32) {
        out->kind = P7501_BLK_EMPTY_ZERO;
    } else if (out->feistel_key_idx >= 0) {
        out->kind = P7501_BLK_ENCRYPTED_FEISTEL;
    } else if (out->zlib_header && (out->entropy_mbit >= 6500u)) {
        out->kind = P7501_BLK_COMPRESSED;
    } else if (out->ascii_pct >= 80u) {
        out->kind = P7501_BLK_ASCII_TEXT;
    } else if ((out->mn103_hits >= 2u) && (out->entropy_mbit >= 4200u) && (out->entropy_mbit <= 6800u)) {
        out->kind = P7501_BLK_MN103S_CODE;
    } else if (out->entropy_mbit >= 7400u) {
        out->kind = P7501_BLK_HIGH_ENTROPY;
    } else {
        out->kind = P7501_BLK_STRUCTURED_DATA;
    }
    return true;
}

uint32_t p7501_math_chi2_q8(const uint32_t hist256[256], uint32_t len) {
    /* Chi^2 requires expected bin frequency >= 1 (len >= 256). */
    if ((hist256 == NULL) || (len < 256u)) {
        return 0u;
    }
    uint64_t sum_sq = 0ULL;
    const int64_t n = (int64_t)len;
    for (size_t b = 0u; b < 256u; ++b) {
        const int64_t diff = ((int64_t)hist256[b] * 256LL) - n;
        sum_sq += (uint64_t)(diff * diff);
    }
    const uint64_t q8 = (sum_sq + (uint64_t)(len >> 1u)) / (uint64_t)len;
    return (q8 > 0xFFFFFFFFULL) ? 0xFFFFFFFFUL : (uint32_t)q8;
}

static bool is_blank_blk8(const uint8_t *blk) {
    const uint8_t b0 = blk[0];
    if ((b0 != 0x00u) && (b0 != 0xFFu)) {
        return false;
    }
    for (size_t i = 1u; i < 8u; ++i) {
        if (blk[i] != b0) {
            return false;
        }
    }
    return true;
}

uint16_t p7501_math_detect_ecb64(const uint8_t *buf, size_t len) {
    if ((buf == NULL) || (len < 16u)) {
        return 0u;
    }
    const size_t nblk = (len / 8u > 512u) ? 512u : (len / 8u);
    uint16_t hits = 0u;
    for (size_t i = 0u; i < nblk; ++i) {
        const uint8_t *const cur = &buf[i * 8u];
        if (is_blank_blk8(cur)) {
            continue;
        }
        for (size_t j = 0u; j < i; ++j) {
            if (memcmp(cur, &buf[j * 8u], 8u) == 0) {
                hits += 1u;
                break;
            }
        }
    }
    return hits;
}

uint32_t p7501_math_detect_addr_mirror(const uint8_t *buf, size_t len) {
    if ((buf == NULL) || (len < 32u)) {
        return 0u;
    }
    uint32_t best = 0u;
    for (uint8_t k = 1u; k <= 5u; ++k) {
        const size_t half = len >> k;
        if (half < 16u) {
            break;
        }
        if ((half * 2u <= len) && (memcmp(buf, &buf[half], half) == 0)) {
            best = (uint32_t)half;
        }
    }
    return best;
}

bool p7501_math_is_crypto_sbox256(const uint8_t win[256]) {
    if (win == NULL) {
        return false;
    }
    uint8_t seen_bm[32] = {0};
    uint16_t id_hits = 0u;
    uint16_t xor_hits = 0u;
    const uint8_t c0 = win[0];
    for (size_t i = 0u; i < 256u; ++i) {
        const uint8_t v = win[i];
        const uint8_t bit = (uint8_t)(1u << (v & 7u));
        if ((seen_bm[v >> 3u] & bit) != 0u) {
            return false;
        }
        seen_bm[v >> 3u] |= bit;
        if ((v == (uint8_t)i) || (v == (uint8_t)(255u - i))) {
            id_hits += 1u;
        }
        if ((uint8_t)(v ^ (uint8_t)i) == c0) {
            xor_hits += 1u;
        }
    }
    return (id_hits < 16u) && (xor_hits < 256u);
}

bool p7501_math_scan_patterns(
    const uint8_t *buf,
    size_t len,
    uint32_t hist256[256],
    p7501_math_pattern_report_t *out
) {
    if ((out == NULL) || !p7501_math_analyze_block(buf, len, hist256, &out->stats)) {
        return false;
    }
    out->chi2_q8 = p7501_math_chi2_q8(hist256, (uint32_t)len);
    out->mirror_period = p7501_math_detect_addr_mirror(buf, len);
    out->ecb64_repeats = p7501_math_detect_ecb64(buf, len);
    out->sbox_256_found = false;
    out->sbox_offset = 0u;
    for (size_t off = 0u; (off + 256u) <= len; off += 4u) {
        if (p7501_math_is_crypto_sbox256(&buf[off])) {
            out->sbox_256_found = true;
            out->sbox_offset = (uint16_t)off;
            break;
        }
    }
    return true;
}
