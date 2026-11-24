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

/** Parse null-terminated hex token (`0x` optional, <= `max_chars` & <= 8 hex digits). */
bool p7501_parse_hex_u32_exact(const char *str, size_t max_chars, uint32_t *out_val);

/** Scan leading hex token (`0x` optional, <= 8 hex digits) inside `buf[start..len]` and return consumed length. */
size_t p7501_scan_hex_u32(const char *buf, size_t len, size_t start, uint32_t *out_val);

/** Parse an exact non-null-terminated slice `start[0..len]` as hex u32 (`0x` optional). */
bool p7501_parse_hex_u32_span(const char *start, size_t len, uint32_t *out_val);

/** Parse hex digits until `delim` (e.g. `,`, `:`, `\0`) and return pointer right after `delim` (or NULL). */
const char *p7501_parse_hex_delim(
    const char *str,
    uint32_t *out_val,
    char delim,
    const char **out_rest
);

/** Format a single byte for readable ASCII console logs (returns chars written excluding NUL). */
size_t p7501_format_ascii_byte(
    uint8_t b,
    char left_delim,
    char right_delim,
    char *out_buf,
    size_t cap
);

/** Print a single byte (`'\r'` suppressed, printable/newline raw, otherwise escaped with delimiters). */
void p7501_print_ascii_byte(uint8_t b, char left_delim, char right_delim);

/** Print a captured byte buffer as printable ASCII + `<HH>` control escapes. */
void p7501_uart_print_ascii_stream(const uint8_t *buf, uint32_t len);

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

/** Update running IEEE 802.3 CRC-32 (`seed_crc` starts at `0u` for a fresh stream). */
uint32_t p7501_crc32_ieee_update(uint32_t seed_crc, const uint8_t *data, size_t len);

/** Compute IEEE 802.3 CRC-32 (`0xEDB88320`) over `data[0..len]` cleanly via compact nibble LUT. */
uint32_t p7501_crc32_ieee(const uint8_t *data, size_t len);

/** Return true when `buf[0..len]` is non-empty and consists exclusively of erased `0xFF` bytes. */
bool p7501_buf_is_all_ff(const uint8_t *buf, size_t len);

/** Count non-erased (`!= 0xFF`) payload bytes across `buf[0..len]`. */
uint32_t p7501_buf_count_non_ff(const uint8_t *buf, size_t len);

/** Encode 32-bit unsigned value as 4 little-endian bytes. */
void p7501_write_u32_le(uint8_t out[4], uint32_t v);

/** Decode 4 little-endian bytes into `uint32_t`. */
uint32_t p7501_read_u32_le(const uint8_t in[4]);

/** Format integer millivolts `mv` as ASCII `"V.cc"` (e.g. `3295` -> `"3.30"`), zero float linkage. */
const char *p7501_fmt_mv_volts(uint32_t mv, char buf[8]);

/** Format a 256-bit opcode bitmap (`bitmap[32]`) as sorted uppercase hex CSV (`"A9,F9"`) or `"NONE"`. */
uint32_t p7501_format_hex_bitmap_csv(const uint8_t bitmap[32], char *out, size_t cap);

/** Split mutable whitespace-separated ASCII tokens in `buf` into `out_toks[0..max_toks)`. */
size_t p7501_split_ws_tokens(char *buf, char *out_toks[], size_t max_toks);

/** Bounded-copy `arg` into `scratch` and split whitespace tokens (returns `0` when empty). */
size_t p7501_copy_and_split_ws(
    const char *arg,
    char *scratch,
    size_t scratch_cap,
    char *out_toks[],
    size_t max_toks
);

/** Accumulate printable ASCII byte into `buf`; terminate on CR/LF and return true to flush line. */
bool p7501_line_accum_push_byte(
    char *buf,
    size_t *io_len,
    size_t cap,
    uint8_t byte
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_HEX_H */
