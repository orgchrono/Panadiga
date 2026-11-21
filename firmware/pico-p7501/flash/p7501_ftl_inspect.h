/**
 * @file p7501_ftl_inspect.h
 * @brief Pure C11 BGO & EEMUL NOR flash FTL structural inspector (`BGO_INSPECT`, `EEMUL_INSPECT`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_FTL_INSPECT_H
#define P7501_FTL_INSPECT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_BGO_HEADER_BYTES      20u
#define P7501_BGO_MAX_SECTORS       64u
#define P7501_EEMUL_MAX_MICROSECS   16u
#define P7501_EEMUL_MICROSEC_BYTES  8192u

typedef enum p7501_bgo_sec_state {
    P7501_BGO_SEC_ERASED = 0,
    P7501_BGO_SEC_VALID = 1,
    P7501_BGO_SEC_CORRUPT = 2,
    P7501_BGO_SEC_DUPLICATE_LBN = 3
} p7501_bgo_sec_state_t;

typedef struct p7501_bgo_report {
    uint16_t total_sectors;
    uint16_t erased_count;
    uint16_t valid_count;
    uint16_t corrupt_count;
    uint16_t duplicate_lbn_count;
    bool healthy_bgo_ftl;
    uint8_t states[P7501_BGO_MAX_SECTORS];
    uint16_t lbns[P7501_BGO_MAX_SECTORS];
} p7501_bgo_report_t;

typedef struct p7501_eemul_report {
    uint16_t total_microsectors;
    uint16_t blank_sectors;
    uint16_t active_sectors;
    uint16_t dirty_non_ff_bytes_pct;
    uint32_t crc32;
    bool healthy_eemul;
} p7501_eemul_report_t;

/** Inspect `sector_count` consecutive 16-byte BGO sector headers (`SA26..SA70`). */
bool p7501_bgo_inspect_headers(
    const uint8_t *headers_16b,
    size_t sector_count,
    p7501_bgo_report_t *out
);

/** Inspect an EEMUL microsector slice (`8 KiB` units, e.g. `64 KiB` or `8 KiB` headers). */
bool p7501_eemul_inspect_slice(
    const uint8_t *data,
    size_t total_bytes,
    size_t microsector_bytes,
    p7501_eemul_report_t *out
);

/** CLI command handlers reading target via `p7501_hw_read_mem32`. */
void p7501_cmd_bgo_inspect(const char *arg);
void p7501_cmd_eemul_inspect(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_FTL_INSPECT_H */
