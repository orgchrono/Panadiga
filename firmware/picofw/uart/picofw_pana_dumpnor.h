/**
 * @file picofw_pana_dumpnor.h
 * @brief Read-only MN103S `PANADMP1` NOR dumper stub builder and `PANARSP DUMPNOR` codec.
 *
 * Strictly read-only: only permits addresses inside 8 MB cached (`0x80000000..0x80800000`)
 * or uncached (`0x84000000..0x84800000`) Spansion NOR windows. Host-testable pure C11.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PANA_DUMPNOR_H
#define PICOFW_PANA_DUMPNOR_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "../generated/picofw_ssot.h"

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_DUMPNOR_DEFAULT_START PICOFW_SSOT_CS0_UNCACHED_BASE
#define PICOFW_DUMPNOR_DEFAULT_LEN   0x00800000UL
#define PICOFW_DUMPNOR_STUB_ADDR     PICOFW_SSOT_SDRAM_STUB_RAW_ADDR
#define PICOFW_DUMPNOR_DIGADIAG_ADDR PICOFW_SSOT_DIGADIAG_LOAD_ADDR
#define PICOFW_DUMPNOR_STUB_SIZE     162u
#define PICOFW_DUMPNOR_SUM_MAGIC     "PANASUM1"

typedef struct picofw_nor_check_result {
    bool valid;
    bool all_blank;
    uint32_t blocks_checked;
    uint32_t blank_blocks;
    uint32_t digest_crc32;
} picofw_nor_check_result_t;

typedef struct picofw_dumpnor_resume_state {
    bool active;
    uint32_t start_addr;
    uint32_t total_len;
    uint32_t completed_bytes;
    uint32_t block_size;
    uint32_t baud;
} picofw_dumpnor_resume_state_t;

typedef struct picofw_dumpnor_ff_tracker {
    uint32_t block_size;
    uint32_t hdr_skip;
    uint32_t blk_pos;
    uint32_t ff_blocks;
    uint32_t total_blocks;
    bool cur_all_ff;
} picofw_dumpnor_ff_tracker_t;

/** Initialize streaming post-header `0xFF` block tracker (`block_size` = 4096 or 256). */
void picofw_dumpnor_ff_tracker_init(picofw_dumpnor_ff_tracker_t *t, uint32_t block_size);

/** Feed one post-`"PANADMP1"` stream byte into the `0xFF` blank-block counter. */
void picofw_dumpnor_ff_tracker_feed(picofw_dumpnor_ff_tracker_t *t, uint8_t b);

/** Shared verified 24-byte MN103S SC0 PUTC(D3) leaf subroutine (matches `PUTC_D3_BYTES`). */
extern const uint8_t k_picofw_mn103_putc_d3[24];

/** Choose verified stream block size (`4096`, else `256`) or `0` if unaligned/invalid. */
uint32_t picofw_dumpnor_block_size(uint32_t total_len);

/** Return true iff `[start_addr, start_addr + total_len)` is a valid read-only NOR dump range. */
bool picofw_dumpnor_validate_range(uint32_t start_addr, uint32_t total_len);

/** Parse `<start_hex>[,<len_hex>]` for `DUMPNOR:<start>[,<len>]`. */
bool picofw_dumpnor_parse_colon_spec(const char *spec, uint32_t *out_start, uint32_t *out_len);

/** Parse optional `0x` hex token cleanly into `uint32_t`. */
bool picofw_dumpnor_parse_hex_tok(const char *tok, uint32_t *out_val);

/**
 * @brief Build 162-byte position-coupled MN103S `PANADMP1..PANADONE` UART NOR dumper stub.
 *
 * Matches `panadiga_core::re::nor_dumper::build_readonly_nor_dumper_stub` byte-for-byte.
 * Returns `PICOFW_DUMPNOR_STUB_SIZE` on success, or `0` on invalid bounds / buffer too small.
 */
size_t picofw_dumpnor_build_stub(
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Build fast sum32-only NOR digest & blank-check MN103S stub (`PANASUM1`, Feature #2). */
size_t picofw_dumpnor_build_checksum_stub(
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Evaluate streamed `"PANASUM1"` + 4-byte block sums + `"PANADONE"` reply. */
bool picofw_dumpnor_eval_sum_only_reply(
    const uint8_t *buf,
    size_t len,
    uint32_t block_size,
    uint32_t expected_blocks,
    picofw_nor_check_result_t *out
);

/** Verify one streamed DUMPNOR block against its trailing 4-byte LE sum and blank state. */
bool picofw_dumpnor_verify_block(
    const uint8_t *blk,
    size_t blk_len,
    const uint8_t sum_le[4],
    bool *out_all_ff
);

/** Check whether `baud` is an approved high-speed DUMPNOR rate (`230400` or `460800`). */
bool picofw_dumpnor_is_fast_baud(uint32_t baud);

/** Resume-state lifecycle helpers (`FAST_DUMPNOR` / `DUMPNOR_RESUME`, Feature #6). */
bool picofw_dumpnor_resume_begin(
    picofw_dumpnor_resume_state_t *st,
    uint32_t start_addr,
    uint32_t total_len,
    uint32_t baud
);
bool picofw_dumpnor_resume_next_slice(
    const picofw_dumpnor_resume_state_t *st,
    uint32_t *out_addr,
    uint32_t *out_len
);
void picofw_dumpnor_resume_advance(picofw_dumpnor_resume_state_t *st, uint32_t verified_bytes);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PANA_DUMPNOR_H */
