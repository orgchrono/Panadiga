/**
 * @file p7501_fast_stream_codec.c
 * @brief Pure C11 MN103S fast binary stub (`0x9043A000`), pre-flight `1->0` guard,
 *        `@P7501_PROG` event formatter, and 142-sector `NOR_MAP` classifier.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_fast_stream_codec.h"
#include "common/p7501_hex.h"
#include "flash/p7501_flash.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

const uint8_t k_p7501_fast_bgo_stub[P7501_FAST_STUB_LEN] = {
    0xCFu, 0xF0u, 0xF8u, 0xFEu, 0xE0u, 0xCDu, 0x14u, 0x02u, 0x00u, 0x00u, 0xA0u, 0x51u,
    0xC8u, 0x28u, 0xA0u, 0x50u, 0xC9u, 0x0Cu, 0x8Fu, 0x50u, 0xCDu, 0x1Cu, 0x02u, 0x00u,
    0x00u, 0xCCu, 0xECu, 0xFFu, 0xA0u, 0x57u, 0xC9u, 0x05u, 0xCCu, 0x22u, 0x00u, 0xA0u,
    0x52u, 0xC9u, 0x05u, 0xCCu, 0x45u, 0x01u, 0x8Fu, 0x3Fu, 0xCDu, 0x04u, 0x02u, 0x00u,
    0x00u, 0xCCu, 0xD4u, 0xFFu, 0x8Fu, 0x51u, 0xCDu, 0xFAu, 0x01u, 0x00u, 0x00u, 0xF8u,
    0xFEu, 0x20u, 0xCEu, 0xF0u, 0xF0u, 0xFCu, 0xCDu, 0x91u, 0x01u, 0x00u, 0x00u, 0x4Au,
    0x10u, 0xCDu, 0xB7u, 0x01u, 0x00u, 0x00u, 0x4Au, 0x14u, 0x85u, 0x01u, 0x46u, 0x18u,
    0xF8u, 0xEEu, 0x01u, 0xC9u, 0x2Cu, 0xAAu, 0x00u, 0xC8u, 0x28u, 0xFCu, 0xCAu, 0x00u,
    0x10u, 0x00u, 0x00u, 0xC5u, 0x20u, 0x58u, 0x10u, 0xF8u, 0xECu, 0x01u, 0xC9u, 0x19u,
    0xFCu, 0xC4u, 0x00u, 0x00u, 0x13u, 0x84u, 0xFCu, 0xC8u, 0x00u, 0x00u, 0x2Du, 0x00u,
    0xC6u, 0x0Bu, 0xE8u, 0xFCu, 0xC8u, 0x00u, 0x00u, 0x2Du, 0x00u, 0xC7u, 0x05u, 0x04u,
    0x46u, 0x18u, 0xFCu, 0xDFu, 0x00u, 0xB0u, 0x43u, 0x90u, 0x59u, 0x14u, 0x08u, 0xCDu,
    0x8Au, 0x01u, 0x00u, 0x00u, 0xF0u, 0x53u, 0xE2u, 0x4Du, 0x29u, 0xFFu, 0xA5u, 0x00u,
    0xC9u, 0xF3u, 0xCDu, 0x7Bu, 0x01u, 0x00u, 0x00u, 0xF8u, 0xE2u, 0xFFu, 0xA8u, 0xC8u,
    0x0Cu, 0x8Fu, 0x43u, 0xCDu, 0x85u, 0x01u, 0x00u, 0x00u, 0xCCu, 0x55u, 0xFFu, 0x58u,
    0x18u, 0xA0u, 0x00u, 0xC9u, 0x0Cu, 0x8Fu, 0x45u, 0xCDu, 0x75u, 0x01u, 0x00u, 0x00u,
    0xCCu, 0x45u, 0xFFu, 0x58u, 0x10u, 0xF1u, 0xE2u, 0xFCu, 0xDFu, 0x00u, 0xB0u, 0x43u,
    0x90u, 0x59u, 0x14u, 0xF8u, 0xC5u, 0x01u, 0x46u, 0x14u, 0xF0u, 0x6Bu, 0xF0u, 0x62u,
    0xA8u, 0xC9u, 0x05u, 0xCCu, 0x76u, 0x00u, 0x83u, 0xF2u, 0x0Bu, 0xABu, 0xC8u, 0x0Cu,
    0x8Fu, 0x56u, 0xCDu, 0x4Au, 0x01u, 0x00u, 0x00u, 0xCCu, 0x1Au, 0xFFu, 0xFCu, 0xDCu,
    0xAAu, 0x0Au, 0x00u, 0x84u, 0xFCu, 0xDDu, 0x54u, 0x05u, 0x00u, 0x84u, 0x80u, 0xAAu,
    0x14u, 0xF0u, 0x70u, 0x80u, 0x55u, 0xF0u, 0x71u, 0x80u, 0xA0u, 0x14u, 0xF0u, 0x70u,
    0xF0u, 0x7Au, 0x8Bu, 0xF8u, 0xE3u, 0x80u, 0xFCu, 0xCDu, 0x00u, 0x00u, 0x40u, 0x00u,
    0xF0u, 0x62u, 0xF8u, 0xE0u, 0x80u, 0xACu, 0xC8u, 0x1Du, 0x29u, 0xFFu, 0xA5u, 0x00u,
    0xC9u, 0xF4u, 0xFCu, 0xDCu, 0x00u, 0x00u, 0x00u, 0x84u, 0x80u, 0xF0u, 0x14u, 0xF0u,
    0x70u, 0x8Fu, 0x54u, 0xCDu, 0x01u, 0x01u, 0x00u, 0x00u, 0xCCu, 0xD1u, 0xFEu, 0xF0u,
    0x62u, 0xA8u, 0xC8u, 0x17u, 0xFCu, 0xDCu, 0x00u, 0x00u, 0x00u, 0x84u, 0x80u, 0xF0u,
    0x14u, 0xF0u, 0x70u, 0x8Fu, 0x4Du, 0xCDu, 0xE7u, 0x00u, 0x00u, 0x00u, 0xCCu, 0xB7u,
    0xFEu, 0x22u, 0x02u, 0x23u, 0x02u, 0x59u, 0x14u, 0x29u, 0xFFu, 0x46u, 0x14u, 0xA5u,
    0x00u, 0xC8u, 0x05u, 0xCCu, 0x75u, 0xFFu, 0x8Fu, 0x4Bu, 0xCDu, 0xCCu, 0x00u, 0x00u,
    0x00u, 0xCCu, 0x9Cu, 0xFEu, 0xCDu, 0x67u, 0x00u, 0x00u, 0x00u, 0xF1u, 0xEAu, 0xCDu,
    0x8Du, 0x00u, 0x00u, 0x00u, 0x4Au, 0x10u, 0xAAu, 0x00u, 0xC9u, 0x05u, 0xCCu, 0x87u,
    0xFEu, 0x9Bu, 0x85u, 0x40u, 0x73u, 0x53u, 0xFCu, 0xC8u, 0xFFu, 0xFFu, 0xFFu, 0xFFu,
    0xC9u, 0x13u, 0x29u, 0xFFu, 0xA5u, 0x00u, 0xC9u, 0xF2u, 0x9Eu, 0x8Fu, 0x00u, 0xCDu,
    0x99u, 0x00u, 0x00u, 0x00u, 0xCCu, 0x27u, 0x00u, 0x8Fu, 0x01u, 0xCDu, 0x8Fu, 0x00u,
    0x00u, 0x00u, 0xFCu, 0xCDu, 0x00u, 0x01u, 0x00u, 0x00u, 0x08u, 0xF0u, 0x42u, 0x83u,
    0xE2u, 0xCDu, 0x7Fu, 0x00u, 0x00u, 0x00u, 0x49u, 0x29u, 0xFFu, 0xA5u, 0x00u, 0xC9u,
    0xF2u, 0x8Bu, 0xCDu, 0x72u, 0x00u, 0x00u, 0x00u, 0x58u, 0x10u, 0x28u, 0xFFu, 0x42u,
    0x10u, 0xA0u, 0x00u, 0xC8u, 0x05u, 0xCCu, 0xB4u, 0xFFu, 0xCCu, 0x35u, 0xFEu, 0xF8u,
    0xFEu, 0xFCu, 0xCDu, 0x43u, 0x00u, 0x00u, 0x00u, 0x82u, 0xCDu, 0x3Du, 0x00u, 0x00u,
    0x00u, 0xF8u, 0xC0u, 0x08u, 0xF2u, 0x12u, 0xCDu, 0x33u, 0x00u, 0x00u, 0x00u, 0xF8u,
    0xC0u, 0x10u, 0xF2u, 0x12u, 0xCDu, 0x29u, 0x00u, 0x00u, 0x00u, 0xF8u, 0xC0u, 0x18u,
    0xF2u, 0x12u, 0xF8u, 0xFEu, 0x04u, 0xDFu, 0x00u, 0x00u, 0xF8u, 0xFEu, 0xFCu, 0xCDu,
    0x16u, 0x00u, 0x00u, 0x00u, 0x82u, 0xCDu, 0x10u, 0x00u, 0x00u, 0x00u, 0xF8u, 0xC0u,
    0x08u, 0xF2u, 0x12u, 0xF8u, 0xFEu, 0x04u, 0xDFu, 0x00u, 0x00u, 0xFCu, 0xACu, 0x0Cu,
    0x20u, 0x00u, 0xD4u, 0x1Cu, 0xF8u, 0xE0u, 0x10u, 0xA0u, 0x00u, 0xC8u, 0xF4u, 0xFCu,
    0xA8u, 0x09u, 0x20u, 0x00u, 0xD4u, 0xDFu, 0x00u, 0x00u, 0xF8u, 0xFEu, 0xFCu, 0x46u,
    0x00u, 0xFCu, 0xCDu, 0x00u, 0x00u, 0x04u, 0x00u, 0xFCu, 0xACu, 0x0Cu, 0x20u, 0x00u,
    0xD4u, 0x1Cu, 0xF8u, 0xE0u, 0xA0u, 0xA0u, 0x00u, 0xC8u, 0x08u, 0x29u, 0xFFu, 0xA5u,
    0x00u, 0xC9u, 0xEEu, 0x8Cu, 0xFCu, 0x82u, 0x08u, 0x20u, 0x00u, 0xD4u, 0x59u, 0x00u,
    0xF8u, 0xFEu, 0x04u, 0xDFu, 0x00u, 0x00u,
};

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

uint32_t p7501_fast_stub_crc32(void) {
    return p7501_crc32_ieee(k_p7501_fast_bgo_stub, sizeof(k_p7501_fast_bgo_stub));
}

uint8_t p7501_fast_sum8(const uint8_t *data, size_t len) {
    return p7501_rsp_checksum(data, len);
}

p7501_bus_deny_t p7501_fast_check_write_guard(
    const p7501_session_ctx_t *ctx,
    uint32_t addr,
    uint32_t len,
    bool upper_nor_unlocked,
    const char **out_reason
) {
    const p7501_bus_deny_t deny = p7501_bus_check_access(
        ctx, P7501_FLASH_SIZE_8MB, addr, (size_t)(len > 0u ? len : 1u),
        true, false, upper_nor_unlocked
    );
    if (deny != P7501_BUS_ALLOW) {
        if (out_reason != NULL) {
            *out_reason = p7501_bus_deny_name(deny);
        }
        return deny;
    }
    if ((len < 2u) || (len > P7501_FAST_STUB_MAX_CHUNK) || ((addr & 1u) != 0u) || ((len & 1u) != 0u)) {
        if (out_reason != NULL) {
            *out_reason = "UNALIGNED_OR_LEN";
        }
        return P7501_BUS_DENY_PROTECTED_RO;
    }
    if ((addr < P7501_FAST_BGO_START_VA) ||
        ((addr - P7501_FAST_BGO_START_VA) > P7501_FAST_BGO_SPAN_BYTES) ||
        (len > (P7501_FAST_BGO_SPAN_BYTES - (addr - P7501_FAST_BGO_START_VA)))) {
        if (out_reason != NULL) {
            *out_reason = "OUTSIDE_BGO_WINDOW";
        }
        return P7501_BUS_DENY_PROTECTED_RO;
    }
    if (out_reason != NULL) {
        *out_reason = "ALLOW";
    }
    return P7501_BUS_ALLOW;
}

bool p7501_fast_check_monotonic_1_to_0(
    const uint8_t *cur,
    const uint8_t *tgt,
    size_t len,
    size_t *out_bad_off
) {
    if ((cur == NULL) || (tgt == NULL)) {
        return false;
    }
    for (size_t i = 0u; i < len; ++i) {
        if ((uint8_t)(cur[i] & tgt[i]) != tgt[i]) {
            if (out_bad_off != NULL) {
                *out_bad_off = i;
            }
            return false;
        }
    }
    return true;
}

size_t p7501_fast_format_prog_line(
    char *out,
    size_t cap,
    const char *phase,
    uint16_t sa,
    uint32_t done,
    uint32_t total,
    uint32_t crc32,
    uint32_t vdd_mv
) {
    if ((out == NULL) || (cap == 0u) || (phase == NULL)) {
        return 0u;
    }
    const int n = snprintf(
        out, cap,
        "@P7501_PROG phase=%s sa=%u done=%" PRIu32 " total=%" PRIu32
        " crc32=0x%08" PRIX32 " vdd_mv=%" PRIu32,
        phase, (unsigned)sa, done, total, crc32, vdd_mv
    );
    return (n > 0) ? (size_t)n : 0u;
}

bool p7501_normap_sector_span(uint16_t sa, uint32_t *out_va, uint32_t *out_size) {
    if ((sa >= P7501_NORMAP_SECTOR_COUNT) || (out_va == NULL) || (out_size == NULL)) {
        return false;
    }
    if (sa < 8u) {
        *out_va = P7501_FLASH_HW_UNCACHED_BASE + ((uint32_t)sa * 0x2000UL);
        *out_size = 0x2000UL;
    } else if (sa < 134u) {
        *out_va = P7501_FLASH_HW_UNCACHED_BASE + 0x10000UL + ((uint32_t)(sa - 8u) * 0x10000UL);
        *out_size = 0x10000UL;
    } else {
        *out_va = P7501_FLASH_HW_UNCACHED_BASE + 0x7F0000UL + ((uint32_t)(sa - 134u) * 0x2000UL);
        *out_size = 0x2000UL;
    }
    return true;
}

p7501_normap_kind_t p7501_normap_classify_sector(
    uint16_t sa,
    const uint8_t *hdr20,
    const uint8_t *z16
) {
    if ((sa >= P7501_NORMAP_SECTOR_COUNT) || (hdr20 == NULL)) {
        return P7501_NORMAP_UNKNOWN;
    }
    if (sa <= 10u) {
        return P7501_NORMAP_IPL;
    }
    if (sa <= 14u) {
        return bytes_all_eq(hdr20, 20u, 0xFFu) ? P7501_NORMAP_NORFROM_BLANK : P7501_NORMAP_NORFROM_USED;
    }
    if (sa <= 25u) {
        if (sa == 15u) {
            return (memcmp(hdr20, "EXTRHEADDRV ", 12u) == 0)
                ? P7501_NORMAP_FE_EXTRHEAD : P7501_NORMAP_UNKNOWN;
        }
        return P7501_NORMAP_FE_DATA;
    }
    if (sa <= 70u) {
        const uint16_t w0 = le16_at(&hdr20[4]);
        const bool rep_ok = (le16_at(&hdr20[8]) == w0) &&
                            (le16_at(&hdr20[12]) == w0) &&
                            (le16_at(&hdr20[16]) == w0);
        const bool z_ok = (z16 == NULL) || bytes_all_eq(z16, 16u, 0x00u);
        if (!rep_ok || !z_ok) {
            return P7501_NORMAP_BGO_CORRUPT;
        }
        if (w0 == 0xFFFFu) {
            return bytes_all_eq(hdr20, 20u, 0xFFu) ? P7501_NORMAP_BGO_SPARE : P7501_NORMAP_BGO_CORRUPT;
        }
        if (w0 == 0xF101u) {
            return P7501_NORMAP_BGO_P1;
        }
        if ((w0 >= 0xF201u) && (w0 <= 0xF23Fu)) {
            return P7501_NORMAP_BGO_P2;
        }
        return P7501_NORMAP_BGO_CORRUPT;
    }
    if (sa <= 133u) {
        static const uint8_t k_eemul_pfx[4] = { 0x0Fu, 0xF0u, 0x00u, 0x00u };
        if (memcmp(hdr20, k_eemul_pfx, 4u) == 0) {
            const uint8_t s4 = hdr20[4];
            const uint8_t s5 = hdr20[5];
            const bool is_1284 = (memcmp(&hdr20[6], "1284", 4u) == 0);
            if ((s4 == 0x00u) && (s5 == 0x00u) && is_1284) {
                return P7501_NORMAP_EEMUL_ACTIVE_1284;
            }
            if ((s4 == 0x00u) && (s5 == 0x2Cu) && is_1284) {
                return P7501_NORMAP_EEMUL_SUPERSEDED_1284;
            }
            if ((s4 == 0x00u) && (s5 == 0x2Cu) && (hdr20[6] == 0xFFu) && (hdr20[7] == 0xFFu)) {
                return P7501_NORMAP_EEMUL_EMPTY_2C;
            }
            if ((s4 == 0x00u) && (s5 == 0x2Du) && (hdr20[6] == 0xFFu) && (hdr20[7] == 0xFFu)) {
                return P7501_NORMAP_EEMUL_SPARE_2D;
            }
        }
        return P7501_NORMAP_EEMUL_OTHER;
    }
    return bytes_all_eq(hdr20, 16u, 0xFFu) ? P7501_NORMAP_NVRAM_BLANK : P7501_NORMAP_NVRAM_ACTIVE;
}

void p7501_normap_accumulate(p7501_normap_summary_t *sum, p7501_normap_kind_t kind) {
    if (sum == NULL) {
        return;
    }
    sum->total_sectors += 1u;
    switch (kind) {
        case P7501_NORMAP_FE_EXTRHEAD: sum->fe_extrhead_ok = true; break;
        case P7501_NORMAP_NORFROM_USED: sum->norfrom_used += 1u; break;
        case P7501_NORMAP_BGO_P1: sum->bgo_p1 += 1u; break;
        case P7501_NORMAP_BGO_P2: sum->bgo_p2 += 1u; break;
        case P7501_NORMAP_BGO_SPARE: sum->bgo_spares += 1u; break;
        case P7501_NORMAP_BGO_CORRUPT: sum->bgo_corrupt += 1u; break;
        case P7501_NORMAP_EEMUL_ACTIVE_1284: sum->eemul_active_1284 += 1u; break;
        case P7501_NORMAP_EEMUL_SUPERSEDED_1284: sum->eemul_superseded_1284 += 1u; break;
        case P7501_NORMAP_EEMUL_EMPTY_2C: sum->eemul_empty_2c += 1u; break;
        case P7501_NORMAP_EEMUL_SPARE_2D: sum->eemul_spare_2d += 1u; break;
        case P7501_NORMAP_EEMUL_OTHER: sum->eemul_other += 1u; break;
        case P7501_NORMAP_NVRAM_ACTIVE: sum->nvram_active += 1u; break;
        case P7501_NORMAP_NVRAM_BLANK: sum->nvram_blank += 1u; break;
        default: break;
    }
}

void p7501_normap_finalize(p7501_normap_summary_t *sum) {
    if (sum == NULL) {
        return;
    }
    sum->healthy = (sum->total_sectors == P7501_NORMAP_SECTOR_COUNT) &&
                   sum->fe_extrhead_ok &&
                   (sum->bgo_p1 == 1u) &&
                   (sum->bgo_p2 == 40u) &&
                   (sum->bgo_spares >= 1u) &&
                   (sum->bgo_corrupt == 0u) &&
                   (sum->nvram_blank >= 1u);
}

static size_t encode_cmd_u32_u16(char cmd, uint32_t arg32, uint16_t arg16, uint8_t out7[7]) {
    if (out7 == NULL) {
        return 0u;
    }
    out7[0] = (uint8_t)cmd;
    p7501_write_u32_le(&out7[1], arg32);
    out7[5] = (uint8_t)(arg16 & 0xFFu);
    out7[6] = (uint8_t)((arg16 >> 8u) & 0xFFu);
    return 7u;
}

uint16_t p7501_fast_va_to_sa(uint32_t va) {
    const uint32_t uncached_base = (uint32_t)P7501_FLASH_HW_UNCACHED_BASE;
    const uint32_t cached_base = (uint32_t)P7501_FLASH_CACHED_BASE;
    const uint32_t cap_bytes = (uint32_t)P7501_FLASH_SIZE_8MB;
    uint32_t off = 0u;
    if ((va >= uncached_base) && ((va - uncached_base) < cap_bytes)) {
        off = va - uncached_base;
    } else if ((va >= cached_base) && ((va - cached_base) < cap_bytes)) {
        off = va - cached_base;
    } else {
        return 0xFFFFu;
    }

    if (off < 0x10000UL) {
        return (uint16_t)(off / 0x2000UL);
    }
    if (off < 0x7F0000UL) {
        return (uint16_t)(8UL + ((off - 0x10000UL) / 0x10000UL));
    }
    return (uint16_t)(134UL + ((off - 0x7F0000UL) / 0x2000UL));
}

size_t p7501_fast_encode_read_pkt(uint32_t page_va, uint16_t page_count, uint8_t out7[7]) {
    return encode_cmd_u32_u16('R', page_va, page_count, out7);
}

size_t p7501_fast_encode_write_hdr(uint32_t addr, uint16_t pay_len, uint8_t out7[7]) {
    return encode_cmd_u32_u16('W', addr, pay_len, out7);
}

bool p7501_fast_compute_page_window(uint32_t addr, uint32_t pay_len, p7501_fast_page_window_t *out) {
    if ((out == NULL) || (pay_len == 0u) || ((UINT32_MAX - addr) < pay_len)) {
        return false;
    }
    const uint32_t mask = (uint32_t)P7501_FAST_STUB_PAGE_BYTES - 1u;
    const uint32_t page_base = addr & ~mask;
    const uint32_t end_va = addr + pay_len;
    const uint32_t page_end = (end_va + mask) & ~mask;
    out->page_base = page_base;
    out->rel_off = addr - page_base;
    out->span_pages = (page_end - page_base) / P7501_FAST_STUB_PAGE_BYTES;
    return true;
}

size_t p7501_fast_format_normap_line(char *out, size_t cap, const p7501_normap_summary_t *sum) {
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

size_t p7501_fast_format_page_hex(const uint8_t *page, size_t page_len, char *out_hex, size_t out_cap) {
    if ((page == NULL) || (out_hex == NULL) || (out_cap < ((page_len * 2u) + 1u))) {
        if ((out_hex != NULL) && (out_cap > 0u)) {
            out_hex[0] = '\0';
        }
        return 0u;
    }
    for (size_t i = 0u; i < page_len; ++i) {
        out_hex[i * 2u] = p7501_nibble_to_hex_char((uint8_t)(page[i] >> 4u));
        out_hex[(i * 2u) + 1u] = p7501_nibble_to_hex_char((uint8_t)(page[i] & 0x0Fu));
    }
    out_hex[page_len * 2u] = '\0';
    return page_len * 2u;
}

const char *p7501_describe_mcu_host(bool is_rp2350, bool has_ble) {
    if (is_rp2350) {
        return "RP2350 (Pico 2 / ARM Cortex-M33 Dual @ 150MHz)";
    }
    if (has_ble) {
        return "RP2040-W (Pico W + CYW43 BLE / ARM Cortex-M0+)";
    }
    return "RP2040 (Pico / ARM Cortex-M0+)";
}

const char *p7501_describe_gdb_target(bool is_rp2350, bool is_riscv) {
    if (is_rp2350) {
        return is_riscv
            ? "RP2350 (Pico 2 RISC-V Hazard3) - 520KB SRAM - 512B Cache"
            : "RP2350 (Pico 2 ARM Cortex-M33) - 520KB SRAM - 512B Cache";
    }
    return "RP2040 (Pico 1 Dual Cortex-M0+) - 264KB SRAM - 512B Cache";
}
