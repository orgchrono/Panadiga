/**
 * @file picofw_ble_nus.c
 * @brief Wireless BLE NUS & Wi-Fi TCP :3333 status, F14 authorization CLI, and disconnect relock.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_ble_nus.h"
#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "hal/picofw_wireless_codec.h"
#include "uart/picofw_uart_codec.h"

#if defined(PICOFW_BLE_ENABLED) && !defined(PICO_HOST_TEST)
#include "pico/cyw43_arch.h"
#endif

#include <inttypes.h>
#include <stdio.h>

static bool s_radio_initialized = false;

bool picofw_ble_is_radio_ready(void) {
    return s_radio_initialized;
}

void picofw_ble_nus_init(void) {
#if defined(PICOFW_BLE_ENABLED) && !defined(PICO_HOST_TEST)
    if (!s_radio_initialized && (cyw43_arch_init() == 0)) {
        s_radio_initialized = true;
    }
#else
    s_radio_initialized = false;
#endif
}

void picofw_ble_nus_poll(void) {
#if defined(PICOFW_BLE_ENABLED) && !defined(PICO_HOST_TEST)
    if (s_radio_initialized) {
        cyw43_arch_poll();
    }
#endif
}

void picofw_cmd_ble(const char *arg) {
    picofw_wireless_auth_state_t *st = picofw_wireless_global_state();
    char tok[24] = {0};
    const bool has_tok = (arg != NULL) && (picofw_next_token(arg, tok, sizeof(tok)) != NULL) && (tok[0] != '\0');
    if (has_tok && picofw_ascii_ieq(tok, "DISCONNECT")) {
        picofw_wireless_on_disconnect(st);
        printf("BLE: OK (DISCONNECTED, NOR_WRITE=LOCKED)\n");
        return;
    }
    if (has_tok && !picofw_ascii_ieq(tok, "STATUS")) {
        picofw_cli_set_result("ERR", "ENOSYS");
        printf("BLE: ERR code=ENOSYS reason=BTSTACK_GATT_NUS_NOT_IMPLEMENTED (requested=%s)\n", tok);
        return;
    }

    const bool has_radio =
#if defined(PICOFW_BLE_ENABLED)
        true;
#else
        false;
#endif

    printf("BLE: OK (radio=%s init=%s gatt_nus=NOT_IMPLEMENTED origin=%s ble_conn=%s wifi_tcp_port=%u nor_auth=%s ops_rem=%" PRIu32 ")\n",
           has_radio ? "CYW43439" : "NONE",
           s_radio_initialized ? "YES" : "NO",
           picofw_origin_name(st->active_origin),
           st->ble_connected ? "YES" : "NO",
           (unsigned)st->wifi_gdb_port,
           st->ble_nor_write_granted ? "GRANTED" : "LOCKED",
           st->ble_nor_grant_remaining_ops);
}

void picofw_cmd_ble_nor_auth(const char *arg) {
    picofw_wireless_auth_state_t *st = picofw_wireless_global_state();
    const picofw_cmd_origin_t caller = st->active_origin;
    if (!picofw_wireless_handle_usb_auth_cmd(st, caller, arg)) {
        if (caller != PICOFW_ORIGIN_USB_CDC) {
            picofw_cli_set_result("ERR", "E05");
            printf("BLE_NOR_AUTH: ERR code=E05 reason=USB_ORIGIN_REQUIRED caller=%s\n",
                   picofw_origin_name(caller));
        } else {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("BLE_NOR_AUTH: ERROR (Usage: BLE_NOR_AUTH <CONFIRM [1..16]|REVOKE|STATUS>)\n");
        }
        return;
    }

    printf("BLE_NOR_AUTH: OK (origin=%s nor_auth=%s ops_rem=%" PRIu32 ")\n",
           picofw_origin_name(st->active_origin),
           st->ble_nor_write_granted ? "GRANTED" : "LOCKED",
           st->ble_nor_grant_remaining_ops);
}
