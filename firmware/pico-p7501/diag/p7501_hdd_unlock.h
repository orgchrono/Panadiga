/**
 * @file p7501_hdd_unlock.h
 * @brief Deterministic zero-hardcode ATA IDENTIFY decoder, MEIHFS full-disk planner & RTSD capacity unlocker.
 *
 * Host-testable ISO C11 (`<= 230 LOC`), driven purely from Rust SSOT constants.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_HDD_UNLOCK_H
#define P7501_HDD_UNLOCK_H

#include "generated/p7501_ssot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct p7501_ata_ident {
    uint16_t word0;
    bool is_atapi;
    bool lba48_supported;
    uint32_t lba28_sectors;
    uint64_t effective_sectors;
    uint64_t mirror_fw_lba;
    uint32_t capacity_mib;
    uint32_t target_blocks_2kb;
    char model[44];
    char serial[24];
    char fw_rev[12];
} p7501_ata_ident_t;

typedef struct p7501_hdd_full_plan {
    uint64_t total_lba;
    uint64_t mirror_fw_lba;
    uint64_t data_start_lba;
    uint32_t primary_fw_lba;
    uint32_t fw_sectors;
    uint32_t usable_blocks_2kb;
    uint32_t target_rtsd_blocks_2kb;
    bool valid;
} p7501_hdd_full_plan_t;

typedef struct p7501_hdd_patch_op {
    uint32_t va;
    uint8_t len;
    uint8_t data[4];
} p7501_hdd_patch_op_t;

typedef struct p7501_hdd_unlock_plan {
    uint32_t site_va;
    uint32_t target_blocks_2kb;
    uint32_t prev_200gb_2kb;
    uint8_t prev_branch_op;
    bool valid;
    p7501_hdd_patch_op_t ops[4];
} p7501_hdd_unlock_plan_t;

bool p7501_ata_parse_identify(
    const uint8_t *raw,
    size_t len,
    bool is_davis_pio_1024,
    p7501_ata_ident_t *out
);

bool p7501_hdd_compute_full_plan(
    uint64_t total_lba,
    uint32_t fw_bytes,
    p7501_hdd_full_plan_t *out
);

bool p7501_hdd_find_clamp_site(
    const uint8_t *buf,
    size_t len,
    size_t *out_off
);

bool p7501_hdd_plan_rtsd_unlock(
    uint32_t site_va,
    const uint8_t win[P7501_SSOT_HDD_CLAMP_WIN_LEN],
    uint32_t target_blocks_2kb,
    p7501_hdd_unlock_plan_t *out
);

bool p7501_hdd_build_stage_tbus(
    uint8_t stage_idx,
    uint8_t out_vfd16[16],
    uint8_t out_wdt16[16],
    const char **out_label
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_HDD_UNLOCK_H */
