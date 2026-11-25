/**
 * @file picofw_bootsel_btn.h
 * @brief Runtime RP2040/RP2350 `.time_critical` QSPI CS BOOTSEL button reader & stand-alone poller.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_BOOTSEL_BTN_H
#define PICOFW_BOOTSEL_BTN_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Read physical BOOTSEL button state safely from RAM (`.time_critical`) without XIP contention. */
bool picofw_bootsel_read_raw(void);

/** Non-blocking idle-loop poll (50 ms cadence): short press -> IDENTIFY+BLACKBOX, 3s hold -> VAULT_BACKUP. */
void picofw_bootsel_poll(void);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_BOOTSEL_BTN_H */
