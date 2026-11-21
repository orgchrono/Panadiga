/**
 * @file p7501_wireless_codec.h
 * @brief Pure ISO C11 USB/BLE/Wi-Fi transport policy, Dual-CDC mode parsing, and F14 NOR write guard.
 *
 * Physical/Security Invariant (F14):
 * - NOR Flash writes/erases/unlocks from BLE NUS or Wi-Fi TCP (:3333) are strictly
 *   blocked (`E05`, FAIL-CLOSED) unless explicitly authorized from physical USB CDC
 *   via `BLE_NOR_AUTH CONFIRM`, and auto-relock on disconnect or budget completion.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_WIRELESS_CODEC_H
#define P7501_WIRELESS_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_WIFI_GDB_TCP_PORT       3333u
#define P7501_BLE_NOR_DEFAULT_OPS     4u
#define P7501_BLE_NOR_MAX_OPS         16u

typedef enum p7501_cmd_origin {
    P7501_ORIGIN_USB_CDC = 0,
    P7501_ORIGIN_BLE_NUS = 1,
    P7501_ORIGIN_WIFI_TCP = 2
} p7501_cmd_origin_t;

typedef enum p7501_usb_mode {
    P7501_USB_MODE_SINGLE = 0,
    P7501_USB_MODE_DUAL_CDC = 1
} p7501_usb_mode_t;

typedef struct p7501_wireless_auth_state {
    p7501_cmd_origin_t active_origin;
    bool ble_connected;
    bool wifi_tcp_connected;
    bool ble_nor_write_granted;
    uint32_t ble_nor_grant_remaining_ops;
    p7501_usb_mode_t usb_mode;
    uint16_t wifi_gdb_port;
} p7501_wireless_auth_state_t;

/** Initialize fail-closed wireless & USB transport policy state. */
void p7501_wireless_state_init(p7501_wireless_auth_state_t *st);

/** Return singleton policy state shared across USB/BLE/Wi-Fi/Flash runtime handlers. */
p7501_wireless_auth_state_t *p7501_wireless_global_state(void);

/** Return true iff active transport origin is allowed to mutate NOR Flash (`F14`). */
bool p7501_wireless_nor_write_allowed(const p7501_wireless_auth_state_t *st);

/**
 * Process `BLE_NOR_AUTH <CONFIRM [ops]|REVOKE|STATUS>` (F14).
 * Only physical `P7501_ORIGIN_USB_CDC` may grant authorization; wireless callers fail closed.
 */
bool p7501_wireless_handle_usb_auth_cmd(
    p7501_wireless_auth_state_t *st,
    p7501_cmd_origin_t caller_origin,
    const char *arg
);

/** Consume one granted NOR write op on wireless transports; auto-relocks when budget hits 0. */
void p7501_wireless_consume_nor_op(p7501_wireless_auth_state_t *st);

/** Immediately revoke wireless NOR write grant and reset origin back to USB CDC. */
void p7501_wireless_on_disconnect(p7501_wireless_auth_state_t *st);

/** Parse `USBMODE [SINGLE|DUAL_CDC|STATUS]`. Returns false only on invalid token. */
bool p7501_usbmode_parse_args(
    const char *arg,
    bool *out_is_status,
    p7501_usb_mode_t *out_mode
);

/** Return static uppercase label for transport origin. */
const char *p7501_origin_name(p7501_cmd_origin_t origin);

#ifdef __cplusplus
}
#endif

#endif /* P7501_WIRELESS_CODEC_H */
