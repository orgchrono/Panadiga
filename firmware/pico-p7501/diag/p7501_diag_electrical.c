/**
 * @file p7501_diag_electrical.c
 * @brief Implementation of P7501 6-pin electrical matrix checks, contention tests, and bus sniffing.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_diag_electrical.h"
#include "common/p7501_hex.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_uart_codec.h"
#include "pico/stdlib.h"
#include "hardware/gpio.h"
#include <stdio.h>
#include <inttypes.h>

void p7501_diag_sample_transitions(
    uint32_t duration_ms,
    p7501_bus_transitions_t *out_counts,
    p7501_bus_levels_t *out_final_levels
) {
    p7501_bus_transitions_t counts = { 0U, 0U, 0U, 0U };
    uint32_t all = gpio_get_all();
    bool last_rxd    = ((all & (1UL << P7501_PIN_RXD)) != 0U);
    bool last_nsw5v  = ((all & (1UL << P7501_PIN_NSW5V)) != 0U);
    bool last_txd    = ((all & (1UL << P7501_PIN_TXD)) != 0U);
    bool last_xreset = ((all & (1UL << P7501_PIN_XRESET)) != 0U);

    const uint64_t t0_us = time_us_64();
    const uint64_t deadline_us = t0_us + ((uint64_t)duration_ms * 1000ULL);
    uint32_t wdt_iter = 0U;
    while (time_us_64() < deadline_us) {
        if ((++wdt_iter & 0x0FFFu) == 0U) {
            if (!p7501_wdt_loop_step(t0_us, deadline_us, true, NULL)) {
                break;
            }
        }
        all = gpio_get_all();
        const bool cur_rxd    = ((all & (1UL << P7501_PIN_RXD)) != 0U);
        const bool cur_nsw5v  = ((all & (1UL << P7501_PIN_NSW5V)) != 0U);
        const bool cur_txd    = ((all & (1UL << P7501_PIN_TXD)) != 0U);
        const bool cur_xreset = ((all & (1UL << P7501_PIN_XRESET)) != 0U);

        if (cur_rxd    != last_rxd)    { counts.rxd++;    last_rxd    = cur_rxd;    }
        if (cur_nsw5v  != last_nsw5v)  { counts.nsw5v++;  last_nsw5v  = cur_nsw5v;  }
        if (cur_txd    != last_txd)    { counts.txd++;    last_txd    = cur_txd;    }
        if (cur_xreset != last_xreset) { counts.xreset++; last_xreset = cur_xreset; }
    }

    if (out_counts != NULL) {
        *out_counts = counts;
    }
    if (out_final_levels != NULL) {
        out_final_levels->rxd    = last_rxd;
        out_final_levels->nsw5v  = last_nsw5v;
        out_final_levels->txd    = last_txd;
        out_final_levels->xreset = last_xreset;
    }
}

void p7501_diag_cmd_sniff(const char *arg) {
    uint32_t duration_ms = 250U;
    if ((arg != NULL) && (*arg != '\0')) {
        if (!p7501_parse_u32_strict(arg, &duration_ms) || (duration_ms == 0U)) {
            duration_ms = 250U;
        } else if (duration_ms > 10000U) {
            duration_ms = 10000U;
        }
    }

    printf("[SNIFF] Passively monitoring P7501 GP2/RXD, GP3/NSW5V, GP4/TXD, GP5/XRESET for %" PRIu32 " ms...\n",
           duration_ms);
    p7501_hw_isolate_bus_hiz();
    sleep_us(200);

    p7501_bus_transitions_t counts;
    p7501_bus_levels_t final_lvl;
    p7501_diag_sample_transitions(duration_ms, &counts, &final_lvl);
    p7501_hw_isolate_bus_hiz();

    printf("SNIFF RESULT (%" PRIu32 " ms):\n", duration_ms);
    printf("  UARTP2M (GP2) : Final=%d, Transitions=%" PRIu32 "\n", final_lvl.rxd ? 1 : 0, counts.rxd);
    printf("  NSW5V   (GP3) : Final=%d, Transitions=%" PRIu32 "\n", final_lvl.nsw5v ? 1 : 0, counts.nsw5v);
    printf("  UARTM2P (GP4) : Final=%d, Transitions=%" PRIu32 "\n", final_lvl.txd ? 1 : 0, counts.txd);
    printf("  XRESET  (GP5) : Final=%d, Transitions=%" PRIu32 "\n", final_lvl.xreset ? 1 : 0, counts.xreset);
    if ((counts.rxd == 0U) && (counts.nsw5v == 0U) && (counts.txd == 0U) && (counts.xreset == 0U)) {
        if (final_lvl.nsw5v && final_lvl.txd) {
            printf("  [STATUS] Lines idle HIGH (3.3V). Target is powered and P7501 pull-ups are intact.\n");
        } else if (!final_lvl.nsw5v && !final_lvl.txd) {
            printf("  [STATUS] Lines steady LOW (0V). Target may be unpowered or lines shorted to GND.\n");
        } else {
            printf("  [STATUS] Lines static: GP2=%d, GP3=%d, GP4=%d, GP5=%d.\n",
                   final_lvl.rxd ? 1 : 0, final_lvl.nsw5v ? 1 : 0,
                   final_lvl.txd ? 1 : 0, final_lvl.xreset ? 1 : 0);
        }
    } else {
        printf("  [STATUS] ACTIVE P7501 BUS ACTIVITY DETECTED! (GP2=%" PRIu32 ", GP3=%" PRIu32 ", GP4=%" PRIu32 ", GP5=%" PRIu32 ")\n",
               counts.rxd, counts.nsw5v, counts.txd, counts.xreset);
    }
    printf("SNIFF: OK\n");
}

void p7501_diag_cmd_pincheck(const char *arg) {
    bool passive_only = false;
    if (arg != NULL) {
        char tok[24];
        const char *cur = arg;
        while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
            if (p7501_ascii_ieq(tok, "PASSIVE") ||
                p7501_ascii_ieq(tok, "NORST") ||
                p7501_ascii_ieq(tok, "SAFE")) {
                passive_only = true;
            }
        }
    }

    static const uint8_t pins[4] = {
        (uint8_t)P7501_PIN_RXD,
        (uint8_t)P7501_PIN_NSW5V,
        (uint8_t)P7501_PIN_TXD,
        (uint8_t)P7501_PIN_XRESET
    };
    static const char *names[4] = {
        "GP2 (Pin6 RXD   )",
        "GP3 (Pin5 NSW5V )",
        "GP4 (Pin3 TXD   )",
        "GP5 (Pin2 XRESET)"
    };

    p7501_hw_isolate_bus_hiz();
    sleep_us(500);
    const uint32_t pre_vdd_mv = p7501_hw_read_vdd_mv();
    const bool target_powered = p7501_hw_is_nsw5v_high() || (pre_vdd_mv >= P7501_SSOT_VDD_MIN_MV);

    printf("\n=== P7501 6-PIN ELECTRICAL MATRIX (PINCHECK) ===\n");
    printf("Pin               | Hi-Z | Pull-Down | Pull-Up | Electrical State\n");
    printf("------------------+------+-----------+---------+----------------------------------------\n");

    for (size_t i = 0U; i < 4U; ++i) {
        const uint8_t p = pins[i];
        /* Keep GP3 (NSW5V) strictly Hi-Z with pulls disabled to guard against ~5.15V upstream rail */
        gpio_disable_pulls(p);
        sleep_us(300);
        const int hiz = gpio_get(p) ? 1 : 0;

        int pd = hiz;
        int pu = hiz;
        const char *state = "NSW5V HI-Z SENSE (Pulls Disabled Safe Rail)";
        if (p != (uint8_t)P7501_PIN_NSW5V) {
            gpio_pull_down(p);
            sleep_us(300);
            pd = gpio_get(p) ? 1 : 0;

            if (target_powered && (p != (uint8_t)P7501_PIN_XRESET)) {
                gpio_pull_up(p);
                sleep_us(300);
                pu = gpio_get(p) ? 1 : 0;
            } else {
                pu = hiz;
            }

            gpio_disable_pulls(p);
            if (!target_powered) {
                state = (pd == 0 && hiz == 0)
                    ? "UNPOWERED / HI-Z (Pull-Up Skipped: Anti-Backpower)"
                    : "UNPOWERED / RESIDUAL LEVEL (Pull-Up Skipped)";
            } else {
                state = "UNKNOWN";
                if ((pd == 1) && (pu == 1)) {
                    state = "STRONG HIGH (External Pull-Up / 3.3V Active)";
                } else if ((pd == 0) && (pu == 0)) {
                    state = "STRONG LOW  (Grounded / Supervisor Held 0V)";
                } else if ((pd == 0) && (pu == 1)) {
                    state = "HI-Z / CMOS INPUT (No External Pull or Open)";
                }
            }
        }
        printf("%-17s |  %d   |     %d     |    %d    | %s\n", names[i], hiz, pd, pu, state);
    }

    const uint32_t mv_hiz = p7501_hw_sample_sw_uart_mv(false, false);
    const uint32_t mv_pd  = p7501_hw_sample_sw_uart_mv(true, false);
    const uint32_t mv_pu  = target_powered ? p7501_hw_sample_sw_uart_mv(false, true) : mv_hiz;
    char s_hiz[8], s_pd[8], s_pu[8];

    printf("GP26(Pin1 SWUART) | %sV|   %sV   |  %sV  | P7501 SW_UART / Target Sense\n",
           p7501_fmt_mv_volts(mv_hiz, s_hiz),
           p7501_fmt_mv_volts(mv_pd, s_pd),
           p7501_fmt_mv_volts(mv_pu, s_pu));

    if (passive_only) {
        printf("------------------------------------------------------------------------\n");
        printf("[PINCHECK] Passive mode: skipping XRESET pulse to preserve active target session.\n");
        printf("PINCHECK: OK\n");
        p7501_hw_isolate_bus_hiz();
        return;
    }

    if (!target_powered) {
        printf("------------------------------------------------------------------------\n");
        printf("[PINCHECK] Target unpowered (anti-backpower guard active): skipping 3.3V pull-ups and XRESET pulse.\n");
        printf("PINCHECK: OK\n");
        p7501_hw_isolate_bus_hiz();
        return;
    }

    p7501_hw_reset_assert();
    p7501_wdt_sleep_ms(P7501_RESET_DIAG_ASSERT_MS);
    const int rst_low_gp2 = gpio_get(P7501_PIN_RXD) ? 1 : 0;
    const int rst_low_gp3 = gpio_get(P7501_PIN_NSW5V) ? 1 : 0;
    const int rst_low_gp4 = gpio_get(P7501_PIN_TXD) ? 1 : 0;
    const int rst_low_gp5 = gpio_get(P7501_PIN_XRESET) ? 1 : 0;
    p7501_hw_reset_release();

    uint32_t rise_us = 0U;
    bool rose = false;
    for (rise_us = 0U; rise_us <= 50000U; rise_us += 10U) {
        if (gpio_get(P7501_PIN_XRESET)) {
            rose = true;
            break;
        }
        sleep_us(10);
    }

    p7501_bus_transitions_t post_counts;
    p7501_diag_sample_transitions(250U, &post_counts, NULL);

    printf("------------------------------------------------------------------------\n");
    printf("While GP5 (XRESET) asserted LOW (0V): GP2=%d, GP3=%d, GP4=%d, GP5=%d\n",
           rst_low_gp2, rst_low_gp3, rst_low_gp4, rst_low_gp5);
    if (rose) {
        printf("XRESET Release Rise Time (Hi-Z)     : %" PRIu32 " us\n", rise_us);
    } else {
        printf("XRESET Release Rise Time (Hi-Z)     : TIMEOUT (>50000 us, remained 0)\n");
    }
    printf("Post-Reset 250ms Transitions        : GP2=%" PRIu32 ", GP3=%" PRIu32 ", GP4=%" PRIu32 ", GP5=%" PRIu32 "\n",
           post_counts.rxd, post_counts.nsw5v, post_counts.txd, post_counts.xreset);
    printf("PINCHECK: OK\n");

    p7501_hw_isolate_bus_hiz();
}

