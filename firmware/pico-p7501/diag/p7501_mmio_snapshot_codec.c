/**
 * @file p7501_mmio_snapshot_codec.c
 * @brief Pure C11 AM34 MMIO safe snapshot table, diff comparator & video bitfield decoder (`MMIOSNAP`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_mmio_snapshot_codec.h"
#include "common/p7501_hex.h"
#include "flash/p7501_bus_policy.h"
#include "p7501_types.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

const p7501_mmio_reg_desc_t *p7501_mmio_safe_table(size_t *out_count) {
    if (out_count != NULL) {
        *out_count = P7501_MMIO_SNAP_COUNT;
    }
    return k_p7501_ssot_mmio_regs;
}

bool p7501_mmio_is_safe_read_addr(uint32_t addr) {
    return !p7501_bus_is_lethal_mmio_range(addr, 4u);
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

uint8_t p7501_mmio_parse_domain(const char *arg, bool *out_diff_only) {
    if (out_diff_only != NULL) {
        *out_diff_only = false;
    }
    char copy[64];
    char *toks[2] = { NULL, NULL };
    const size_t n = p7501_copy_and_split_ws(arg, copy, sizeof(copy), toks, 2u);
    if (n == 0u) {
        return P7501_SSOT_MMIO_DOM_CORE;
    }
    uint8_t dom = 0u;
    for (size_t i = 0u; i < n; ++i) {
        if (p7501_ascii_ieq(toks[i], "CORE")) {
            dom |= P7501_SSOT_MMIO_DOM_CORE;
        } else if (p7501_ascii_ieq(toks[i], "VIDEO")) {
            dom |= P7501_SSOT_MMIO_DOM_VIDEO;
        } else if (p7501_ascii_ieq(toks[i], "STORAGE")) {
            dom |= P7501_SSOT_MMIO_DOM_STORAGE;
        } else if (p7501_ascii_ieq(toks[i], "ALL")) {
            dom |= P7501_SSOT_MMIO_DOM_ALL;
        } else if (p7501_ascii_ieq(toks[i], "DIFF")) {
            if (out_diff_only != NULL) {
                *out_diff_only = true;
            }
        } else {
            return 0u;
        }
    }
    return (dom != 0u) ? dom : P7501_SSOT_MMIO_DOM_CORE;
}

size_t p7501_mmio_decode_video_reg(uint32_t addr, uint32_t val, char *out, size_t cap) {
    if ((out == NULL) || (cap == 0u)) {
        return 0u;
    }
    out[0] = '\0';
    const uint32_t lo16 = val & 0xFFFFUL;
    const uint32_t hi16 = (val >> 16u) & 0xFFFFUL;
    int n = 0;
    switch (addr) {
        case (P7501_SSOT_AIOF_BASE + 0x00UL): {
            const char *mode = "CUSTOM";
            if (lo16 == 0x8020UL) {
                mode = "NTSC";
            } else if (lo16 == 0x8021UL) {
                mode = "PAL-M";
            } else if (lo16 == 0x8027UL) {
                mode = "PAL";
            }
            n = snprintf(out, cap, "mode=%s(0x%04" PRIX32 ")", mode, lo16);
            break;
        }
        case (P7501_SSOT_AIOF_BASE + 0x2CUL):
            n = snprintf(out, cap, "fs0=%s(0x%04" PRIX32 ")",
                         (lo16 == 0xC0F0UL) ? "NONSTD_TBC" : ((lo16 == 0x40F0UL) ? "STD" : "CUSTOM"),
                         lo16);
            break;
        case (P7501_SSOT_AIOF_BASE + 0x64UL):
            n = snprintf(out, cap, "line_tbc=%u clk_start=%u",
                         ((lo16 & 0x0020UL) != 0u) ? 1u : 0u,
                         ((hi16 & 0x0001UL) != 0u) ? 1u : 0u);
            break;
        case (P7501_SSOT_AIOF_BASE + 0xA4UL):
            n = snprintf(out, cap, "setup=%s",
                         ((lo16 & 0x4000UL) != 0u) ? "0IRE" :
                         (((lo16 & 0x8000UL) != 0u) ? "7.5IRE" : "RAW"));
            break;
        case (P7501_SSOT_AVDD_BASE + 0x0404UL):
        case (P7501_SSOT_AVDD_BASE + 0x0484UL):
        case (P7501_SSOT_AVDD_BASE + 0x0624UL):
        case (P7501_SSOT_AVDD_BASE + 0x0858UL):
            n = snprintf(out, cap, "lo16=0x%04" PRIX32, lo16);
            break;
        default:
            return 0u;
    }
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}
