/**
 * @file p7501_uart_tx.c
 * @brief Active PIO1 TX (`UARTTXHEX`), pattern-triggered inject (`WAITTX`), and `BRIDGE`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_tx.h"

#include "cli/p7501_cli.h"
#include "common/p7501_hex.h"
#include "hal/p7501_led.h"
#include "hal/p7501_usb_dual_cdc.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

typedef struct tx_stream_ctx {
    const char *trigger;
    uint8_t ring[P7501_WAITTX_MAX_TRIGGER + 1u];
    size_t ring_len;
    bool triggered;
} tx_stream_ctx_t;

static void tx_print_on_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    tx_stream_ctx_t *ctx = (tx_stream_ctx_t *)raw_ctx;
    p7501_print_ascii_byte(byte, '[', ']');
    if (ctx != NULL && ctx->trigger != NULL && !ctx->triggered &&
        p7501_rolling_match_push(ctx->ring, &ctx->ring_len, sizeof(ctx->ring), byte, ctx->trigger)) {
        ctx->triggered = true;
        if (out_stop != NULL) {
            *out_stop = true;
        }
    }
}

static void waittx_on_sw_rel(void *raw_ctx, uint32_t rel_us, const char *reason) {
    (void)raw_ctx;
    (void)rel_us;
    printf("\n[SW_UART] %s -> Hi-Z -> Q7514 active\n", reason);
    fflush(stdout);
}

static void run_tx_print_loop(
    uint32_t window_ms,
    p7501_boot_action_t boot_action,
    uint32_t jig_hold_ms,
    bool auto_rel,
    p7501_pio_loop_sw_rel_fn on_sw_rel,
    tx_stream_ctx_t *ctx,
    p7501_pio_loop_metrics_t *out_m
) {
    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = window_ms,
        .boot_action = boot_action,
        .jig_hold_ms = jig_hold_ms,
        .manage_session = false,
        .auto_release_serial_boot = auto_rel,
        .allow_cdc_abort = true,
        .on_rx_byte = tx_print_on_byte,
        .on_tick = NULL,
        .on_sw_rel = on_sw_rel,
        .ctx = ctx,
    };
    p7501_pio_run_session(&cfg, out_m);
}

void p7501_cmd_uart_tx_hex(const char *arg) {
    static char s_tok1[(P7501_UART_HEX_MAX_BYTES * 2u) + 12u];
    static uint8_t s_tx_buf[P7501_UART_HEX_MAX_BYTES];
    char tok2[24] = {0};
    s_tok1[0] = '\0';
    const char *cur = p7501_next_token(arg, s_tok1, sizeof(s_tok1));
    if (cur == NULL || s_tok1[0] == '\0') {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("[UARTTXHEX] ERROR: Usage UARTTXHEX [LOW:]<hex|BRK> [rx_ms]\n");
        return;
    }

    bool force_low = false;
    const char *payload_tok = s_tok1;
    if (p7501_ascii_iprefix(s_tok1, "LOW:")) {
        force_low = true;
        payload_tok = s_tok1 + 4;
    }

    uint32_t wait_ms = 1500u;
    if (p7501_next_token(cur, tok2, sizeof(tok2)) != NULL) {
        wait_ms = p7501_parse_uint32_clamped(tok2, 1500u, 100u, 20000u);
    }

    size_t tx_len = 0u;
    const bool is_break = p7501_ascii_ieq(payload_tok, "BRK") || p7501_ascii_ieq(payload_tok, "BREAK");
    if (!is_break && !p7501_decode_hex_bytes(payload_tok, s_tx_buf, sizeof(s_tx_buf), &tx_len)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("[UARTTXHEX] ERROR: Invalid hex payload '%s'.\n", payload_tok);
        return;
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_TX_WAIT);
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
    if (is_break) {
        p7501_pio_tx_break_ms(120u, force_low);
    } else {
        p7501_pio_tx_bytes(s_tx_buf, tx_len, force_low);
    }

    tx_stream_ctx_t ctx = { .trigger = NULL, .ring_len = 0u, .triggered = false };
    p7501_pio_loop_metrics_t m;
    run_tx_print_loop(
        wait_ms, P7501_BOOT_ACTION_PASSIVE, 0u, false, NULL, &ctx, &m
    );

    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    if (m.rx_bytes > 0u) {
        putchar('\n');
    }
    printf("UARTTXHEX: OK (tx=%s, sw_uart=%s, rx=%" PRIu32 " bytes)\n",
           is_break ? "BREAK_120MS" : "HEX", force_low ? "LOW" : "HIZ_Q7514_ON", m.rx_bytes);
}

void p7501_cmd_waittx(const char *arg) {
    p7501_waittx_spec_t spec;
    if (!p7501_parse_waittx_args(arg, &spec)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("[WAITTX] ERROR: Usage WAITTX <timeout_ms> <NORMAL|JIG|JIG:ms|NOPULL> \"<trigger>\" <hex|BRK> [post_ms]\n");
        return;
    }

    printf("=== WAITTX: baud=%" PRIu32 " timeout=%" PRIu32 "ms hold=%" PRIu32 "ms trigger=\"%s\" post=%" PRIu32 "ms ===\n",
           p7501_pio_get_baud(), spec.timeout_ms, spec.jig_hold_ms, spec.trigger, spec.post_rx_ms);
    fflush(stdout);

    p7501_wdt_set_stage(P7501_WDT_STAGE_RESET_SYNC);
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    tx_stream_ctx_t ctx = { .trigger = spec.trigger, .ring_len = 0u, .triggered = false };
    p7501_pio_loop_metrics_t m1;
    run_tx_print_loop(
        spec.timeout_ms,
        spec.boot_action,
        spec.jig_hold_ms,
        spec.boot_action == P7501_BOOT_ACTION_JIG,
        waittx_on_sw_rel,
        &ctx,
        &m1
    );

    uint32_t total_rx = m1.rx_bytes;
    if (ctx.triggered && !m1.aborted) {
        p7501_wdt_set_stage(P7501_WDT_STAGE_TX_WAIT);
        printf("\n[WAITTX] Trigger matched! Injecting payload via Q7514 (SW_UART=Hi-Z)...\n");
        fflush(stdout);
        if (spec.tx_break) {
            p7501_pio_tx_break_ms(120u, false);
        } else {
            p7501_pio_tx_bytes(spec.tx_bytes, spec.tx_len, false);
        }

        ctx.trigger = NULL;
        p7501_pio_loop_metrics_t m2;
        run_tx_print_loop(
            spec.post_rx_ms, P7501_BOOT_ACTION_PASSIVE, 0u, false, NULL, &ctx, &m2
        );
        total_rx += m2.rx_bytes;
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    printf("\nWAITTX: OK (triggered=%s, rx=%" PRIu32 " bytes)\n",
           ctx.triggered ? "YES" : "NO", total_rx);
}

typedef struct bridge_usb_ring {
    uint8_t buf[256];
    size_t head;
    size_t tail;
    size_t count;
    uint32_t rx_count;
} bridge_usb_ring_t;

static bool bridge_flush_usb_ring(bridge_usb_ring_t *r) {
    if (r->count == 0u) {
        return false;
    }
    while (r->count > 0u) {
        putchar_raw((int)r->buf[r->tail]);
        r->tail = (r->tail + 1u) % sizeof(r->buf);
        --r->count;
    }
    fflush(stdout);
    return true;
}

static void bridge_push_usb_rx_byte(void *raw_ctx, uint8_t rx_byte) {
    bridge_usb_ring_t *r = (bridge_usb_ring_t *)raw_ctx;
    p7501_usb_mirror_rx_byte(rx_byte);
    if (r->count >= sizeof(r->buf)) {
        (void)bridge_flush_usb_ring(r);
    }
    r->buf[r->head] = rx_byte;
    r->head = (r->head + 1u) % sizeof(r->buf);
    ++r->count;
    ++r->rx_count;
}

static void bridge_drain_pio_rx(bridge_usb_ring_t *r) {
    uint8_t rx_byte = 0u;
    while (p7501_pio_pop_rx_byte(&rx_byte)) {
        bridge_push_usb_rx_byte(r, rx_byte);
    }
}

static void bridge_tx_duplex(
    bridge_usb_ring_t *r,
    const uint8_t *batch,
    size_t len,
    uint32_t *io_tx_count
) {
    p7501_pio_tx_bytes_duplex(batch, len, false, true, bridge_push_usb_rx_byte, r);
    *io_tx_count += (uint32_t)len;
    (void)bridge_flush_usb_ring(r);
}

void p7501_cmd_bridge(const char *arg) {
    const uint32_t orig_baud = p7501_pio_get_baud();
    p7501_bridge_spec_t spec;
    p7501_parse_bridge_args(arg, orig_baud, &spec);
    (void)p7501_pio_set_baud(spec.baud);

    printf("[BRIDGE] USB CDC <-> P7501 SC0 %" PRIu32 " %s ctrl=%s active (exit: send '~~~' or unplug USB).\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()),
           spec.log_ctrl_edges ? "ON" : "OFF");
    fflush(stdout);

    p7501_wdt_set_stage(P7501_WDT_STAGE_BRIDGE);
    p7501_led_set_auto_mode(LED_MODE_GDB_RSP);
    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    const uint64_t t0_us = time_us_64();
    uint8_t prev_ctrl3 = (uint8_t)((gpio_get_all() >> P7501_PIN_NSW5V) & 0x07u);
    uint32_t ctrl_edges = 0u;
    bridge_usb_ring_t usb_ring = {0};
    uint32_t tx_count = 0u;
    uint32_t ferr_count = 0u;
    uint8_t tilde_run = 0u;
    uint64_t last_tx_char_us = 0ULL;
    uint64_t tilde_first_us = 0ULL;
    uint8_t tx_batch[16];
    bool exit_requested = false;
    absolute_time_t disconnect_deadline = at_the_end_of_time;
    bool counting_disconnect = false;

    while (!exit_requested) {
        p7501_wdt_kick();
        (void)p7501_pio_poll_sw_uart_release();
        if (p7501_pio_poll_and_clear_framing_err()) {
            ++ferr_count;
        }
        if (spec.log_ctrl_edges) {
            const uint8_t cur_ctrl3 = (uint8_t)((gpio_get_all() >> P7501_PIN_NSW5V) & 0x07u);
            if (cur_ctrl3 != prev_ctrl3) {
                const uint32_t rel_ms = (uint32_t)((time_us_64() - t0_us) / 1000ULL);
                printf("\n[CTRL +%" PRIu32 "ms] NSW5V=%u TXD=%u XRESET=%u\n",
                       rel_ms,
                       (unsigned)(cur_ctrl3 & 1u),
                       (unsigned)((cur_ctrl3 >> 1u) & 1u),
                       (unsigned)((cur_ctrl3 >> 2u) & 1u));
                fflush(stdout);
                prev_ctrl3 = cur_ctrl3;
                ++ctrl_edges;
            }
        }

        if (!stdio_usb_connected()) {
            if (!counting_disconnect) {
                counting_disconnect = true;
                disconnect_deadline = make_timeout_time_ms(2000u);
            } else if (time_reached(disconnect_deadline)) {
                break;
            }
        } else {
            counting_disconnect = false;
        }

        bridge_drain_pio_rx(&usb_ring);
        const bool had_rx = bridge_flush_usb_ring(&usb_ring);

        size_t batch_len = 0u;
        const uint64_t poll_us = time_us_64();
        /* Flush pending single/double '~' after 250 ms if user wasn't typing '~~~'. */
        if ((tilde_run > 0u) && ((poll_us - tilde_first_us) >= 250000ULL)) {
            while (tilde_run > 0u) {
                tx_batch[batch_len++] = (uint8_t)'~';
                --tilde_run;
            }
            last_tx_char_us = poll_us;
        }
        while (batch_len < sizeof(tx_batch)) {
            bridge_drain_pio_rx(&usb_ring);
            const int ch = getchar_timeout_us(0u);
            if (ch == PICO_ERROR_TIMEOUT) {
                break;
            }
            const uint64_t now_us = time_us_64();
            if (ch == '~') {
                /* Hayes 50 ms pre-guard prevents binary streams containing '~~~' from exiting. */
                if ((tilde_run > 0u) || (last_tx_char_us == 0ULL) || ((now_us - last_tx_char_us) >= 50000ULL)) {
                    if (tilde_run == 0u) {
                        tilde_first_us = now_us;
                    }
                    ++tilde_run;
                    if (tilde_run >= 3u) {
                        exit_requested = true;
                        break;
                    }
                    continue;
                }
            }
            while (tilde_run > 0u) {
                if (batch_len == sizeof(tx_batch)) {
                    bridge_tx_duplex(&usb_ring, tx_batch, batch_len, &tx_count);
                    batch_len = 0u;
                }
                tx_batch[batch_len++] = (uint8_t)'~';
                --tilde_run;
            }
            last_tx_char_us = now_us;
            tx_batch[batch_len++] = (uint8_t)ch;
        }
        if (batch_len > 0u) {
            bridge_tx_duplex(&usb_ring, tx_batch, batch_len, &tx_count);
        } else if (!had_rx && !exit_requested) {
            bridge_drain_pio_rx(&usb_ring);
            if (!bridge_flush_usb_ring(&usb_ring)) {
                sleep_us(25u);
            }
        }
    }

    bridge_drain_pio_rx(&usb_ring);
    (void)bridge_flush_usb_ring(&usb_ring);
    if (p7501_pio_poll_and_clear_framing_err()) {
        ++ferr_count;
    }
    const bool ovf = p7501_pio_had_rx_overflow();
    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    (void)p7501_pio_set_baud(orig_baud);
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    printf("\nBRIDGE: OK (rx=%" PRIu32 " tx=%" PRIu32 " ferr=%" PRIu32 " ctrl_edges=%" PRIu32 " overflow=%s)\n",
           usb_ring.rx_count, tx_count, ferr_count, ctrl_edges, ovf ? "YES" : "NO");
}
