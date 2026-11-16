/**
 * @file p7501_diag_jigsweep.c
 * @brief Target driver for `JIGSWEEP` (HOLD / DELAY / PREBOOT timing & handshake discovery).
 *
 * Uses PIO1 dual RX/TX state machines, exact microsecond XRESET release timing,
 * and temporary `8E1`/`8N1` / multi-baud switching across `0..10ms` pre-DDR boot.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_jigsweep_codec.h"

#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#define JIGSWEEP_TRIAL_RX_CAP 2048u
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

static void sweep_run_hold_or_delay_trial(p7501_jigsweep_mode_t mode, uint32_t param_ms) {
    static uint8_t rx_buf[JIGSWEEP_TRIAL_RX_CAP];
    const p7501_boot_action_t start_action =
        (mode == P7501_JIGSWEEP_HOLD) ? P7501_BOOT_ACTION_JIG : P7501_BOOT_ACTION_NORMAL;
    const uint32_t release_ms =
        (mode == P7501_JIGSWEEP_HOLD) ? param_ms : ((param_ms + 35u > 50u) ? (param_ms + 35u) : 50u);

    p7501_pio_begin_rx_session_ex(start_action, (mode == P7501_JIGSWEEP_HOLD) ? param_ms : 0u);
    const uint64_t t0_us = time_us_64();
    const absolute_time_t end_t = make_timeout_time_ms(JIGSWEEP_WINDOW_MS);

    bool delay_asserted = false;
    bool delay_released = false;
    bool first_seen = false;
    uint32_t first_us = 0u;
    size_t rx_len = 0u;
    uint32_t ferr = 0u;
    uint8_t byte = 0u;

    while (!time_reached(end_t)) {
        const uint32_t rel_us = (uint32_t)(time_us_64() - t0_us);
        if (mode == P7501_JIGSWEEP_HOLD) {
            (void)p7501_pio_poll_sw_uart_release();
        } else {
            if (!delay_asserted && rel_us >= (param_ms * 1000u)) {
                p7501_pio_set_sw_uart(true);
                delay_asserted = true;
            }
            if (delay_asserted && !delay_released && rel_us >= (release_ms * 1000u)) {
                p7501_pio_release_sw_uart_now();
                delay_released = true;
            }
        }
        if (p7501_pio_poll_and_clear_framing_err()) {
            ++ferr;
        }
        while (p7501_pio_pop_rx_byte(&byte)) {
            if (!first_seen) {
                first_seen = true;
                first_us = (uint32_t)(time_us_64() - t0_us);
            }
            if (rx_len < sizeof(rx_buf)) {
                rx_buf[rx_len++] = byte;
            }
        }
        p7501_wdt_kick();
    }

    const bool ovf = p7501_pio_had_rx_overflow();
    p7501_pio_end_session();

    const p7501_jigsweep_outcome_t outcome = p7501_jigsweep_classify_rx(rx_buf, rx_len);
    printf("  [%s=%3" PRIu32 "ms] outcome=%-11s bytes=%4u first=%6" PRIu32 "us ferr=%" PRIu32 " ovf=%s | ",
           (mode == P7501_JIGSWEEP_HOLD) ? "HOLD" : "DELAY",
           param_ms,
           p7501_jigsweep_outcome_name(outcome),
           (unsigned)rx_len,
           first_us,
           ferr,
           ovf ? "YES" : "NO");
    sweep_print_preview(rx_buf, rx_len);
    putchar('\n');
}

static void sweep_run_preboot_trial(uint32_t profile) {
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
    const p7501_uart_framing_t pre_frame =
        (profile == 0u || profile == 1u || profile == 4u) ? P7501_FRAMING_8E1 : P7501_FRAMING_8N1;

    (void)p7501_pio_set_baud(pre_baud);
    (void)p7501_pio_set_framing(pre_frame);

    if (profile == 2u) {
        p7501_pio_begin_rx_session_ex(P7501_BOOT_ACTION_PASSIVE, 0u);
        p7501_pio_set_sw_uart(false);
        gpio_init(P7501_PIN_TXD);
        gpio_put(P7501_PIN_TXD, false);
        gpio_set_dir(P7501_PIN_TXD, GPIO_OUT);
        sleep_ms(2u);
        p7501_pio_pulse_xreset_ms(15u);
    } else {
        const p7501_boot_action_t b_act = (profile == 4u) ? P7501_BOOT_ACTION_JIG : P7501_BOOT_ACTION_NORMAL;
        p7501_pio_begin_rx_session_ex(b_act, (profile == 4u) ? 30u : 0u);
    }

    const uint64_t t0_us = time_us_64();
    uint8_t pre_buf[64];
    size_t pre_len = 0u;
    static uint8_t post_buf[JIGSWEEP_TRIAL_RX_CAP];
    size_t post_len = 0u;
    uint32_t pre_ferr = 0u;
    uint32_t post_ferr = 0u;
    uint32_t bursts = 0u;
    bool switched_post = false;
    bool break_released = (profile != 2u);
    uint8_t byte = 0u;

    const absolute_time_t end_t = make_timeout_time_ms(JIGSWEEP_WINDOW_MS);
    while (!time_reached(end_t)) {
        const uint32_t rel_us = (uint32_t)(time_us_64() - t0_us);
        (void)p7501_pio_poll_sw_uart_release();

        if (!switched_post && rel_us < JIGSWEEP_PRE_DDR_US) {
            if ((profile == 0u || profile == 1u) && (bursts == 0u || (bursts < 3u && rel_us >= bursts * 3000u))) {
                p7501_pio_tx_bytes(k_amobp, sizeof(k_amobp), false);
                ++bursts;
            } else if (profile == 3u && (bursts == 0u || (bursts < 3u && rel_us >= bursts * 3000u))) {
                p7501_pio_tx_bytes(k_ctrl, sizeof(k_ctrl), false);
                ++bursts;
            } else if (profile == 5u && bursts == 0u) {
                p7501_pio_tx_bytes(k_serial, sizeof(k_serial), false);
                ++bursts;
            }
        }

        if (!break_released && rel_us >= 9000u) {
            gpio_set_dir(P7501_PIN_TXD, GPIO_IN);
            gpio_pull_up(P7501_PIN_TXD);
            break_released = true;
        }

        if (!switched_post && rel_us >= JIGSWEEP_PRE_DDR_US) {
            while (p7501_pio_pop_rx_byte(&byte)) {
                if (pre_len < sizeof(pre_buf)) pre_buf[pre_len++] = byte;
            }
            (void)p7501_pio_set_baud(P7501_UART_DEFAULT_BAUD);
            (void)p7501_pio_set_framing(P7501_FRAMING_8N1);
            switched_post = true;
        }

        if (p7501_pio_poll_and_clear_framing_err()) {
            if (!switched_post) ++pre_ferr; else ++post_ferr;
        }
        while (p7501_pio_pop_rx_byte(&byte)) {
            if (!switched_post) {
                if (pre_len < sizeof(pre_buf)) pre_buf[pre_len++] = byte;
            } else {
                if (post_len < sizeof(post_buf)) post_buf[post_len++] = byte;
            }
        }
        p7501_wdt_kick();
    }

    const uint32_t perr = p7501_pio_get_and_clear_parity_errors();
    p7501_pio_end_session();

    const p7501_jigsweep_outcome_t outcome = p7501_jigsweep_classify_rx(post_buf, post_len);
    printf("  [PREBOOT P%" PRIu32 " %-22s] pre_rx=%u(ferr=%" PRIu32 ",perr=%" PRIu32 ") post=%-11s bytes=%u ferr=%" PRIu32 " | ",
           profile % 6u,
           k_names[profile % 6u],
           (unsigned)pre_len,
           pre_ferr,
           perr,
           p7501_jigsweep_outcome_name(outcome),
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
}

void p7501_cmd_jigsweep(const char *arg) {
    p7501_jigsweep_spec_t spec;
    if (!p7501_jigsweep_parse_args(arg, &spec)) {
        printf("[JIGSWEEP] ERROR: Invalid syntax. Use JIGSWEEP [HOLD|DELAY|PREBOOT] [ms_list]\n");
        return;
    }

    const uint32_t saved_baud = p7501_pio_get_baud();
    const p7501_uart_framing_t saved_framing = p7501_pio_get_framing();
    const char *mode_lbl = (spec.mode == P7501_JIGSWEEP_DELAY)
        ? "DELAY"
        : ((spec.mode == P7501_JIGSWEEP_PREBOOT) ? "PREBOOT" : "HOLD");

    printf("=== JIGSWEEP %s (%u trials, %ums window each) ===\n",
           mode_lbl, (unsigned)spec.point_count, (unsigned)JIGSWEEP_WINDOW_MS);
    fflush(stdout);

    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    for (size_t i = 0u; i < spec.point_count; ++i) {
        if (spec.mode == P7501_JIGSWEEP_PREBOOT) {
            sweep_run_preboot_trial(spec.points_ms[i]);
        } else {
            sweep_run_hold_or_delay_trial(spec.mode, spec.points_ms[i]);
        }
        p7501_wdt_sleep_ms(40u);
    }

    (void)p7501_pio_set_baud(saved_baud);
    (void)p7501_pio_set_framing(saved_framing);
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    printf("JIGSWEEP: OK\n");
}
