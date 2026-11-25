/**
 * @file picofw_sector_ops.h
 * @brief Deterministic NOR sector inspector, signature scanner, Feistel key locator & bit-clear patcher (`SECTOR`).
 *
 * Host-testable C11 core + Pico hardware CLI executor (`@PICO_FRAME` staged).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_SECTOR_OPS_H
#define PICOFW_SECTOR_OPS_H

#include "flash/picofw_payloads.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct picofw_sector_info {
    uint32_t query_addr;
    uint32_t uncached_addr;
    uint32_t sector_base;
    uint32_t sector_size;
    uint32_t offset_in_sector;
    int32_t sa_index;
    const char *zone;
    picofw_trap_audit_result_t write_audit;
} picofw_sector_info_t;

typedef struct picofw_sector_patch_plan {
    bool allowed;
    bool bitclear_monotonic;
    bool needs_erase;
    size_t first_bad_offset;
    picofw_trap_audit_result_t audit;
} picofw_sector_patch_plan_t;

/** Describe NOR sector geometry, zone classification, and write policy for `addr`. */
bool picofw_sector_describe(
    const picofw_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    bool upper_unlocked,
    picofw_sector_info_t *out
);

/** Scan `buf[0..buf_len)` for exact matches of `needle[0..needle_len)`; returns hit count. */
size_t picofw_sig_find_in_buf(
    const uint8_t *buf,
    size_t buf_len,
    uint32_t base_va,
    const uint8_t *needle,
    size_t needle_len,
    uint32_t *out_hits,
    size_t max_hits
);

/** Validate trap/policy gate and NOR 1->0 bit-clear monotonicity for a planned patch. */
bool picofw_sector_plan_patch(
    const picofw_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    bool upper_unlocked,
    uint32_t target_va,
    const uint8_t *old_bytes,
    const uint8_t *new_bytes,
    size_t len,
    picofw_sector_patch_plan_t *out_plan
);

/** CLI `SECTOR <INFO <addr>|READ <addr> [len]|FIND <SIG|hex> [start] [len]|PATCH <addr> <hex> [CONFIRM]>`. */
void picofw_cmd_sector(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_SECTOR_OPS_H */
