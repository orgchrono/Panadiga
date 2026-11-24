/**
 * @file p7501_sector_ops.h
 * @brief Deterministic NOR sector inspector, signature scanner, Feistel key locator & bit-clear patcher (`SECTOR`).
 *
 * Host-testable C11 core + Pico hardware CLI executor (`@P7501_FRAME` staged).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_SECTOR_OPS_H
#define P7501_SECTOR_OPS_H

#include "flash/p7501_payloads.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct p7501_sector_info {
    uint32_t query_addr;
    uint32_t uncached_addr;
    uint32_t sector_base;
    uint32_t sector_size;
    uint32_t offset_in_sector;
    int32_t sa_index;
    const char *zone;
    p7501_trap_audit_result_t write_audit;
} p7501_sector_info_t;

typedef struct p7501_sector_patch_plan {
    bool allowed;
    bool bitclear_monotonic;
    bool needs_erase;
    size_t first_bad_offset;
    p7501_trap_audit_result_t audit;
} p7501_sector_patch_plan_t;

/** Describe NOR sector geometry, zone classification, and write policy for `addr`. */
bool p7501_sector_describe(
    const p7501_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    bool upper_unlocked,
    p7501_sector_info_t *out
);

/** Scan `buf[0..buf_len)` for exact matches of `needle[0..needle_len)`; returns hit count. */
size_t p7501_sig_find_in_buf(
    const uint8_t *buf,
    size_t buf_len,
    uint32_t base_va,
    const uint8_t *needle,
    size_t needle_len,
    uint32_t *out_hits,
    size_t max_hits
);

/** Validate trap/policy gate and NOR 1->0 bit-clear monotonicity for a planned patch. */
bool p7501_sector_plan_patch(
    const p7501_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    bool upper_unlocked,
    uint32_t target_va,
    const uint8_t *old_bytes,
    const uint8_t *new_bytes,
    size_t len,
    p7501_sector_patch_plan_t *out_plan
);

/** CLI `SECTOR <INFO <addr>|READ <addr> [len]|FIND <SIG|hex> [start] [len]|PATCH <addr> <hex> [CONFIRM]>`. */
void p7501_cmd_sector(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_SECTOR_OPS_H */
