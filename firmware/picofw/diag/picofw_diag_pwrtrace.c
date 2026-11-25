/**
 * @file picofw_diag_pwrtrace.c
 * @brief Boot rail voltage (ADC0 SW_UART) & NSW5V/RXD/TXD glitch logger (`PWRTRACE`).
 *
 * Strictly preserves GP3 (`NSW5V`) as Hi-Z input (no pulls) at all times.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_diag_pwrtrace.h"

#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "hal/picofw_hw.h"
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

typedef struct pwrtrace_ctx {
    picofw_adc0_trace_t adc_trace;
    uint32_t sw_rel_ms;
    bool sw_rel_seen;
    const char *sw_rel_reason;
    uint32_t nsw5v_edges;
    uint32_t rxd_edges;
    uint32_t txd_edges;
    bool prev_nsw;
    bool prev_rxd;
    bool prev_txd;
    absolute_time_t next_adc_t;
} pwrtrace_ctx_t;

static uint32_t read_sw_uart_mv(bool *is_driven_out) {
    const bool driven = gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
    if (is_driven_out != NULL) {
        *is_driven_out = driven;
    }
    if (driven) {
        return gpio_get(P7501_PIN_SW_UART_ADC) ? PICOFW_SSOT_VDD_NOMINAL_MV : 0u;
    }
    return picofw_hw_sample_sw_uart_mv(false, false);
}

static void update_minmax_pair_mv(uint32_t *min_mv, uint32_t *max_mv, uint32_t mv) {
    if (min_mv != NULL && mv < *min_mv) {
        *min_mv = mv;
    }
    if (max_mv != NULL && mv > *max_mv) {
        *max_mv = mv;
    }
}

void picofw_diag_adc0_trace_init(picofw_adc0_trace_t *trace) {
    if (trace == NULL) {
        return;
    }
    trace->min_mv = 9999u;
    trace->max_mv = 0u;
    trace->hiz_min_mv = 9999u;
    trace->hiz_max_mv = 0u;
    trace->low_samples = 0u;
    trace->hiz_samples = 0u;
}

void picofw_diag_adc0_trace_sample(picofw_adc0_trace_t *trace) {
    if (trace == NULL) {
        return;
    }
    bool driven = false;
    const uint32_t mv = read_sw_uart_mv(&driven);
    update_minmax_pair_mv(&trace->min_mv, &trace->max_mv, mv);
    if (driven) {
        ++trace->low_samples;
    } else {
        update_minmax_pair_mv(&trace->hiz_min_mv, &trace->hiz_max_mv, mv);
        ++trace->hiz_samples;
    }
}

void picofw_diag_adc0_trace_finalize(picofw_adc0_trace_t *trace) {
    if (trace == NULL) {
        return;
    }
    if (trace->min_mv > trace->max_mv) {
        trace->min_mv = 0u;
    }
    if (trace->hiz_min_mv > trace->hiz_max_mv) {
        trace->hiz_min_mv = 0u;
    }
}

void picofw_diag_adc0_trace_format(const picofw_adc0_trace_t *trace, char *out_buf, size_t cap) {
    if ((trace == NULL) || (out_buf == NULL) || (cap == 0u)) {
        return;
    }
    char s_min[8], s_max[8], s_hmin[8], s_hmax[8];
    (void)snprintf(
        out_buf,
        cap,
        "min=%sV max=%sV (low=%" PRIu32 " hiz=%s..%sV n=%" PRIu32 ")",
        picofw_fmt_mv_volts(trace->min_mv, s_min),
        picofw_fmt_mv_volts(trace->max_mv, s_max),
        trace->low_samples,
        picofw_fmt_mv_volts(trace->hiz_min_mv, s_hmin),
        picofw_fmt_mv_volts(trace->hiz_max_mv, s_hmax),
        trace->hiz_samples
    );
}

void picofw_diag_sample_adc0_minmax(uint32_t *min_mv, uint32_t *max_mv) {
    const uint32_t mv = read_sw_uart_mv(NULL);
    update_minmax_pair_mv(min_mv, max_mv, mv);
}

static void pwrtrace_on_sw_rel(void *raw_ctx, uint32_t rel_us, const char *reason) {
    pwrtrace_ctx_t *ctx = (pwrtrace_ctx_t *)raw_ctx;
    if (!ctx->sw_rel_seen) {
        ctx->sw_rel_seen = true;
        ctx->sw_rel_ms = rel_us / 1000u;
        ctx->sw_rel_reason = reason;
        picofw_diag_adc0_trace_sample(&ctx->adc_trace);
    }
}

static void pwrtrace_on_tick(void *raw_ctx, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    (void)out_stop;
    pwrtrace_ctx_t *ctx = (pwrtrace_ctx_t *)raw_ctx;

    const bool cur_nsw = gpio_get(P7501_PIN_NSW5V);
    const bool cur_rxd = gpio_get(P7501_PIN_RXD);
    const bool cur_txd = gpio_get(P7501_PIN_TXD);

    if (cur_nsw != ctx->prev_nsw) { ++ctx->nsw5v_edges; ctx->prev_nsw = cur_nsw; }
    if (cur_rxd != ctx->prev_rxd) { ++ctx->rxd_edges; ctx->prev_rxd = cur_rxd; }
    if (cur_txd != ctx->prev_txd) { ++ctx->txd_edges; ctx->prev_txd = cur_txd; }

    if (time_reached(ctx->next_adc_t)) {
        ctx->next_adc_t = make_timeout_time_ms(10u);
        picofw_led_tick();
        picofw_diag_adc0_trace_sample(&ctx->adc_trace);
    }
}

void picofw_cmd_pwrtrace(const char *arg) {
    picofw_window_boot_spec_t spec;
    picofw_parse_window_boot_args(arg, 4000u, 100u, PICOFW_TRACE_MAX_MS, &spec);

    printf("=== PWRTRACE: window=%" PRIu32 "ms boot=%d hold=%" PRIu32 "ms ===\n",
           spec.window_ms, (int)spec.boot_action, spec.jig_hold_ms);
    fflush(stdout);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RESET_SYNC);
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    picofw_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);
    if (spec.boot_action != PICOFW_BOOT_ACTION_JIG) {
        picofw_hw_restore_sw_uart_adc0();
    }

    pwrtrace_ctx_t ctx = {
        .sw_rel_ms = 0u,
        .sw_rel_seen = false,
        .sw_rel_reason = "NONE",
        .nsw5v_edges = 0u,
        .rxd_edges = 0u,
        .txd_edges = 0u,
        .prev_nsw = gpio_get(P7501_PIN_NSW5V),
        .prev_rxd = gpio_get(P7501_PIN_RXD),
        .prev_txd = gpio_get(P7501_PIN_TXD),
        .next_adc_t = make_timeout_time_ms(10u),
    };
    picofw_diag_adc0_trace_init(&ctx.adc_trace);
    picofw_diag_adc0_trace_sample(&ctx.adc_trace);

    const picofw_pio_loop_cfg_t cfg = {
        .window_ms = spec.window_ms,
        .boot_action = spec.boot_action,
        .jig_hold_ms = spec.jig_hold_ms,
        .manage_session = false,
        .auto_release_serial_boot = (spec.boot_action == PICOFW_BOOT_ACTION_JIG),
        .allow_cdc_abort = true,
        .on_rx_byte = NULL,
        .on_tick = pwrtrace_on_tick,
        .on_sw_rel = pwrtrace_on_sw_rel,
        .ctx = &ctx,
    };
    picofw_pio_loop_metrics_t m;
    picofw_pio_run_session(&cfg, &m);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    picofw_diag_adc0_trace_finalize(&ctx.adc_trace);
    char adc_summary[96];
    picofw_diag_adc0_trace_format(&ctx.adc_trace, adc_summary, sizeof(adc_summary));
    printf("[PWRTRACE] GP26_SWUART: %s | sw_rel=%s",
           adc_summary,
           ctx.sw_rel_seen ? "YES@" : "NO");
    if (ctx.sw_rel_seen) {
        printf("%" PRIu32 "ms(%s)", ctx.sw_rel_ms, ctx.sw_rel_reason);
    }
    printf(" | Edges: RXD=%" PRIu32 " NSW5V=%" PRIu32 " TXD=%" PRIu32 " (final NSW5V=%d)\n",
           ctx.rxd_edges, ctx.nsw5v_edges, ctx.txd_edges, gpio_get(P7501_PIN_NSW5V) ? 1 : 0);
    if (m.aborted) {
        picofw_cli_set_result("ERR", "EABORT");
    }
    printf("PWRTRACE: OK%s\n", m.aborted ? " (aborted=YES)" : "");
}
