/**
 * @file p7501_uart_baud.c
 * @brief PIO1 multi-baud configuration (`BAUD`) and hardware GP2 autobaud detector (`BAUDSCAN`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_baud.h"

#include "common/p7501_hex.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_autobaud.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

void p7501_cmd_baud(const char *arg) {
    char tok[24] = {0};

    if (p7501_next_token(arg, tok, sizeof(tok)) == NULL || tok[0] == '\0') {
        printf("[BAUD] Active PIO1 baud: %" PRIu32 " bps (%s)\n",
               p7501_pio_get_baud(), p7501_framing_name(p7501_pio_get_framing()));
        printf("BAUD: OK\n");
        return;
    }

    const uint32_t req = p7501_parse_uint32_clamped(
        tok, 0u, P7501_UART_MIN_BAUD, P7501_UART_MAX_BAUD
    );
    const uint32_t raw_exact = p7501_parse_uint32_clamped(tok, 0u, 1u, 10000000u);
    if (!p7501_baud_is_valid(raw_exact) || !p7501_pio_set_baud(req)) {
        printf("[BAUD] ERROR: Invalid baudrate '%s' (valid range: %" PRIu32 "..%" PRIu32 ").\n",
               tok, (uint32_t)P7501_UART_MIN_BAUD, (uint32_t)P7501_UART_MAX_BAUD);
        return;
    }

    printf("[BAUD] PIO1 RX/TX configured for %" PRIu32 " bps (%s, bit_ns=%" PRIu32 ").\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()),
           p7501_baud_to_bit_ns(p7501_pio_get_baud()));
    printf("BAUD: OK\n");
}

void p7501_cmd_framing(const char *arg) {
    char tok[16] = {0};
    if (p7501_next_token(arg, tok, sizeof(tok)) == NULL || tok[0] == '\0') {
        printf("[FRAMING] Active PIO1 framing: %s (baud=%" PRIu32 ")\n",
               p7501_framing_name(p7501_pio_get_framing()), p7501_pio_get_baud());
        printf("FRAMING: OK\n");
        return;
    }
    p7501_uart_framing_t next_framing = P7501_FRAMING_8N1;
    if (!p7501_parse_framing(tok, &next_framing) || !p7501_pio_set_framing(next_framing)) {
        printf("[FRAMING] ERROR: Unsupported framing '%s' (expected 8N1, 8E1, or 8O1).\n", tok);
        return;
    }
    printf("[FRAMING] PIO1 RX/TX configured for %s @ %" PRIu32 " bps.\n",
           p7501_framing_name(p7501_pio_get_framing()), p7501_pio_get_baud());
    printf("FRAMING: OK\n");
}

void p7501_cmd_baudscan(const char *arg) {
    p7501_window_boot_spec_t spec;
    bool auto_apply = false;

    p7501_parse_window_boot_args(arg, 4000u, 200u, P7501_TRACE_MAX_MS, &spec);
    {
        char tok[24];
        const char *cur = arg;
        while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
            if (p7501_ascii_ieq(tok, "APPLY")) {
                auto_apply = true;
            }
        }
    }

    if (spec.wait_for_wake) {
        float wake_v = 0.0f;
        uint32_t wake_ms = 0u;
        const bool prestrap = spec.no_xreset && (spec.boot_action == P7501_BOOT_ACTION_JIG);
        if (!p7501_pio_wait_standby_wake("BAUDSCAN", spec.wake_timeout_ms, prestrap, &wake_v, &wake_ms)) {
            printf("[BAUDSCAN] Notice: Standby wake not observed within %" PRIu32 " ms (VDD=%.2fV).\n",
                   wake_ms, (double)wake_v);
            printf("BAUDSCAN: OK (no_signal)\n");
            return;
        }
    }

    printf("[BAUDSCAN] Measuring GP2 (UARTP2M_T RXD) dual-polarity pulses & PIO1 framing over %" PRIu32 " ms...\n",
           spec.window_ms);
    fflush(stdout);

    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session_cfg(spec.boot_action, spec.jig_hold_ms, spec.no_xreset);

    const uint64_t t0_us = time_us_64();
    const uint64_t deadline_us = t0_us + ((uint64_t)spec.window_ms * 1000ULL);
    p7501_baud_hist_t hist;
    p7501_baud_hist_init(&hist);
    uint32_t edges = 0u;
    uint32_t pio_rx_bytes = 0u;
    uint32_t pio_ascii_bytes = 0u;
    uint32_t pio_framing_errs = 0u;
    bool prev_level = gpio_get(P7501_PIN_RXD);
    uint32_t last_edge_us = time_us_32();
    bool aborted = false;

    p7501_wdt_set_stage(P7501_WDT_STAGE_RX_WAIT);
    while (!aborted && p7501_wdt_loop_step(t0_us, deadline_us, true, &aborted)) {
        (void)p7501_pio_poll_sw_uart_release();
        const bool cur = gpio_get(P7501_PIN_RXD);
        if (cur != prev_level) {
            const uint32_t now_us = time_us_32();
            const uint32_t width_us = now_us - last_edge_us;
            ++edges;
            p7501_baud_hist_push_pulse(&hist, width_us, prev_level);
            last_edge_us = now_us;
            prev_level = cur;
        }
        uint8_t rx_b = 0u;
        if (p7501_pio_pop_rx_byte(&rx_b)) {
            ++pio_rx_bytes;
            if ((rx_b >= 0x20u && rx_b <= 0x7Eu) || rx_b == '\r' || rx_b == '\n' || rx_b == '\t') {
                ++pio_ascii_bytes;
            }
        }
        if (p7501_pio_poll_and_clear_framing_err()) {
            ++pio_framing_errs;
        }
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    if (edges == 0u || (hist.low_pulses + hist.high_pulses) == 0u) {
        printf("[BAUDSCAN] No valid UART pulses observed on GP2 (edges=%" PRIu32 " glitches=%" PRIu32 ").\n",
               edges, hist.glitches_rejected);
        printf("BAUDSCAN: OK (no_signal)\n");
        return;
    }

    p7501_baud_estimate_t est;
    p7501_baud_hist_finish(&hist, &est);
    printf("[BAUDSCAN] edges=%" PRIu32 " min_low_us=%" PRIu32 " min_high_us=%" PRIu32
           " raw_bps=%" PRIu32 " snapped_baud=%" PRIu32 " cluster_hits=%" PRIu32
           " confidence=%" PRIu32 "%% glitches=%" PRIu32
           " rx_bytes=%" PRIu32 " ascii=%" PRIu32 " framing_errs=%" PRIu32 "\n",
           edges,
           (hist.min_low_us == UINT32_MAX) ? 0u : hist.min_low_us,
           (hist.min_high_us == UINT32_MAX) ? 0u : hist.min_high_us,
           est.raw_bps, est.snapped_baud, est.best_cluster_hits, est.confidence_pct,
           hist.glitches_rejected, pio_rx_bytes, pio_ascii_bytes, pio_framing_errs);

    if (auto_apply && est.snapped_baud > 0u && p7501_pio_set_baud(est.snapped_baud)) {
        printf("[BAUDSCAN] Auto-applied PIO1 baudrate -> %" PRIu32 " bps.\n", est.snapped_baud);
    }
    printf("BAUDSCAN: OK\n");
}
