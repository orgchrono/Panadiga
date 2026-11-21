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

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_MMIO_SNAP_COUNT 8u

typedef struct p7501_mmio_reg_desc {
    uint32_t addr;
    const char *name;
} p7501_mmio_reg_desc_t;

typedef struct p7501_mmio_snapshot {
    uint32_t values[P7501_MMIO_SNAP_COUNT];
    bool valid[P7501_MMIO_SNAP_COUNT];
} p7501_mmio_snapshot_t;

/** Read-only catalog of safe AM34 diagnostic registers (NEVER includes `0xC000100C`). */
const p7501_mmio_reg_desc_t *p7501_mmio_safe_table(size_t *out_count);

/** Return true iff `addr` is strictly safe to read (blocks `0xC000100C` unconditionally). */
bool p7501_mmio_is_safe_read_addr(uint32_t addr);

/** Compute bitmask of entries whose values changed between `prev` and `curr`. */
uint32_t p7501_mmio_diff_mask(
    const p7501_mmio_snapshot_t *prev,
    const p7501_mmio_snapshot_t *curr
);

/** CLI command `MMIOSNAP [DIFF]`. */
void p7501_cmd_mmio_snap(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_MMIO_SNAPSHOT_CODEC_H */
