/**
 * @file p7501_ftl_inspect.c
 * @brief Pure C11 BGO & EEMUL NOR flash FTL analyzer + live inspector CLI.
 *
 * Uses `g_p7501_cmd_arena.raw` instead of Core-0 stack for sector header tables,
 * uses `p7501_hw_read_block_checked()` to reject offline bus errors, and validates
 * `+0x100` zero-markers ONLY on `P7501_BGO_SEC_VALID` sectors (never erased spares).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_ftl_inspect.h"
#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "flash/p7501_fast_stream_codec.h"
#include "flash/p7501_flash.h"
#include "hal/p7501_hw.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static bool all_ff_bytes(const uint8_t *buf, size_t n) {
    for (size_t i = 0u; i < n; ++i) {
        if (buf[i] != 0xFFu) {
            return false;
        }
    }
    return true;
}

static uint16_t read_u16_le(const uint8_t *p) {
    return (uint16_t)p[0] | (uint16_t)((uint16_t)p[1] << 8u);
}

bool p7501_bgo_inspect_headers(
    const uint8_t *headers_20b,
    size_t sector_count,
    p7501_bgo_report_t *out
) {
    if ((headers_20b == NULL) || (out == NULL) || (sector_count == 0u) ||
        (sector_count > P7501_BGO_MAX_SECTORS)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->total_sectors = (uint16_t)sector_count;

    uint16_t seen_p2[P7501_BGO_MAX_SECTORS];
    size_t seen_count = 0u;
    size_t p1_count = 0u;

    for (size_t i = 0u; i < sector_count; ++i) {
        const uint8_t *hdr = &headers_20b[i * P7501_BGO_HEADER_BYTES];
        const uint16_t w0 = read_u16_le(&hdr[4]);
        const bool consistent = (read_u16_le(&hdr[8]) == w0) &&
                                (read_u16_le(&hdr[12]) == w0) &&
                                (read_u16_le(&hdr[16]) == w0);
        out->lbns[i] = w0;
        if (!consistent) {
            out->states[i] = (uint8_t)P7501_BGO_SEC_CORRUPT;
            out->corrupt_count += 1u;
            continue;
        }
        if (w0 == 0xFFFFu) {
            if (all_ff_bytes(hdr, P7501_BGO_HEADER_BYTES)) {
                out->states[i] = (uint8_t)P7501_BGO_SEC_ERASED;
                out->erased_count += 1u;
            } else {
                out->states[i] = (uint8_t)P7501_BGO_SEC_CORRUPT;
                out->corrupt_count += 1u;
            }
            continue;
        }
        if (w0 == 0xF101u) {
            out->states[i] = (uint8_t)P7501_BGO_SEC_VALID;
            out->valid_count += 1u;
            p1_count += 1u;
            continue;
        }
        if ((w0 >= 0xF201u) && (w0 <= 0xF23Fu)) {
            bool dup = false;
            for (size_t s = 0u; s < seen_count; ++s) {
                if (seen_p2[s] == w0) {
                    dup = true;
                    break;
                }
            }
            if (dup) {
                out->states[i] = (uint8_t)P7501_BGO_SEC_DUPLICATE_LBN;
                out->duplicate_lbn_count += 1u;
            } else {
                seen_p2[seen_count++] = w0;
                out->states[i] = (uint8_t)P7501_BGO_SEC_VALID;
                out->valid_count += 1u;
            }
            continue;
        }
        out->states[i] = (uint8_t)P7501_BGO_SEC_CORRUPT;
        out->corrupt_count += 1u;
    }

    const size_t min_p2 = (sector_count >= P7501_BGO_SECTOR_COUNT) ? 40u : 1u;
    out->healthy_bgo_ftl = (p1_count >= 1u) &&
                           (seen_count >= min_p2) &&
                           (out->valid_count > 0u) &&
                           (out->corrupt_count == 0u) &&
                           (out->duplicate_lbn_count == 0u) &&
                           (out->erased_count > 0u);
    return true;
}

bool p7501_eemul_inspect_slice(
    const uint8_t *data,
    size_t total_bytes,
    size_t microsector_bytes,
    p7501_eemul_report_t *out
) {
    if ((data == NULL) || (out == NULL) || (microsector_bytes == 0u) ||
        (total_bytes == 0u) || ((total_bytes % microsector_bytes) != 0u)) {
        return false;
    }
    const size_t nsec = total_bytes / microsector_bytes;
    if (nsec > P7501_EEMUL_MAX_MICROSECS) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->total_microsectors = (uint16_t)nsec;
    out->crc32 = p7501_crc32_ieee(data, total_bytes);

    size_t non_ff = 0u;
    for (size_t i = 0u; i < total_bytes; ++i) {
        if (data[i] != 0xFFu) {
            non_ff += 1u;
        }
    }
    out->dirty_non_ff_bytes_pct = (uint16_t)((non_ff * 100u) / total_bytes);

    for (size_t s = 0u; s < nsec; ++s) {
        if (all_ff_bytes(&data[s * microsector_bytes], microsector_bytes)) {
            out->blank_sectors += 1u;
        } else {
            out->active_sectors += 1u;
        }
    }

    /* Healthy EEMUL always retains >= 1 erased wear-leveling spare sector or fresh blank state */
    out->healthy_eemul = (out->blank_sectors >= 1u);
    return true;
}

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
static bool check_ftl_target_ready(const char *tag) {
    if (!p7501_hw_is_target_powered() || !p7501_hw_is_target_online()) {
        p7501_cli_set_result("ERR", "EOFFLINE");
        printf("%s: ERROR (Target offline or unpowered)\n", tag);
        return false;
    }
    return true;
}

