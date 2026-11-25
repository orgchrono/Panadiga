/**
 * @file picofw_feistel.c
 * @brief Pure ISO C11 Matsushita MN103S 64-bit Feistel cipher, 12-key autodetect & Adler-32.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "crypto/picofw_feistel.h"
#include "generated/picofw_ssot.h"

#include <string.h>

static uint8_t rotl8(uint8_t v, unsigned bits) {
    return (uint8_t)(((v << bits) | (v >> (8u - bits))) & 0xFFu);
}

static uint8_t rotr8(uint8_t v, unsigned bits) {
    return (uint8_t)(((v >> bits) | (v << (8u - bits))) & 0xFFu);
}

void picofw_feistel_f_round(const uint8_t blk4[4], const uint8_t key4[4], uint8_t out4[4]) {
    const uint8_t v3 = (uint8_t)(key4[1] ^ blk4[1]);
    const uint8_t v4 = (uint8_t)(key4[0] ^ blk4[0]);
    const uint8_t x3 = (uint8_t)(key4[3] ^ blk4[3]);
    const uint8_t x2 = (uint8_t)(key4[2] ^ blk4[2]);

    const uint8_t v1 = (uint8_t)(x3 >> 2u);
    const uint8_t v5 = rotr8(x3, 2u);
    const uint8_t v2 = (uint8_t)(x2 >> 5u);
    const uint8_t v6 = rotr8(x2, 5u);

    const uint8_t o3 = (uint8_t)((v6 >> 1u) | (uint8_t)(v1 << 7u));
    const uint8_t o2 = (uint8_t)((v3 & 0x7Fu) | (uint8_t)(v2 << 7u));
    const uint8_t o1 = (uint8_t)((v4 >> 1u) | (v3 & 0x80u));
    const uint8_t o0 = (uint8_t)((v5 >> 1u) | (uint8_t)(v4 << 7u));

    const uint8_t s_idx = (uint8_t)(v4 ^ rotl8(v3, 1u) ^ v6 ^ v5);
    const uint8_t s = k_picofw_ssot_feistel_sbox[s_idx];

    out4[0] = (uint8_t)(o0 + rotl8(s, 3u));
    out4[1] = (uint8_t)(o1 + rotl8(s, 2u));
    out4[2] = (uint8_t)(o2 + rotl8(s, 1u));
    out4[3] = (uint8_t)(o3 + s);
}

void picofw_feistel_derive_rk_7b(const uint8_t key7[7], const uint8_t iv4[4], uint8_t rk32[32]) {
    memset(rk32, 0, 32u);
    memcpy(rk32, key7, 7u);
    uint8_t state[4];
    memcpy(state, iv4, 4u);

    for (size_t round = 0u; round < 16u; ++round) {
        const size_t off = (round & 7u) * 4u;
        uint8_t next[4];
        picofw_feistel_f_round(&rk32[off], state, next);
        memcpy(&rk32[off], next, 4u);
        memcpy(state, next, 4u);
    }
}

void picofw_feistel_derive_rk_8b(const uint8_t key8[8], uint8_t rk32[32]) {
    uint8_t key7[7];
    memcpy(key7, key8, 7u);
    key7[1] ^= key8[7];
    key7[4] ^= key8[7];
    picofw_feistel_derive_rk_7b(key7, k_picofw_ssot_feistel_iv, rk32);
}

void picofw_feistel_crypt_block(const uint8_t in8[8], const uint8_t rk32[32], bool decrypt, uint8_t out8[8]) {
    uint8_t l[4];
    uint8_t r[4];
    memcpy(l, &in8[0], 4u);
    memcpy(r, &in8[4], 4u);

    for (size_t step = 0u; step < 16u; ++step) {
        const size_t round = decrypt ? (15u - step) : step;
        const size_t k_off = (round & 7u) * 4u;
        uint8_t f_out[4];
        picofw_feistel_f_round(l, &rk32[k_off], f_out);
        uint8_t prev_l[4];
        memcpy(prev_l, l, 4u);
        for (size_t i = 0u; i < 4u; ++i) {
            l[i] = (uint8_t)(f_out[i] ^ r[i]);
        }
        memcpy(r, prev_l, 4u);
    }

    memcpy(&out8[0], r, 4u);
    memcpy(&out8[4], l, 4u);
}

void picofw_feistel_crypt_buffer(uint8_t *buf, size_t len, const uint8_t rk32[32], bool decrypt) {
    if ((buf == NULL) || (rk32 == NULL)) {
        return;
    }
    const size_t aligned = len & ~(size_t)7u;
    for (size_t off = 0u; off < aligned; off += 8u) {
        uint8_t out[8];
        picofw_feistel_crypt_block(&buf[off], rk32, decrypt, out);
        memcpy(&buf[off], out, 8u);
    }
}

void picofw_feistel_crypt_edges(uint8_t *buf, size_t len, size_t span, const uint8_t rk32[32], bool encrypt) {
    const size_t aligned_span = span & ~(size_t)7u;
    const size_t aligned_len = len & ~(size_t)7u;
    if ((buf == NULL) || (rk32 == NULL) || (aligned_span == 0u) || (aligned_len == 0u)) {
        return;
    }
    const bool is_decrypt = !encrypt;
    if (aligned_len <= (2u * aligned_span)) {
        picofw_feistel_crypt_buffer(buf, aligned_len, rk32, is_decrypt);
    } else {
        picofw_feistel_crypt_buffer(buf, aligned_span, rk32, is_decrypt);
        picofw_feistel_crypt_buffer(&buf[aligned_len - aligned_span], aligned_span, rk32, is_decrypt);
    }
}

static bool is_known_decrypted_header(const uint8_t blk8[8]) {
    static const char *const k_magics[] = {
        "EXTRHEAD",
        "Firm lis",
        "PANA",
        "MAIN",
        "DIAG",
        "DRV ",
    };
    for (size_t i = 0u; i < (sizeof(k_magics) / sizeof(k_magics[0])); ++i) {
        const size_t mlen = strlen(k_magics[i]);
        if (memcmp(blk8, k_magics[i], mlen) == 0) {
            return true;
        }
    }
    return false;
}

bool picofw_feistel_autodetect_key(const uint8_t blk8[8], uint8_t out_plain8[8], size_t *out_key_idx) {
    if ((blk8 == NULL) || (out_plain8 == NULL)) {
        return false;
    }
    for (size_t k = 0u; k < PICOFW_SSOT_FEISTEL_KEY_COUNT; ++k) {
        uint8_t rk[32];
        picofw_feistel_derive_rk_8b(k_picofw_ssot_feistel_keys[k].key8, rk);
        uint8_t cand[8];
        picofw_feistel_crypt_block(blk8, rk, true, cand);
        if (is_known_decrypted_header(cand)) {
            memcpy(out_plain8, cand, 8u);
            if (out_key_idx != NULL) {
                *out_key_idx = k;
            }
            return true;
        }
    }
    return false;
}

static uint32_t adler32_core(const uint8_t *data, size_t len, size_t stride, bool rfc1950_order) {
    if (data == NULL) {
        return 1u;
    }
    const uint32_t mod_adler = 65521UL;
    const size_t step = (stride == 0u) ? 1u : stride;
    uint32_t s1 = 1UL;
    uint32_t s2 = 0UL;
    for (size_t i = 0u; i < len; i += step) {
        s1 = (s1 + (uint32_t)data[i]) % mod_adler;
        s2 = (s2 + s1) % mod_adler;
    }
    return rfc1950_order ? ((s2 << 16u) | s1) : (s1 | (s2 << 16u));
}

uint32_t picofw_adler32_strided(const uint8_t *data, size_t len, size_t stride) {
    return adler32_core(data, len, stride, false);
}

uint32_t picofw_adler32_rfc1950(const uint8_t *data, size_t len) {
    return adler32_core(data, len, 1u, true);
}

bool picofw_feistel_selftest(void) {
    uint8_t rk[32];
    picofw_feistel_derive_rk_8b(k_picofw_ssot_feistel_keys[0].key8, rk);
    const uint8_t plain[8] = { 'E', 'X', 'T', 'R', 'H', 'E', 'A', 'D' };
    uint8_t enc[8];
    uint8_t dec[8];
    picofw_feistel_crypt_block(plain, rk, false, enc);
    picofw_feistel_crypt_block(enc, rk, true, dec);
    uint8_t auto_plain[8];
    size_t kidx = 99u;
    return (memcmp(plain, enc, 8u) != 0) &&
           (memcmp(plain, dec, 8u) == 0) &&
           picofw_feistel_autodetect_key(enc, auto_plain, &kidx) &&
           (kidx == 0u) &&
           (memcmp(auto_plain, plain, 8u) == 0) &&
           (picofw_adler32_strided(plain, 8u, 1u) != 0u);
}
