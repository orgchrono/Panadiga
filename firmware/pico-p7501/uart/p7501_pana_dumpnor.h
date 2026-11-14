/**
 * @file p7501_pana_dumpnor.h
 * @brief Read-only MN103S `PANADMP1` NOR dumper stub builder and `PANARSP DUMPNOR` codec.
 *
 * Strictly read-only: only permits addresses inside 8 MB cached (`0x80000000..0x80800000`)
 * or uncached (`0x84000000..0x84800000`) Spansion NOR windows. Host-testable pure C11.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PANA_DUMPNOR_H
#define P7501_PANA_DUMPNOR_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_DUMPNOR_DEFAULT_START 0x84000000UL
#define P7501_DUMPNOR_DEFAULT_LEN   0x00800000UL
#define P7501_DUMPNOR_STUB_ADDR     0x51800000UL
#define P7501_DUMPNOR_STUB_SIZE     162u

/** Choose verified stream block size (`4096`, else `256`) or `0` if unaligned/invalid. */
uint32_t p7501_dumpnor_block_size(uint32_t total_len);

/** Return true iff `[start_addr, start_addr + total_len)` is a valid read-only NOR dump range. */
bool p7501_dumpnor_validate_range(uint32_t start_addr, uint32_t total_len);

/** Parse `<start_hex>[,<len_hex>]` for `DUMPNOR:<start>[,<len>]`. */
bool p7501_dumpnor_parse_colon_spec(const char *spec, uint32_t *out_start, uint32_t *out_len);

/** Parse optional `0x` hex token cleanly into `uint32_t`. */
bool p7501_dumpnor_parse_hex_tok(const char *tok, uint32_t *out_val);

/**
 * @brief Build 162-byte position-coupled MN103S `PANADMP1..PANADONE` UART NOR dumper stub.
 *
 * Matches `panadiga_core::re::nor_dumper::build_readonly_nor_dumper_stub` byte-for-byte.
 * Returns `P7501_DUMPNOR_STUB_SIZE` on success, or `0` on invalid bounds / buffer too small.
 */
size_t p7501_dumpnor_build_stub(
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_DUMPNOR_H */
