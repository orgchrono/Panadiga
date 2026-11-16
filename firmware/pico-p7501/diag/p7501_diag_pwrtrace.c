/**
 * @file p7501_diag_pwrtrace.c
 * @brief Boot rail voltage (ADC0 SW_UART) & NSW5V/RXD/TXD glitch logger (`PWRTRACE`).
 *
 * Strictly preserves GP3 (`NSW5V`) as Hi-Z input (no pulls) at all times.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_diag_pwrtrace.h"

#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/adc.h"
#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

typedef struct pwrtrace_ctx {
    p7501_adc0_trace_t adc_trace;
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

static float read_sw_uart_voltage(bool *is_driven_out) {
    const bool driven = gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
    if (is_driven_out != NULL) {
        *is_driven_out = driven;
    }
    if (driven) {
        return gpio_get(P7501_PIN_SW_UART_ADC) ? P7501_ADC_VREF : 0.00f;
    }
    adc_select_input(P7501_ADC_CH_VDD);
    return ((float)adc_read()) * (P7501_ADC_VREF / P7501_ADC_RESOLUTION);
}

static void update_minmax_pair(float *min_v, float *max_v, float v) {
    if (min_v != NULL && v < *min_v) {
        *min_v = v;
    }
    if (max_v != NULL && v > *max_v) {
        *max_v = v;
    }
}

void p7501_diag_adc0_trace_init(p7501_adc0_trace_t *trace) {
    if (trace == NULL) {
        return;
    }
    trace->min_v = 9.99f;
    trace->max_v = 0.00f;
    trace->hiz_min_v = 9.99f;
    trace->hiz_max_v = 0.00f;
    trace->low_samples = 0u;
    trace->hiz_samples = 0u;
}

void p7501_diag_adc0_trace_sample(p7501_adc0_trace_t *trace) {
    if (trace == NULL) {
        return;
    }
    bool driven = false;
    const float v = read_sw_uart_voltage(&driven);
    update_minmax_pair(&trace->min_v, &trace->max_v, v);
    if (driven) {
        ++trace->low_samples;
    } else {
        update_minmax_pair(&trace->hiz_min_v, &trace->hiz_max_v, v);
        ++trace->hiz_samples;
    }
}

void p7501_diag_adc0_trace_finalize(p7501_adc0_trace_t *trace) {
    if (trace == NULL) {
        return;
    }
    if (trace->min_v > trace->max_v) {
        trace->min_v = 0.00f;
    }
    if (trace->hiz_min_v > trace->hiz_max_v) {
        trace->hiz_min_v = 0.00f;
    }
}

void p7501_diag_sample_adc0_minmax(float *min_v, float *max_v) {
    const float v = read_sw_uart_voltage(NULL);
    update_minmax_pair(min_v, max_v, v);
}

static void pwrtrace_on_sw_rel(void *raw_ctx, uint32_t rel_us, const char *reason) {
    pwrtrace_ctx_t *ctx = (pwrtrace_ctx_t *)raw_ctx;
    if (!ctx->sw_rel_seen) {
        ctx->sw_rel_seen = true;
        ctx->sw_rel_ms = rel_us / 1000u;
        ctx->sw_rel_reason = reason;
        p7501_diag_adc0_trace_sample(&ctx->adc_trace);
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
        p7501_led_tick();
        p7501_diag_adc0_trace_sample(&ctx->adc_trace);
    }
}

void p7501_cmd_pwrtrace(const char *arg) {
    p7501_window_boot_spec_t spec;
    p7501_parse_window_boot_args(arg, 4000u, 100u, P7501_TRACE_MAX_MS, &spec);

    printf("=== PWRTRACE: window=%" PRIu32 "ms boot=%d hold=%" PRIu32 "ms ===\n",
           spec.window_ms, (int)spec.boot_action, spec.jig_hold_ms);
    fflush(stdout);

    p7501_wdt_set_stage(P7501_WDT_STAGE_RESET_SYNC);
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);
    if (spec.boot_action != P7501_BOOT_ACTION_JIG) {
        p7501_hw_restore_sw_uart_adc0();
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
    p7501_diag_adc0_trace_init(&ctx.adc_trace);
    p7501_diag_adc0_trace_sample(&ctx.adc_trace);

    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = spec.window_ms,
        .boot_action = spec.boot_action,
        .jig_hold_ms = spec.jig_hold_ms,
        .manage_session = false,
        .auto_release_serial_boot = (spec.boot_action == P7501_BOOT_ACTION_JIG),
        .allow_cdc_abort = true,
        .on_rx_byte = NULL,
        .on_tick = pwrtrace_on_tick,
        .on_sw_rel = pwrtrace_on_sw_rel,
        .ctx = &ctx,
    };
    p7501_pio_run_session(&cfg, NULL);

    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    p7501_diag_adc0_trace_finalize(&ctx.adc_trace);
    printf("[PWRTRACE] GP26_SWUART: min=%.2fV max=%.2fV (low=%" PRIu32 " hiz=%.2f..%.2fV n=%" PRIu32 ") | sw_rel=%s",
           (double)ctx.adc_trace.min_v, (double)ctx.adc_trace.max_v,
           ctx.adc_trace.low_samples, (double)ctx.adc_trace.hiz_min_v, (double)ctx.adc_trace.hiz_max_v,
           ctx.adc_trace.hiz_samples, ctx.sw_rel_seen ? "YES@" : "NO");
    if (ctx.sw_rel_seen) {
        printf("%" PRIu32 "ms(%s)", ctx.sw_rel_ms, ctx.sw_rel_reason);
    }
    printf(" | Edges: RXD=%" PRIu32 " NSW5V=%" PRIu32 " TXD=%" PRIu32 " (final NSW5V=%d)\n",
           ctx.rxd_edges, ctx.nsw5v_edges, ctx.txd_edges, gpio_get(P7501_PIN_NSW5V) ? 1 : 0);
    printf("PWRTRACE: OK\n");
}
