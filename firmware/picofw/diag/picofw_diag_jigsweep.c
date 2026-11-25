/**
 * @file picofw_diag_jigsweep.c
 * @brief Target driver for `JIGSWEEP` (HOLD / DELAY / PREBOOT timing & handshake discovery).
 *
 * Uses PIO1 dual RX/TX state machines, exact microsecond XRESET release timing,
 * and temporary `8E1`/`8N1` / multi-baud switching across `0..10ms` pre-DDR boot.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_jigsweep_codec.h"

#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "uart/picofw_uart_codec.h"
#include "uart/picofw_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#define JIGSWEEP_TRIAL_RX_CAP PICOFW_ARENA_JIGSWEEP_CAP
#define JIGSWEEP_WINDOW_MS 320u
#define JIGSWEEP_PRE_DDR_US 9500u

static void sweep_print_preview(const uint8_t *buf, size_t len) {
    const size_t max_chars = (len > 72u) ? 72u : len;
    putchar('"');
    for (size_t i = 0u; i < max_chars; ++i) {
        const uint8_t c = buf[i];
        if (c == '\r' || c == '\n') {
            putchar(' ');
        } else if (c >= 0x20u && c < 0x7Fu) {
            putchar((int)c);
        } else {
            putchar('.');
        }
    }
    if (len > max_chars) {
        printf("...");
    }
    putchar('"');
}

static bool sweep_run_hold_or_delay_trial(picofw_jigsweep_mode_t mode, uint32_t param_ms) {
    uint8_t *const rx_buf = g_picofw_cmd_arena.jigsweep_buf;
    const picofw_boot_action_t start_action =
        (mode == PICOFW_JIGSWEEP_HOLD) ? PICOFW_BOOT_ACTION_JIG : PICOFW_BOOT_ACTION_NORMAL;
    const uint32_t release_ms =
        (mode == PICOFW_JIGSWEEP_HOLD) ? param_ms : ((param_ms + 35u > 50u) ? (param_ms + 35u) : 50u);

    picofw_pio_begin_rx_session_ex(start_action, (mode == PICOFW_JIGSWEEP_HOLD) ? param_ms : 0u);
    /* HOLD=0ms must release SW_UART immediately at reset release instead of latching LOW all trial. */
    if ((mode == PICOFW_JIGSWEEP_HOLD) && (param_ms == 0u)) {
        picofw_pio_release_sw_uart_now();
    }
    const uint64_t t0_us = time_us_64();
    const uint64_t deadline_us = t0_us + ((uint64_t)JIGSWEEP_WINDOW_MS * 1000ULL);

    bool delay_asserted = false;
    bool delay_released = false;
    bool first_seen = false;
    bool aborted = false;
    uint32_t first_us = 0u;
    size_t rx_len = 0u;
    uint32_t ferr = 0u;
    uint8_t byte = 0u;

    while (picofw_wdt_loop_step(t0_us, deadline_us, true, &aborted)) {
        const uint32_t rel_us = (uint32_t)(time_us_64() - t0_us);
        if (mode == PICOFW_JIGSWEEP_HOLD) {
            (void)picofw_pio_poll_sw_uart_release();
        } else {
            if (!delay_asserted && rel_us >= (param_ms * 1000u)) {
                picofw_hw_sw_uart_set_low(true);
                delay_asserted = true;
            }
            if (delay_asserted && !delay_released && rel_us >= (release_ms * 1000u)) {
                picofw_pio_release_sw_uart_now();
                delay_released = true;
            }
        }
        if (picofw_pio_poll_and_clear_framing_err()) {
            ++ferr;
        }
        for (uint32_t k = 0u; (k < 4096u) && picofw_pio_pop_rx_byte(&byte); ++k) {
            if ((k & 63u) == 63u) {
                picofw_wdt_kick();
            }
            if (!first_seen) {
                first_seen = true;
                first_us = (uint32_t)(time_us_64() - t0_us);
            }
            if (rx_len < JIGSWEEP_TRIAL_RX_CAP) {
                rx_buf[rx_len++] = byte;
            }
        }
    }

    const bool ovf = picofw_pio_had_rx_overflow();
    picofw_pio_end_session();

    const picofw_jigsweep_outcome_t outcome = picofw_jigsweep_classify_rx(rx_buf, rx_len);
    printf("  [%s=%3" PRIu32 "ms] outcome=%-11s bytes=%4u first=%6" PRIu32 "us ferr=%" PRIu32 " ovf=%s | ",
           (mode == PICOFW_JIGSWEEP_HOLD) ? "HOLD" : "DELAY",
           param_ms,
           picofw_jigsweep_outcome_name(outcome),
           (unsigned)rx_len,
           first_us,
           ferr,
           ovf ? "YES" : "NO");
    sweep_print_preview(rx_buf, rx_len);
    putchar('\n');
    return !aborted;
}

