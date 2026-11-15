/**
 * @file p7501_pana_rsp_internal.h
 * @brief Internal shared ISO C11 codec helpers shared between `p7501_pana_rsp_codec.c` and `p7501_pana_rsp_parse.c`.
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PANA_RSP_INTERNAL_H
#define P7501_PANA_RSP_INTERNAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Parse up to `max_chars` (optional `0x` prefix + <=8 hex digits) into `*out_val`. */
bool p7501_rsp_parse_hex_u32(const char *str, size_t max_chars, uint32_t *out_val);

/** Parse a non-null-terminated span of `len` hex characters into `*out_val`. */
bool p7501_rsp_parse_hex_u32_span(const char *start, size_t len, uint32_t *out_val);

/** Encode `value_le` as 4 little-endian bytes. */
void p7501_rsp_encode_u32_le(uint32_t value_le, uint8_t out_le[4]);

/** Build an unframed `$M` payload (`M<addr>,<len>:<hex>`) into `out_payload`. */
bool p7501_rsp_build_write_mem_payload(
    uint32_t addr,
    const uint8_t *data,
    size_t len,
    char *out_payload,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_RSP_INTERNAL_H */
