/**
 * @file p7501_meihfs_probe.c
 * @brief Lightweight MEIHFS Sector 0 ('SH' MBR) & IDE2SAT2/DAVIS bridge micro-triage.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_meihfs_probe.h"

#include "common/p7501_hex.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

bool p7501_meihfs_inspect_sector0(const uint8_t sec[512], p7501_meihfs_mbr_info_t *out) {
    if ((sec == NULL) || (out == NULL)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->journal_lba = P7501_MEIHFS_JOURNAL_LBA;
    out->mirror_lba = P7501_MEIHFS_MIRROR_LBA;
    out->crc32 = p7501_crc32_ieee(sec, 512u);

    bool ff = true;
    bool zero = true;
    for (size_t i = 0u; i < 512u; ++i) {
        if (sec[i] != 0xFFu) {
            ff = false;
        }
        if (sec[i] != 0x00u) {
            zero = false;
        }
    }
    out->all_ff = ff;
    out->all_zero = zero;
    out->valid_sh_magic = (sec[0] == 'S') && (sec[1] == 'H');
    out->byte_swapped_hs = (sec[0] == 'H') && (sec[1] == 'S');
    out->ver_hi = sec[2];
    out->ver_lo = sec[3];
    return true;
}

size_t p7501_meihfs_format_report(
    uint32_t src_va,
    const p7501_meihfs_mbr_info_t *info,
    char *out,
    size_t cap
) {
    if ((info == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    const char *const blank = info->all_ff ? "FF" : (info->all_zero ? "ZERO" : "DATA");
    const int n = snprintf(
        out, cap,
        "@P7501_SURG op=MEIHFS va=0x%08" PRIX32 " valid_sh=%u swap_hs=%u blank=%s "
        "ver=%02X%02X crc32=0x%08" PRIX32 " jrn_lba=%u mir_lba=%u",
        src_va,
        info->valid_sh_magic ? 1u : 0u,
        info->byte_swapped_hs ? 1u : 0u,
        blank,
        (unsigned)info->ver_hi,
        (unsigned)info->ver_lo,
        info->crc32,
        (unsigned)info->journal_lba,
        (unsigned)info->mirror_lba
    );
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}
