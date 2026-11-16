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

#define P7501_JEDEC_STUB_ADDR     P7501_DUMPNOR_STUB_ADDR
#define P7501_JEDEC_MAX_DATA_LEN  16u
#define P7501_JEDEC_MAX_STUB_SIZE 384u
#define P7501_JEDEC_OK_TOKEN      "PANAJEDEC_OK"
#define P7501_JEDEC_TO_TOKEN      "PANAJEDEC_TO"

/** Validate target sector address under active 2-layer NOR policy; outputs even uncached VA. */
bool p7501_jedec_validate_erase_addr(
    uint32_t addr,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_even
);

/** Validate target write span (`1..=16` bytes) under active 2-layer NOR policy; outputs uncached VA. */
bool p7501_jedec_validate_prog_range(
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked,
    uint32_t *out_uncached_start
);

/** Build position-coupled MN103S 6-cycle Spansion JEDEC Sector Erase SRAM stub; returns size or 0. */
size_t p7501_jedec_build_erase_stub(
    uint32_t target_addr,
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

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_JEDEC_H */
