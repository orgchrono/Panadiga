/**
 * @file picofw_pana_rsp_internal.h
 * @brief Internal shared ISO C11 codec helpers shared between `picofw_pana_rsp_codec.c` and `picofw_pana_rsp_parse.c`.
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PANA_RSP_INTERNAL_H
#define PICOFW_PANA_RSP_INTERNAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "uart/picofw_pana_rsp.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Parse up to `max_chars` (optional `0x` prefix + <=8 hex digits) into `*out_val`. */
bool picofw_rsp_parse_hex_u32(const char *str, size_t max_chars, uint32_t *out_val);

/** Parse a non-null-terminated span of `len` hex characters into `*out_val`. */
bool picofw_rsp_parse_hex_u32_span(const char *start, size_t len, uint32_t *out_val);

/** Encode `value_le` as 4 little-endian bytes. */
void picofw_rsp_encode_u32_le(uint32_t value_le, uint8_t out_le[4]);

/** Build an unframed `$M` payload (`M<addr>,<len>:<hex>`) into `out_payload`. */
bool picofw_rsp_build_write_mem_payload(
    uint32_t addr,
    const uint8_t *data,
    size_t len,
    char *out_payload,
    size_t cap
);

/** Validate `PANARSP RAW:` / `HEX:` bytes against E05, lethal MMIO `0xC000100C`, and F14 rules. */
bool picofw_rsp_validate_raw_or_hex_guard(
    const uint8_t *buf,
    size_t len,
    bool eff_unlocked,
    bool is_raw_text,
    bool *out_e05
);

/** Decode a single `PANARSP` action token (e.g. `MEM:`, `WMEM:`, `JEDEC_*`, `DUMPNOR`). */
bool picofw_rsp_apply_action_token(const char *tok, picofw_pana_rsp_spec_t *out, bool *saw_action);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PANA_RSP_INTERNAL_H */
