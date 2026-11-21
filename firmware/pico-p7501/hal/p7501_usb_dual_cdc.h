/**
 * @file p7501_usb_dual_cdc.h
 * @brief USB Dual-CDC (`ITF0` Control CLI + `ITF1` P7501 Passive Stream Mirror) runtime module (`#20`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_USB_DUAL_CDC_H
#define P7501_USB_DUAL_CDC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "hal/p7501_wireless_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Switch or query USB Dual-CDC passive stream mirroring (`ITF0` + `ITF1`). */
void p7501_cmd_usbmode(const char *arg);

/** Mirror one passive P7501 RX byte to USB CDC `ITF1` (or tagged @EVT stream when active). */
void p7501_usb_mirror_rx_byte(uint8_t byte_val);

/** Return true when Dual-CDC stream mirroring mode is active. */
bool p7501_usb_is_dual_cdc(void);

#ifdef __cplusplus
}
#endif

#endif /* P7501_USB_DUAL_CDC_H */