void p7501_cmd_bgo_inspect(const char *arg) {
    const bool detail = (arg != NULL) &&
                        ((strstr(arg, "DETAIL") != NULL) || (strstr(arg, "detail") != NULL));
    if (!check_ftl_target_ready("BGO_INSPECT")) {
        return;
    }
    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(p7501_flash_get_active_size());
    if ((geo.bgo_sector_count == 0u) || (geo.bgo_sector_count > P7501_BGO_MAX_SECTORS)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("BGO_INSPECT: ERROR (Target geometry has no BGO partition)\n");
        return;
    }
    /* Stage headers in shared 32 KiB arena instead of Core-0 stack. */
    uint8_t *const headers = g_p7501_cmd_arena.raw;
    for (uint32_t i = 0u; i < geo.bgo_sector_count; ++i) {
        const uint32_t sec_base = geo.bgo_hw_base + (i * P7501_BGO_SECTOR_SIZE);
        if (!p7501_hw_read_block_checked(
                sec_base, &headers[i * P7501_BGO_HEADER_BYTES], P7501_BGO_HEADER_BYTES, true)) {
            p7501_cli_set_result("ERR", "EBUS");
            printf("BGO_INSPECT: ERROR (Bus read failed at sector %" PRIu32 " @ 0x%08" PRIX32 ")\n",
                   i, sec_base);
            return;
        }
    }
    p7501_bgo_report_t rep;
    (void)p7501_bgo_inspect_headers(headers, geo.bgo_sector_count, &rep);

    /* Verify +0x100 zero-marker on VALID and ERASED (formatted spare) sectors; SA70 all-FF +0x100 is unformatted. */
    uint16_t zero_marker_bad = 0u;
    for (uint32_t i = 0u; i < geo.bgo_sector_count; ++i) {
        const uint32_t sec_base = geo.bgo_hw_base + (i * P7501_BGO_SECTOR_SIZE);
        bool z100_ok = true;
        if ((rep.states[i] == (uint8_t)P7501_BGO_SEC_VALID) ||
            (rep.states[i] == (uint8_t)P7501_BGO_SEC_ERASED)) {
            const uint32_t z_addr = sec_base + 0x100UL;
            uint8_t zbuf[16];
            if (!p7501_hw_read_block_checked(z_addr, zbuf, sizeof(zbuf), true)) {
                p7501_cli_set_result("ERR", "EBUS");
                printf("BGO_INSPECT: ERROR (Bus read failed at zmarker 0x%08" PRIX32 ")\n", z_addr);
                return;
            }
            for (size_t k = 0u; k < sizeof(zbuf); ++k) {
                if (zbuf[k] != 0x00u) {
                    z100_ok = false;
                    zero_marker_bad += 1u;
                    break;
                }
            }
        } else {
            z100_ok = false;
        }
        if (detail) {
            printf("@P7501_BGO_SEC sa=%u va=0x%08" PRIX32 " lbn=0x%04X state=%u z100_ok=%u\n",
                   (unsigned)p7501_fast_va_to_sa(sec_base), sec_base, (unsigned)rep.lbns[i],
                   (unsigned)rep.states[i], z100_ok ? 1u : 0u);
        }
    }
    if (zero_marker_bad > 0u) {
        rep.corrupt_count = (uint16_t)(rep.corrupt_count + zero_marker_bad);
        rep.healthy_bgo_ftl = false;
    }
    printf("[BGO_INSPECT] sectors=%u erased=%u valid=%u corrupt=%u dup_lbn=%u zmarker_bad=%u healthy=%s\n",
           (unsigned)rep.total_sectors, (unsigned)rep.erased_count,
           (unsigned)rep.valid_count, (unsigned)rep.corrupt_count,
           (unsigned)rep.duplicate_lbn_count, (unsigned)zero_marker_bad,
           rep.healthy_bgo_ftl ? "YES" : "NO");
    printf("BGO_INSPECT: OK\n");
}

