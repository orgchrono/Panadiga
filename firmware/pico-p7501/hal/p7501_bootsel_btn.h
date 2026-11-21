/**
 * @file p7501_bootsel_btn.h
 * @brief Runtime RP2040/RP2350 `.time_critical` QSPI CS BOOTSEL button reader & stand-alone poller.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_BOOTSEL_BTN_H
#define P7501_BOOTSEL_BTN_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Read physical BOOTSEL button state safely from RAM (`.time_critical`) without XIP contention. */
bool p7501_bootsel_read_raw(void);

/** Non-blocking idle-loop poll (50 ms cadence): short press -> IDENTIFY+BLACKBOX, 3s hold -> VAULT_BACKUP. */
void p7501_bootsel_poll(void);

#ifdef __cplusplus
}
#endif

#endif /* P7501_BOOTSEL_BTN_H */
