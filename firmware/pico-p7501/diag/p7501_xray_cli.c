/**
 * @file p7501_xray_cli.c
 * @brief Pico CLI handler `XRAY` orchestrating 12-stage real-time PiE-OS boot tracing.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_gps_cli.h"

#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "diag/p7501_diag_boottrace.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static p7501_xray_trace_t s_xray_trace;
static bool s_xray_init = false;

p7501_xray_trace_t *p7501_xray_global_trace(void) {
    if (!s_xray_init) {
        p7501_xray_init(&s_xray_trace, false);
        s_xray_init = true;
    }
    return &s_xray_trace;
}

typedef struct xray_run_ctx {
    char line[160];
    size_t line_len;
    p7501_boot_classifier_t cls;
    p7501_xray_event_t ev;
    char fmt[224];
} xray_run_ctx_t;

static void xray_flush_line(xray_run_ctx_t *ctx, uint64_t ts_us) {
    if (ctx->line_len == 0u) {
        return;
    }
    ctx->line[ctx->line_len] = '\0';
    p7501_boot_classifier_feed_line(&ctx->cls, ctx->line);
    p7501_gps_env_t *const env = p7501_gps_global_env();
    p7501_gps_calibrate_from_classifier(env, &ctx->cls);
    p7501_xray_trace_t *const tr = p7501_xray_global_trace();
    if (p7501_xray_feed_line(tr, env, ts_us, ctx->line, &ctx->ev)) {
        if (p7501_xray_format_event(&ctx->ev, tr->color_enabled, ctx->fmt, sizeof(ctx->fmt)) > 0u) {
            printf("%s\n", ctx->fmt);
        }
    }
    ctx->line_len = 0u;
}

static void xray_on_rx_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)out_stop;
    xray_run_ctx_t *const ctx = (xray_run_ctx_t *)raw_ctx;
    if (p7501_line_accum_push_byte(ctx->line, &ctx->line_len, sizeof(ctx->line), byte)) {
        xray_flush_line(ctx, (uint64_t)rel_us);
    }
}

static void emit_xray_summary(const p7501_xray_trace_t *tr) {
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    (void)p7501_xray_format_summary(tr, p7501_gps_global_env(), msg, 256u);
    p7501_gps_emit_tag("XRAY", msg);
}

void p7501_cmd_xray(const char *arg) {
    p7501_xray_trace_t *const tr = p7501_xray_global_trace();
    char copy[48];
    char *toks[2] = { NULL, NULL };
    const size_t n = p7501_copy_and_split_ws(arg, copy, sizeof(copy), toks, 2u);

    if ((n == 2u) && p7501_ascii_ieq(toks[0], "COLOR")) {
        if (p7501_ascii_ieq(toks[1], "ON")) {
            tr->color_enabled = true;
        } else if (p7501_ascii_ieq(toks[1], "OFF")) {
            tr->color_enabled = false;
        } else {
            p7501_cli_set_result("ERR", "EINVAL");
            printf("XRAY: ERROR (Usage: XRAY COLOR <ON|OFF>)\n");
            return;
        }
        printf("[XRAY] color=%s\nXRAY: OK\n", tr->color_enabled ? "ON" : "OFF");
        return;
    }

    if ((n == 0u) || ((n == 1u) && p7501_ascii_ieq(toks[0], "SUMMARY"))) {
        emit_xray_summary(tr);
        printf("XRAY: OK\n");
        return;
    }

    bool boot_reset = false;
    bool wait_wake = false;
    uint32_t window_ms = 3000u;
    size_t idx = 0u;
    if (p7501_ascii_ieq(toks[0], "BOOT")) {
        boot_reset = true;
        window_ms = 15000u;
        idx = 1u;
    } else if (p7501_ascii_ieq(toks[0], "WAKE")) {
        wait_wake = true;
        window_ms = 20000u;
        idx = 1u;
    }
    if ((idx < n) && !p7501_parse_u32_auto(toks[idx], &window_ms)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("XRAY: ERROR (Usage: XRAY [ms|BOOT [ms]|WAKE [ms]|SUMMARY|COLOR <ON|OFF>])\n");
        return;
    }
    window_ms = (window_ms < 200u) ? 200u : ((window_ms > 30000u) ? 30000u : window_ms);
    if (wait_wake) {
        uint32_t w_mv = 0u;
        uint32_t w_ms = 0u;
        if (!p7501_pio_wait_standby_wake("XRAY", 30000u, false, &w_mv, &w_ms)) {
            p7501_cli_set_result("ERR", "ETIMEOUT");
            printf("XRAY: WAKE TIMEOUT (%" PRIu32 "ms, vdd=%" PRIu32 "mV)\n", w_ms, w_mv);
            return;
        }
        printf("[XRAY] WAKE +%" PRIu32 "ms (vdd=%" PRIu32 "mV)\n", w_ms, w_mv);
    }
    p7501_xray_init(tr, tr->color_enabled);

    _Static_assert(sizeof(xray_run_ctx_t) <= sizeof(g_p7501_cmd_arena.raw),
                   "xray_run_ctx_t must fit inside command arena");
    xray_run_ctx_t *const ctx = (xray_run_ctx_t *)(void *)g_p7501_cmd_arena.raw;
    memset(ctx, 0, sizeof(*ctx));
    p7501_boot_classifier_init(&ctx->cls);

    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = window_ms,
        .boot_action = boot_reset ? P7501_BOOT_ACTION_NORMAL : P7501_BOOT_ACTION_PASSIVE,
        .jig_hold_ms = 0u,
        .manage_session = true,
        .no_xreset = !boot_reset,
        .auto_release_serial_boot = true,
        .allow_cdc_abort = true,
        .on_rx_byte = xray_on_rx_byte,
        .on_tick = NULL,
        .on_sw_rel = NULL,
        .ctx = ctx,
    };
    p7501_pio_loop_metrics_t m;
    p7501_pio_run_session(&cfg, &m);
    xray_flush_line(ctx, (uint64_t)m.elapsed_ms * 1000ULL);
    p7501_boottrace_emit_reports(&ctx->cls);

    emit_xray_summary(tr);
    printf("XRAY: OK (bytes=%" PRIu32 ")\n", m.rx_bytes);
}