static bool sweep_run_preboot_trial(uint32_t profile) {
    static const char *const k_names[6] = {
        "38400_8E1_AMOBP_SYNC",
        "115200_8E1_AMOBP_SYNC",
        "115200_8N1_BREAK_9MS",
        "115200_8N1_CTRL_BURST",
        "JIG_0_30MS_8E1_LISTEN",
        "38400_8N1_SERIAL_SYNC"
    };
    static const uint8_t k_amobp[] = { 0x00u, 0x55u, 0xAAu, 0x0Du };
    static const uint8_t k_ctrl[] = { 0x03u, 0x1Au, 0x1Bu, 0x20u, 0x3Fu, 0x0Du };
    static const uint8_t k_serial[] = { 's', 'e', 'r', 'i', 'a', 'l', '\r', 0x00u, 0x55u };

    const uint32_t pre_baud = (profile == 0u || profile == 5u) ? 38400u : 115200u;
    const picofw_uart_framing_t pre_frame =
        (profile == 0u || profile == 1u || profile == 4u) ? PICOFW_FRAMING_8E1 : PICOFW_FRAMING_8N1;

    (void)picofw_pio_set_baud(pre_baud);
    (void)picofw_pio_set_framing(pre_frame);

    if (profile == 2u) {
        picofw_pio_begin_rx_session_ex(PICOFW_BOOT_ACTION_PASSIVE, 0u);
        picofw_hw_sw_uart_set_low(false);
        picofw_hw_drive_txd_low();
        picofw_wdt_sleep_ms(2u);
        picofw_hw_pulse_xreset_ms(PICOFW_RESET_DIAG_ASSERT_MS);
    } else {
        const picofw_boot_action_t b_act = (profile == 4u) ? PICOFW_BOOT_ACTION_JIG : PICOFW_BOOT_ACTION_NORMAL;
        picofw_pio_begin_rx_session_ex(b_act, (profile == 4u) ? 30u : 0u);
    }

    const uint64_t t0_us = time_us_64();
    const uint64_t deadline_us = t0_us + ((uint64_t)JIGSWEEP_WINDOW_MS * 1000ULL);
    uint8_t pre_buf[64];
    size_t pre_len = 0u;
    uint8_t *const post_buf = g_picofw_cmd_arena.jigsweep_buf;
    size_t post_len = 0u;
    uint32_t pre_ferr = 0u;
    uint32_t post_ferr = 0u;
    uint32_t bursts = 0u;
    bool switched_post = false;
    bool break_released = (profile != 2u);
    bool aborted = false;
    uint8_t byte = 0u;

    while (picofw_wdt_loop_step(t0_us, deadline_us, true, &aborted)) {
        const uint32_t rel_us = (uint32_t)(time_us_64() - t0_us);
        (void)picofw_pio_poll_sw_uart_release();

        if (!switched_post && rel_us < JIGSWEEP_PRE_DDR_US) {
            if ((profile == 0u || profile == 1u) && (bursts == 0u || (bursts < 3u && rel_us >= bursts * 3000u))) {
                picofw_pio_tx_bytes(k_amobp, sizeof(k_amobp), false);
                ++bursts;
            } else if (profile == 3u && (bursts == 0u || (bursts < 3u && rel_us >= bursts * 3000u))) {
                picofw_pio_tx_bytes(k_ctrl, sizeof(k_ctrl), false);
                ++bursts;
            } else if (profile == 5u && bursts == 0u) {
                picofw_pio_tx_bytes(k_serial, sizeof(k_serial), false);
                ++bursts;
            }
        }

        if (!break_released && rel_us >= 9000u) {
            picofw_hw_release_txd_hiz(true);
            break_released = true;
        }

        if (!switched_post && rel_us >= JIGSWEEP_PRE_DDR_US) {
            while (picofw_pio_pop_rx_byte(&byte)) {
                if (pre_len < sizeof(pre_buf)) pre_buf[pre_len++] = byte;
            }
            (void)picofw_pio_set_baud(PICOFW_UART_DEFAULT_BAUD);
            (void)picofw_pio_set_framing(PICOFW_FRAMING_8N1);
            switched_post = true;
        }

        if (picofw_pio_poll_and_clear_framing_err()) {
            if (!switched_post) ++pre_ferr; else ++post_ferr;
        }
        while (picofw_pio_pop_rx_byte(&byte)) {
            if (!switched_post) {
                if (pre_len < sizeof(pre_buf)) pre_buf[pre_len++] = byte;
            } else {
                if (post_len < JIGSWEEP_TRIAL_RX_CAP) post_buf[post_len++] = byte;
            }
        }
    }

    const uint32_t perr = picofw_pio_get_and_clear_parity_errors();
    picofw_pio_end_session();

    const picofw_jigsweep_outcome_t outcome = picofw_jigsweep_classify_rx(post_buf, post_len);
    printf("  [PREBOOT P%" PRIu32 " %-22s] pre_rx=%u(ferr=%" PRIu32 ",perr=%" PRIu32 ") post=%-11s bytes=%u ferr=%" PRIu32 " | ",
           profile % 6u,
           k_names[profile % 6u],
           (unsigned)pre_len,
           pre_ferr,
           perr,
           picofw_jigsweep_outcome_name(outcome),
           (unsigned)post_len,
           post_ferr);
    if (pre_len > 0u) {
        printf("PRE=");
        for (size_t i = 0u; i < pre_len && i < 12u; ++i) {
            printf("%02X", pre_buf[i]);
        }
        printf(" ");
    }
    sweep_print_preview(post_buf, post_len);
    putchar('\n');
    return !aborted;
}

