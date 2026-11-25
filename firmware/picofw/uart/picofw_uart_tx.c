/**
 * @file picofw_uart_tx.c
 * @brief Active PIO1 TX (`UARTTXHEX`), pattern-triggered inject (`WAITTX`), and `BRIDGE`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_uart_tx.h"

#include "cli/picofw_cli.h"
#include "cli/picofw_cli_sys.h"
#include "common/picofw_hex.h"
#include "flash/picofw_bus_policy.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"
#include "hal/picofw_usb_dual_cdc.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "uart/picofw_pio_loop.h"
#include "uart/picofw_uart_codec.h"
#include "uart/picofw_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

typedef struct tx_stream_ctx {
    const char *trigger;
    uint8_t ring[PICOFW_WAITTX_MAX_TRIGGER + 1u];
    size_t ring_len;
    bool triggered;
} tx_stream_ctx_t;

static void tx_print_on_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    tx_stream_ctx_t *ctx = (tx_stream_ctx_t *)raw_ctx;
    picofw_print_ascii_byte(byte, '[', ']');
    if (ctx != NULL && ctx->trigger != NULL && !ctx->triggered &&
        picofw_rolling_match_push(ctx->ring, &ctx->ring_len, sizeof(ctx->ring), byte, ctx->trigger)) {
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
    picofw_boot_action_t boot_action,
    uint32_t jig_hold_ms,
    bool auto_rel,
    picofw_pio_loop_sw_rel_fn on_sw_rel,
    tx_stream_ctx_t *ctx,
    picofw_pio_loop_metrics_t *out_m
) {
    const picofw_pio_loop_cfg_t cfg = {
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
    picofw_pio_run_session(&cfg, out_m);
}

void picofw_cmd_uart_tx_hex(const char *arg) {
    static char s_tok1[(PICOFW_UART_HEX_MAX_BYTES * 2u) + 12u];
    static uint8_t s_tx_buf[PICOFW_UART_HEX_MAX_BYTES];
    char tok2[24] = {0};
    s_tok1[0] = '\0';
    const char *cur = picofw_next_token(arg, s_tok1, sizeof(s_tok1));
    if (cur == NULL || s_tok1[0] == '\0') {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[UARTTXHEX] ERROR: Usage UARTTXHEX [LOW:]<hex|BRK> [rx_ms]\n");
        return;
    }

    bool force_low = false;
    const char *payload_tok = s_tok1;
    if (picofw_ascii_iprefix(s_tok1, "LOW:")) {
        force_low = true;
        payload_tok = s_tok1 + 4;
    }

    uint32_t wait_ms = 1500u;
    if (picofw_next_token(cur, tok2, sizeof(tok2)) != NULL) {
        wait_ms = picofw_parse_uint32_clamped(tok2, 1500u, 100u, 20000u);
    }

    size_t tx_len = 0u;
    const bool is_break = picofw_ascii_ieq(payload_tok, "BRK") || picofw_ascii_ieq(payload_tok, "BREAK");
    if (!is_break && !picofw_decode_hex_bytes(payload_tok, s_tx_buf, sizeof(s_tx_buf), &tx_len)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[UARTTXHEX] ERROR: Invalid hex payload '%s'.\n", payload_tok);
        return;
    }
    picofw_bus_deny_t deny = PICOFW_BUS_ALLOW;
    if (!is_break && !picofw_bus_check_uart_tx_payload(s_tx_buf, tx_len, false, &deny)) {
        picofw_cli_set_result("ERR", "E05");
        printf("[UARTTXHEX] BLOCKED (%s)\n", picofw_bus_deny_name(deny));
        return;
    }

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_TX_WAIT);
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    if (is_break) {
        picofw_pio_tx_break_ms(120u, force_low);
    } else {
        picofw_pio_tx_bytes(s_tx_buf, tx_len, force_low);
    }

    tx_stream_ctx_t ctx = { .trigger = NULL, .ring_len = 0u, .triggered = false };
    picofw_pio_loop_metrics_t m;
    run_tx_print_loop(
        wait_ms, PICOFW_BOOT_ACTION_PASSIVE, 0u, false, NULL, &ctx, &m
    );

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    if (m.rx_bytes > 0u) {
        putchar('\n');
    }
    printf("UARTTXHEX: OK (tx=%s, sw_uart=%s, rx=%" PRIu32 " bytes)\n",
           is_break ? "BREAK_120MS" : "HEX", force_low ? "LOW" : "HIZ_Q7514_ON", m.rx_bytes);
}

void picofw_cmd_waittx(const char *arg) {
    picofw_waittx_spec_t spec;
    if (!picofw_parse_waittx_args(arg, &spec)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[WAITTX] ERROR: Usage WAITTX <timeout_ms> <NORMAL|JIG|JIG:ms|NOPULL> \"<trigger>\" <hex|BRK> [post_ms]\n");
        return;
    }
    picofw_bus_deny_t deny = PICOFW_BUS_ALLOW;
    if (!spec.tx_break && !picofw_bus_check_uart_tx_payload(spec.tx_bytes, spec.tx_len, false, &deny)) {
        picofw_cli_set_result("ERR", "E05");
        printf("[WAITTX] BLOCKED (%s)\n", picofw_bus_deny_name(deny));
        return;
    }

    printf("=== WAITTX: baud=%" PRIu32 " timeout=%" PRIu32 "ms hold=%" PRIu32 "ms trigger=\"%s\" post=%" PRIu32 "ms ===\n",
           picofw_pio_get_baud(), spec.timeout_ms, spec.jig_hold_ms, spec.trigger, spec.post_rx_ms);
    fflush(stdout);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RESET_SYNC);
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    picofw_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    tx_stream_ctx_t ctx = { .trigger = spec.trigger, .ring_len = 0u, .triggered = false };
    picofw_pio_loop_metrics_t m1;
    run_tx_print_loop(
        spec.timeout_ms,
        spec.boot_action,
        spec.jig_hold_ms,
        spec.boot_action == PICOFW_BOOT_ACTION_JIG,
        waittx_on_sw_rel,
        &ctx,
        &m1
    );

    uint32_t total_rx = m1.rx_bytes;
    if (ctx.triggered && !m1.aborted) {
        picofw_wdt_set_stage(PICOFW_WDT_STAGE_TX_WAIT);
        printf("\n[WAITTX] Trigger matched! Injecting payload via Q7514 (SW_UART=Hi-Z)...\n");
        fflush(stdout);
        if (spec.tx_break) {
            picofw_pio_tx_break_ms(120u, false);
        } else {
            picofw_pio_tx_bytes(spec.tx_bytes, spec.tx_len, false);
        }

        ctx.trigger = NULL;
        picofw_pio_loop_metrics_t m2;
        run_tx_print_loop(
            spec.post_rx_ms, PICOFW_BOOT_ACTION_PASSIVE, 0u, false, NULL, &ctx, &m2
        );
        total_rx += m2.rx_bytes;
    }

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
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
    picofw_usb_mirror_rx_byte(rx_byte);
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
    for (uint32_t i = 0u; (i < 4096u) && picofw_pio_pop_rx_byte(&rx_byte); ++i) {
        if ((i & 63u) == 63u) {
            picofw_wdt_kick();
        }
        bridge_push_usb_rx_byte(r, rx_byte);
    }
}

static void bridge_tx_duplex(
    bridge_usb_ring_t *r,
    const uint8_t *batch,
    size_t len,
    uint32_t *io_tx_count
) {
    if (len == 1u) {
        picofw_pio_tx_bytes_duplex(batch, 1u, false, true, bridge_push_usb_rx_byte, r);
        *io_tx_count += 1u;
    } else {
        for (size_t k = 0u; k < len; ++k) {
            picofw_wdt_kick();
            picofw_pio_tx_bytes_duplex(&batch[k], 1u, false, true, bridge_push_usb_rx_byte, r);
            *io_tx_count += 1u;
            if ((k + 1u) < len) {
                sleep_us(150u);
                bridge_drain_pio_rx(r);
            }
        }
    }
    (void)bridge_flush_usb_ring(r);
}

static picofw_bridge_tx_filter_t s_bridge_pol;
static uint8_t s_bridge_tx_frame[PICOFW_BRIDGE_RSP_CAP];

static void bridge_filter_tx_byte(
    bridge_usb_ring_t *r,
    uint8_t b,
    uint32_t *io_tx_count
) {
    size_t out_len = 0u;
    picofw_bus_deny_t deny = PICOFW_BUS_ALLOW;
    const picofw_bridge_filter_action_t act = picofw_bridge_tx_filter_feed(
        &s_bridge_pol, b, s_bridge_tx_frame, sizeof(s_bridge_tx_frame), &out_len, &deny
    );
    if ((act == PICOFW_BRIDGE_ACT_FORWARD) || (act == PICOFW_BRIDGE_ACT_PASS_CTL)) {
        if (out_len > 0u) {
            bridge_tx_duplex(r, s_bridge_tx_frame, out_len, io_tx_count);
        }
    } else if (act == PICOFW_BRIDGE_ACT_BLOCKED) {
        printf("\r\n[BRIDGE-POLICY] BLOCKED (%s)\r\n", picofw_bus_deny_name(deny));
        fflush(stdout);
    }
}

void picofw_cmd_bridge(const char *arg) {
    picofw_bridge_tx_filter_init(&s_bridge_pol);
    if (!picofw_probe_dispatch_event(PICOFW_EV_BRIDGE_ENTER, true)) {
        picofw_cli_set_result("ERR", picofw_hw_is_nsw5v_fault_latched() ? "EDROPOUT" : "EOFFLINE");
        printf("BRIDGE: ERROR (reason=%s)\n",
               picofw_hw_is_nsw5v_fault_latched() ? "NSW5V_DROPOUT" : "TARGET_UNPOWERED");
        return;
    }

    const uint32_t orig_baud = picofw_pio_get_baud();
    picofw_bridge_spec_t spec;
    picofw_parse_bridge_args(arg, orig_baud, &spec);
    (void)picofw_pio_set_baud(spec.baud);

    printf("[BRIDGE] USB CDC <-> P7501 SC0 %" PRIu32 " %s ctrl=%s active (exit: send '~~~' or unplug USB).\n",
           picofw_pio_get_baud(),
           picofw_framing_name(picofw_pio_get_framing()),
           spec.log_ctrl_edges ? "ON" : "OFF");
    fflush(stdout);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_BRIDGE);
    picofw_led_set_auto_mode(LED_MODE_GDB_RSP);
    picofw_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    const uint64_t t0_us = time_us_64();
    uint8_t prev_ctrl3 = (uint8_t)((gpio_get_all() >> P7501_PIN_NSW5V) & 0x07u);
    uint32_t ctrl_edges = 0u;
    bridge_usb_ring_t usb_ring = {0};
    uint32_t tx_count = 0u;
    uint32_t ferr_count = 0u;
    uint8_t tilde_run = 0u;
    uint64_t last_tx_char_us = 0ULL;
    uint64_t tilde_first_us = 0ULL;
    bool exit_requested = false;
    bool dropout_abort = false;
    absolute_time_t disconnect_deadline = at_the_end_of_time;
    bool counting_disconnect = false;

    while (!exit_requested) {
        picofw_wdt_kick();
        if (picofw_hw_is_nsw5v_fault_latched() || !picofw_hw_is_nsw5v_high()) {
            dropout_abort = true;
            break;
        }
        (void)picofw_pio_poll_sw_uart_release();
        if (picofw_pio_poll_and_clear_framing_err()) {
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

        const uint64_t poll_us = time_us_64();
        /* Flush pending single/double '~' after 250 ms if user wasn't typing '~~~'. */
        if ((tilde_run > 0u) && ((poll_us - tilde_first_us) >= 250000ULL)) {
            while (tilde_run > 0u) {
                bridge_filter_tx_byte(&usb_ring, (uint8_t)'~', &tx_count);
                --tilde_run;
            }
            last_tx_char_us = poll_us;
        }
        for (size_t burst = 0u; burst < 16u; ++burst) {
            bridge_drain_pio_rx(&usb_ring);
            if (picofw_hw_is_nsw5v_fault_latched() || !picofw_hw_is_nsw5v_high()) {
                dropout_abort = true;
                exit_requested = true;
                break;
            }
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
                bridge_filter_tx_byte(&usb_ring, (uint8_t)'~', &tx_count);
                --tilde_run;
            }
            last_tx_char_us = now_us;
            bridge_filter_tx_byte(&usb_ring, (uint8_t)ch, &tx_count);
        }
        if (!had_rx && !exit_requested) {
            bridge_drain_pio_rx(&usb_ring);
            if (!bridge_flush_usb_ring(&usb_ring)) {
                sleep_us(25u);
            }
        }
    }

    bridge_drain_pio_rx(&usb_ring);
    (void)bridge_flush_usb_ring(&usb_ring);
    if (picofw_pio_poll_and_clear_framing_err()) {
        ++ferr_count;
    }
    const bool ovf = picofw_pio_had_rx_overflow();
    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    (void)picofw_probe_dispatch_event(PICOFW_EV_BRIDGE_EXIT, false);
    (void)picofw_pio_set_baud(orig_baud);
    picofw_led_set_auto_mode(dropout_abort ? LED_MODE_BUS_FAULT : LED_MODE_HEARTBEAT_IDLE);
    if (dropout_abort) {
        picofw_cli_set_result("ERR", "EDROPOUT");
        printf("\nBRIDGE: ERROR (reason=NSW5V_DROPOUT rx=%" PRIu32 " tx=%" PRIu32 " ferr=%" PRIu32 " ctrl_edges=%" PRIu32 " overflow=%s)\n",
               usb_ring.rx_count, tx_count, ferr_count, ctrl_edges, ovf ? "YES" : "NO");
        return;
    }
    printf("\nBRIDGE: OK (rx=%" PRIu32 " tx=%" PRIu32 " ferr=%" PRIu32 " ctrl_edges=%" PRIu32 " overflow=%s)\n",
           usb_ring.rx_count, tx_count, ferr_count, ctrl_edges, ovf ? "YES" : "NO");
}
