/**
 * @file p7501_ble_nus.h
 * @brief Wireless BLE Nordic UART Service (CYW43439) & Wi-Fi TCP :3333 GDB bridge (`#21`, `F14`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_BLE_NUS_H
#define P7501_BLE_NUS_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Initialize CYW43439 BLE NUS / Wi-Fi TCP :3333 state when compiled for Pico W / Pico 2 W. */
void p7501_ble_nus_init(void);

/** Return true once CYW43439 radio driver (`cyw43_arch_init`) is ready for GPIO/LED calls. */
bool p7501_ble_is_radio_ready(void);

/** Non-blocking poll invoked in main idle loop to service BLE NUS / TCP GDB frames. */
void p7501_ble_nus_poll(void);

/** CLI command `BLE [STATUS|DISCONNECT|WIFI_GDB]` for wireless telemetry status & disconnect relock. */
void p7501_cmd_ble(const char *arg);

/** CLI command `BLE_NOR_AUTH <CONFIRM [ops]|REVOKE|STATUS>` (USB CDC only, F14 fail-closed guard). */
void p7501_cmd_ble_nor_auth(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_BLE_NUS_H */