void picofw_cmd_jigsweep(const char *arg) {
    picofw_jigsweep_spec_t spec;
    if (!picofw_jigsweep_parse_args(arg, &spec)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[JIGSWEEP] ERROR: Invalid syntax. Use JIGSWEEP [HOLD|DELAY|PREBOOT] [ms_list]\n");
        return;
    }

    const uint32_t saved_baud = picofw_pio_get_baud();
    const picofw_uart_framing_t saved_framing = picofw_pio_get_framing();
    const char *mode_lbl = (spec.mode == PICOFW_JIGSWEEP_DELAY)
        ? "DELAY"
        : ((spec.mode == PICOFW_JIGSWEEP_PREBOOT) ? "PREBOOT" : "HOLD");

    printf("=== JIGSWEEP %s (%u trials, %ums window each) ===\n",
           mode_lbl, (unsigned)spec.point_count, (unsigned)JIGSWEEP_WINDOW_MS);
    fflush(stdout);

    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    bool aborted = false;
    for (size_t i = 0u; i < spec.point_count; ++i) {
        const bool keep_going = (spec.mode == PICOFW_JIGSWEEP_PREBOOT)
            ? sweep_run_preboot_trial(spec.points_ms[i])
            : sweep_run_hold_or_delay_trial(spec.mode, spec.points_ms[i]);
        if (!keep_going || picofw_wdt_poll_abort()) {
            aborted = true;
            break;
        }
        picofw_wdt_sleep_ms(40u);
    }

    (void)picofw_pio_set_baud(saved_baud);
    (void)picofw_pio_set_framing(saved_framing);
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    printf("JIGSWEEP: OK%s\n", aborted ? " (aborted=YES)" : "");
}
