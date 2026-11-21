/**
 * @file p7501_pwrosc.c
 * @brief Imperative `PWROSC` power-rail oscilloscope & die-temperature monitor (`ADC0` + `ADC4`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_pwrosc_codec.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "flash/p7501_probe_cfg_codec.h"
#include "flash/p7501_qspi_storage.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_uart_codec.h"

#include "hardware/adc.h"
#include "hardware/dma.h"
#include "hardware/regs/adc.h"
#include "hardware/regs/dreq.h"
#include "hardware/structs/adc.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

static uint32_t pwrosc_raw_to_cal_mv(uint16_t raw, const p7501_probe_cfg_t *cfg) {
    const uint32_t masked = (uint32_t)(raw & 0x0FFFu);
    const uint32_t raw_mv = (uint32_t)(((uint64_t)masked * P7501_SSOT_VDD_NOMINAL_MV) / 4096ULL);
    return p7501_probe_cfg_apply_adc_cal_mv(raw_mv, cfg);
}

static size_t pwrosc_capture_dma(uint32_t ms, bool *out_used_dma, bool *out_aborted) {
    *out_used_dma = false;
    uint32_t n_samples = (ms <= 80u) ? (ms * 50u) : P7501_ARENA_PWROSC_CAP;
    if (n_samples < 64u) {
        n_samples = 64u;
    }
    if (n_samples > P7501_ARENA_PWROSC_CAP) {
        n_samples = P7501_ARENA_PWROSC_CAP;
    }

    const int dma_ch = dma_claim_unused_channel(false);
    if (dma_ch < 0) {
        return 0u;
    }
    *out_used_dma = true;

    uint64_t cycles = (48000ULL * (uint64_t)ms) / (uint64_t)n_samples;
    if (cycles < 960ULL) {
        cycles = 960ULL;
    } else if (cycles > 65536ULL) {
        cycles = 65536ULL;
    }
    const uint32_t div_int = (uint32_t)(cycles - 1ULL);

    adc_run(false);
    adc_set_round_robin(0u);
    adc_fifo_drain();
    adc_fifo_setup(true, true, 1u, false, false);
    adc_hw->div = (div_int << ADC_DIV_INT_LSB);

    dma_channel_config cfg = dma_channel_get_default_config((uint)dma_ch);
    channel_config_set_transfer_data_size(&cfg, DMA_SIZE_16);
    channel_config_set_read_increment(&cfg, false);
    channel_config_set_write_increment(&cfg, true);
    channel_config_set_dreq(&cfg, DREQ_ADC);

    dma_channel_configure(
        (uint)dma_ch,
        &cfg,
        g_p7501_cmd_arena.adc_samples,
        &adc_hw->fifo,
        n_samples,
        true
    );

    const uint64_t t0 = time_us_64();
    const uint64_t deadline = t0 + (((uint64_t)ms + 300ULL) * 1000ULL);
    adc_run(true);

    while (dma_channel_is_busy((uint)dma_ch) &&
           p7501_wdt_loop_step(t0, deadline, true, out_aborted)) {
    }

    adc_run(false);
    /* Disable ADC FIFO and DREQ_ADC BEFORE aborting DMA so no stale DREQ races with abort. */
    adc_fifo_setup(false, false, 0u, false, false);
    size_t collected = n_samples;
    if (dma_channel_is_busy((uint)dma_ch)) {
        const uint32_t rem = dma_channel_hw_addr((uint)dma_ch)->transfer_count & 0x0FFFFFFFUL;
        collected = (rem <= n_samples) ? (size_t)(n_samples - rem) : 0u;
    }
    dma_channel_abort((uint)dma_ch);
    dma_channel_unclaim((uint)dma_ch);
    adc_fifo_drain();
    adc_set_round_robin(0u);
    adc_hw->div = 0u;
    return collected;
}

void p7501_cmd_pwrosc(const char *arg) {
    uint32_t ms = p7501_parse_uint32_clamped(arg, 500u, 50u, 10000u);
    p7501_probe_cfg_t cfg;
    p7501_probe_cfg_load_active_hw(&cfg);

    adc_run(false);
    adc_fifo_setup(false, false, 0u, false, false);
    adc_hw->div = 0u;
    adc_set_temp_sensor_enabled(true);
    adc_select_input(ADC_TEMPERATURE_CHANNEL_NUM);
    (void)adc_read();
    const int32_t die_mc = p7501_adc_raw_to_temp_mc(adc_read());

    p7501_pwrosc_stats_t st;
    p7501_pwrosc_stats_init(&st, die_mc);

    p7501_hw_restore_sw_uart_adc0();
    adc_select_input(P7501_ADC_CH_VDD);
    (void)adc_read();

    bool used_dma = false;
    bool aborted = false;
    const size_t dma_samples = pwrosc_capture_dma(ms, &used_dma, &aborted);
    if (used_dma) {
        for (size_t i = 0u; i < dma_samples; ++i) {
            p7501_pwrosc_stats_feed_mv(
                &st, pwrosc_raw_to_cal_mv(g_p7501_cmd_arena.adc_samples[i], &cfg)
            );
        }
    } else {
        const uint64_t t0 = time_us_64();
        const uint64_t deadline = t0 + ((uint64_t)ms * 1000ULL);
        while (p7501_wdt_loop_step(t0, deadline, true, &aborted) &&
               st.sample_count < P7501_ARENA_PWROSC_CAP) {
            p7501_pwrosc_stats_feed_mv(&st, pwrosc_raw_to_cal_mv(adc_read(), &cfg));
            sleep_us(200u);
        }
    }

    adc_set_temp_sensor_enabled(false);
    p7501_pwrosc_stats_finalize(&st);
    printf("[PWROSC] mode=%s samples=%" PRIu32 " min=%" PRIu32 "mV avg=%" PRIu32 "mV max=%" PRIu32 "mV ripple=%" PRIu32 "mV brownouts=%" PRIu32 " die=%ld.%03ldC cal_ppm=%" PRIu32 " cal_off=%" PRId32 " drops=%" PRIu32 "%s\n",
           used_dma ? "DMA_FIFO" : "POLL",
           st.sample_count, st.min_mv, st.avg_mv, st.max_mv, st.ripple_mv, st.brownouts,
           (long)(st.die_temp_mc / 1000),
           (long)((st.die_temp_mc < 0 ? -st.die_temp_mc : st.die_temp_mc) % 1000),
           cfg.adc0_gain_ppm, cfg.adc0_offset_mv,
           p7501_hw_get_nsw5v_dropouts(),
           aborted ? " aborted=YES" : "");
    printf("PWROSC: OK\n");
}
