/**
 * @file picofw_diag_boottrace.c
 * @brief Correlated PIO1 UART + GP3/GP4/GP5 control-edge + GP26 ADC0 boot profiler (`BOOTTRACE`).
 *
 * Captures timestamped UART lines without losing control-pin edges (NSW5V, TXD, XRESET)
 * to GP2 serial chatter, while monitoring SW_UART ADC0 rail continuously.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_diag_boottrace.h"
#include "diag/picofw_boottrace_codec.h"
#include "diag/picofw_diag_pwrtrace.h"
#include "diag/picofw_gps_cli.h"

#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_push_stream.h"
#include "hal/picofw_led.h"
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
    picofw_adc0_trace_t adc_trace;
    absolute_time_t next_adc_t;
    char line[160];
    size_t line_len;
    uint32_t line_start_ms;
    picofw_boot_classifier_t classifier;
} boottrace_ctx_t;

static uint8_t read_ctrl3(void) {
    const uint32_t all = gpio_get_all();
    return (uint8_t)((all >> P7501_PIN_NSW5V) & 0x07u);
}

static void flush_uart_line(boottrace_ctx_t *ctx) {
    if (ctx->line_len == 0u) {
        return;
    }
    ctx->line[ctx->line_len] = '\0';
    printf("[BOOTTRACE +%4" PRIu32 "ms] %s\n", ctx->line_start_ms, ctx->line);
    picofw_boot_classifier_feed_line(&ctx->classifier, ctx->line);
    ctx->line_len = 0u;
}

void picofw_boottrace_emit_reports(const picofw_boot_classifier_t *cls) {
    picofw_gps_sync_from_classifier(cls);
    char summary[224];
    if (picofw_boot_classifier_format_summary(cls, summary, sizeof(summary)) > 0u) {
        printf("%s\n", summary);
    }
    char deep[256];
    const size_t dlen = picofw_boot_classifier_format_deep(cls, deep, sizeof(deep));
    if (dlen > 0u) {
        printf("%s\n", deep);
        const char *kv = deep;
        if (strncmp(deep, PICOFW_WIRE_TAG_DIAG_DEEP " ", sizeof(PICOFW_WIRE_TAG_DIAG_DEEP)) == 0) {
            kv = deep + sizeof(PICOFW_WIRE_TAG_DIAG_DEEP);
        }
        (void)picofw_push_emit_str("DIAG_DEEP", kv);
        (void)picofw_push_flush_global();
    }
}

static void boottrace_on_sw_rel(void *raw_ctx, uint32_t rel_us, const char *reason) {
    boottrace_ctx_t *ctx = (boottrace_ctx_t *)raw_ctx;
    if (!ctx->sw_rel_seen) {
        ctx->sw_rel_seen = true;
        ctx->sw_rel_us = rel_us;
        ctx->sw_rel_reason = reason;
        picofw_diag_adc0_trace_sample(&ctx->adc_trace);
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
        flush_uart_line(ctx);
        return;
    }
    if (ctx->line_len == 0u) {
        ctx->line_start_ms = rel_us / 1000u;
    }
    if (ctx->line_len < (sizeof(ctx->line) - 1u)) {
        ctx->line[ctx->line_len++] = (byte >= 0x20u && byte < 0x7Fu) ? (char)byte : '.';
    }
    if (ctx->line_len >= (sizeof(ctx->line) - 1u)) {
        flush_uart_line(ctx);
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
        picofw_led_tick();
        picofw_diag_adc0_trace_sample(&ctx->adc_trace);
    }
}

void picofw_cmd_boottrace(const char *arg) {
    picofw_window_boot_spec_t spec;
    picofw_parse_window_boot_args(arg, 4500u, 100u, PICOFW_TRACE_MAX_MS, &spec);
    if (spec.boot_action == PICOFW_BOOT_ACTION_JIG && spec.jig_hold_ms == 0u) {
        spec.jig_hold_ms = spec.no_xreset ? 3000u : PICOFW_JIG_DEFAULT_HOLD_MS;
    }

    if (spec.wait_for_wake) {
        uint32_t wake_mv = 0u;
        uint32_t wake_ms = 0u;
        char s_wake[8];
        const bool prestrap = spec.no_xreset && (spec.boot_action == PICOFW_BOOT_ACTION_JIG);
        if (!picofw_pio_wait_standby_wake("BOOTTRACE", spec.wake_timeout_ms, prestrap, &wake_mv, &wake_ms)) {
            printf("[BOOTTRACE] Notice: Standby wake not observed within %" PRIu32 " ms (VDD=%sV)\n",
                   wake_ms, picofw_fmt_mv_volts(wake_mv, s_wake));
            printf("[BOOTTRACE] UART: bytes=0 first_byte=0us ferr=0 perr=0 ovf=NO | SW_rel=NO\n");
            printf("[BOOTTRACE] CtrlEdges: nsw5v_edges=0 rxd_edges=0 txd_edges=0 wake_timeout=YES\n");
            printf("BOOTTRACE: OK\n");
            return;
        }
        printf("[BOOTTRACE +%" PRIu32 "ms] WAKE edge detected (NSW5V=%u, VDD=%sV) -> starting capture\n",
               wake_ms, gpio_get(P7501_PIN_NSW5V) ? 1u : 0u, picofw_fmt_mv_volts(wake_mv, s_wake));
    }

    printf("=== BOOTTRACE: window=%" PRIu32 "ms boot=%d hold=%" PRIu32 "ms sticky=%s no_xrst=%s (%s @ %" PRIu32 ") ===\n",
           spec.window_ms, (int)spec.boot_action, spec.jig_hold_ms,
           spec.sticky_jig ? "YES" : "NO", spec.no_xreset ? "YES" : "NO",
           picofw_framing_name(picofw_pio_get_framing()), picofw_pio_get_baud());
    fflush(stdout);

    _Static_assert(sizeof(boottrace_ctx_t) <= sizeof(g_picofw_cmd_arena.raw),
                   "boottrace_ctx_t must fit inside shared command arena");
    boottrace_ctx_t *const ctx = (boottrace_ctx_t *)(void *)g_picofw_cmd_arena.raw;
    memset(ctx, 0, sizeof(*ctx));
    ctx->prev_ctrl3 = read_ctrl3();
    ctx->sw_rel_reason = "NONE";
    ctx->next_adc_t = make_timeout_time_ms(10u);
    picofw_boot_classifier_init(&ctx->classifier);
    picofw_diag_adc0_trace_init(&ctx->adc_trace);
    picofw_diag_adc0_trace_sample(&ctx->adc_trace);

    const picofw_pio_loop_cfg_t cfg = {
        .window_ms = spec.window_ms,
        .boot_action = spec.boot_action,
        .jig_hold_ms = spec.jig_hold_ms,
        .manage_session = true,
        .no_xreset = spec.no_xreset,
        .auto_release_serial_boot = !spec.sticky_jig,
        .allow_cdc_abort = true,
        .on_rx_byte = boottrace_on_byte,
        .on_tick = boottrace_on_tick,
        .on_sw_rel = boottrace_on_sw_rel,
        .ctx = ctx,
    };
    picofw_pio_loop_metrics_t m;
    picofw_pio_run_session(&cfg, &m);

    flush_uart_line(ctx);
    picofw_diag_adc0_trace_finalize(&ctx->adc_trace);

    printf("[BOOTTRACE] UART: bytes=%" PRIu32 " first_byte=%" PRIu32 "us ferr=%" PRIu32
           " perr=%" PRIu32 " ovf=%s | SW_rel=%s",
           m.rx_bytes, ctx->first_byte_us, m.framing_errors, m.parity_errors,
           m.overflow ? "YES" : "NO", ctx->sw_rel_seen ? "YES@" : "NO");
    if (ctx->sw_rel_seen) {
        printf("%" PRIu32 "us(%s)", ctx->sw_rel_us, ctx->sw_rel_reason);
    }
    char adc_summary[96];
    picofw_diag_adc0_trace_format(&ctx->adc_trace, adc_summary, sizeof(adc_summary));
    printf("\n[BOOTTRACE] ADC0_SWUART: %s | CtrlEdges: NSW5V=%" PRIu32
           " TXD=%" PRIu32 " XRESET=%" PRIu32 " (logged=%u)\n",
           adc_summary,
           ctx->nsw_edges, ctx->txd_edges, ctx->xrst_edges, (unsigned)ctx->ev_count);

    const size_t preview = (ctx->ev_count > 16u) ? 16u : ctx->ev_count;
    for (size_t i = 0u; i < preview; ++i) {
        printf("  [CTRL +%7" PRIu32 "us] NSW5V=%u TXD=%u XRESET=%u\n",
               ctx->events[i].us,
               (ctx->events[i].state3 >> 0) & 1u,
               (ctx->events[i].state3 >> 1) & 1u,
               (ctx->events[i].state3 >> 2) & 1u);
    }
    if (m.aborted) {
        picofw_cli_set_result("ERR", "EABORT");
    }
    picofw_boottrace_emit_reports(&ctx->classifier);
    printf("BOOTTRACE: OK%s\n", m.aborted ? " (aborted=YES)" : "");
}
