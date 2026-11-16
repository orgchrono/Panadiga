/**
 * @file p7501_drv_pana.c
 * @brief Runtime `DRVPANA` Matshita R6-MV4 Drive Factory CLI dispatcher over PIO1.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_drv_pana.h"
#include "uart/p7501_drv_pana_codec.h"
#include "uart/p7501_uart_pio_engine.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static void drvpana_drain_ms(uint32_t wait_ms, uint32_t *rx_count, uint32_t *ferr_count) {
    const uint64_t t0_us = time_us_64();
    const uint64_t deadline = t0_us + ((uint64_t)wait_ms * 1000ULL);
    while (p7501_wdt_loop_step(t0_us, deadline, false, NULL)) {
        p7501_pio_poll_sw_uart_release();
        if (p7501_pio_poll_and_clear_framing_err()) {
            *ferr_count += 1u;
        }
        uint8_t b = 0u;
        while (p7501_pio_pop_rx_byte(&b)) {
            putchar((int)b);
            *rx_count += 1u;
            if ((*rx_count & 0xFFu) == 0u) {
                stdio_flush();
            }
        }
        tight_loop_contents();
    }
}

void p7501_cmd_drv_pana(const char *arg) {
    p7501_drv_pana_spec_t spec;
    if (!p7501_drv_pana_parse_args(arg, &spec)) {
        printf("DRVPANA: ERROR (Usage: DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd...>)\n");
        return;
    }

    char cmd_line[104];
    const size_t cmd_len = p7501_drv_pana_format_cmd(&spec, cmd_line, sizeof(cmd_line));
    if (cmd_len == 0u) {
        printf("DRVPANA: ERROR (command length overflow)\n");
        return;
    }

    p7501_pio_enforce_nsw5v_hiz();
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    printf("=== DRVPANA PIO1 (baud=%" PRIu32 " ms=%" PRIu32 " boot=%d hold=%" PRIu32 " pass=%s cmd=\"%s\") ===\n",
           p7501_pio_get_baud(),
           spec.timeout_ms,
           (int)spec.boot_action,
           spec.jig_hold_ms,
           spec.send_pass ? "AUTO" : "SKIP",
           spec.cmd);

    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    uint32_t rx_count = 0u;
    uint32_t ferr_count = 0u;
    if (spec.boot_action != P7501_BOOT_ACTION_PASSIVE) {
        const uint32_t settle_ms = (spec.jig_hold_ms > 250u) ? spec.jig_hold_ms : 250u;
        drvpana_drain_ms(settle_ms, &rx_count, &ferr_count);
    }

    if (spec.send_pass) {
        static const char k_pass_cmd[] = P7501_DRVPANA_PASS_LINE;
        p7501_pio_tx_bytes((const uint8_t *)k_pass_cmd, sizeof(k_pass_cmd) - 1u, false);
        drvpana_drain_ms(120u, &rx_count, &ferr_count);
    }

    p7501_pio_tx_bytes((const uint8_t *)cmd_line, cmd_len, false);
    drvpana_drain_ms(spec.timeout_ms, &rx_count, &ferr_count);
    stdio_flush();

    const bool fifo_ovf = p7501_pio_had_rx_overflow();
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf("\nDRVPANA: OK (rx=%" PRIu32 " bytes, ferr=%" PRIu32 ", pass=%s, ovf=%s, cmd=\"%s\")\n",
           rx_count,
           ferr_count,
           spec.send_pass ? "YES" : "NO",
           fifo_ovf ? "YES" : "NO",
           spec.cmd);
}
