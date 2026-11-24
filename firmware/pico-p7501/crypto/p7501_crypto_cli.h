/**
 * @file p7501_crypto_cli.h
 * @brief CLI handlers `CRYPTO` and `DECOMP` routing on-Pico Feistel, LZSS, zlib, and Adler-32.
 *
 * All large scratch/input/output buffers reside inside `g_p7501_cmd_arena.raw` and
 * results push deterministically via `p7501_push_emit_str` + `p7501_push_flush_global`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_CRYPTO_CLI_H
#define P7501_CRYPTO_CLI_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI `CRYPTO <SELFTEST|KEYS|DEC <k|hex> <hex>|ENC <k|hex> <hex>|AUTODETECT <hex16>|ADLER32 <stride> <hex>>`. */
void p7501_cmd_crypto(const char *arg);

/** CLI `DECOMP <SELFTEST|LZSS <hex>|ZLIB <hex>|RAW <hex>>`. */
void p7501_cmd_decomp(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_CRYPTO_CLI_H */
