/**
 * @file p7501_identify.c
 * @brief Runtime 4-stage `IDENTIFY`/`AUTOPROBE` execution over P7501 ADC0, GPIO, and PIO1.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_identify.h"
#include "cli/p7501_cli.h"
#include "common/p7501_hex.h"
#include "flash/p7501_flash.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_uart_capture.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>

static char s_ident_line[320];

void p7501_identify_run_hw(const char *arg, p7501_ident_report_t *out_rep) {
    p7501_hw_enforce_nsw5v_hiz();
    p7501_wdt_kick();

    uint32_t sniff_ms = 80u;
    p7501_boot_action_t boot_action = P7501_BOOT_ACTION_PASSIVE;
    uint32_t jig_hold_ms = 0u;
    bool no_xreset = false;
    if (arg != NULL) {
        char tok[24];
        const char *cur = arg;
        while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
            if (tok[0] == '\0') {
                continue;
            }
            if (p7501_ascii_is_digits(tok)) {
                sniff_ms = p7501_parse_uint32_clamped(tok, 80u, 10u, 5000u);
            } else if (p7501_is_boot_spec_token(tok)) {
                boot_action = p7501_parse_boot_spec_ex(tok, &jig_hold_ms, &no_xreset);
            } else if (p7501_flash_set_target_profile_name(tok)) {
                /* Target profile pre-selected via IDENTIFY <MODEL> */
            } else {
                p7501_cli_set_result("ERR", "EINVAL");
                printf("IDENTIFY: ERROR (Usage: IDENTIFY [PASSIVE|NORMAL|JIG[:ms]|PREJIG[:ms]] [ms] [<MODEL>])\n");
                return;
            }
        }
    }

    const uint32_t vdd_mv = p7501_hw_read_vdd_mv();
    const bool nsw_h = gpio_get(P7501_PIN_NSW5V);
    const bool target_powered = nsw_h || (vdd_mv >= P7501_SSOT_VDD_MIN_MV);
    /* Avoid enabling GP26 internal 3.3V pull-up against an unpowered target board. */
    const uint32_t sw_pu_mv = target_powered ? p7501_hw_sample_sw_uart_mv(false, true) : vdd_mv;
    const bool rxd_h = gpio_get(P7501_PIN_RXD);
    const bool txd_h = gpio_get(P7501_PIN_TXD);

    size_t rx_cap = 0u;
    uint8_t *rx_buf = p7501_uart_rx_buffer(&rx_cap);
    p7501_pio_begin_rx_session_cfg(boot_action, jig_hold_ms, no_xreset);
    const uint32_t rx_len = p7501_uart_capture_window(sniff_ms * 1000UL, 1024u);
    p7501_pio_end_session();

    uint32_t ipl_crc = 0u;
    if (target_powered) {
        uint8_t ipl_words[16];
        if (p7501_hw_read_block_checked(P7501_FLASH_CACHED_BASE, ipl_words, sizeof(ipl_words), false)) {
            bool all_ff = true;
            for (size_t i = 0u; i < sizeof(ipl_words); ++i) {
                if (ipl_words[i] != 0xFFu) {
                    all_ff = false;
                    break;
                }
            }
            if (!all_ff) {
                ipl_crc = p7501_crc32_ieee(ipl_words, sizeof(ipl_words));
            }
        }
    }

    const p7501_session_ctx_t *sess = p7501_session_default_ctx();
    const uint16_t mfg = sess->jedec_verified ? sess->jedec_mfg : 0u;
    const uint16_t dev0 = sess->jedec_verified ? sess->jedec_dev[0] : 0u;

    p7501_ident_report_t local_rep;
    p7501_identify_evaluate(
        vdd_mv,
        sw_pu_mv,
        nsw_h,
        rxd_h,
        txd_h,
        rx_buf,
        (rx_len <= rx_cap) ? (size_t)rx_len : rx_cap,
        mfg,
        dev0,
        ipl_crc,
        true,
        &local_rep
    );

    (void)p7501_identify_format_line(&local_rep, s_ident_line, sizeof(s_ident_line));
    printf("%s\n", s_ident_line);
    if ((local_rep.vdd_mv < P7501_SSOT_VDD_MIN_MV) && !local_rep.nsw5v_high) {
        p7501_cli_set_result("ERR", "ENOPWR");
        printf("IDENTIFY: OFFLINE (model=%s vdd_mv=%" PRIu32 " nsw5v=0)\n",
               local_rep.model_name, local_rep.vdd_mv);
    } else {
        printf("IDENTIFY: OK (model=%s flash=%s vdd_mv=%" PRIu32 " xip_ro=%s)\n",
               local_rep.model_name,
               local_rep.flash_part,
               local_rep.vdd_mv,
               local_rep.xip_main_ro_locked ? "LOCKED" : "NONE");
    }

    if (out_rep != NULL) {
        *out_rep = local_rep;
    }
}

void p7501_cmd_identify(const char *arg) {
    p7501_identify_run_hw(arg, NULL);
}
