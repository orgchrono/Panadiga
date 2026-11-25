/**
 * @file picofw_decomp.h
 * @brief Pure ISO C11 zero-malloc Panasonic Okumura LZSS & RFC 1950/1951 zlib/DEFLATE engine.
 *
 * Uses caller-supplied `win4k[4096]` and `picofw_inflate_scratch_t` buffers (staged in
 * `g_picofw_cmd_arena.raw` on Pico) so Core-0 stack frames remain strictly below 512 B.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_DECOMP_H
#define PICOFW_DECOMP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_INF_MAXBITS   15u
#define PICOFW_INF_FIXLCODES 288u
#define PICOFW_INF_MAXLCODES 286u
#define PICOFW_INF_MAXDCODES 30u

typedef struct picofw_inf_huff {
    int16_t count[PICOFW_INF_MAXBITS + 1u];
    int16_t symbol[PICOFW_INF_FIXLCODES];
} picofw_inf_huff_t;

typedef struct picofw_inflate_scratch {
    picofw_inf_huff_t lencode;
    picofw_inf_huff_t distcode;
    int16_t lengths[PICOFW_INF_FIXLCODES + PICOFW_INF_MAXDCODES];
} picofw_inflate_scratch_t;

/** Reset Okumura 4 KiB sliding window (`0x00`) and return initial ring cursor `0x0FEE`. */
size_t picofw_lzss_win_init(uint8_t win4k[4096]);

/**
 * @brief Decompress a Panasonic Okumura LZSS (`algo == 2`, `0x0FEE` window) stream.
 */
bool picofw_lzss_decompress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    uint8_t win4k[4096]
);

/** Compress `src[0..src_len)` into Panasonic Okumura LZSS (`algo == 2`, `0x0FEE` window). */
bool picofw_lzss_compress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    uint8_t win4k[4096]
);

/**
 * @brief Decompress raw RFC 1951 DEFLATE blocks (`stored`, `fixed Huffman`, `dynamic Huffman`).
 */
bool picofw_inflate_raw(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    picofw_inflate_scratch_t *scratch
);

/**
 * @brief Decompress RFC 1950 zlib stream and verify big-endian Adler-32 trailer.
 */
bool picofw_zlib_decompress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len,
    picofw_inflate_scratch_t *scratch
);

/** Compress `src[0..src_len)` as RFC 1950 zlib stored stream + big-endian Adler-32 trailer. */
bool picofw_zlib_compress(
    const uint8_t *src,
    size_t src_len,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_len
);

/** Run deterministic Okumura LZSS & RFC 1950 zlib selftest using `arena_12k` (`>= 12288` B). */
bool picofw_decomp_selftest(uint8_t *arena_12k, bool *out_lzss_ok, bool *out_zlib_ok);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_DECOMP_H */
