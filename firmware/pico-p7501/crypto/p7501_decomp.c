/**
 * @file p7501_decomp.c
 * @brief Pure ISO C11 zero-malloc Panasonic Okumura LZSS & RFC 1950/1951 zlib/DEFLATE engine.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "crypto/p7501_decomp.h"
#include "crypto/p7501_feistel.h"
#include "generated/p7501_ssot.h"

#include <string.h>

size_t p7501_lzss_win_init(uint8_t win4k[4096]) {
    memset(win4k, 0, P7501_SSOT_LZSS_WINDOW_SIZE);
    return P7501_SSOT_LZSS_WINDOW_INIT;
}

bool p7501_lzss_decompress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    uint8_t win4k[4096]
) {
    if ((src == NULL) || (dst == NULL) || (out_len == NULL) || (win4k == NULL)) {
        return false;
    }
    const size_t win_mask = P7501_SSOT_LZSS_WINDOW_SIZE - 1u;
    size_t win_pos = p7501_lzss_win_init(win4k);

    size_t src_i = 0u;
    size_t dst_i = 0u;
    uint16_t flags = 0u;

    while (src_i < src_len) {
        flags >>= 1u;
        if ((flags & 0x100u) == 0u) {
            if (src_i >= src_len) {
                break;
            }
            flags = (uint16_t)src[src_i++] | 0xFF00u;
        }
        if ((flags & 1u) == 0u) {
            if ((src_i + 1u) >= src_len) {
                break;
            }
            const uint8_t b1 = src[src_i++];
            const uint8_t b2 = src[src_i++];
            size_t off = (size_t)b1 | (((size_t)(b2 & 0xF0u)) << 4u);
            const size_t match_len = ((size_t)(b2 & 0x0Fu)) + 3u;
            for (size_t k = 0u; k < match_len; ++k) {
                if (dst_i >= dst_cap) {
                    return false;
                }
                const uint8_t c = win4k[off];
                dst[dst_i++] = c;
                win4k[win_pos] = c;
                win_pos = (win_pos + 1u) & win_mask;
                off = (off + 1u) & win_mask;
            }
        } else {
            if (src_i >= src_len) {
                break;
            }
            if (dst_i >= dst_cap) {
                return false;
            }
            const uint8_t c = src[src_i++];
            dst[dst_i++] = c;
            win4k[win_pos] = c;
            win_pos = (win_pos + 1u) & win_mask;
        }
    }
    *out_len = dst_i;
    return true;
}

typedef struct inf_state {
    const uint8_t *in;
    size_t in_len;
    size_t in_cnt;
    uint8_t *out;
    size_t out_cap;
    size_t out_cnt;
    uint32_t bitbuf;
    int bitcnt;
} inf_state_t;

static int inf_bits(inf_state_t *s, int need, uint32_t *out_val) {
    uint32_t val = s->bitbuf;
    while (s->bitcnt < need) {
        if (s->in_cnt >= s->in_len) {
            return -1;
        }
        val |= ((uint32_t)s->in[s->in_cnt++]) << (unsigned)s->bitcnt;
        s->bitcnt += 8;
    }
    s->bitbuf = val >> (unsigned)need;
    s->bitcnt -= need;
    *out_val = val & ((1UL << (unsigned)need) - 1UL);
    return 0;
}

static int inf_construct(p7501_inf_huff_t *h, const int16_t *length, int n) {
    int16_t offs[P7501_INF_MAXBITS + 1u];
    for (int len = 0; len <= (int)P7501_INF_MAXBITS; ++len) {
        h->count[len] = 0;
    }
    for (int sym = 0; sym < n; ++sym) {
        h->count[length[sym]] += 1;
    }
    if (h->count[0] == n) {
        return 0;
    }
    int left = 1;
    for (int len = 1; len <= (int)P7501_INF_MAXBITS; ++len) {
        left <<= 1;
        left -= h->count[len];
        if (left < 0) {
            return left;
        }
    }
    offs[1] = 0;
    for (int len = 1; len < (int)P7501_INF_MAXBITS; ++len) {
        offs[len + 1] = (int16_t)(offs[len] + h->count[len]);
    }
    for (int sym = 0; sym < n; ++sym) {
        if (length[sym] != 0) {
            h->symbol[offs[length[sym]]++] = (int16_t)sym;
        }
    }
    return left;
}

static int inf_decode(inf_state_t *s, const p7501_inf_huff_t *h) {
    int code = 0;
    int first = 0;
    int index = 0;
    uint32_t bitbuf = s->bitbuf;
    int left = s->bitcnt;
    int next_len = 1;

    for (int len = 1; len <= (int)P7501_INF_MAXBITS; ++len) {
        if (left == 0) {
            if (s->in_cnt >= s->in_len) {
                return -10;
            }
            bitbuf = (uint32_t)s->in[s->in_cnt++];
            left = 8;
        }
        code |= (int)(bitbuf & 1u);
        bitbuf >>= 1u;
        left -= 1;
        const int count = h->count[next_len++];
        if ((code - count) < first) {
            s->bitbuf = bitbuf;
            s->bitcnt = left;
            return h->symbol[index + (code - first)];
        }
        index += count;
        first = (first + count) << 1;
        code <<= 1;
    }
    return -10;
}

static int inf_stored(inf_state_t *s) {
    s->bitbuf = 0u;
    s->bitcnt = 0;
    if ((s->in_cnt + 4u) > s->in_len) {
        return -1;
    }
    const uint16_t len = (uint16_t)s->in[s->in_cnt] | (uint16_t)((uint16_t)s->in[s->in_cnt + 1u] << 8u);
    const uint16_t nlen = (uint16_t)s->in[s->in_cnt + 2u] | (uint16_t)((uint16_t)s->in[s->in_cnt + 3u] << 8u);
    s->in_cnt += 4u;
    if ((uint16_t)(len ^ nlen) != 0xFFFFu) {
        return -2;
    }
    if ((s->in_cnt + len) > s->in_len || (s->out_cnt + len) > s->out_cap) {
        return -3;
    }
    if (len > 0u) {
        memcpy(&s->out[s->out_cnt], &s->in[s->in_cnt], len);
        s->in_cnt += len;
        s->out_cnt += len;
    }
    return 0;
}

static int inf_codes(inf_state_t *s, const p7501_inf_huff_t *lencode, const p7501_inf_huff_t *distcode) {
    static const uint16_t k_lens[29] = {
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31,
        35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258
    };
    static const uint8_t k_lext[29] = {
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0
    };
    static const uint16_t k_dists[30] = {
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193,
        257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577
    };
    static const uint8_t k_dext[30] = {
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
        7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13
    };

    for (;;) {
        const int sym = inf_decode(s, lencode);
        if (sym < 0) {
            return sym;
        }
        if (sym < 256) {
            if (s->out_cnt >= s->out_cap) {
                return -4;
            }
            s->out[s->out_cnt++] = (uint8_t)sym;
        } else if (sym == 256) {
            return 0;
        } else {
            const int lidx = sym - 257;
            if (lidx >= 29) {
                return -5;
            }
            uint32_t extra = 0u;
            if (inf_bits(s, k_lext[lidx], &extra) != 0) {
                return -6;
            }
            const size_t len = (size_t)k_lens[lidx] + (size_t)extra;
            const int dsym = inf_decode(s, distcode);
            if ((dsym < 0) || (dsym >= 30)) {
                return -7;
            }
            if (inf_bits(s, k_dext[dsym], &extra) != 0) {
                return -8;
            }
            const size_t dist = (size_t)k_dists[dsym] + (size_t)extra;
            if ((dist == 0u) || (dist > s->out_cnt) || ((s->out_cnt + len) > s->out_cap)) {
                return -9;
            }
            for (size_t k = 0u; k < len; ++k) {
                s->out[s->out_cnt] = s->out[s->out_cnt - dist];
                s->out_cnt += 1u;
            }
        }
    }
}

static int inf_fixed(inf_state_t *s, p7501_inflate_scratch_t *sc) {
    int sym = 0;
    for (; sym < 144; ++sym) sc->lengths[sym] = 8;
    for (; sym < 256; ++sym) sc->lengths[sym] = 9;
    for (; sym < 280; ++sym) sc->lengths[sym] = 7;
    for (; sym < 288; ++sym) sc->lengths[sym] = 8;
    (void)inf_construct(&sc->lencode, sc->lengths, (int)P7501_INF_FIXLCODES);
    for (sym = 0; sym < (int)P7501_INF_MAXDCODES; ++sym) sc->lengths[sym] = 5;
    (void)inf_construct(&sc->distcode, sc->lengths, (int)P7501_INF_MAXDCODES);
    return inf_codes(s, &sc->lencode, &sc->distcode);
}

static int inf_dynamic(inf_state_t *s, p7501_inflate_scratch_t *sc) {
    static const uint8_t k_order[19] = {
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15
    };
    uint32_t v = 0u;
    if (inf_bits(s, 5, &v) != 0) return -1;
    const int nlen = (int)v + 257;
    if (inf_bits(s, 5, &v) != 0) return -1;
    const int ndist = (int)v + 1;
    if (inf_bits(s, 4, &v) != 0) return -1;
    const int ncode = (int)v + 4;
    if ((nlen > (int)P7501_INF_MAXLCODES) || (ndist > (int)P7501_INF_MAXDCODES)) {
        return -11;
    }

    int idx = 0;
    for (; idx < ncode; ++idx) {
        if (inf_bits(s, 3, &v) != 0) return -1;
        sc->lengths[k_order[idx]] = (int16_t)v;
    }
    for (; idx < 19; ++idx) {
        sc->lengths[k_order[idx]] = 0;
    }
    if (inf_construct(&sc->lencode, sc->lengths, 19) != 0) {
        return -12;
    }

    idx = 0;
    while (idx < (nlen + ndist)) {
        const int sym = inf_decode(s, &sc->lencode);
        if (sym < 0) return sym;
        if (sym < 16) {
            sc->lengths[idx++] = (int16_t)sym;
        } else {
            int16_t rep_val = 0;
            int rep_bits = 7;
            int rep_base = 11;
            if (sym == 16) {
                if (idx == 0) return -13;
                rep_val = sc->lengths[idx - 1];
                rep_bits = 2;
                rep_base = 3;
            } else if (sym == 17) {
                rep_bits = 3;
                rep_base = 3;
            }
            if (inf_bits(s, rep_bits, &v) != 0) return -1;
            const int rep_len = rep_base + (int)v;
            if ((idx + rep_len) > (nlen + ndist)) return -14;
            for (int r = 0; r < rep_len; ++r) {
                sc->lengths[idx++] = rep_val;
            }
        }
    }
    if (sc->lengths[256] == 0) return -15;
    if (inf_construct(&sc->lencode, sc->lengths, nlen) < 0) return -16;
    if (inf_construct(&sc->distcode, &sc->lengths[nlen], ndist) < 0) return -17;
    return inf_codes(s, &sc->lencode, &sc->distcode);
}

static bool inflate_core(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    size_t *out_consumed,
    p7501_inflate_scratch_t *scratch
) {
    if ((src == NULL) || (dst == NULL) || (out_len == NULL) || (scratch == NULL)) {
        return false;
    }
    inf_state_t st;
    memset(&st, 0, sizeof(st));
    st.in = src;
    st.in_len = src_len;
    st.out = dst;
    st.out_cap = dst_cap;

    uint32_t last = 0u;
    do {
        uint32_t btype = 0u;
        if (inf_bits(&st, 1, &last) != 0 || inf_bits(&st, 2, &btype) != 0) {
            return false;
        }
        int err = -1;
        if (btype == 0u) {
            err = inf_stored(&st);
        } else if (btype == 1u) {
            err = inf_fixed(&st, scratch);
        } else if (btype == 2u) {
            err = inf_dynamic(&st, scratch);
        }
        if (err != 0) {
            return false;
        }
    } while (last == 0u);

    *out_len = st.out_cnt;
    if (out_consumed != NULL) {
        const size_t unused_bytes = (size_t)(st.bitcnt / 8);
        *out_consumed = (st.in_cnt >= unused_bytes) ? (st.in_cnt - unused_bytes) : st.in_cnt;
    }
    return true;
}

bool p7501_inflate_raw(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    p7501_inflate_scratch_t *scratch
) {
    return inflate_core(src, src_len, dst, dst_cap, out_len, NULL, scratch);
}

bool p7501_zlib_decompress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    p7501_inflate_scratch_t *scratch
) {
    if ((src == NULL) || (src_len < 6u) || (dst == NULL) || (out_len == NULL) || (scratch == NULL)) {
        return false;
    }
    const uint8_t cmf = src[0];
    const uint8_t flg = src[1];
    const uint16_t hdr = (uint16_t)((uint16_t)cmf << 8u) | (uint16_t)flg;
    if (((cmf & 0x0Fu) != 8u) || ((cmf >> 4u) > 7u) || ((flg & 0x20u) != 0u) || ((hdr % 31u) != 0u)) {
        return false;
    }
    size_t consumed = 0u;
    size_t dec_len = 0u;
    if (!inflate_core(&src[2], src_len - 2u, dst, dst_cap, &dec_len, &consumed, scratch)) {
        return false;
    }
    const size_t adler_off = 2u + consumed;
    if ((adler_off + 4u) > src_len) {
        return false;
    }
    const uint32_t expected_adler = ((uint32_t)src[adler_off] << 24u) |
                                    ((uint32_t)src[adler_off + 1u] << 16u) |
                                    ((uint32_t)src[adler_off + 2u] << 8u) |
                                    (uint32_t)src[adler_off + 3u];
    if (p7501_adler32_rfc1950(dst, dec_len) != expected_adler) {
        return false;
    }
    *out_len = dec_len;
    return true;
}

bool p7501_decomp_selftest(uint8_t *arena_12k, bool *out_lzss_ok, bool *out_zlib_ok) {
    /* Verified Okumura LZSS literal 4-byte stream: flag=0x0F + "PANA" */
    static const uint8_t k_lzss_vec[5] = { 0x0Fu, 'P', 'A', 'N', 'A' };
    /* Verified RFC 1950 zlib stored stream for "PANADIGA" (len=8, adler32=0x0A220236) */
    static const uint8_t k_zlib_vec[19] = {
        0x78u, 0x01u, 0x01u, 0x08u, 0x00u, 0xF7u, 0xFFu,
        'P', 'A', 'N', 'A', 'D', 'I', 'G', 'A',
        0x0Au, 0x22u, 0x02u, 0x36u
    };
    if (arena_12k == NULL) {
        return false;
    }
    uint8_t *const dst = arena_12k;
    uint8_t *const win = &arena_12k[4096u];
    p7501_inflate_scratch_t *const sc = (p7501_inflate_scratch_t *)(void *)&arena_12k[8192u];

    size_t l_out = 0u;
    size_t z_out = 0u;
    const bool lz_ok = p7501_lzss_decompress(k_lzss_vec, sizeof(k_lzss_vec), dst, 64u, &l_out, win) &&
                       (l_out == 4u) && (memcmp(dst, "PANA", 4u) == 0);
    const bool zl_ok = p7501_zlib_decompress(k_zlib_vec, sizeof(k_zlib_vec), dst, 64u, &z_out, sc) &&
                       (z_out == 8u) && (memcmp(dst, "PANADIGA", 8u) == 0);
    static const uint8_t k_rt[17] = "PANADIGA_PANAPAN";
    uint8_t *const enc_buf = &arena_12k[64u];
    size_t enc_len = 0u, dec_len = 0u;
    const bool lz_rt = p7501_lzss_compress(k_rt, 16u, enc_buf, 64u, &enc_len, win) &&
                       p7501_lzss_decompress(enc_buf, enc_len, dst, 64u, &dec_len, win) &&
                       (dec_len == 16u) && (memcmp(dst, k_rt, 16u) == 0);
    const bool zl_rt = p7501_zlib_compress(k_rt, 16u, enc_buf, 64u, &enc_len) &&
                       p7501_zlib_decompress(enc_buf, enc_len, dst, 64u, &dec_len, sc) &&
                       (dec_len == 16u) && (memcmp(dst, k_rt, 16u) == 0);
    if (out_lzss_ok != NULL) {
        *out_lzss_ok = lz_ok && lz_rt;
    }
    if (out_zlib_ok != NULL) {
        *out_zlib_ok = zl_ok && zl_rt;
    }
    return lz_ok && zl_ok && lz_rt && zl_rt;
}
