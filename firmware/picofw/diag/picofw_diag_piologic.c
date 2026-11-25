/**
 * @file picofw_diag_piologic.c
 * @brief Simultaneous 4-pin PIO0 logic sampler (`PIOLOGIC`: GP2/GP3/GP4/GP5).
 *
 * Strictly leaves GP3 (NSW5V) as Hi-Z input without pulls and never invokes
 * `pio_gpio_init` on any pin, eliminating electrical contention risk.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_piologic_codec.h"
#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "picofw_logic_sampler.pio.h"

#include "hardware/gpio.h"
#include "hardware/pio.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#define PIOLOGIC_MAX_EDGES PICOFW_ARENA_PIOLOGIC_EDGES

static uint8_t piologic_read_pins4_now(void) {
    const uint32_t all = gpio_get_all();
    return (uint8_t)((all >> P7501_PIN_RXD) & 0x0Fu);
}

static uint32_t piologic_fmt_min_us(uint32_t val) {
    return (val == UINT32_MAX) ? 0u : val;
}

void picofw_cmd_piologic(const char *arg) {
    picofw_logic_edge_t *const s_edges = g_picofw_cmd_arena.piologic_edges;
    picofw_window_boot_spec_t boot;
    uint32_t sample_khz = 1000u;
    bool ctrl_only = false;

    picofw_logic_parse_args_ex(arg, &boot, &sample_khz, &ctrl_only);

    picofw_hw_enforce_nsw5v_hiz();

    if (!pio_can_add_program(pio0, &picofw_logic_sampler_program)) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("PIOLOGIC: ERROR (No PIO0 instruction slots left)\n");
        return;
    }
    const int sm_signed = pio_claim_unused_sm(pio0, false);
    if (sm_signed < 0) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("PIOLOGIC: ERROR (No free PIO0 state machine)\n");
        return;
    }
    const uint sm = (uint)sm_signed;
    const uint offset = pio_add_program(pio0, &picofw_logic_sampler_program);
    picofw_logic_sampler_program_init(
        pio0, sm, offset, P7501_PIN_RXD, sample_khz * 1000u
    );

    bool hold_jig_active = false;
    bool release_xreset_pending = false;
    if (boot.boot_action != PICOFW_BOOT_ACTION_PASSIVE) {
        if (boot.boot_action == PICOFW_BOOT_ACTION_JIG) {
            picofw_hw_sw_uart_set_low(true);
            hold_jig_active = (boot.jig_hold_ms > 0u);
        } else {
            picofw_hw_sw_uart_set_low(false);
        }
        picofw_hw_reset_assert();
        picofw_wdt_sleep_ms(30u);
        release_xreset_pending = true;
    }

    const uint8_t initial_state4 = piologic_read_pins4_now();
    uint8_t last_state4 = initial_state4;
    uint32_t rep_word = (uint32_t)last_state4 * 0x11111111u;
    size_t edge_count = 0u;
    bool overflow = false;
    bool saw_reset_rise = (boot.boot_action == PICOFW_BOOT_ACTION_PASSIVE);
    bool saw_first_rxd_fall = false;

    const uint64_t total_samples = ((uint64_t)boot.window_ms * (uint64_t)sample_khz);
    uint64_t sample_idx = 0u;
    const uint64_t reset_release_sample = (uint64_t)sample_khz; /* Release XRESET at +1 ms inside capture */
    const uint64_t hold_samples = (uint64_t)boot.jig_hold_ms * (uint64_t)sample_khz;
    const uint64_t hard_deadline_us = time_us_64() + ((uint64_t)boot.window_ms * 1000ULL) + 50000ULL;

    pio_sm_clear_fifos(pio0, sm);
    pio0->fdebug = (1u << (PIO_FDEBUG_RXSTALL_LSB + sm));
    pio_sm_set_enabled(pio0, sm, true);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RX_WAIT);
    while (sample_idx < total_samples && time_us_64() < hard_deadline_us) {
        if (release_xreset_pending && sample_idx >= reset_release_sample) {
            picofw_hw_reset_release();
            release_xreset_pending = false;
        }
        if (hold_jig_active && sample_idx >= hold_samples) {
            picofw_hw_sw_uart_set_low(false);
            hold_jig_active = false;
        }
        if (pio_sm_is_rx_fifo_empty(pio0, sm)) {
            if (picofw_wdt_poll_abort()) {
                break;
            }
            tight_loop_contents();
            continue;
        }
        const uint32_t w = pio_sm_get(pio0, sm);
        if ((sample_idx & 0x0FFFu) == 0u && picofw_wdt_poll_abort()) {
            break;
        }
        if (w != rep_word) {
            for (uint32_t nib = 0u; nib < 8u; ++nib) {
                const uint8_t cur = (uint8_t)((w >> (nib * 4u)) & 0x0Fu);
                if (cur != last_state4) {
                    const uint8_t diff = (uint8_t)(last_state4 ^ cur);
                    if ((diff & (1u << PICOFW_LOGIC_CH_XRESET)) != 0u &&
                        (cur & (1u << PICOFW_LOGIC_CH_XRESET)) != 0u) {
                        saw_reset_rise = true;
                    }
                    const bool is_first_rxd_fall =
                        saw_reset_rise && !saw_first_rxd_fall &&
                        ((diff & (1u << PICOFW_LOGIC_CH_RXD)) != 0u) &&
                        ((cur & (1u << PICOFW_LOGIC_CH_RXD)) == 0u);
                    if (is_first_rxd_fall) {
                        saw_first_rxd_fall = true;
                    }
                    const bool store_edge = !ctrl_only || is_first_rxd_fall || ((diff & 0x0Eu) != 0u);
                    if (store_edge) {
                        if (edge_count < PIOLOGIC_MAX_EDGES) {
                            const uint64_t abs_sample = sample_idx + nib;
                            s_edges[edge_count].t_us = (uint32_t)((abs_sample * 1000ULL) / sample_khz);
                            s_edges[edge_count].state4 = cur;
                            ++edge_count;
                        } else {
                            overflow = true;
                        }
                    }
                    last_state4 = cur;
                }
            }
            rep_word = (uint32_t)last_state4 * 0x11111111u;
        }
        sample_idx += 8u;
    }

    const bool rx_stall = (pio0->fdebug & (1u << (PIO_FDEBUG_RXSTALL_LSB + sm))) != 0u;
    pio_sm_set_enabled(pio0, sm, false);
    pio_remove_program(pio0, &picofw_logic_sampler_program, offset);
    pio_sm_unclaim(pio0, sm);

    if (release_xreset_pending) {
        picofw_hw_reset_release();
    }
    picofw_hw_isolate_bus_hiz();

    picofw_logic_stats_t stats;
    picofw_logic_analyze_edges(s_edges, edge_count, initial_state4, &stats);

    const uint32_t span_us = (edge_count > 0u) ? s_edges[edge_count - 1u].t_us : 0u;
    printf("=== PIOLOGIC PIO0 (%" PRIu32 "ms @ %" PRIu32 "kHz%s, boot=%d, hold=%" PRIu32 "ms) ===\n",
           boot.window_ms, sample_khz, ctrl_only ? " CTRL" : "", (int)boot.boot_action, boot.jig_hold_ms);
    printf("Initial: RXD=%u NSW5V=%u TXD=%u XRESET=%u | fifo_stall=%s%s | edges=%u%s | span=%" PRIu32 "us\n",
           (initial_state4 >> 0) & 1u, (initial_state4 >> 1) & 1u,
           (initial_state4 >> 2) & 1u, (initial_state4 >> 3) & 1u,
           rx_stall ? "YES" : "NO",
           rx_stall ? " (TIMING_SKEW)" : "",
           (unsigned)edge_count,
           overflow ? " (TRUNCATED)" : "",
           span_us);

    static const char *const k_names[PICOFW_LOGIC_CH_COUNT] = {
        "RXD   (GP2)", "NSW5V (GP3)", "TXD   (GP4)", "XRESET(GP5)"
    };
    for (size_t ch = 0u; ch < PICOFW_LOGIC_CH_COUNT; ++ch) {
        printf("  %s : Final=%u, Transitions=%" PRIu32 " (rise=%" PRIu32 " fall=%" PRIu32
               " minLow=%" PRIu32 "us minHigh=%" PRIu32 "us)\n",
               k_names[ch],
               (last_state4 >> ch) & 1u,
               stats.edges[ch],
               stats.rising[ch],
               stats.falling[ch],
               piologic_fmt_min_us(stats.min_low_us[ch]),
               piologic_fmt_min_us(stats.min_high_us[ch]));
    }
    if (stats.saw_reset_release) {
        printf("[PIOLOGIC] XRESET_rise -> first_RXD_fall: %" PRIu32 " us\n",
               stats.reset_to_first_rxd_us);
    }
    if (stats.saw_tx_to_rx) {
        printf("[PIOLOGIC] TXD_fall -> RXD_fall turnaround: min=%" PRIu32 " us max=%" PRIu32 " us\n",
               stats.min_tx_to_rx_us, stats.max_tx_to_rx_us);
    }

    const size_t preview = (edge_count > 32u) ? 32u : edge_count;
    for (size_t i = 0u; i < preview; ++i) {
        const uint8_t s = s_edges[i].state4;
        const uint32_t dt = (i == 0u) ? s_edges[i].t_us : (s_edges[i].t_us - s_edges[i - 1u].t_us);
        printf("  +%07" PRIu32 " us (+%5" PRIu32 "us): RXD=%u NSW5V=%u TXD=%u XRESET=%u\n",
               s_edges[i].t_us, dt, (s >> 0) & 1u, (s >> 1) & 1u, (s >> 2) & 1u, (s >> 3) & 1u);
    }
    printf("PIOLOGIC: OK\n");
}
