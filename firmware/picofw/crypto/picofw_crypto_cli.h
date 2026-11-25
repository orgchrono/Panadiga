/**
 * @file picofw_crypto_cli.h
 * @brief CLI handlers `CRYPTO` and `DECOMP` routing on-Pico Feistel, LZSS, zlib, and Adler-32.
 *
 * All large scratch/input/output buffers reside inside `g_picofw_cmd_arena.raw` and
 * results push deterministically via `picofw_push_emit_str` + `picofw_push_flush_global`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_CRYPTO_CLI_H
#define PICOFW_CRYPTO_CLI_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI `CRYPTO <SELFTEST|KEYS|DEC <k|hex> <hex>|ENC <k|hex> <hex>|AUTODETECT <hex16>|ADLER32 <stride> <hex>>`. */
void picofw_cmd_crypto(const char *arg);

/** CLI `DECOMP <SELFTEST|LZSS <hex>|ZLIB <hex>|RAW <hex>>`. */
void picofw_cmd_decomp(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_CRYPTO_CLI_H */
