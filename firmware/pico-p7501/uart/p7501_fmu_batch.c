/**
 * @file p7501_fmu_batch.c
 * @brief Batch single-byte FMU response sweep (`FMUBATCH`) over P7501 SC0.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_fmu_probe.h"
#include "uart/p7501_fmu_internal.h"
#include "uart/p7501_uart_capture.h"
#include "uart/p7501_uart_pio_engine.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"

#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include "pico/stdlib.h"
#include "pico/time.h"

void p7501_cmd_fmu_batch(const char *arg) {
    uint32_t start_b = 0U;
    uint32_t count_b = 16U;
    uint32_t offset_ms = 20U;
    p7501_fmu_parse_batch_args(arg, &start_b, &count_b, &offset_ms);

    printf("\n=== FMUBATCH (0x%02" PRIX32 "..0x%02" PRIX32 ", offset=%" PRIu32 " ms after wait 1 second) ===\n",
           start_b, start_b + count_b - 1U, offset_ms);

    size_t rx_cap = 0U;
    uint8_t *rx_buf = p7501_uart_rx_buffer(&rx_cap);
    const uint32_t buf_lim = (rx_cap < 512U) ? (uint32_t)rx_cap : 512U;
    bool aborted = false;

    for (uint32_t b = start_b; !aborted && (b < (start_b + count_b)); b++) {
        const uint8_t bval = (uint8_t)b;
        p7501_fmu_phase_state_t st;
        p7501_fmu_start_phase_session(&st);
        p7501_wdt_set_stage(P7501_WDT_STAGE_PROBE_SWEEP);

        uint32_t rx_len = 0U;
        bool sent = false;
        uint8_t post_byte = 0U;
        bool got_post = false;
        uint32_t post_dt = 0U;
        uint32_t tx_us = 0U;
        uint32_t target_tx_us = 0U;
        const uint64_t t0_us = time_us_64();
        const uint64_t deadline_us = t0_us + ((uint64_t)P7501_FMU_BATCH_WINDOW_MS * 1000ULL);

        while (!got_post && !aborted && p7501_wdt_loop_step(t0_us, deadline_us, true, &aborted)) {
            const uint32_t now_us = time_us_32();
            while (p7501_fmu_pop_rx_byte(rx_buf, &rx_len, buf_lim)) {
                const uint8_t rx_b = rx_buf[rx_len - 1U];
                if (!sent) {
                    bool rel_sw = false;
                    bool saw_serial = false;
                    bool saw_wait1 = false;
                    p7501_fmu_feed_byte(&st, 0U, rx_b, now_us, &rel_sw, &saw_serial, &saw_wait1);
                    if ((offset_ms == 8888U) && saw_serial) {
                        p7501_pio_tx_bytes(&bval, 1u, false);
                        sent = true;
                        tx_us = time_us_32();
                    } else if (saw_wait1) {
                        target_tx_us = now_us + (offset_ms * 1000UL);
                    }
                } else {
                    post_byte = rx_b;
                    post_dt = time_us_32() - tx_us;
                    got_post = true;
                    break;
                }
            }

            if (!aborted && st.wait1_seen && (!sent) && (offset_ms != 8888U) &&
                ((int32_t)(now_us - target_tx_us) >= 0)) {
                p7501_pio_tx_bytes(&bval, 1u, false);
                sent = true;
                tx_us = time_us_32();
            }
        }

        const char printable = ((post_byte >= 0x20U) && (post_byte <= 0x7EU)) ? (char)post_byte : '.';
        printf("  0x%02X: trig=%d delay=%" PRIu32 " us first=0x%02X ('%c')\n",
               bval, sent ? 1 : 0, post_dt, post_byte, printable);
        if (got_post && ((post_dt < 980000UL) || (post_dt == 0U))) {
            printf("  [!!! HIT ON BYTE 0x%02X !!!]\n", bval);
            break;
        }
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    printf("FMUBATCH: OK%s\n", aborted ? " (aborted=YES)" : "");
}
