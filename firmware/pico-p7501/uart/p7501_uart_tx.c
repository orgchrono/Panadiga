/**
 * @file p7501_uart_tx.c
 * @brief Active PIO1 TX (`UARTTXHEX`), pattern-triggered inject (`WAITTX`), and `BRIDGE`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_tx.h"

#include "common/p7501_hex.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

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

static void emit_printable_byte(uint8_t b) {
    if (b == '\r') {
        return;
    }
    if (b == '\n' || (b >= 0x20u && b <= 0x7Eu)) {
        putchar((int)b);
    } else {
        printf("[%02X]", (unsigned)b);
    }
}

static void tx_print_on_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    tx_stream_ctx_t *ctx = (tx_stream_ctx_t *)raw_ctx;
    emit_printable_byte(byte);
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
    char tok1[(P7501_UART_HEX_MAX_BYTES * 2u) + 12u] = {0};
    char tok2[24] = {0};
    const char *cur = p7501_next_token(arg, tok1, sizeof(tok1));
    if (cur == NULL || tok1[0] == '\0') {
        printf("[UARTTXHEX] ERROR: Usage UARTTXHEX [LOW:]<hex|BRK> [rx_ms]\n");
        return;
    }

    bool force_low = false;
    const char *payload_tok = tok1;
    if (p7501_ascii_iprefix(tok1, "LOW:")) {
        force_low = true;
        payload_tok = tok1 + 4;
    }

    uint32_t wait_ms = 1500u;
    if (p7501_next_token(cur, tok2, sizeof(tok2)) != NULL) {
        wait_ms = p7501_parse_uint32_clamped(tok2, 1500u, 100u, 20000u);
    }

    uint8_t tx_buf[P7501_UART_HEX_MAX_BYTES];
    size_t tx_len = 0u;
    const bool is_break = p7501_ascii_ieq(payload_tok, "BRK") || p7501_ascii_ieq(payload_tok, "BREAK");
    if (!is_break && !p7501_decode_hex_bytes(payload_tok, tx_buf, sizeof(tx_buf), &tx_len)) {
        printf("[UARTTXHEX] ERROR: Invalid hex payload '%s'.\n", payload_tok);
        return;
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_TX_WAIT);
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
    if (is_break) {
        p7501_pio_tx_break_ms(120u, force_low);
    } else {
        p7501_pio_tx_bytes(tx_buf, tx_len, force_low);
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

void p7501_cmd_bridge(const char *arg) {
    char tok1[24] = {0};
    char tok2[24] = {0};
    const uint32_t orig_baud = p7501_pio_get_baud();
    p7501_boot_action_t action = P7501_BOOT_ACTION_PASSIVE;
    uint32_t jig_hold_ms = 0u;

    const char *cur = p7501_next_token(arg, tok1, sizeof(tok1));
    if (cur != NULL && tok1[0] != '\0') {
        const uint32_t req = p7501_parse_uint32_clamped(
            tok1, orig_baud, P7501_UART_MIN_BAUD, P7501_UART_MAX_BAUD
        );
        (void)p7501_pio_set_baud(req);
        if (p7501_next_token(cur, tok2, sizeof(tok2)) != NULL) {
            action = p7501_parse_boot_spec(tok2, &jig_hold_ms);
        }
    }

    printf("[BRIDGE] USB CDC <-> P7501 SC0 %" PRIu32 " %s active (exit: send '~~~' or unplug USB).\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()));
    fflush(stdout);

    p7501_wdt_set_stage(P7501_WDT_STAGE_BRIDGE);
    p7501_led_set_auto_mode(LED_MODE_GDB_RSP);
    p7501_pio_begin_rx_session_ex(action, jig_hold_ms);

    uint32_t rx_count = 0u;
    uint32_t tx_count = 0u;
    uint32_t ferr_count = 0u;
    uint8_t tilde_run = 0u;
    uint8_t rx_byte = 0u;
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

        bool had_rx = false;
        while (p7501_pio_pop_rx_byte(&rx_byte)) {
            putchar_raw((int)rx_byte);
            ++rx_count;
            had_rx = true;
        }
        if (had_rx) {
            fflush(stdout);
        }

        size_t batch_len = 0u;
        while (batch_len < sizeof(tx_batch)) {
            const int ch = getchar_timeout_us(0u);
            if (ch == PICO_ERROR_TIMEOUT) {
                break;
            }
            if (ch == '~') {
                ++tilde_run;
                if (tilde_run >= 3u) {
                    exit_requested = true;
                    break;
                }
                continue;
            }
            while (tilde_run > 0u) {
                if (batch_len == sizeof(tx_batch)) {
                    p7501_pio_tx_bytes(tx_batch, batch_len, false);
                    tx_count += (uint32_t)batch_len;
                    batch_len = 0u;
                }
                tx_batch[batch_len++] = (uint8_t)'~';
                --tilde_run;
            }
            tx_batch[batch_len++] = (uint8_t)ch;
        }
        if (batch_len > 0u) {
            p7501_pio_tx_bytes(tx_batch, batch_len, false);
            tx_count += (uint32_t)batch_len;
        } else if (!had_rx && !exit_requested) {
            sleep_us(25u);
        }
    }

    if (p7501_pio_poll_and_clear_framing_err()) {
        ++ferr_count;
    }
    const bool ovf = p7501_pio_had_rx_overflow();
    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    (void)p7501_pio_set_baud(orig_baud);
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    printf("\nBRIDGE: OK (rx=%" PRIu32 " tx=%" PRIu32 " framing_err=%" PRIu32 " overflow=%s)\n",
           rx_count, tx_count, ferr_count, ovf ? "YES" : "NO");
}
