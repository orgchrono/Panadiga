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
        printf("DIAGWATCH: ERROR (Usage: DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] [AUTO|BYPASS|READ[:a[,n]]|WRITE:<a,v>|FORMAT|DUMP|JUMP:<a>|ERASE:<a_nvram>|QUIT|NONE] [RELEASE|HOLD])\n");
        return;
    }

    p7501_diag_watch_state_t st;
    p7501_diag_watch_init(&st);

    p7501_pio_enforce_nsw5v_hiz();
    if (p7501_pio_get_baud() != P7501_UART_DEFAULT_BAUD) {
        (void)p7501_pio_set_baud(P7501_UART_DEFAULT_BAUD);
    }
    if (p7501_pio_get_framing() != P7501_FRAMING_8N1) {
        (void)p7501_pio_set_framing(P7501_FRAMING_8N1);
    }
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    printf("=== DIAGWATCH PIO1 (baud=%" PRIu32 " framing=%s ms=%" PRIu32 " boot=%d hold=%" PRIu32 " act=%d rel_ddr=%s) ===\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()),
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
            if ((hit & P7501_DIAG_EVT_SD_HW_OK) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] STAGE: SD hardware 4-bit init OK (... SD init end)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_FAT_FAIL) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: FAT16 Mount/Open error emitted by bootloader (-10010/-10020/-10025)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_SD_DIAG_LOAD_FAIL) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: SD diag load failed[0] (if no FAT Mount/Open error: fat_read 0x20000 failed / <128KiB)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: DIGADIAG uncompress/header error detected\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_DIAG_EXEC) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] STAGE: DIGADIAG.FRM decompressed & launched\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_SD_ERR) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: SDDIAG 4-bit status failure detected (-91006/-90002/-90001)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_ODD_TRAY_FAIL) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: ODD tray close error detected (** can not close tray ** / 80045300)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_CD_APC_ERR) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: CDDIAG laser APC error detected (80040904 / 0x83A7)\n", elapsed_ms);
            }
            if ((hit & P7501_DIAG_EVT_BGO_FAIL) != 0u) {
                printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: BGO driver init failure detected\n", elapsed_ms);
            }
            if (tx_ch != '\0') {
                char tx_buf[36];
                const size_t tx_len = p7501_diag_watch_format_tx(&spec, tx_buf, sizeof(tx_buf));
                if (tx_len <= 1u) {
                    const uint8_t tx_u8 = (uint8_t)tx_ch;
                    p7501_pio_tx_bytes(&tx_u8, 1u, false);
                    printf("\n[DIAGWATCH +%" PRIu32 "ms] PANAMON1> matched -> injected '%c'\n", elapsed_ms, tx_ch);
                } else {
                    for (size_t ti = 0u; ti < tx_len; ++ti) {
                        const uint8_t tx_u8 = (uint8_t)tx_buf[ti];
                        p7501_pio_tx_bytes(&tx_u8, 1u, false);
                        sleep_us(200u);
                    }
                    printf("\n[DIAGWATCH +%" PRIu32 "ms] PANAMON1> matched -> injected %.*s\n",
                           elapsed_ms,
                           (int)(tx_len - 1u),
                           tx_buf);
                }
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

    printf("\nDIAGWATCH: OK (rx=%" PRIu32 " bytes, ferr=%" PRIu32 ", events=0x%04" PRIX32 ", sw_rel=%s, tx=%s, ovf=%s)\n",
           rx_count,
           ferr_count,
           st.seen_events,
           st.sw_uart_released ? "YES" : "NO",
           st.tx_dispatched ? "YES" : "NO",
           fifo_ovf ? "YES" : "NO");
    printf("[DIAGWATCH] Diagnosis: %s\n", p7501_diag_watch_diagnosis(st.seen_events));
}
