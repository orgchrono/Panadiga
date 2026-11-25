/**
 * @file picofw_compress.c
 * @brief Pure ISO C11 zero-malloc Panasonic Okumura LZSS & RFC 1950 zlib encoders.
 *
 * Exact algebraic inverse of `picofw_lzss_decompress` and `picofw_zlib_decompress`
 * (`crypto/picofw_decomp.c`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "crypto/picofw_decomp.h"
#include "crypto/picofw_feistel.h"
#include "generated/picofw_ssot.h"

#include <string.h>

static uint8_t lzss_ring_or_lookahead(
    const uint8_t win4k[4096],
    size_t cand_off,
    size_t dist,
    size_t k,
    const uint8_t *src,
    size_t src_i
) {
    if (k < dist) {
        return win4k[(cand_off + k) & (PICOFW_SSOT_LZSS_WINDOW_SIZE - 1u)];
    }
    return src[src_i + k - dist];
}

static void find_best_lzss_match(
    const uint8_t win4k[4096],
    size_t win_pos,
    size_t populated,
    const uint8_t *src,
    size_t src_i,
    size_t src_len,
    size_t *out_off,
    size_t *out_len
) {
    const size_t win_mask = PICOFW_SSOT_LZSS_WINDOW_SIZE - 1u;
    const size_t max_len = ((src_len - src_i) < 18u) ? (src_len - src_i) : 18u;
    const size_t scan_lim = (populated < PICOFW_SSOT_LZSS_WINDOW_SIZE) ? populated : PICOFW_SSOT_LZSS_WINDOW_SIZE;
    size_t best_len = 0u;
    size_t best_off = 0u;

    if (max_len >= 3u) {
        for (size_t dist = 1u; dist < scan_lim; ++dist) {
            const size_t cand = (win_pos - dist) & win_mask;
            if (win4k[cand] != src[src_i]) {
                continue;
            }
            size_t k = 1u;
            while (k < max_len) {
                if (lzss_ring_or_lookahead(win4k, cand, dist, k, src, src_i) != src[src_i + k]) {
                    break;
                }
                ++k;
            }
            if ((k >= 3u) && (k > best_len)) {
                best_len = k;
                best_off = cand;
                if (k == max_len) {
                    break;
                }
            }
        }
    }
    *out_off = best_off;
    *out_len = best_len;
}

bool picofw_lzss_compress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    uint8_t win4k[4096]
) {
    if ((win4k == NULL) || (out_len == NULL)) {
        return false;
    }
    if ((src == NULL) || (dst == NULL) || (src_len == 0u)) {
        return false;
    }
    size_t win_pos = picofw_lzss_win_init(win4k);
    const size_t win_mask = PICOFW_SSOT_LZSS_WINDOW_SIZE - 1u;

    size_t src_i = 0u;
    size_t dst_i = 0u;
    size_t populated = 1u;
    uint8_t blk[17];

    while (src_i < src_len) {
        blk[0] = 0u;
        size_t blk_pos = 1u;
        for (uint8_t bit = 0u; (bit < 8u) && (src_i < src_len); ++bit) {
            size_t m_off = 0u, m_len = 0u;
            find_best_lzss_match(win4k, win_pos, populated, src, src_i, src_len, &m_off, &m_len);
            if (m_len >= 3u) {
                blk[blk_pos++] = (uint8_t)(m_off & 0xFFu);
                blk[blk_pos++] = (uint8_t)(((m_off >> 4u) & 0xF0u) | ((m_len - 3u) & 0x0Fu));
                for (size_t k = 0u; k < m_len; ++k) {
                    win4k[win_pos] = src[src_i++];
                    win_pos = (win_pos + 1u) & win_mask;
                    populated += 1u;
                }
            } else {
                const uint8_t c = src[src_i++];
                blk[0] |= (uint8_t)(1u << bit);
                blk[blk_pos++] = c;
                win4k[win_pos] = c;
                win_pos = (win_pos + 1u) & win_mask;
                populated += 1u;
            }
        }
        if ((dst_i + blk_pos) > dst_cap) {
            return false;
        }
        memcpy(&dst[dst_i], blk, blk_pos);
        dst_i += blk_pos;
    }
    *out_len = dst_i;
    return true;
}

bool picofw_zlib_compress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len
) {
    if ((src == NULL) || (dst == NULL) || (out_len == NULL)) {
        return false;
    }
    if (dst_cap < 6u) {
        return false;
    }
    /* RFC 1950 header: CM=8 (DEFLATE), CINFO=7 (32K), FCHECK makes (0x7801 % 31 == 0) */
    dst[0] = 0x78u;
    dst[1] = 0x01u;
    size_t pos = 2u;
    size_t off = 0u;
    do {
        const size_t rem = src_len - off;
        const size_t chunk = (rem > 65535u) ? 65535u : rem;
        const bool is_last = ((off + chunk) == src_len);
        if ((pos + 5u + chunk + 4u) > dst_cap) {
            return false;
        }
        const uint16_t len16 = (uint16_t)chunk;
        const uint16_t nlen16 = (uint16_t)~len16;
        dst[pos++] = is_last ? 0x01u : 0x00u;
        dst[pos++] = (uint8_t)(len16 & 0xFFu);
        dst[pos++] = (uint8_t)((len16 >> 8u) & 0xFFu);
        dst[pos++] = (uint8_t)(nlen16 & 0xFFu);
        dst[pos++] = (uint8_t)((nlen16 >> 8u) & 0xFFu);
        if (chunk > 0u) {
            memcpy(&dst[pos], &src[off], chunk);
            pos += chunk;
            off += chunk;
        }
    } while (off < src_len);

    const uint32_t adler = picofw_adler32_rfc1950(src, src_len);
    dst[pos++] = (uint8_t)((adler >> 24u) & 0xFFu);
    dst[pos++] = (uint8_t)((adler >> 16u) & 0xFFu);
    dst[pos++] = (uint8_t)((adler >> 8u) & 0xFFu);
    dst[pos++] = (uint8_t)(adler & 0xFFu);
    *out_len = pos;
    return true;
}
