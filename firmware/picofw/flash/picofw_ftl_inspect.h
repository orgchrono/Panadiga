/**
 * @file picofw_ftl_inspect.h
 * @brief Pure C11 BGO & EEMUL NOR flash FTL structural inspector (`BGO_INSPECT`, `EEMUL_INSPECT`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_FTL_INSPECT_H
#define PICOFW_FTL_INSPECT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_BGO_HEADER_BYTES      20u
#define PICOFW_BGO_MAX_SECTORS       64u
#define PICOFW_EEMUL_MAX_MICROSECS   16u
#define PICOFW_EEMUL_MICROSEC_BYTES  8192u

typedef enum picofw_bgo_sec_state {
    PICOFW_BGO_SEC_ERASED = 0,
    PICOFW_BGO_SEC_VALID = 1,
    PICOFW_BGO_SEC_CORRUPT = 2,
    PICOFW_BGO_SEC_DUPLICATE_LBN = 3
} picofw_bgo_sec_state_t;

typedef struct picofw_bgo_report {
    uint16_t total_sectors;
    uint16_t erased_count;
    uint16_t valid_count;
    uint16_t corrupt_count;
    uint16_t duplicate_lbn_count;
    uint16_t p1_sa_index;
    bool healthy_bgo_ftl;
    uint8_t states[PICOFW_BGO_MAX_SECTORS];
    uint16_t lbns[PICOFW_BGO_MAX_SECTORS];
} picofw_bgo_report_t;

typedef enum picofw_norfrom_slot_state {
    PICOFW_NORFROM_ERASED_FF = 0,
    PICOFW_NORFROM_AUTOBOOT_CAPSULE = 1,
    PICOFW_NORFROM_EXTRHEAD = 2,
    PICOFW_NORFROM_NON_EMPTY = 3
} picofw_norfrom_slot_state_t;

typedef struct picofw_eemul_report {
    uint16_t total_microsectors;
    uint16_t blank_sectors;
    uint16_t active_sectors;
    uint16_t dirty_non_ff_bytes_pct;
    uint32_t crc32;
    bool healthy_eemul;
} picofw_eemul_report_t;

/** Inspect `sector_count` consecutive 16-byte BGO sector headers (`SA26..SA70`). */
bool picofw_bgo_inspect_headers(
    const uint8_t *headers_16b,
    size_t sector_count,
    picofw_bgo_report_t *out
);

/** Inspect an EEMUL microsector slice (`8 KiB` units, e.g. `64 KiB` or `8 KiB` headers). */
bool picofw_eemul_inspect_slice(
    const uint8_t *data,
    size_t total_bytes,
    size_t microsector_bytes,
    picofw_eemul_report_t *out
);

/** Return canonical ASCII label for `picofw_norfrom_slot_state_t`. */
const char *picofw_norfrom_state_name(picofw_norfrom_slot_state_t st);

/** Classify NOR autoboot capsule header (`256 B` at `SA11 @ 0x84040000`) & compute CRC32. */
picofw_norfrom_slot_state_t picofw_norfrom_classify_prefix(
    const uint8_t *buf,
    size_t len,
    uint32_t *out_crc32
);

/** CLI command handlers reading target via `picofw_hw_read_mem32`. */
void picofw_cmd_bgo_inspect(const char *arg);
void picofw_cmd_eemul_inspect(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_FTL_INSPECT_H */
