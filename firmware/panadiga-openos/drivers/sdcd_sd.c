/**
 * @file sdcd_sd.c
 * @brief Read-only AM34 `SDCD` SD/MMC Host Controller Status & Bus-Width Driver.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

#include <string.h>

bool openos_sdcd_probe_status(openos_sdcd_status_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    uint32_t ctrl = 0UL;
    uint32_t cfg = 0UL;
    if (!am34_mmio_read32(AM34_OPENOS_SDCD_BASE, &ctrl) ||
        !am34_mmio_read32(AM34_OPENOS_SDCD_BUS_CFG, &cfg)) {
        return false;
    }
    out->ctrl_reg = ctrl;
    out->bus_cfg_reg = cfg;
    /* BOOT.BIN clears 0x89001850 bit 15 (0x8000) when switching SD controller to 4-bit mode. */
    out->bus_width_4bit = (cfg & AM34_OPENOS_SDCD_1BIT_MASK) == 0UL;
    return true;
}
