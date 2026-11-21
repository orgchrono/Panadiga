/**
 * @file p7501_ble_nus.c
 * @brief Wireless BLE NUS & Wi-Fi TCP :3333 status, F14 authorization CLI, and disconnect relock.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/p7501_ble_nus.h"
#include "cli/p7501_cli.h"
#include "common/p7501_hex.h"
#include "hal/p7501_wireless_codec.h"
#include "uart/p7501_uart_codec.h"

#if defined(PANADIGA_BLE_ENABLED) && !defined(PICO_HOST_TEST)
#include "pico/cyw43_arch.h"
#endif

#include <inttypes.h>
#include <stdio.h>

static bool s_radio_initialized = false;

bool p7501_ble_is_radio_ready(void) {
    return s_radio_initialized;
}

void p7501_ble_nus_init(void) {
#if defined(PANADIGA_BLE_ENABLED) && !defined(PICO_HOST_TEST)
    if (!s_radio_initialized && (cyw43_arch_init() == 0)) {
        s_radio_initialized = true;
    }
#else
    s_radio_initialized = false;
#endif
}

void p7501_ble_nus_poll(void) {
#if defined(PANADIGA_BLE_ENABLED) && !defined(PICO_HOST_TEST)
    if (s_radio_initialized) {
        cyw43_arch_poll();
    }
#endif
}

void p7501_cmd_ble(const char *arg) {
    p7501_wireless_auth_state_t *st = p7501_wireless_global_state();
    char tok[24] = {0};
    const bool has_tok = (arg != NULL) && (p7501_next_token(arg, tok, sizeof(tok)) != NULL) && (tok[0] != '\0');
    if (has_tok && p7501_ascii_ieq(tok, "DISCONNECT")) {
        p7501_wireless_on_disconnect(st);
        printf("BLE: OK (DISCONNECTED, NOR_WRITE=LOCKED)\n");
        return;
    }
    if (has_tok && !p7501_ascii_ieq(tok, "STATUS")) {
        p7501_cli_set_result("ERR", "ENOSYS");
        printf("BLE: ERR code=ENOSYS reason=BTSTACK_GATT_NUS_NOT_IMPLEMENTED (requested=%s)\n", tok);
        return;
    }

    const bool has_radio =
#if defined(PANADIGA_BLE_ENABLED)
        true;
#else
        false;
#endif

    printf("BLE: OK (radio=%s init=%s gatt_nus=NOT_IMPLEMENTED origin=%s ble_conn=%s wifi_tcp_port=%u nor_auth=%s ops_rem=%" PRIu32 ")\n",
           has_radio ? "CYW43439" : "NONE",
           s_radio_initialized ? "YES" : "NO",
           p7501_origin_name(st->active_origin),
           st->ble_connected ? "YES" : "NO",
           (unsigned)st->wifi_gdb_port,
           st->ble_nor_write_granted ? "GRANTED" : "LOCKED",
           st->ble_nor_grant_remaining_ops);
}

void p7501_cmd_ble_nor_auth(const char *arg) {
    p7501_wireless_auth_state_t *st = p7501_wireless_global_state();
    const p7501_cmd_origin_t caller = st->active_origin;
    if (!p7501_wireless_handle_usb_auth_cmd(st, caller, arg)) {
        if (caller != P7501_ORIGIN_USB_CDC) {
            p7501_cli_set_result("ERR", "E05");
            printf("BLE_NOR_AUTH: ERR code=E05 reason=USB_ORIGIN_REQUIRED caller=%s\n",
                   p7501_origin_name(caller));
        } else {
            p7501_cli_set_result("ERR", "EINVAL");
            printf("BLE_NOR_AUTH: ERROR (Usage: BLE_NOR_AUTH <CONFIRM [1..16]|REVOKE|STATUS>)\n");
        }
        return;
    }

    printf("BLE_NOR_AUTH: OK (origin=%s nor_auth=%s ops_rem=%" PRIu32 ")\n",
           p7501_origin_name(st->active_origin),
           st->ble_nor_write_granted ? "GRANTED" : "LOCKED",
           st->ble_nor_grant_remaining_ops);
}
