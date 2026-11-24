/**
 * @file p7501_pana_jedec.h
 * @brief MN103S SRAM JEDEC Sector Erase / Byte-Program stub builder (`PANARSP JEDEC_ERASE` / `JEDEC_PROG`).
 *
 * Host-testable ISO C11 module reusing centralized 2-layer Spansion write policy (`SA0..SA3` always E05).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PANA_JEDEC_H
#define P7501_PANA_JEDEC_H

#include "uart/p7501_pana_dumpnor.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_JEDEC_STUB_ADDR        P7501_DUMPNOR_STUB_ADDR
#define P7501_JEDEC_MAX_DATA_LEN     16u
#define P7501_JEDEC_MAX_STUB_SIZE    384u
#define P7501_JEDEC_ERASE_POLL_ITERS 0x01000000UL
#define P7501_JEDEC_PROG_POLL_ITERS  0x00100000UL
#define P7501_JEDEC_OK_TOKEN         "PANAJEDEC_OK"
#define P7501_JEDEC_TO_TOKEN         "PANAJEDEC_TO"

/** Validate target sector address for explicit flash aperture (`2/4/8/16 MiB`); outputs even uncached VA. */
bool p7501_jedec_validate_erase_addr_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_even
);

/** Validate target sector address under active flash aperture & 2-layer NOR policy; outputs even uncached VA. */
bool p7501_jedec_validate_erase_addr(
    uint32_t addr,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_even
);

/** Validate target write span (`1..=16` bytes) for explicit flash aperture; outputs uncached VA. */
bool p7501_jedec_validate_prog_range_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_start
);

/** Validate target write span (`1..=16` bytes) under active flash aperture & 2-layer NOR policy. */
bool p7501_jedec_validate_prog_range(
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_start
);

/** Build position-coupled MN103S 6-cycle Spansion JEDEC Sector Erase SRAM stub for explicit flash aperture. */
size_t p7501_jedec_build_erase_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t target_addr,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Build position-coupled MN103S 6-cycle Spansion JEDEC Sector Erase SRAM stub; returns size or 0. */
size_t p7501_jedec_build_erase_stub(
    uint32_t target_addr,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/**
 * Build NVRAM-only multi-sector Spansion JEDEC Erase SRAM stub for explicit flash aperture.
 * Validates every sector against EEMUL-only policy (`SA0..SA10` & Upper NOR blocked).
 */
size_t p7501_jedec_build_erase_range_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t start_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Build NVRAM-only multi-sector Spansion JEDEC Erase SRAM stub under active flash aperture. */
size_t p7501_jedec_build_erase_range_stub(
    uint32_t start_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Build position-coupled MN103S 4-cycle Spansion JEDEC Program SRAM stub for explicit flash aperture. */
size_t p7501_jedec_build_prog_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Build position-coupled MN103S 4-cycle Spansion JEDEC Program SRAM stub (`1..=16` bytes); returns size or 0. */
size_t p7501_jedec_build_prog_stub(
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

#define P7501_JEDEC_ID_MAGIC  "PANAJID1"
#define P7501_CFI_QUERY_MAGIC "PANACFI1"

typedef struct p7501_jedec_id_reply {
    bool valid;
    bool is_cfi;
    uint16_t mfg;
    uint16_t dev[3];
    uint8_t qry[3];
    uint32_t size_bytes;
} p7501_jedec_id_reply_t;

/** Build MN103S stub querying JEDEC Autoselect (`0x90`) or CFI (`0x98`) halfwords via SC0 (Feature #1). */
size_t p7501_jedec_build_autoselect_stub(
    bool is_cfi,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Build autoselect/CFI stub with selectable GDB halt (`BRA +0`) or `PANAMON1>` caller return (`MOVM; RETS`). */
size_t p7501_jedec_build_autoselect_stub_ex(
    bool is_cfi,
    bool return_to_caller,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

/** Parse streamed `"PANAJID1"` or `"PANACFI1"` + 8-byte LE reply + `"PANADONE"`. */
bool p7501_jedec_parse_autoselect_reply(
    const uint8_t *buf,
    size_t len,
    bool is_cfi,
    p7501_jedec_id_reply_t *out
);

/** Verify decoded JEDEC ID matches expected SSOT flash chip (`false` on mismatch). */
bool p7501_jedec_verify_chip_match(uint16_t mfg, const uint16_t dev[3], uint32_t flash_size_bytes);

/** Validate NOR 1->0 bit-clear operation (`BGO_BITCLEAR`, Feature #5). */
bool p7501_jedec_validate_bitclear(
    const uint8_t *expected_old,
    const uint8_t *desired_new,
    size_t len
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_JEDEC_H */
