/**
 * @file nor_bgo_ro.c
 * @brief Strictly Read-Only Nor-FROM (`SA11`), BGO (`SA26..SA70`), and EEMUL (`SA134`) Inspector.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Never issues erase/program writes into NOR Flash (`SA0..SA10` remain `E05` blocked).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

#include <string.h>

#define OPENOS_P4ZU_MAGIC_LE 0x555A3450UL

bool openos_nor_inspect_unbrick(openos_unbrick_status_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->bgo_p1_sa = 0xFFFFu;

    uint32_t sa11_w0 = 0UL;
    if (!am34_mmio_read32(OPENOS_SSOT_NORFROM_SA11_BASE, &sa11_w0)) {
        return false;
    }
    out->sa11_is_erased = (sa11_w0 == 0xFFFFFFFFUL);
    out->sa11_has_p4zu = (sa11_w0 == OPENOS_P4ZU_MAGIC_LE);

    for (uint32_t idx = 0UL; idx < OPENOS_SSOT_BGO_SECTOR_COUNT; ++idx) {
        const uint32_t lbn_va =
            OPENOS_SSOT_BGO_BASE + (idx * OPENOS_SSOT_BGO_SECTOR_BYTES) + 4UL;
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
    if (!am34_mmio_read32(OPENOS_SSOT_EEMUL_BASE, &eemul_w0)) {
        return false;
    }
    out->eemul_has_1284 = (eemul_w0 == OPENOS_SSOT_EEMUL_TAG_1284_LE);
    out->nor_autoboot_ready =
        (out->bgo_p1_count >= 1u) && (out->bgo_p2_count >= 40u) && out->sa11_is_erased;
    return true;
}
