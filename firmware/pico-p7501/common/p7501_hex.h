/**
 * @file p7501_hex.h
 * @brief Shared pure C11 hexadecimal and ASCII case-folding utilities (SSOT/DRY).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_HEX_H
#define P7501_HEX_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Convert a single ASCII hex character ('0'-'9', 'a'-'f', 'A'-'F') to nibble (0..15) or -1. */
int p7501_hex_char_to_nibble(char c);

/** Convert a 4-bit nibble (0..15) to lowercase ASCII hex character ('0'..'f'). */
char p7501_nibble_to_hex_char(uint8_t nibble);

/** Case-insensitive equality for null-terminated ASCII strings. */
bool p7501_ascii_ieq(const char *a, const char *b);

/** Case-insensitive prefix check for null-terminated ASCII strings. */
bool p7501_ascii_iprefix(const char *str, const char *prefix);

/** Return true if `str` is non-empty and consists solely of ASCII decimal digits `'0'..'9'`. */
bool p7501_ascii_is_digits(const char *str);

/** Strictly parse an unsigned base-10 integer within `[0, UINT32_MAX]` with whitespace trim only. */
bool p7501_parse_u32_strict(const char *str, uint32_t *out_val);

/** Strictly parse an unsigned 32-bit integer supporting `0x`/`0X` hex or base-10 decimal. */
bool p7501_parse_u32_auto(const char *str, uint32_t *out_val);

/** Parse leading hex token (`0x` optional, <= 32 bits) and return unconsumed tail via `out_rest`. */
bool p7501_parse_hex_u32_prefix(const char *str, uint32_t *out_val, const char **out_rest);

/** Copy ASCII string up to `dst_cap - 1` chars converting lowercase letters to uppercase. */
void p7501_ascii_upper_copy(char *dst, size_t dst_cap, const char *src);

/**
 * @brief Decode ASCII hex byte stream (spaces, ':', '-', '_' delimiters permitted).
 *
 * Rejects empty inputs, non-hex characters, odd nibble counts, and buffer overflows.
 */
bool p7501_decode_hex_bytes(
    const char *hex,
    uint8_t *out_bytes,
    size_t max_bytes,
    size_t *out_len
);

/** Compute standard 8-bit mod-256 RSP frame checksum over raw byte span. */
uint8_t p7501_rsp_checksum(const uint8_t *data, size_t len);

/** Compute standard 8-bit mod-256 RSP frame checksum over a null-terminated string. */
uint8_t p7501_rsp_checksum_str(const char *str);

#ifdef __cplusplus
}
#endif

#endif /* P7501_HEX_H */
