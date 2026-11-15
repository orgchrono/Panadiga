/**
 * @file p7501_diag_boottrace.c
 * @brief Correlated PIO1 UART + GP3/GP4/GP5 control-edge + GP26 ADC0 boot profiler (`BOOTTRACE`).
 *
 * Captures timestamped UART lines without losing control-pin edges (NSW5V, TXD, XRESET)
 * to GP2 serial chatter, while monitoring SW_UART ADC0 rail continuously.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_diag_boottrace.h"
#include "diag/p7501_diag_pwrtrace.h"

#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "p7501_types.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/adc.h"
#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

#define BOOTTRACE_MAX_CTRL_EVENTS 64u

typedef struct boottrace_ctrl_event {
    uint32_t us;
    uint8_t state3; /* bit0=NSW5V(GP3), bit1=TXD(GP4), bit2=XRESET(GP5) */
} boottrace_ctrl_event_t;

static uint8_t read_ctrl3(void) {
    const uint32_t all = gpio_get_all();
    return (uint8_t)((all >> P7501_PIN_NSW5V) & 0x07u);
}

static void flush_uart_line(char *line, size_t *line_len, uint32_t line_start_ms) {
    if (*line_len == 0u) {
        return;
    }
    line[*line_len] = '\0';
    printf("[BOOTTRACE +%4" PRIu32 "ms] %s\n", line_start_ms, line);
    *line_len = 0u;
}

