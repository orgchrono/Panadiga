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

#include "hal/p7501_led.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

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

typedef struct boottrace_ctx {
    boottrace_ctrl_event_t events[BOOTTRACE_MAX_CTRL_EVENTS];
    size_t ev_count;
    uint8_t prev_ctrl3;
    uint32_t nsw_edges;
    uint32_t txd_edges;
    uint32_t xrst_edges;
    uint32_t first_byte_us;
    bool first_byte_seen;
    bool sw_rel_seen;
    uint32_t sw_rel_us;
    const char *sw_rel_reason;
    p7501_adc0_trace_t adc_trace;
    absolute_time_t next_adc_t;
    char line[160];
    size_t line_len;
    uint32_t line_start_ms;
} boottrace_ctx_t;

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

static void boottrace_on_sw_rel(void *raw_ctx, uint32_t rel_us, const char *reason) {
    boottrace_ctx_t *ctx = (boottrace_ctx_t *)raw_ctx;
    if (!ctx->sw_rel_seen) {
        ctx->sw_rel_seen = true;
        ctx->sw_rel_us = rel_us;
        ctx->sw_rel_reason = reason;
        p7501_diag_adc0_trace_sample(&ctx->adc_trace);
    }
}

static void boottrace_on_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)out_stop;
    boottrace_ctx_t *ctx = (boottrace_ctx_t *)raw_ctx;
    if (!ctx->first_byte_seen) {
        ctx->first_byte_seen = true;
        ctx->first_byte_us = rel_us;
    }
    if (byte == '\r' || byte == '\n') {
        flush_uart_line(ctx->line, &ctx->line_len, ctx->line_start_ms);
        return;
    }
    if (ctx->line_len == 0u) {
        ctx->line_start_ms = rel_us / 1000u;
    }
    if (ctx->line_len < (sizeof(ctx->line) - 1u)) {
        ctx->line[ctx->line_len++] = (byte >= 0x20u && byte < 0x7Fu) ? (char)byte : '.';
    }
    if (ctx->line_len >= (sizeof(ctx->line) - 1u)) {
        flush_uart_line(ctx->line, &ctx->line_len, ctx->line_start_ms);
    }
}

static void boottrace_on_tick(void *raw_ctx, uint32_t rel_us, bool *out_stop) {
    (void)out_stop;
    boottrace_ctx_t *ctx = (boottrace_ctx_t *)raw_ctx;

    const uint8_t cur_ctrl3 = read_ctrl3();
    if (cur_ctrl3 != ctx->prev_ctrl3) {
        const uint8_t diff = (uint8_t)(cur_ctrl3 ^ ctx->prev_ctrl3);
        if ((diff & 1u) != 0u) ++ctx->nsw_edges;
        if ((diff & 2u) != 0u) ++ctx->txd_edges;
        if ((diff & 4u) != 0u) ++ctx->xrst_edges;
        if (ctx->ev_count < BOOTTRACE_MAX_CTRL_EVENTS) {
            ctx->events[ctx->ev_count].us = rel_us;
            ctx->events[ctx->ev_count].state3 = cur_ctrl3;
            ++ctx->ev_count;
        }
        ctx->prev_ctrl3 = cur_ctrl3;
    }

    if (time_reached(ctx->next_adc_t)) {
        ctx->next_adc_t = make_timeout_time_ms(10u);
        p7501_led_tick();
        p7501_diag_adc0_trace_sample(&ctx->adc_trace);
    }
}

void p7501_cmd_boottrace(const char *arg) {
    p7501_window_boot_spec_t spec;
    p7501_parse_window_boot_args(arg, 4500u, 100u, P7501_TRACE_MAX_MS, &spec);
    if (spec.boot_action == P7501_BOOT_ACTION_JIG && spec.jig_hold_ms == 0u) {
        spec.jig_hold_ms = P7501_JIG_DEFAULT_HOLD_MS;
    }

    printf("=== BOOTTRACE: window=%" PRIu32 "ms boot=%d hold=%" PRIu32 "ms (%s @ %" PRIu32 ") ===\n",
           spec.window_ms, (int)spec.boot_action, spec.jig_hold_ms,
           p7501_framing_name(p7501_pio_get_framing()), p7501_pio_get_baud());
    fflush(stdout);

    boottrace_ctx_t ctx = {
        .ev_count = 0u,
        .prev_ctrl3 = read_ctrl3(),
        .nsw_edges = 0u,
        .txd_edges = 0u,
        .xrst_edges = 0u,
        .first_byte_us = 0u,
        .first_byte_seen = false,
        .sw_rel_seen = false,
        .sw_rel_us = 0u,
        .sw_rel_reason = "NONE",
        .next_adc_t = make_timeout_time_ms(10u),
        .line_len = 0u,
        .line_start_ms = 0u,
    };
    p7501_diag_adc0_trace_init(&ctx.adc_trace);
    p7501_diag_adc0_trace_sample(&ctx.adc_trace);

    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = spec.window_ms,
        .boot_action = spec.boot_action,
        .jig_hold_ms = spec.jig_hold_ms,
        .manage_session = true,
        .auto_release_serial_boot = true,
        .allow_cdc_abort = true,
        .on_rx_byte = boottrace_on_byte,
        .on_tick = boottrace_on_tick,
        .on_sw_rel = boottrace_on_sw_rel,
        .ctx = &ctx,
    };
    p7501_pio_loop_metrics_t m;
    p7501_pio_run_session(&cfg, &m);

    flush_uart_line(ctx.line, &ctx.line_len, ctx.line_start_ms);
    p7501_diag_adc0_trace_finalize(&ctx.adc_trace);

    printf("[BOOTTRACE] UART: bytes=%" PRIu32 " first_byte=%" PRIu32 "us ferr=%" PRIu32
           " perr=%" PRIu32 " ovf=%s | SW_rel=%s",
           m.rx_bytes, ctx.first_byte_us, m.framing_errors, m.parity_errors,
           m.overflow ? "YES" : "NO", ctx.sw_rel_seen ? "YES@" : "NO");
    if (ctx.sw_rel_seen) {
        printf("%" PRIu32 "us(%s)", ctx.sw_rel_us, ctx.sw_rel_reason);
    }
    printf("\n[BOOTTRACE] ADC0_SWUART: min=%.2fV max=%.2fV (low=%" PRIu32 " hiz=%.2f..%.2fV n=%" PRIu32 ") | CtrlEdges: NSW5V=%" PRIu32
           " TXD=%" PRIu32 " XRESET=%" PRIu32 " (logged=%u)\n",
           (double)ctx.adc_trace.min_v, (double)ctx.adc_trace.max_v,
           ctx.adc_trace.low_samples, (double)ctx.adc_trace.hiz_min_v, (double)ctx.adc_trace.hiz_max_v,
           ctx.adc_trace.hiz_samples, ctx.nsw_edges, ctx.txd_edges, ctx.xrst_edges, (unsigned)ctx.ev_count);

    const size_t preview = (ctx.ev_count > 16u) ? 16u : ctx.ev_count;
    for (size_t i = 0u; i < preview; ++i) {
        printf("  [CTRL +%7" PRIu32 "us] NSW5V=%u TXD=%u XRESET=%u\n",
               ctx.events[i].us,
               (ctx.events[i].state3 >> 0) & 1u,
               (ctx.events[i].state3 >> 1) & 1u,
               (ctx.events[i].state3 >> 2) & 1u);
    }
    printf("BOOTTRACE: OK\n");
}
