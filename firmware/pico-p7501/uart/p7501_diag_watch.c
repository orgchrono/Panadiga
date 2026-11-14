/**
 * @file p7501_diag_watch.c
 * @brief Runtime PIO1 boot recovery watchdog (`DIAGWATCH`) for DMR-EH55 P7501.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_diag_watch.h"
#include "uart/p7501_diag_watch_codec.h"
#include "uart/p7501_uart_pio_engine.h"
#include "hal/p7501_led.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>

void p7501_cmd_diag_watch(const char *arg) {
    p7501_diag_watch_spec_t spec;
    if (!p7501_diag_watch_parse_args(arg, &spec)) {
        printf("DIAGWATCH: ERROR (Usage: DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] [AUTO|BYPASS|FORMAT|DUMP|NONE] [RELEASE|HOLD])\n");
        return;
    }

    p7501_diag_watch_state_t st;
    p7501_diag_watch_init(&st);

    p7501_pio_enforce_nsw5v_hiz();
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    printf("=== DIAGWATCH PIO1 (baud=%" PRIu32 " ms=%" PRIu32 " boot=%d hold=%" PRIu32 " act=%d rel_ddr=%s) ===\n",
           p7501_pio_get_baud(),
           spec.timeout_ms,
           (int)spec.boot_action,
           spec.jig_hold_ms,
           (int)spec.action,
           spec.release_on_ddr ? "YES" : "NO");

    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    const uint64_t t0_us = time_us_64();
    const uint64_t deadline = t0_us + ((uint64_t)spec.timeout_ms * 1000ULL);
    uint32_t rx_count = 0u;
    uint32_t ferr_count = 0u;

    while (time_us_64() < deadline) {
        p7501_pio_poll_sw_uart_release();
        if (p7501_pio_poll_and_clear_framing_err()) {
            ferr_count += 1u;
        }
        uint8_t b = 0u;
        while (p7501_pio_pop_rx_byte(&b)) {
            putchar((int)b);
            rx_count += 1u;
            const uint32_t elapsed_ms = (uint32_t)((time_us_64() - t0_us) / 1000ULL);

            char tx_ch = '\0';
            bool release_now = false;
            const uint32_t hit = p7501_diag_watch_feed_byte(&st, &spec, b, &tx_ch, &release_now);
            if (release_now) {
                p7501_pio_release_sw_uart_now();
                printf("\n[DIAGWATCH +%" PRIu32 "ms] %s -> SW_UART released Hi-Z (Q7514 TX ready)\n",
                       elapsed_ms,
                       ((hit & P7501_DIAG_EVT_SERIAL_BOOT) != 0u) ? "serial boot mode" : "DDR CHECK: OK");
            }
            if ((hit & P7501_DIAG_EVT_SD_ERR) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: SDDIAG 4-bit status failure detected (-91006/-90002)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_CD_APC_ERR) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: CDDIAG laser APC error detected (80040904 / 0x83A7)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_BGO_FAIL) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: BGO driver init failure detected\n", elapsed_ms);
            }
            if (tx_ch != '\0') {
                const uint8_t tx_u8 = (uint8_t)tx_ch;
                p7501_pio_tx_bytes(&tx_u8, 1u, false);
                printf("\n[DIAGWATCH +%" PRIu32 "ms] PANAMON1> matched -> injected '%c'\n", elapsed_ms, tx_ch);
            }
        }
        if ((rx_count & 0xFFu) == 0u) {
            stdio_flush();
        }
        tight_loop_contents();
    }
    stdio_flush();

    const bool fifo_ovf = p7501_pio_had_rx_overflow();
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf("\nDIAGWATCH: OK (rx=%" PRIu32 " bytes, ferr=%" PRIu32 ", events=0x%02" PRIX32 ", sw_rel=%s, tx=%s, ovf=%s)\n",
           rx_count,
           ferr_count,
           st.seen_events,
           st.sw_uart_released ? "YES" : "NO",
           st.tx_dispatched ? "YES" : "NO",
           fifo_ovf ? "YES" : "NO");
    printf("[DIAGWATCH] Diagnosis: %s\n", p7501_diag_watch_diagnosis(st.seen_events));
}