void p7501_cmd_boottrace(const char *arg) {
    p7501_window_boot_spec_t spec;
    p7501_parse_window_boot_args(arg, 4500u, 100u, 30000u, &spec);
    if (spec.boot_action == P7501_BOOT_ACTION_JIG && spec.jig_hold_ms == 0u) {
        spec.jig_hold_ms = 2500u;
    }

    printf("=== BOOTTRACE: window=%" PRIu32 "ms boot=%d hold=%" PRIu32 "ms (%s @ %" PRIu32 ") ===\n",
           spec.window_ms, (int)spec.boot_action, spec.jig_hold_ms,
           p7501_framing_name(p7501_pio_get_framing()), p7501_pio_get_baud());
    fflush(stdout);

    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    const uint64_t t0_us = time_us_64();
    const absolute_time_t end_t = make_timeout_time_ms(spec.window_ms);
    absolute_time_t next_adc_t = make_timeout_time_ms(10u);

    boottrace_ctrl_event_t events[BOOTTRACE_MAX_CTRL_EVENTS];
    size_t ev_count = 0u;
    uint8_t prev_ctrl3 = read_ctrl3();
    uint32_t nsw_edges = 0u;
    uint32_t txd_edges = 0u;
    uint32_t xrst_edges = 0u;

    uint32_t rx_bytes = 0u;
    uint32_t ferr = 0u;
    uint32_t first_byte_us = 0u;
    bool first_byte_seen = false;
    bool sw_rel_seen = false;
    uint32_t sw_rel_us = 0u;
    const char *sw_rel_reason = "NONE";
    p7501_adc0_trace_t adc_trace;
    p7501_diag_adc0_trace_init(&adc_trace);
    p7501_diag_adc0_trace_sample(&adc_trace);

    char line[160];
    size_t line_len = 0u;
    uint32_t line_start_ms = 0u;
    uint8_t ring[24];
    size_t ring_len = 0u;
    uint8_t byte = 0u;

    while (!time_reached(end_t)) {
        const uint64_t now_us = time_us_64();
        const uint32_t rel_us = (uint32_t)(now_us - t0_us);

        if (p7501_pio_poll_sw_uart_release()) {
            sw_rel_seen = true;
            sw_rel_us = rel_us;
            sw_rel_reason = "TIMER";
            p7501_diag_adc0_trace_sample(&adc_trace);
        }

        const uint8_t cur_ctrl3 = read_ctrl3();
        if (cur_ctrl3 != prev_ctrl3) {
            const uint8_t diff = (uint8_t)(cur_ctrl3 ^ prev_ctrl3);
            if ((diff & 1u) != 0u) ++nsw_edges;
            if ((diff & 2u) != 0u) ++txd_edges;
            if ((diff & 4u) != 0u) ++xrst_edges;
            if (ev_count < BOOTTRACE_MAX_CTRL_EVENTS) {
                events[ev_count].us = rel_us;
                events[ev_count].state3 = cur_ctrl3;
                ++ev_count;
            }
            prev_ctrl3 = cur_ctrl3;
        }

        if (p7501_pio_poll_and_clear_framing_err()) {
            ++ferr;
        }

        while (p7501_pio_pop_rx_byte(&byte)) {
            const uint32_t b_rel_us = (uint32_t)(time_us_64() - t0_us);
            if (!first_byte_seen) {
                first_byte_seen = true;
                first_byte_us = b_rel_us;
            }
            ++rx_bytes;

            if (!sw_rel_seen &&
                p7501_pio_poll_jig_serial_release(ring, &ring_len, sizeof(ring), byte)) {
                sw_rel_seen = true;
                sw_rel_us = b_rel_us;
                sw_rel_reason = "SERIAL_BOOT";
                p7501_diag_adc0_trace_sample(&adc_trace);
            }

            if (byte == '\r' || byte == '\n') {
                flush_uart_line(line, &line_len, line_start_ms);
            } else {
                if (line_len == 0u) {
                    line_start_ms = b_rel_us / 1000u;
                }
                if (line_len < (sizeof(line) - 1u)) {
                    line[line_len++] = (byte >= 0x20u && byte < 0x7Fu) ? (char)byte : '.';
                }
                if (line_len >= (sizeof(line) - 1u)) {
                    flush_uart_line(line, &line_len, line_start_ms);
                }
            }
        }

        if (time_reached(next_adc_t)) {
            next_adc_t = make_timeout_time_ms(10u);
            p7501_led_tick();
            p7501_diag_adc0_trace_sample(&adc_trace);
        }
    }

    flush_uart_line(line, &line_len, line_start_ms);
    const uint32_t perr = p7501_pio_get_and_clear_parity_errors();
    const bool ovf = p7501_pio_had_rx_overflow();
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    p7501_diag_adc0_trace_finalize(&adc_trace);

    printf("[BOOTTRACE] UART: bytes=%" PRIu32 " first_byte=%" PRIu32 "us ferr=%" PRIu32
           " perr=%" PRIu32 " ovf=%s | SW_rel=%s",
           rx_bytes, first_byte_us, ferr, perr, ovf ? "YES" : "NO",
           sw_rel_seen ? "YES@" : "NO");
    if (sw_rel_seen) {
        printf("%" PRIu32 "us(%s)", sw_rel_us, sw_rel_reason);
    }
    printf("\n[BOOTTRACE] ADC0_SWUART: min=%.2fV max=%.2fV (low=%" PRIu32 " hiz=%.2f..%.2fV n=%" PRIu32 ") | CtrlEdges: NSW5V=%" PRIu32
           " TXD=%" PRIu32 " XRESET=%" PRIu32 " (logged=%u)\n",
           (double)adc_trace.min_v, (double)adc_trace.max_v,
           adc_trace.low_samples, (double)adc_trace.hiz_min_v, (double)adc_trace.hiz_max_v,
           adc_trace.hiz_samples, nsw_edges, txd_edges, xrst_edges, (unsigned)ev_count);

    const size_t preview = (ev_count > 16u) ? 16u : ev_count;
    for (size_t i = 0u; i < preview; ++i) {
        printf("  [CTRL +%7" PRIu32 "us] NSW5V=%u TXD=%u XRESET=%u\n",
               events[i].us,
               (events[i].state3 >> 0) & 1u,
               (events[i].state3 >> 1) & 1u,
               (events[i].state3 >> 2) & 1u);
    }
    printf("BOOTTRACE: OK\n");
}
