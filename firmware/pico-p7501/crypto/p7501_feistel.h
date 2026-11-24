/**
 * @file p7501_feistel.h
 * @brief Pure ISO C11 Matsushita MN103S 64-bit Feistel cipher, 12-key autodetect & Adler-32.
 *
 * Mirrors `crates/panadiga-crypto/src/feistel.rs` bit-for-bit using `.rodata` tables
 * exported by Rust SSOT (`k_p7501_ssot_feistel_sbox`, `k_p7501_ssot_feistel_iv`,
 * `k_p7501_ssot_feistel_keys`). Zero malloc, small stack frames (< 512 B).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_FEISTEL_H
#define P7501_FEISTEL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Evaluate the 32-bit Matsushita Feistel F-function (`FUN_44012191`). */
void p7501_feistel_f_round(const uint8_t blk4[4], const uint8_t key4[4], uint8_t out4[4]);

/** Expand a 7-byte master key and 4-byte IV into the 32-byte round-key table (`FUN_44011d42`). */
void p7501_feistel_derive_rk_7b(const uint8_t key7[7], const uint8_t iv4[4], uint8_t rk32[32]);

/** Expand an 8-byte XORed key using SSOT `k_p7501_ssot_feistel_iv` (`FUN_29f055dc`). */
void p7501_feistel_derive_rk_8b(const uint8_t key8[8], uint8_t rk32[32]);

/** Encrypt or decrypt a single 8-byte ECB Feistel block. */
void p7501_feistel_crypt_block(const uint8_t in8[8], const uint8_t rk32[32], bool decrypt, uint8_t out8[8]);

/** Encrypt or decrypt all complete 8-byte blocks in `buf[0 .. len & ~7]` in-place. */
void p7501_feistel_crypt_buffer(uint8_t *buf, size_t len, const uint8_t rk32[32], bool decrypt);

/** Encrypt or decrypt leading + trailing `span` edges without double-transforming middle overlap. */
void p7501_feistel_crypt_edges(uint8_t *buf, size_t len, size_t span, const uint8_t rk32[32], bool encrypt);

/** Try all 12 SSOT Feistel keys against 8-byte encrypted header `blk8`; returns true on magic match. */
bool p7501_feistel_autodetect_key(const uint8_t blk8[8], uint8_t out_plain8[8], size_t *out_key_idx);

/** Matsushita strided Adler-32 checksum (`FUN_4401e5a3`, packed `s1 | (s2 << 16)`). */
uint32_t p7501_adler32_strided(const uint8_t *data, size_t len, size_t stride);

/** Standard RFC 1950 big-endian-order Adler-32 value (`(s2 << 16) | s1`, `stride = 1`). */
uint32_t p7501_adler32_rfc1950(const uint8_t *data, size_t len);

/** Run built-in deterministic Feistel roundtrip, 12-key autodetect & strided Adler-32 selftest. */
bool p7501_feistel_selftest(void);

#ifdef __cplusplus
}
#endif

#endif /* P7501_FEISTEL_H */
