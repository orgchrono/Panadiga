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
#include "common/picofw_wdt_logic.h"
#include "hal/picofw_led.h"
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

/* ------------------------------------------------------------------------- *
 * BOOTFOLLOW: passive, unbounded, boot-cycle-aware trace
 *
 * - Never drives XRESET / SW_UART / NSW5V (PASSIVE session, `NOPULL` equivalent).
 * - Runs until Ctrl+C. The PIO RX session stays open across internal segments, so the RX DMA
 *   ring keeps buffering between segments: no byte is lost and no timestamp is rewound.
 * - Cycles follow the target: XRESET falling edge, NSW5V rising edge (wake) or a fresh
 *   `DDR CHECK: OK` banner restart the relative clock at +0 ms; NSW5V falling ends the cycle.
 * ------------------------------------------------------------------------- */

#define BOOTFOLLOW_SEGMENT_MS      30000u
#define BOOTFOLLOW_DEFAULT_IDLE_MS 8000u

typedef struct follow_ctx {
    picofw_boot_cycle_t cycle;
    picofw_boot_classifier_t classifier;
    char line[160];
    size_t line_len;
    uint64_t line_start_us;
    uint64_t follow_t0_us;
    absolute_time_t next_led_t;
} follow_ctx_t;

static void follow_close_cycle(
    follow_ctx_t *f,
    uint32_t cycle_no,
    uint64_t start_us,
    uint32_t lines,
    const char *reason,
    uint64_t end_us
) {
    const uint32_t dur_ms = (end_us >= start_us) ? (uint32_t)((end_us - start_us) / 1000ULL) : 0u;
    printf("[FOLLOW] CYCLE %" PRIu32 " END reason=%s dur=%" PRIu32 "ms lines=%" PRIu32 "\n",
           cycle_no, reason, dur_ms, lines);
    picofw_boottrace_emit_reports(&f->classifier);
    picofw_boot_classifier_init(&f->classifier);
}

static void follow_apply(follow_ctx_t *f, picofw_cycle_event_t ev, uint64_t now_us) {
    const picofw_boot_cycle_t *cy = &f->cycle;
    switch (ev) {
        case PICOFW_CYCLE_EV_START:
            if (cy->had_previous) {
                follow_close_cycle(f, cy->prev_cycle_no, cy->prev_start_us, cy->prev_lines,
                                   "NEW_BOOT", cy->start_us);
            }
            picofw_boot_classifier_init(&f->classifier);
            printf("[FOLLOW] CYCLE %" PRIu32 " START reason=%s t_abs=%" PRIu32 "ms\n",
                   cy->cycle_no, picofw_boot_cycle_reason_name(cy->start_reason),
                   (uint32_t)((cy->start_us - f->follow_t0_us) / 1000ULL));
            break;
        case PICOFW_CYCLE_EV_END:
            follow_close_cycle(f, cy->cycle_no, cy->start_us, cy->lines_in_cycle,
                               picofw_boot_cycle_reason_name(cy->end_reason), now_us);
            break;
        case PICOFW_CYCLE_EV_IDLE_REPORT:
            printf("[FOLLOW] CYCLE %" PRIu32 " IDLE_REPORT lines=%" PRIu32 " t=%" PRIu32 "ms\n",
                   cy->cycle_no, cy->lines_in_cycle, picofw_boot_cycle_rel_ms(cy, now_us));
            picofw_boottrace_emit_reports(&f->classifier);
            break;
        default:
            return;
    }
    fflush(stdout);
}

static void follow_flush_line(follow_ctx_t *f) {
    if (f->line_len == 0u) {
        return;
    }
    f->line[f->line_len] = '\0';
    follow_apply(f, picofw_boot_cycle_on_line(&f->cycle, f->line_start_us, f->line), f->line_start_us);
    printf("[BOOTTRACE +%4" PRIu32 "ms] %s\n",
           picofw_boot_cycle_rel_ms(&f->cycle, f->line_start_us), f->line);
    picofw_boot_classifier_feed_line(&f->classifier, f->line);
    f->line_len = 0u;
    fflush(stdout);
}

static void follow_on_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    (void)out_stop;
    follow_ctx_t *f = (follow_ctx_t *)raw_ctx;
    if ((byte == '\r') || (byte == '\n')) {
        follow_flush_line(f);
        return;
    }
    if (f->line_len == 0u) {
        f->line_start_us = time_us_64();
    }
    if (f->line_len < (sizeof(f->line) - 1u)) {
        f->line[f->line_len++] = ((byte >= 0x20u) && (byte < 0x7Fu)) ? (char)byte : '.';
    }
    if (f->line_len >= (sizeof(f->line) - 1u)) {
        follow_flush_line(f);
    }
}

