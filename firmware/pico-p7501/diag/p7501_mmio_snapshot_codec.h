/**
 * @file p7501_mmio_snapshot_codec.h
 * @brief Pure C11 AM34 MMIO register snapshot table & diff codec (`MMIOSNAP`, Feature #7).
 *
 * Strictly excludes the lethal AM34 bus-lock/WDT register (`0xC000100C`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_MMIO_SNAPSHOT_CODEC_H
#define P7501_MMIO_SNAPSHOT_CODEC_H

#include "generated/p7501_ssot.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_MMIO_SNAP_COUNT P7501_SSOT_MMIO_REG_COUNT

typedef p7501_ssot_mmio_reg_t p7501_mmio_reg_desc_t;

typedef struct p7501_mmio_snapshot {
    uint32_t values[P7501_MMIO_SNAP_COUNT];
    bool valid[P7501_MMIO_SNAP_COUNT];
} p7501_mmio_snapshot_t;

/** Read-only catalog of safe AM34 diagnostic registers (NEVER includes `0xC000100C` or `0xDC0F0000`). */
const p7501_mmio_reg_desc_t *p7501_mmio_safe_table(size_t *out_count);

/** Return true iff `addr` is strictly safe to read (blocks `0xC000100C` and `0xDC0F0000`). */
bool p7501_mmio_is_safe_read_addr(uint32_t addr);

/** Compute bitmask of entries whose values changed between `prev` and `curr`. */
uint32_t p7501_mmio_diff_mask(
    const p7501_mmio_snapshot_t *prev,
    const p7501_mmio_snapshot_t *curr
);

/** Parse `MMIO_SNAP [CORE|VIDEO|STORAGE|ALL|DIFF]` (defaults to `CORE`, returns 0 on invalid). */
uint8_t p7501_mmio_parse_domain(const char *arg, bool *out_diff_only);

/** Decode AIOF/AVDD video register fields into `out[0..cap)`. Returns bytes written (0 if N/A). */
size_t p7501_mmio_decode_video_reg(uint32_t addr, uint32_t val, char *out, size_t cap);

/** Hardware read of all registers matching `domain_mask`, pushing `@P7501_FRAME` `type=MMIO`. */
bool p7501_mmio_snap_domain_hw(uint8_t domain_mask, p7501_mmio_snapshot_t *out_snap, uint32_t *out_valid);

/** CLI command `MMIO_SNAP [CORE|VIDEO|STORAGE|ALL|DIFF]`. */
void p7501_cmd_mmio_snap(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_MMIO_SNAPSHOT_CODEC_H */