void p7501_cmd_eemul_inspect(const char *arg) {
    (void)arg;
    if (!check_ftl_target_ready("EEMUL_INSPECT")) {
        return;
    }
    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(p7501_flash_get_active_size());
    if ((geo.nvram_size == 0UL) || (geo.sector_count == 0u) ||
        (geo.sector_count > P7501_EEMUL_MAX_MICROSECS)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("EEMUL_INSPECT: ERROR (Target has no EEMUL NVRAM partition)\n");
        return;
    }
    /* Stage microsector headers in shared 32 KiB arena instead of Core-0 stack. */
    uint8_t *const heads = g_p7501_cmd_arena.raw;
    const uint32_t step = geo.nvram_size / geo.sector_count;
    for (uint32_t i = 0u; i < geo.sector_count; ++i) {
        const uint32_t sec_addr = geo.hw_nvram_base + (i * step);
        if (!p7501_hw_read_block_checked(sec_addr, &heads[i * 64u], 64u, true)) {
            p7501_cli_set_result("ERR", "EBUS");
            printf("EEMUL_INSPECT: ERROR (Bus read failed at microsector %" PRIu32 " @ 0x%08" PRIX32 ")\n",
                   i, sec_addr);
            return;
        }
    }
    p7501_eemul_report_t rep;
    (void)p7501_eemul_inspect_slice(heads, geo.sector_count * 64u, 64u, &rep);
    printf("[EEMUL_INSPECT] base=0x%08" PRIX32 " sectors=%u blank=%u active=%u hdr_dirty=%u%% crc32=0x%08" PRIX32 " healthy=%s\n",
           geo.hw_nvram_base, (unsigned)rep.total_microsectors,
           (unsigned)rep.blank_sectors, (unsigned)rep.active_sectors,
           (unsigned)rep.dirty_non_ff_bytes_pct, rep.crc32,
           rep.healthy_eemul ? "YES" : "NO");
    printf("EEMUL_INSPECT: OK\n");
}
#endif
