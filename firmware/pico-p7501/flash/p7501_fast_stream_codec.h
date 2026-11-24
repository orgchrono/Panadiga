/**
 * @file p7501_fast_stream_codec.h
 * @brief Pure C11 MN103S fast binary stub (`0x9043A000`), pre-flight `1->0` guard,
 *        `@P7501_PROG` event formatter, and 142-sector `NOR_MAP` classifier.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_FAST_STREAM_CODEC_H
#define P7501_FAST_STREAM_CODEC_H

#include "flash/p7501_bus_policy.h"
#include "generated/p7501_ssot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_FAST_STUB_LOAD_VA     0x9043A000UL
#define P7501_FAST_STUB_BUF_VA      0x9043B000UL
#define P7501_FAST_STUB_LEN         606u
#define P7501_FAST_STUB_MAX_CHUNK   4096u
#define P7501_FAST_STUB_PAGE_BYTES  P7501_SSOT_FAST_STUB_PAGE_BYTES
#define P7501_MAX_FAST_WRITE_BYTES  P7501_SSOT_MAX_FAST_WRITE_BYTES
#define P7501_FAST_STUB_CRC32       0x3D2D3D4AUL
#define P7501_FAST_BGO_START_VA     0x84130000UL
#define P7501_FAST_BGO_SPAN_BYTES   0x002D0000UL
#define P7501_NORMAP_SECTOR_COUNT   142u

typedef enum p7501_normap_kind {
    P7501_NORMAP_IPL = 0,
    P7501_NORMAP_NORFROM_BLANK = 1,
    P7501_NORMAP_NORFROM_USED = 2,
    P7501_NORMAP_FE_EXTRHEAD = 3,
    P7501_NORMAP_FE_DATA = 4,
    P7501_NORMAP_BGO_P1 = 5,
    P7501_NORMAP_BGO_P2 = 6,
    P7501_NORMAP_BGO_SPARE = 7,
    P7501_NORMAP_BGO_CORRUPT = 8,
    P7501_NORMAP_EEMUL_ACTIVE_1284 = 9,
    P7501_NORMAP_EEMUL_SUPERSEDED_1284 = 10,
    P7501_NORMAP_EEMUL_EMPTY_2C = 11,
    P7501_NORMAP_EEMUL_SPARE_2D = 12,
    P7501_NORMAP_EEMUL_OTHER = 13,
    P7501_NORMAP_NVRAM_ACTIVE = 14,
    P7501_NORMAP_NVRAM_BLANK = 15,
    P7501_NORMAP_UNKNOWN = 16
} p7501_normap_kind_t;

typedef struct p7501_normap_summary {
    uint16_t total_sectors;
    uint32_t ipl_crc32;
    bool fe_extrhead_ok;
    uint16_t norfrom_used;
    uint16_t bgo_p1;
    uint16_t bgo_p2;
    uint16_t bgo_spares;
    uint16_t bgo_corrupt;
    uint16_t eemul_active_1284;
    uint16_t eemul_superseded_1284;
    uint16_t eemul_empty_2c;
    uint16_t eemul_spare_2d;
    uint16_t eemul_other;
    uint16_t nvram_active;
    uint16_t nvram_blank;
    bool healthy;
} p7501_normap_summary_t;

typedef struct p7501_fast_page_window {
    uint32_t page_base;
    uint32_t rel_off;
    uint32_t span_pages;
} p7501_fast_page_window_t;

/** Verified 606-byte MN103S binary fast-BGO stub (`'P'`, `'R'`, `'W'`, `'Q'`). */
extern const uint8_t k_p7501_fast_bgo_stub[P7501_FAST_STUB_LEN];

/** IEEE 802.3 CRC-32 of `k_p7501_fast_bgo_stub` (matches `P7501_FAST_STUB_CRC32`). */
uint32_t p7501_fast_stub_crc32(void);

/** Compute 8-bit sum checksum used by the `0x9043A000` `'R'` and `'W'` wire packets. */
uint8_t p7501_fast_sum8(const uint8_t *data, size_t len);

/**
 * Validate `FAST_WRITE` target against single-authoritative `p7501_bus_check_access`
 * (permanent `SA0..SA10` `E05` RO + F14 wireless lockout + `CONFIRM_UPPER_NOR`)
 * and the `0x9043A000` stub's BGO halfword alignment constraints.
 */