static void follow_on_tick(void *raw_ctx, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    (void)out_stop;
    follow_ctx_t *f = (follow_ctx_t *)raw_ctx;
    const uint64_t now_us = time_us_64();

    const uint8_t ctrl3 = read_ctrl3();
    if (ctrl3 != f->cycle.prev_ctrl3) {
        follow_apply(f, picofw_boot_cycle_on_ctrl(&f->cycle, now_us, ctrl3), now_us);
        printf("  [CTRL +%7" PRIu32 "us] NSW5V=%u TXD=%u XRESET=%u\n",
               picofw_boot_cycle_rel_us(&f->cycle, now_us),
               (ctrl3 >> 0) & 1u, (ctrl3 >> 1) & 1u, (ctrl3 >> 2) & 1u);
        fflush(stdout);
    }
    follow_apply(f, picofw_boot_cycle_tick(&f->cycle, now_us), now_us);

    if (time_reached(f->next_led_t)) {
        f->next_led_t = make_timeout_time_ms(10u);
        picofw_led_tick();
    }
}

void picofw_cmd_bootfollow(const char *arg) {
    uint32_t idle_ms = BOOTFOLLOW_DEFAULT_IDLE_MS;
    char tok[24] = {0};
    if (picofw_next_token(arg, tok, sizeof(tok)) != NULL) {
        idle_ms = picofw_parse_uint32_clamped(tok, BOOTFOLLOW_DEFAULT_IDLE_MS, 1000u, 120000u);
    }

    _Static_assert(sizeof(follow_ctx_t) <= sizeof(g_picofw_cmd_arena.raw),
                   "follow_ctx_t must fit inside shared command arena");
    follow_ctx_t *const f = (follow_ctx_t *)(void *)g_picofw_cmd_arena.raw;
    memset(f, 0, sizeof(*f));
    picofw_boot_classifier_init(&f->classifier);
    const uint8_t ctrl3 = read_ctrl3();
    picofw_boot_cycle_init(&f->cycle, idle_ms, ctrl3);
    f->follow_t0_us = time_us_64();
    f->next_led_t = make_timeout_time_ms(10u);

    printf("=== BOOTFOLLOW: passive continuous, idle_report=%" PRIu32 "ms, never drives XRESET/SW_UART; Ctrl+C to stop ===\n",
           idle_ms);
    fflush(stdout);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RESET_SYNC);
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    picofw_pio_begin_rx_session_cfg(PICOFW_BOOT_ACTION_PASSIVE, 0u, false);
    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RX_WAIT);

    if ((ctrl3 & PICOFW_CTRL3_NSW5V) != 0u) {
        picofw_boot_cycle_attach(&f->cycle, f->follow_t0_us);
        follow_apply(f, PICOFW_CYCLE_EV_START, f->follow_t0_us);
    } else {
        printf("[FOLLOW] armed: target in standby (NSW5V=0), waiting for wake/boot\n");
        fflush(stdout);
    }

    const picofw_pio_loop_cfg_t cfg = {
        .window_ms = BOOTFOLLOW_SEGMENT_MS,
        .boot_action = PICOFW_BOOT_ACTION_PASSIVE,
        .jig_hold_ms = 0u,
        .manage_session = false,
        .no_xreset = false,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = follow_on_byte,
        .on_tick = follow_on_tick,
        .on_sw_rel = NULL,
        .ctx = f,
    };

    uint32_t total_rx = 0u;
    uint32_t total_ferr = 0u;
    uint32_t total_perr = 0u;
    uint32_t overflow_segments = 0u;
    picofw_pio_loop_metrics_t m;
    do {
        memset(&m, 0, sizeof(m));
        picofw_pio_run_session(&cfg, &m);
        total_rx += m.rx_bytes;
        total_ferr += m.framing_errors;
        total_perr += m.parity_errors;
        if (m.overflow) {
            ++overflow_segments;
            printf("[FOLLOW] WARN rx_overflow (host too slow draining CDC)\n");
            fflush(stdout);
        }
    } while (!m.aborted);

    const uint64_t end_us = time_us_64();
    follow_flush_line(f);
    if (f->cycle.active) {
        follow_close_cycle(f, f->cycle.cycle_no, f->cycle.start_us, f->cycle.lines_in_cycle,
                           picofw_boot_cycle_reason_name(PICOFW_CYCLE_REASON_ABORT), end_us);
    }
    printf("[FOLLOW] session: cycles=%" PRIu32 " rx_bytes=%" PRIu32 " ferr=%" PRIu32 " perr=%" PRIu32
           " overflow_segments=%" PRIu32 "\n",
           f->cycle.cycle_no, total_rx, total_ferr, total_perr, overflow_segments);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    printf("BOOTFOLLOW: OK (aborted=YES)\n");
}
