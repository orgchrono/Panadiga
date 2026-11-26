/**
 * @file nor_bgo_ro.c
 * @brief Strictly Read-Only Nor-FROM (`SA11`), BGO (`SA26..SA70`), and EEMUL (`SA134`) Inspector for `DigaOS`.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Never issues erase/program writes into NOR Flash (`SA0..SA10` remain `E05` blocked).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/digaos_drivers.h"

#include <string.h>

bool digaos_nor_inspect_unbrick(digaos_unbrick_status_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->bgo_p1_sa = 0xFFFFu;

    uint32_t sa11_w0 = 0UL;
    uint32_t sa11_magic = 0UL;
    if (!am34_mmio_read32(DIGAOS_SSOT_NORFROM_SA11_BASE, &sa11_w0) ||
        !am34_mmio_read32(
            DIGAOS_SSOT_NORFROM_SA11_BASE + DIGAOS_SSOT_NOR_AUTOBOOT_MAGIC_OFFSET,
            &sa11_magic)) {
        return false;
    }
    out->sa11_is_erased = (sa11_w0 == 0xFFFFFFFFUL) && (sa11_magic == 0xFFFFFFFFUL);
    out->sa11_has_autoboot_capsule = (sa11_magic == DIGAOS_SSOT_NOR_AUTOBOOT_MAGIC_LE);

    for (uint32_t idx = 0UL; idx < DIGAOS_SSOT_BGO_SECTOR_COUNT; ++idx) {
        const uint32_t lbn_va =
            DIGAOS_SSOT_BGO_BASE + (idx * DIGAOS_SSOT_BGO_SECTOR_BYTES) + 4UL;
        uint16_t lbn = 0u;
        if (!am34_mmio_read16(lbn_va, &lbn)) {
            return false;
        }
        if (lbn == 0xFFFFu) {
            out->bgo_erased_count = (uint16_t)(out->bgo_erased_count + 1u);
        } else if (lbn == 0xF101u) {
            if (out->bgo_p1_sa == 0xFFFFu) {
                out->bgo_p1_sa = (uint16_t)(26u + idx);
            }
            out->bgo_p1_count = (uint16_t)(out->bgo_p1_count + 1u);
        } else if ((lbn >= 0xF201u) && (lbn <= 0xF23Fu)) {
            out->bgo_p2_count = (uint16_t)(out->bgo_p2_count + 1u);
        }
    }

    uint32_t eemul_w0 = 0UL;
    if (!am34_mmio_read32(DIGAOS_SSOT_EEMUL_BASE, &eemul_w0)) {
        return false;
    }
    out->eemul_has_1284 = (eemul_w0 == DIGAOS_SSOT_EEMUL_TAG_1284_LE);
    out->nor_autoboot_ready =
        (out->bgo_p1_count >= 1u) && (out->bgo_p2_count >= 40u) && out->sa11_is_erased;
    return true;
}

bool digaos_autoboot_stamp_matches(uint32_t prog_stamp_0x504395fc, uint32_t hdd_extrhead_0x28) {
    if ((prog_stamp_0x504395fc == 0UL) || (prog_stamp_0x504395fc == 0xFFFFFFFFUL)) {
        return false;
    }
    return prog_stamp_0x504395fc == hdd_extrhead_0x28;
}