p7501_bus_deny_t p7501_fast_check_write_guard(
    const p7501_session_ctx_t *ctx,
    uint32_t addr,
    uint32_t len,
    bool upper_nor_unlocked,
    const char **out_reason
);

/**
 * Validate `FAST_ERASE` sector VA against single-authoritative `p7501_bus_check_access`
 * (permanent `SA0..SA10` `E05` RO + F14 lockout + `CONFIRM_UPPER_NOR`) plus 64 KiB alignment
 * within the BGO FTL window (`0x84130000..0x843FFFFF`).
 */
p7501_bus_deny_t p7501_fast_check_erase_guard(
    const p7501_session_ctx_t *ctx,
    uint32_t sector_va,
    bool upper_nor_unlocked,
    const char **out_reason
);

/**
 * On-Pico pre-flight NOR monotonicity validator (`1 -> 0` transitions only).
 * Returns `false` and records the first offending byte offset in `out_bad_off`
 * if any bit requires `0 -> 1` (sector erase mandatory).
 */
bool p7501_fast_check_monotonic_1_to_0(
    const uint8_t *cur,
    const uint8_t *tgt,
    size_t len,
    size_t *out_bad_off
);

/** Format a structured `@P7501_PROG` event-driven push telemetry line. */
size_t p7501_fast_format_prog_line(
    char *out,
    size_t cap,
    const char *phase,
    uint16_t sa,
    uint32_t done,
    uint32_t total,
    uint32_t crc32,
    uint32_t vdd_mv
);

/** Map physical Spansion `S29JL064H` sector index `sa` (`0..141`) to `(va, size_bytes)`. */
bool p7501_normap_sector_span(uint16_t sa, uint32_t *out_va, uint32_t *out_size);

/** Classify one physical sector (`SA0..SA141`) from its 20-byte header and optional 16-byte `+0x100` zmarker. */
p7501_normap_kind_t p7501_normap_classify_sector(
    uint16_t sa,
    const uint8_t *hdr20,
    const uint8_t *z16
);

/** Accumulate one classified sector into a 142-sector `NOR_MAP` summary. */
void p7501_normap_accumulate(p7501_normap_summary_t *sum, p7501_normap_kind_t kind);

/** Finalize `healthy` verdict after accumulating all `142` sectors. */
void p7501_normap_finalize(p7501_normap_summary_t *sum);

/** Map AM34 cached (`0x80000000..`) or uncached (`0x84000000..`) VA to `SA0..SA141` (`0xFFFF` if invalid). */
uint16_t p7501_fast_va_to_sa(uint32_t va);

/** Encode 7-byte `'R'` packet (`'R' + u32_le(page_va) + u16_le(page_count)`). Returns `7` or `0`. */
size_t p7501_fast_encode_read_pkt(uint32_t page_va, uint16_t page_count, uint8_t out7[7]);

/** Encode 7-byte `'W'` header (`'W' + u32_le(addr) + u16_le(pay_len)`). Returns `7` or `0`. */
size_t p7501_fast_encode_write_hdr(uint32_t addr, uint16_t pay_len, uint8_t out7[7]);

/** Compute 256-byte page window (`page_base`, `rel_off`, `span_pages`) for `[addr, addr + pay_len)`. */
bool p7501_fast_compute_page_window(uint32_t addr, uint32_t pay_len, p7501_fast_page_window_t *out);

/** Format a deterministic `[NOR_MAP]` summary line (without trailing newline). Returns written length. */
size_t p7501_fast_format_normap_line(char *out, size_t cap, const p7501_normap_summary_t *sum);

/** Encode `page[0..page_len)` into uppercase/lowercase ASCII hex (`2 * page_len` chars + NUL). */
size_t p7501_fast_format_page_hex(const uint8_t *page, size_t page_len, char *out_hex, size_t out_cap);

/** Pure chip-capability descriptor used by CLI help (`HELP`). */
const char *p7501_describe_mcu_host(bool is_rp2350, bool has_ble);

/** Pure chip-capability descriptor used by GDB RSP (`qPanadigaInfo`). */
const char *p7501_describe_gdb_target(bool is_rp2350, bool is_riscv);

#ifdef __cplusplus
}
#endif

#endif /* P7501_FAST_STREAM_CODEC_H */
