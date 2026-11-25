/**
 * @file picofw_wireless_codec.c
 * @brief Pure ISO C11 implementation of F14 wireless NOR write guard and Dual-CDC mode parsing.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_wireless_codec.h"
#include "common/picofw_hex.h"
#include "flash/picofw_flash.h"
#include "uart/picofw_uart_codec.h"

#include <string.h>

static picofw_wireless_auth_state_t s_global_wireless_state = {
    .active_origin = PICOFW_ORIGIN_USB_CDC,
    .ble_connected = false,
    .wifi_tcp_connected = false,
    .ble_nor_write_granted = false,
    .ble_nor_grant_remaining_ops = 0u,
    .usb_mode = PICOFW_USB_MODE_SINGLE,
    .wifi_gdb_port = PICOFW_WIFI_GDB_TCP_PORT,
};

void picofw_wireless_state_init(picofw_wireless_auth_state_t *st) {
    if (st == NULL) {
        return;
    }
    memset(st, 0, sizeof(*st));
    st->active_origin = PICOFW_ORIGIN_USB_CDC;
    st->usb_mode = PICOFW_USB_MODE_SINGLE;
    st->wifi_gdb_port = PICOFW_WIFI_GDB_TCP_PORT;
}

picofw_wireless_auth_state_t *picofw_wireless_global_state(void) {
    return &s_global_wireless_state;
}

bool picofw_wireless_nor_write_allowed(const picofw_wireless_auth_state_t *st) {
    if (st == NULL) {
        return false;
    }
    if (st->active_origin == PICOFW_ORIGIN_USB_CDC) {
        return true;
    }
    return st->ble_nor_write_granted && (st->ble_nor_grant_remaining_ops > 0u);
}

bool picofw_wireless_handle_usb_auth_cmd(
    picofw_wireless_auth_state_t *st,
    picofw_cmd_origin_t caller_origin,
    const char *arg
) {
    if (st == NULL) {
        return false;
    }
    char tok1[24];
    char tok2[24];
    const char *cur = (arg != NULL) ? picofw_next_token(arg, tok1, sizeof(tok1)) : NULL;
    if ((cur == NULL) || (tok1[0] == '\0') || picofw_ascii_ieq(tok1, "STATUS")) {
        return true;
    }
    if (picofw_ascii_ieq(tok1, "REVOKE")) {
        st->ble_nor_write_granted = false;
        st->ble_nor_grant_remaining_ops = 0u;
        picofw_flash_set_upper_nor_unlocked(false);
        return true;
    }
    if (!picofw_ascii_ieq(tok1, "CONFIRM")) {
        return false;
    }
    /* F14 fail-closed invariant: only physical USB CDC may authorize wireless NOR mutations. */
    if (caller_origin != PICOFW_ORIGIN_USB_CDC) {
        st->ble_nor_write_granted = false;
        st->ble_nor_grant_remaining_ops = 0u;
        return false;
    }
    uint32_t ops = PICOFW_BLE_NOR_DEFAULT_OPS;
    if ((picofw_next_token(cur, tok2, sizeof(tok2)) != NULL) && (tok2[0] != '\0')) {
        if (!picofw_ascii_is_digits(tok2)) {
            return false;
        }
        ops = picofw_parse_uint32_clamped(tok2, PICOFW_BLE_NOR_DEFAULT_OPS, 1u, PICOFW_BLE_NOR_MAX_OPS);
    }
    st->ble_nor_write_granted = true;
    st->ble_nor_grant_remaining_ops = ops;
    return true;
}

void picofw_wireless_consume_nor_op(picofw_wireless_auth_state_t *st) {
    if ((st == NULL) || (st->active_origin == PICOFW_ORIGIN_USB_CDC)) {
        return;
    }
    if (st->ble_nor_grant_remaining_ops > 0u) {
        --st->ble_nor_grant_remaining_ops;
    }
    if (st->ble_nor_grant_remaining_ops == 0u) {
        st->ble_nor_write_granted = false;
    }
}

void picofw_wireless_on_disconnect(picofw_wireless_auth_state_t *st) {
    if (st == NULL) {
        return;
    }
    st->ble_connected = false;
    st->wifi_tcp_connected = false;
    st->ble_nor_write_granted = false;
    st->ble_nor_grant_remaining_ops = 0u;
    st->active_origin = PICOFW_ORIGIN_USB_CDC;
    picofw_flash_set_upper_nor_unlocked(false);
}

bool picofw_usbmode_parse_args(
    const char *arg,
    bool *out_is_status,
    picofw_usb_mode_t *out_mode
) {
    if ((out_is_status == NULL) || (out_mode == NULL)) {
        return false;
    }
    *out_is_status = true;
    *out_mode = PICOFW_USB_MODE_SINGLE;
    if (arg == NULL) {
        return true;
    }

    char tok[24];
    if ((picofw_next_token(arg, tok, sizeof(tok)) == NULL) || (tok[0] == '\0') ||
        picofw_ascii_ieq(tok, "STATUS")) {
        *out_is_status = true;
        return true;
    }
    *out_is_status = false;
    if (picofw_ascii_ieq(tok, "SINGLE") || picofw_ascii_ieq(tok, "CDC1")) {
        *out_mode = PICOFW_USB_MODE_SINGLE;
        return true;
    }
    if (picofw_ascii_ieq(tok, "DUAL_CDC") || picofw_ascii_ieq(tok, "DUAL") ||
        picofw_ascii_ieq(tok, "CDC2")) {
        *out_mode = PICOFW_USB_MODE_DUAL_CDC;
        return true;
    }
    return false;
}

const char *picofw_origin_name(picofw_cmd_origin_t origin) {
    switch (origin) {
        case PICOFW_ORIGIN_BLE_NUS:
            return "BLE_NUS";
        case PICOFW_ORIGIN_WIFI_TCP:
            return "WIFI_TCP";
        case PICOFW_ORIGIN_USB_CDC:
        default:
            return "USB_CDC";
    }
}
