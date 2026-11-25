/**
 * @file picofw_ftl_inspect.c
 * @brief Pure C11 BGO & EEMUL NOR flash FTL analyzer + live inspector CLI.
 *
 * Uses `g_picofw_cmd_arena.raw` instead of Core-0 stack for sector header tables,
 * uses `picofw_hw_read_block_checked()` to reject offline bus errors, and validates
 * `+0x100` zero-markers ONLY on `PICOFW_BGO_SEC_VALID` sectors (never erased spares).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_ftl_inspect.h"
#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "flash/picofw_fast_stream_codec.h"
#include "flash/picofw_flash.h"
#include "generated/picofw_ssot.h"
#include "hal/picofw_hw.h"

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

bool picofw_bgo_inspect_headers(
    const uint8_t *headers_20b,
    size_t sector_count,
    picofw_bgo_report_t *out
) {
    if ((headers_20b == NULL) || (out == NULL) || (sector_count == 0u) ||
        (sector_count > PICOFW_BGO_MAX_SECTORS)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->total_sectors = (uint16_t)sector_count;
    out->p1_sa_index = 0xFFFFu;

    uint16_t seen_p2[PICOFW_BGO_MAX_SECTORS];
    size_t seen_count = 0u;
    size_t p1_count = 0u;

    for (size_t i = 0u; i < sector_count; ++i) {
        const uint8_t *hdr = &headers_20b[i * PICOFW_BGO_HEADER_BYTES];
        const uint16_t w0 = read_u16_le(&hdr[4]);
        const bool consistent = (read_u16_le(&hdr[8]) == w0) &&
                                (read_u16_le(&hdr[12]) == w0) &&
                                (read_u16_le(&hdr[16]) == w0);
        out->lbns[i] = w0;
        if (!consistent) {
            out->states[i] = (uint8_t)PICOFW_BGO_SEC_CORRUPT;
            out->corrupt_count += 1u;
            continue;
        }
        if (w0 == 0xFFFFu) {
            if (all_ff_bytes(hdr, PICOFW_BGO_HEADER_BYTES)) {
                out->states[i] = (uint8_t)PICOFW_BGO_SEC_ERASED;
                out->erased_count += 1u;
            } else {
                out->states[i] = (uint8_t)PICOFW_BGO_SEC_CORRUPT;
                out->corrupt_count += 1u;
            }
            continue;
        }
        if (w0 == 0xF101u) {
            if (out->p1_sa_index == 0xFFFFu) {
                out->p1_sa_index = (uint16_t)(26u + i);
            }
            out->states[i] = (uint8_t)PICOFW_BGO_SEC_VALID;
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
                out->states[i] = (uint8_t)PICOFW_BGO_SEC_DUPLICATE_LBN;
                out->duplicate_lbn_count += 1u;
            } else {
                seen_p2[seen_count++] = w0;
                out->states[i] = (uint8_t)PICOFW_BGO_SEC_VALID;
                out->valid_count += 1u;
            }
            continue;
        }
        out->states[i] = (uint8_t)PICOFW_BGO_SEC_CORRUPT;
        out->corrupt_count += 1u;
    }

    const size_t min_p2 = (sector_count >= PICOFW_BGO_SECTOR_COUNT) ? 40u : 1u;
    out->healthy_bgo_ftl = (p1_count >= 1u) &&
                           (seen_count >= min_p2) &&
                           (out->valid_count > 0u) &&
                           (out->corrupt_count == 0u) &&
                           (out->duplicate_lbn_count == 0u) &&
                           (out->erased_count > 0u);
    return true;
}

bool picofw_eemul_inspect_slice(
    const uint8_t *data,
    size_t total_bytes,
    size_t microsector_bytes,
    picofw_eemul_report_t *out
) {
    if ((data == NULL) || (out == NULL) || (microsector_bytes == 0u) ||
        (total_bytes == 0u) || ((total_bytes % microsector_bytes) != 0u)) {
        return false;
    }
    const size_t nsec = total_bytes / microsector_bytes;
    if (nsec > PICOFW_EEMUL_MAX_MICROSECS) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->total_microsectors = (uint16_t)nsec;
    out->crc32 = picofw_crc32_ieee(data, total_bytes);

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

const char *picofw_norfrom_state_name(picofw_norfrom_slot_state_t st) {
    switch (st) {
        case PICOFW_NORFROM_ERASED_FF:
            return "ERASED_FF";
        case PICOFW_NORFROM_AUTOBOOT_CAPSULE:
            return "NOR_AUTOBOOT";
        case PICOFW_NORFROM_EXTRHEAD:
            return "EXTRHEAD";
        default:
            return "NON_EMPTY";
    }
}

picofw_norfrom_slot_state_t picofw_norfrom_classify_prefix(
    const uint8_t *buf,
    size_t len,
    uint32_t *out_crc32
) {
    if (out_crc32 != NULL) {
        *out_crc32 = ((buf != NULL) && (len > 0u)) ? picofw_crc32_ieee(buf, len) : 0u;
    }
    if ((buf == NULL) || (len < 8u)) {
        return PICOFW_NORFROM_NON_EMPTY;
    }
    if (all_ff_bytes(buf, len)) {
        return PICOFW_NORFROM_ERASED_FF;
    }
    if ((len >= PICOFW_SSOT_NOR_AUTOBOOT_HDR_BYTES) &&
        (picofw_read_u32_le(&buf[PICOFW_SSOT_NOR_AUTOBOOT_MAGIC_OFF]) ==
         PICOFW_SSOT_NOR_AUTOBOOT_MAGIC_LE)) {
        return PICOFW_NORFROM_AUTOBOOT_CAPSULE;
    }
    if (memcmp(buf, "EXTRHEAD", 8u) == 0) {
        return PICOFW_NORFROM_EXTRHEAD;
    }
    return PICOFW_NORFROM_NON_EMPTY;
}

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
static bool arg_has_token_ci(const char *arg, const char *upper_tok) {
    if ((arg == NULL) || (upper_tok == NULL) || (*upper_tok == '\0')) {
        return false;
    }
    const size_t nlen = strlen(upper_tok);
    for (size_t i = 0u; arg[i] != '\0'; ++i) {
        size_t j = 0u;
        while ((j < nlen) && (arg[i + j] != '\0')) {
            char c = arg[i + j];
            if ((c >= 'a') && (c <= 'z')) {
                c = (char)(c - ('a' - 'A'));
            }
            if (c != upper_tok[j]) {
                break;
            }
            j += 1u;
        }
        if (j == nlen) {
            return true;
        }
    }
    return false;
}

static bool check_ftl_target_ready(const char *tag) {
    if (!picofw_hw_is_target_powered() || !picofw_hw_is_target_online()) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("%s: ERROR (Target offline or unpowered)\n", tag);
        return false;
    }
    return true;
}

void picofw_cmd_bgo_inspect(const char *arg) {
    const bool detail = arg_has_token_ci(arg, "DETAIL");
    const bool norfrom = arg_has_token_ci(arg, "NORFROM")
        || arg_has_token_ci(arg, "NOR_AUTOBOOT")
        || arg_has_token_ci(arg, "FULL");
    if (!check_ftl_target_ready("BGO_INSPECT")) {
        return;
    }
    const picofw_flash_geometry_t geo = picofw_flash_get_geometry(picofw_flash_get_active_size());
    if ((geo.bgo_sector_count == 0u) || (geo.bgo_sector_count > PICOFW_BGO_MAX_SECTORS)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("BGO_INSPECT: ERROR (Target geometry has no BGO partition)\n");
        return;
    }
    /* Stage headers in shared 32 KiB arena instead of Core-0 stack. */
    uint8_t *const headers = g_picofw_cmd_arena.raw;
    for (uint32_t i = 0u; i < geo.bgo_sector_count; ++i) {
        const uint32_t sec_base = geo.bgo_hw_base + (i * PICOFW_BGO_SECTOR_SIZE);
        if (!picofw_hw_read_block_checked(
                sec_base, &headers[i * PICOFW_BGO_HEADER_BYTES], PICOFW_BGO_HEADER_BYTES, true)) {
            picofw_cli_set_result("ERR", "EBUS");
            printf("BGO_INSPECT: ERROR (Bus read failed at sector %" PRIu32 " @ 0x%08" PRIX32 ")\n",
                   i, sec_base);
            return;
        }
    }
    picofw_bgo_report_t rep;
    (void)picofw_bgo_inspect_headers(headers, geo.bgo_sector_count, &rep);

    /* Verify +0x100 zero-marker on VALID and ERASED (formatted spare) sectors; SA70 all-FF +0x100 is unformatted. */
    uint16_t zero_marker_bad = 0u;
    for (uint32_t i = 0u; i < geo.bgo_sector_count; ++i) {
        const uint32_t sec_base = geo.bgo_hw_base + (i * PICOFW_BGO_SECTOR_SIZE);
        bool z100_ok = true;
        if ((rep.states[i] == (uint8_t)PICOFW_BGO_SEC_VALID) ||
            (rep.states[i] == (uint8_t)PICOFW_BGO_SEC_ERASED)) {
            const uint32_t z_addr = sec_base + 0x100UL;
            uint8_t zbuf[16];
            if (!picofw_hw_read_block_checked(z_addr, zbuf, sizeof(zbuf), true)) {
                picofw_cli_set_result("ERR", "EBUS");
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
            printf("@PICO_BGO_SEC sa=%u va=0x%08" PRIX32 " lbn=0x%04X state=%u z100_ok=%u\n",
                   (unsigned)picofw_fast_va_to_sa(sec_base), sec_base, (unsigned)rep.lbns[i],
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
    if (norfrom) {
        const uint32_t sa11_va = PICOFW_SSOT_IPL_LAYER1_MIRROR_START + PICOFW_SSOT_IPL_LAYER1_SIZE;
        uint8_t *const sa11_hdr = g_picofw_cmd_arena.raw;
        if (!picofw_hw_read_block_checked(
                sa11_va, sa11_hdr, PICOFW_SSOT_NOR_AUTOBOOT_HDR_BYTES, true)) {
            picofw_cli_set_result("ERR", "EBUS");
            printf("BGO_INSPECT: ERROR (Bus read failed at SA11 @ 0x%08" PRIX32 ")\n", sa11_va);
            return;
        }
        uint32_t sa11_crc = 0u;
        const picofw_norfrom_slot_state_t st11 = picofw_norfrom_classify_prefix(
            sa11_hdr, PICOFW_SSOT_NOR_AUTOBOOT_HDR_BYTES, &sa11_crc);
        char frame_msg[128];
        (void)snprintf(
            frame_msg, sizeof(frame_msg),
            "p1_sa=%u valid=%u erased=%u corrupt=%u sa11=%s sa11_crc32=0x%08" PRIX32,
            (unsigned)rep.p1_sa_index, (unsigned)rep.valid_count,
            (unsigned)rep.erased_count, (unsigned)rep.corrupt_count,
            picofw_norfrom_state_name(st11), sa11_crc
        );
        (void)picofw_push_emit_str("UNBRICK_READY", frame_msg);
        (void)picofw_push_flush_global();
        printf("[NORFROM] SA11_0x%08" PRIX32 "=%s crc32=0x%08" PRIX32 " p1_sa=%u\n",
               sa11_va, picofw_norfrom_state_name(st11), sa11_crc, (unsigned)rep.p1_sa_index);
    }
    printf("BGO_INSPECT: OK\n");
}

void picofw_cmd_eemul_inspect(const char *arg) {
    (void)arg;
    if (!check_ftl_target_ready("EEMUL_INSPECT")) {
        return;
    }
    const picofw_flash_geometry_t geo = picofw_flash_get_geometry(picofw_flash_get_active_size());
    if ((geo.nvram_size == 0UL) || (geo.sector_count == 0u) ||
        (geo.sector_count > PICOFW_EEMUL_MAX_MICROSECS)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("EEMUL_INSPECT: ERROR (Target has no EEMUL NVRAM partition)\n");
        return;
    }
    /* Stage microsector headers in shared 32 KiB arena instead of Core-0 stack. */
    uint8_t *const heads = g_picofw_cmd_arena.raw;
    const uint32_t step = geo.nvram_size / geo.sector_count;
    for (uint32_t i = 0u; i < geo.sector_count; ++i) {
        const uint32_t sec_addr = geo.hw_nvram_base + (i * step);
        if (!picofw_hw_read_block_checked(sec_addr, &heads[i * 64u], 64u, true)) {
            picofw_cli_set_result("ERR", "EBUS");
            printf("EEMUL_INSPECT: ERROR (Bus read failed at microsector %" PRIu32 " @ 0x%08" PRIX32 ")\n",
                   i, sec_addr);
            return;
        }
    }
    picofw_eemul_report_t rep;
    (void)picofw_eemul_inspect_slice(heads, geo.sector_count * 64u, 64u, &rep);
    printf("[EEMUL_INSPECT] base=0x%08" PRIX32 " sectors=%u blank=%u active=%u hdr_dirty=%u%% crc32=0x%08" PRIX32 " healthy=%s\n",
           geo.hw_nvram_base, (unsigned)rep.total_microsectors,
           (unsigned)rep.blank_sectors, (unsigned)rep.active_sectors,
           (unsigned)rep.dirty_non_ff_bytes_pct, rep.crc32,
           rep.healthy_eemul ? "YES" : "NO");
    printf("EEMUL_INSPECT: OK\n");
}
#endif
