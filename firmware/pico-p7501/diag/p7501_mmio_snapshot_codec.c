/**
 * @file p7501_mmio_snapshot_codec.c
 * @brief Pure C11 AM34 MMIO safe snapshot table and diff comparator (`MMIOSNAP`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_mmio_snapshot_codec.h"
#include "p7501_types.h"
#include <string.h>

static const p7501_mmio_reg_desc_t k_safe_mmio_regs[P7501_MMIO_SNAP_COUNT] = {
    { 0xD4002008UL, "SC0TXB" },
    { (uint32_t)P7501_SSOT_AM34_SC0_CTR_ADDR, "SC0CTR" },
    { 0xD400200CUL, "SC0STR" },
    { 0xC0001000UL, "OCD_CTRL" },
    { 0xC0001008UL, "OCD_STAT" },
    { 0x10000000UL, "SYS_ID" },
    { 0x515B2E40UL, "SER_CB" },
    { 0x5316AE8CUL, "ISR_CTX" },
};

const p7501_mmio_reg_desc_t *p7501_mmio_safe_table(size_t *out_count) {
    if (out_count != NULL) {
        *out_count = P7501_MMIO_SNAP_COUNT;
    }
    return k_safe_mmio_regs;
}

bool p7501_mmio_is_safe_read_addr(uint32_t addr) {
    if ((addr >= (uint32_t)P7501_SSOT_AM34_LETHAL_REG_ADDR) &&
        (addr < ((uint32_t)P7501_SSOT_AM34_LETHAL_REG_ADDR + (uint32_t)P7501_SSOT_AM34_LETHAL_REG_LEN))) {
        return false;
    }
    return true;
}

uint32_t p7501_mmio_diff_mask(
    const p7501_mmio_snapshot_t *prev,
    const p7501_mmio_snapshot_t *curr
) {
    if ((prev == NULL) || (curr == NULL)) {
        return 0u;
    }
    uint32_t mask = 0u;
    for (size_t i = 0u; i < P7501_MMIO_SNAP_COUNT; ++i) {
        if (prev->valid[i] && curr->valid[i] && (prev->values[i] != curr->values[i])) {
            mask |= (1u << i);
        }
    }
    return mask;
}
