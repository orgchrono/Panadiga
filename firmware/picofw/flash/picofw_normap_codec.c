/**
 * @file picofw_normap_codec.c
 * @brief Pure C11 142-sector Spansion S29JL064H `NOR_MAP` geometry, classifier, and line formatter.
 *
 * Split from `picofw_fast_stream_codec.c` (`picofw_fast_stream_codec.c` owns the SRAM fast stub & packet codec).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_fast_stream_codec.h"
#include "flash/picofw_flash.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static bool bytes_all_eq(const uint8_t *buf, size_t n, uint8_t val) {
    for (size_t i = 0u; i < n; ++i) {
        if (buf[i] != val) {
            return false;
        }
    }
    return true;
}

static uint16_t le16_at(const uint8_t *p) {
    return (uint16_t)p[0] | (uint16_t)((uint16_t)p[1] << 8u);
}

bool picofw_normap_sector_span(uint16_t sa, uint32_t *out_va, uint32_t *out_size) {
    if ((sa >= PICOFW_NORMAP_SECTOR_COUNT) || (out_va == NULL) || (out_size == NULL)) {
        return false;
    }
    if (sa < 8u) {
        *out_va = PICOFW_FLASH_HW_UNCACHED_BASE + ((uint32_t)sa * 0x2000UL);
        *out_size = 0x2000UL;
    } else if (sa < 134u) {
        *out_va = PICOFW_FLASH_HW_UNCACHED_BASE + 0x10000UL + ((uint32_t)(sa - 8u) * 0x10000UL);
        *out_size = 0x10000UL;
    } else {
        *out_va = PICOFW_FLASH_HW_UNCACHED_BASE + 0x7F0000UL + ((uint32_t)(sa - 134u) * 0x2000UL);
        *out_size = 0x2000UL;
    }
    return true;
}

picofw_normap_kind_t picofw_normap_classify_sector(
    uint16_t sa,
    const uint8_t *hdr20,
    const uint8_t *z16
) {
    if ((sa >= PICOFW_NORMAP_SECTOR_COUNT) || (hdr20 == NULL)) {
        return PICOFW_NORMAP_UNKNOWN;
    }
    if (sa <= 10u) {
        return PICOFW_NORMAP_IPL;
    }
    if (sa <= 14u) {
        return bytes_all_eq(hdr20, 20u, 0xFFu) ? PICOFW_NORMAP_NORFROM_BLANK : PICOFW_NORMAP_NORFROM_USED;
    }
    if (sa <= 25u) {
        if (sa == 15u) {
            return (memcmp(hdr20, "EXTRHEADDRV ", 12u) == 0)
                ? PICOFW_NORMAP_FE_EXTRHEAD : PICOFW_NORMAP_UNKNOWN;
        }
        return PICOFW_NORMAP_FE_DATA;
    }
    if (sa <= 70u) {
        const uint16_t w0 = le16_at(&hdr20[4]);
        const bool rep_ok = (le16_at(&hdr20[8]) == w0) &&
                            (le16_at(&hdr20[12]) == w0) &&
                            (le16_at(&hdr20[16]) == w0);
        const bool z_ok = (z16 == NULL) || bytes_all_eq(z16, 16u, 0x00u);
        if (!rep_ok || !z_ok) {
            return PICOFW_NORMAP_BGO_CORRUPT;
        }
        if (w0 == 0xFFFFu) {
            return bytes_all_eq(hdr20, 20u, 0xFFu) ? PICOFW_NORMAP_BGO_SPARE : PICOFW_NORMAP_BGO_CORRUPT;
        }
        if (w0 == 0xF101u) {
            return PICOFW_NORMAP_BGO_P1;
        }
        if ((w0 >= 0xF201u) && (w0 <= 0xF23Fu)) {
            return PICOFW_NORMAP_BGO_P2;
        }
        return PICOFW_NORMAP_BGO_CORRUPT;
    }
    if (sa <= 133u) {
        static const uint8_t k_eemul_pfx[4] = { 0x0Fu, 0xF0u, 0x00u, 0x00u };
        if (memcmp(hdr20, k_eemul_pfx, 4u) == 0) {
            const uint8_t s4 = hdr20[4];
            const uint8_t s5 = hdr20[5];
            const bool is_1284 = (memcmp(&hdr20[6], "1284", 4u) == 0);
            if ((s4 == 0x00u) && (s5 == 0x00u) && is_1284) {
                return PICOFW_NORMAP_EEMUL_ACTIVE_1284;
            }
            if ((s4 == 0x00u) && (s5 == 0x2Cu) && is_1284) {
                return PICOFW_NORMAP_EEMUL_SUPERSEDED_1284;
            }
            if ((s4 == 0x00u) && (s5 == 0x2Cu) && (hdr20[6] == 0xFFu) && (hdr20[7] == 0xFFu)) {
                return PICOFW_NORMAP_EEMUL_EMPTY_2C;
            }
            if ((s4 == 0x00u) && (s5 == 0x2Du) && (hdr20[6] == 0xFFu) && (hdr20[7] == 0xFFu)) {
                return PICOFW_NORMAP_EEMUL_SPARE_2D;
            }
        }
        return PICOFW_NORMAP_EEMUL_OTHER;
    }
    return bytes_all_eq(hdr20, 16u, 0xFFu) ? PICOFW_NORMAP_NVRAM_BLANK : PICOFW_NORMAP_NVRAM_ACTIVE;
}

void picofw_normap_accumulate(picofw_normap_summary_t *sum, picofw_normap_kind_t kind) {
    if (sum == NULL) {
        return;
    }
    sum->total_sectors += 1u;
    switch (kind) {
        case PICOFW_NORMAP_FE_EXTRHEAD: sum->fe_extrhead_ok = true; break;
        case PICOFW_NORMAP_NORFROM_USED: sum->norfrom_used += 1u; break;
        case PICOFW_NORMAP_BGO_P1: sum->bgo_p1 += 1u; break;
        case PICOFW_NORMAP_BGO_P2: sum->bgo_p2 += 1u; break;
        case PICOFW_NORMAP_BGO_SPARE: sum->bgo_spares += 1u; break;
        case PICOFW_NORMAP_BGO_CORRUPT: sum->bgo_corrupt += 1u; break;
        case PICOFW_NORMAP_EEMUL_ACTIVE_1284: sum->eemul_active_1284 += 1u; break;
        case PICOFW_NORMAP_EEMUL_SUPERSEDED_1284: sum->eemul_superseded_1284 += 1u; break;
        case PICOFW_NORMAP_EEMUL_EMPTY_2C: sum->eemul_empty_2c += 1u; break;
        case PICOFW_NORMAP_EEMUL_SPARE_2D: sum->eemul_spare_2d += 1u; break;
        case PICOFW_NORMAP_EEMUL_OTHER: sum->eemul_other += 1u; break;
        case PICOFW_NORMAP_NVRAM_ACTIVE: sum->nvram_active += 1u; break;
        case PICOFW_NORMAP_NVRAM_BLANK: sum->nvram_blank += 1u; break;
        default: break;
    }
}

void picofw_normap_finalize(picofw_normap_summary_t *sum) {
    if (sum == NULL) {
        return;
    }
    sum->healthy = (sum->total_sectors == PICOFW_NORMAP_SECTOR_COUNT) &&
                   sum->fe_extrhead_ok &&
                   (sum->bgo_p1 == 1u) &&
                   (sum->bgo_p2 == 40u) &&
                   (sum->bgo_spares >= 1u) &&
                   (sum->bgo_corrupt == 0u) &&
                   (sum->nvram_blank >= 1u);
}

size_t picofw_fast_format_normap_line(char *out, size_t cap, const picofw_normap_summary_t *sum) {
    if ((out == NULL) || (cap == 0u) || (sum == NULL)) {
        return 0u;
    }
    const int n = snprintf(
        out,
        cap,
        "[NOR_MAP] sectors=%u ipl_crc32=0x%08" PRIX32 " fe_extrhead=%s norfrom_used=%u "
        "bgo_p1=%u bgo_p2=%u bgo_spares=%u bgo_corrupt=%u "
        "eemul_active=%u eemul_superseded=%u eemul_empty=%u eemul_spare=%u "
        "nvram_active=%u nvram_blank=%u healthy=%s",
        (unsigned)sum->total_sectors,
        sum->ipl_crc32,
        sum->fe_extrhead_ok ? "YES" : "NO",
        (unsigned)sum->norfrom_used,
        (unsigned)sum->bgo_p1,
        (unsigned)sum->bgo_p2,
        (unsigned)sum->bgo_spares,
        (unsigned)sum->bgo_corrupt,
        (unsigned)sum->eemul_active_1284,
        (unsigned)sum->eemul_superseded_1284,
        (unsigned)sum->eemul_empty_2c,
        (unsigned)sum->eemul_spare_2d,
        (unsigned)sum->nvram_active,
        (unsigned)sum->nvram_blank,
        sum->healthy ? "YES" : "NO"
    );
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}
