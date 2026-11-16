/**
 * @file p7501_fmu_probe.c
 * @brief Panasonic Bootrom FMU handshake probes (`FMUSCAN`, `FMUTEST`).
 *
 * Pure trigger detection and hit gating are delegated to `p7501_fmu_codec`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_fmu_probe.h"
#include "uart/p7501_fmu_internal.h"
#include "uart/p7501_uart_bitbang.h"
#include "uart/p7501_uart_capture.h"
#include "uart/p7501_uart_pio_engine.h"
#include "common/p7501_hex.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"

#include <inttypes.h>
#include <stdio.h>
#include "pico/stdlib.h"
#include "pico/time.h"

void p7501_fmu_start_phase_session(p7501_fmu_phase_state_t *out_st) {
    p7501_fmu_state_init(out_st);
    p7501_wdt_set_stage(P7501_WDT_STAGE_RESET_SYNC);
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session_ex(P7501_BOOT_ACTION_JIG, P7501_JIG_DEFAULT_HOLD_MS);
}

bool p7501_fmu_pop_rx_byte(uint8_t *rx_buf, uint32_t *rx_len, uint32_t max_cap) {
    uint8_t b = 0u;
    if (!p7501_pio_pop_rx_byte(&b)) {
        return false;
    }
    if ((rx_buf != NULL) && (rx_len != NULL) && (*rx_len < max_cap)) {
        rx_buf[*rx_len] = b;
        ++(*rx_len);
    }
    return true;
}

static bool run_single_fmu_scan_mode(uint32_t fmode) {
    size_t rx_cap = 0U;
    uint8_t *rx_buf = p7501_uart_rx_buffer(&rx_cap);
    p7501_fmu_phase_state_t st;
    p7501_fmu_start_phase_session(&st);
    p7501_wdt_set_stage(P7501_WDT_STAGE_PROBE_SWEEP);

    uint32_t rx_len = 0U;
    const uint64_t t0_us = time_us_64();
    const uint64_t deadline_us = t0_us + ((uint64_t)P7501_FMU_PROBE_WINDOW_MS * 1000ULL);
    bool sweep_done = false;
    bool aborted = false;
    int hit_byte = -1;
    uint16_t bval = 0U;
    uint32_t last_tx_us = 0U;
    uint32_t next_tx_us = 0U;
    bool waiting_reply = false;

    while (!aborted && p7501_wdt_loop_step(t0_us, deadline_us, true, &aborted)) {
        const uint32_t now_us = time_us_32();
        while (p7501_fmu_pop_rx_byte(rx_buf, &rx_len, (uint32_t)rx_cap)) {
            const uint8_t b = rx_buf[rx_len - 1U];
            bool rel_sw = false;
            bool saw_wait1 = false;
            p7501_fmu_feed_byte(&st, fmode, b, now_us, &rel_sw, NULL, &saw_wait1);
            if (rel_sw) {
                p7501_pio_release_sw_uart_now();
            }
            if (saw_wait1) {
                next_tx_us = now_us + 2000UL;
            }
            if (p7501_fmu_is_echo_hit(bval, waiting_reply, st.sd_started, now_us - last_tx_us)) {
                hit_byte = (int)(bval - 1U);
                sweep_done = true;
                waiting_reply = false;
            }
            if (st.sd_started && bval > 0U) {
                sweep_done = true;
                waiting_reply = false;
            }
        }

        if (st.wait1_seen && !sweep_done && !st.sd_started &&
            ((int32_t)(now_us - next_tx_us) >= 0) &&
            ((now_us - st.wait1_us) < 1100000UL)) {
            if ((bval == 0U) && (fmode == 1U) && (!st.sw_uart_flipped)) {
                p7501_pio_release_sw_uart_now();
                st.sw_uart_flipped = true;
            }
            if (bval < 256U) {
                const uint8_t tx_b = (uint8_t)bval;
                p7501_pio_tx_bytes(&tx_b, 1u, !st.sw_uart_flipped);
                last_tx_us = time_us_32();
                bval++;
                waiting_reply = true;
                next_tx_us = last_tx_us + 2200UL;
            } else {
                waiting_reply = false;
                sweep_done = true;
            }
        }
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    if (hit_byte >= 0) {
        printf("\n=== FMUSCAN MODE %" PRIu32 " RESULT (hit=0x%02X/%d, bytes=%" PRIu32 "%s) ===\n",
               fmode, hit_byte, hit_byte, rx_len, aborted ? ", aborted=YES" : "");
    } else {
        printf("\n=== FMUSCAN MODE %" PRIu32 " RESULT (hit=NONE, probes=%u, bytes=%" PRIu32 "%s) ===\n",
               fmode, (unsigned)bval, rx_len, aborted ? ", aborted=YES" : "");
    }
    p7501_uart_print_ascii_stream(rx_buf, rx_len);
    return !aborted;
}

void p7501_cmd_fmu_scan(const char *arg) {
    if ((arg != NULL) && p7501_ascii_ieq(arg, "all")) {
        for (uint32_t m = 0U; m < 4U; ++m) {
            if (!run_single_fmu_scan_mode(m)) {
                break;
            }
            p7501_wdt_sleep_ms(40u);
        }
        printf("\nFMUSCAN: OK\n");
        return;
    }

    uint32_t fmode = 0U;
    if ((arg != NULL) && (*arg != '\0')) {
        if (!p7501_parse_u32_strict(arg, &fmode)) {
            printf("[FMUSCAN] Invalid mode '%s'! Defaulting to mode 0.\n", arg);
            fmode = 0U;
        }
    }
    (void)run_single_fmu_scan_mode(fmode);
    printf("\nFMUSCAN: OK\n");
}

void p7501_cmd_fmu_test(const char *arg) {
    uint32_t fmode = 2U;
    bool repeat_25ms = false;
    const char *hex_str = "0D";
    p7501_fmu_parse_test_args(arg, &fmode, &repeat_25ms, &hex_str);

    uint8_t tx_bytes[64];
    const size_t tx_len = p7501_fmu_decode_or_default_cr(hex_str, tx_bytes, sizeof(tx_bytes));

    size_t rx_cap = 0U;
    uint8_t *rx_buf = p7501_uart_rx_buffer(&rx_cap);
    p7501_fmu_phase_state_t st;
    p7501_fmu_start_phase_session(&st);
    p7501_wdt_set_stage(P7501_WDT_STAGE_TX_WAIT);

    uint32_t rx_len = 0U;
    uint32_t next_tx_us = 0U;
    uint32_t tx_count = 0U;
    bool aborted = false;
    const uint64_t t0_us = time_us_64();
    const uint64_t deadline_us = t0_us + ((uint64_t)P7501_FMU_PROBE_WINDOW_MS * 1000ULL);

    while (!aborted && p7501_wdt_loop_step(t0_us, deadline_us, true, &aborted)) {
        const uint32_t now_us = time_us_32();
        while (p7501_fmu_pop_rx_byte(rx_buf, &rx_len, (uint32_t)rx_cap) && (!st.wait1_seen)) {
            const uint8_t b = rx_buf[rx_len - 1U];
            bool rel_sw = false;
            bool saw_serial = false;
            bool saw_wait1 = false;
            p7501_fmu_feed_byte(&st, fmode, b, now_us, &rel_sw, &saw_serial, &saw_wait1);
            if (rel_sw) {
                p7501_pio_release_sw_uart_now();
            }
            if ((fmode == 4U) && (!st.sent_after_serial_msg) && saw_serial) {
                p7501_pio_tx_bytes(tx_bytes, tx_len, !st.sw_uart_flipped);
                st.sent_after_serial_msg = true;
                tx_count++;
            }
            if (saw_wait1) {
                if ((fmode == 1U) && (!st.sw_uart_flipped)) {
                    p7501_pio_release_sw_uart_now();
                    st.sw_uart_flipped = true;
                }
                next_tx_us = now_us + 2000UL;
            }
        }

        if (st.wait1_seen && ((tx_count == 0U) || repeat_25ms) &&
            ((int32_t)(now_us - next_tx_us) >= 0) &&
            ((uint32_t)(now_us - st.wait1_us) < 1050000UL)) {
            p7501_pio_tx_bytes(tx_bytes, tx_len, !st.sw_uart_flipped);
            tx_count++;
            next_tx_us = time_us_32() + 25000UL;
        }
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf("\n=== FMUTEST MODE %" PRIu32 " (%u tx bytes x%" PRIu32 ") CAPTURE (%" PRIu32 " bytes%s) ===\n",
           fmode, (unsigned)tx_len, tx_count, rx_len, aborted ? ", aborted=YES" : "");
    p7501_uart_print_ascii_stream(rx_buf, rx_len);
    printf("\nFMUTEST: OK\n");
}
