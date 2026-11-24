/**
 * @file davis_sata.c
 * @brief AM34 `DAVIS` CH0/CH1 ATA/ATAPI & StarTech `IDE2SAT2` Safe Post-Reset Driver.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

#define OPENOS_NATIVE_PATA_WAIT_MS  2U

bool openos_davis_reset_and_wait_ready(bool is_ch1_sata_bridge, uint32_t *out_waited_ms) {
    const uint32_t rst_addr = is_ch1_sata_bridge ? AM34_OPENOS_DAVIS_CH1_RST
                                                 : AM34_OPENOS_DAVIS_CH0_RST;
    const uint32_t base_addr = is_ch1_sata_bridge ? AM34_OPENOS_DAVIS_CH1_BASE
                                                  : AM34_OPENOS_DAVIS_CH0_BASE;
    const uint16_t rst_val = is_ch1_sata_bridge ? 0x0000U : 0x0002U;
    const uint32_t wait_ms = is_ch1_sata_bridge ? OPENOS_SATA_BRIDGE_WAIT_MS
                                                : OPENOS_NATIVE_PATA_WAIT_MS;

    if (!am34_mmio_write16(rst_addr, rst_val)) {
        return false;
    }
    openos_host_sleep_ms(wait_ms);
    if (out_waited_ms != NULL) {
        *out_waited_ms = wait_ms;
    }

    uint16_t status_word = 0u;
    return am34_mmio_read16(base_addr, &status_word);
}
