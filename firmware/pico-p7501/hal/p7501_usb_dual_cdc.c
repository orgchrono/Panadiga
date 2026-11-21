/**
 * @file p7501_usb_dual_cdc.c
 * @brief USB Dual-CDC runtime router and CLI handler (`USBMODE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/p7501_usb_dual_cdc.h"
#include "cli/p7501_cli.h"
#include "hal/p7501_wireless_codec.h"

#if !defined(PICO_HOST_TEST) && defined(CFG_TUD_CDC) && (CFG_TUD_CDC >= 2)
#include "tusb.h"
#endif

#include <stdio.h>

bool p7501_usb_is_dual_cdc(void) {
#if defined(CFG_TUD_CDC) && (CFG_TUD_CDC >= 2)
    return p7501_wireless_global_state()->usb_mode == P7501_USB_MODE_DUAL_CDC;
#else
    return false;
#endif
}

void p7501_usb_mirror_rx_byte(uint8_t byte_val) {
    if (!p7501_usb_is_dual_cdc()) {
        return;
    }
#if !defined(PICO_HOST_TEST) && defined(CFG_TUD_CDC) && (CFG_TUD_CDC >= 2)
    if (tud_cdc_n_connected(1u)) {
        tud_cdc_n_write_char(1u, (char)byte_val);
        if ((byte_val == '\n') || (tud_cdc_n_write_available(1u) < 8u)) {
            tud_cdc_n_write_flush(1u);
        }
    }
#else
    (void)byte_val;
#endif
}

void p7501_cmd_usbmode(const char *arg) {
    bool is_status = true;
    p7501_usb_mode_t mode = P7501_USB_MODE_SINGLE;
    if (!p7501_usbmode_parse_args(arg, &is_status, &mode)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("USBMODE: ERROR (Usage: USBMODE [SINGLE|DUAL_CDC|STATUS])\n");
        return;
    }

    p7501_wireless_auth_state_t *st = p7501_wireless_global_state();
#if defined(CFG_TUD_CDC) && (CFG_TUD_CDC >= 2)
    const char *itf1_hw = "TINYUSB_CDC_ITF1";
    if (!is_status) {
        st->usb_mode = mode;
    }
#else
    const char *itf1_hw = "TAGGED_ITF0_ONLY";
    if (!is_status && (mode == P7501_USB_MODE_DUAL_CDC)) {
        p7501_cli_set_result("ERR", "ENOSYS");
        printf("USBMODE: ERR code=ENOSYS reason=CFG_TUD_CDC_SINGLE_ONLY itf1=%s\n", itf1_hw);
        return;
    }
    if (!is_status) {
        st->usb_mode = mode;
    }
#endif

    printf("USBMODE: OK (mode=%s itf0=CLI itf1=%s active=%s)\n",
           (st->usb_mode == P7501_USB_MODE_DUAL_CDC) ? "DUAL_CDC" : "SINGLE",
           itf1_hw,
           p7501_usb_is_dual_cdc() ? "YES" : "NO");
}
