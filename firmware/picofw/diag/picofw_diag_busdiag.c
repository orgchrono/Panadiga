/**
 * @file picofw_diag_busdiag.c
 * @brief P7501 TX drive-strength contention & boot transition profiler (`BUSDIAG`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_diag_electrical.h"
#include "cli/picofw_cli.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "pico/stdlib.h"
#include "hardware/gpio.h"
#include <stdio.h>
#include <inttypes.h>

static void configure_busdiag_hiz_and_sw_uart(bool sw_uart_low) {
    picofw_hw_isolate_bus_hiz();
    picofw_hw_sw_uart_set_low(sw_uart_low);
}

void picofw_diag_cmd_busdiag(void) {
    printf("\n=== P7501 ELECTRICAL CONTENTION & DUAL-CHANNEL DIAGNOSTIC (BUSDIAG) ===\n");
    static const enum gpio_drive_strength ds_list[4] = {
        GPIO_DRIVE_STRENGTH_2MA,
        GPIO_DRIVE_STRENGTH_4MA,
        GPIO_DRIVE_STRENGTH_8MA,
        GPIO_DRIVE_STRENGTH_12MA
    };
    static const char *ds_names[4] = { "2mA", "4mA", "8mA", "12mA" };

    bool aborted = false;
    for (uint32_t st = 0U; st < 3U && !aborted; ++st) {
        if (picofw_wdt_poll_abort()) {
            aborted = true;
            break;
        }
        configure_busdiag_hiz_and_sw_uart(st != 0U);
        if (st == 2U) {
            picofw_hw_reset_assert();
        } else {
            picofw_hw_reset_release();
        }
        picofw_wdt_sleep_ms(PICOFW_RESET_POST_SETTLE_MS);

        printf("State %" PRIu32 " (%s):\n", st,
               (st == 0U) ? "SW_UART=HiZ, XRESET=HiZ" :
               (st == 1U) ? "SW_UART=LOW, XRESET=HiZ" : "SW_UART=LOW, XRESET=LOW");

        for (uint32_t d = 0U; d < 4U; ++d) {
            if (picofw_wdt_poll_abort()) {
                aborted = true;
                break;
            }
            gpio_init(P7501_PIN_TXD);
            gpio_disable_pulls(P7501_PIN_TXD);
            gpio_set_drive_strength(P7501_PIN_TXD, ds_list[d]);
            gpio_put(P7501_PIN_TXD, false);
            gpio_set_dir(P7501_PIN_TXD, GPIO_OUT);
            sleep_us(25U);
            const int pad_while_low = gpio_get(P7501_PIN_TXD) ? 1 : 0;

            uint32_t ticks = 0U;
            gpio_set_dir(P7501_PIN_TXD, GPIO_IN);
            while (!gpio_get(P7501_PIN_TXD) && (ticks < 50000U)) {
                ticks++;
            }
            printf("  GP4 (MAX RXD) @ %-4s -> pad_low=%d, release_rise_ticks=%" PRIu32 "\n",
                   ds_names[d], pad_while_low, ticks);
            sleep_us(100U);
        }
    }

    /* Always restore nominal 4mA drive strength on GP4 before leaving contention sweep. */
    gpio_set_drive_strength(P7501_PIN_TXD, GPIO_DRIVE_STRENGTH_4MA);

    for (uint32_t mode = 0U; mode < 2U && !aborted; ++mode) {
        if (picofw_wdt_poll_abort()) {
            aborted = true;
            break;
        }
        configure_busdiag_hiz_and_sw_uart(mode != 0U);
        picofw_wdt_sleep_ms(2U);
        picofw_hw_pulse_xreset_ms(PICOFW_RESET_DIAG_ASSERT_MS);

        picofw_bus_transitions_t phase_a;
        picofw_bus_transitions_t phase_b;
        picofw_diag_sample_transitions(1300U, &phase_a, NULL);
        if (picofw_wdt_poll_abort()) {
            aborted = true;
            break;
        }
        picofw_diag_sample_transitions(2200U, &phase_b, NULL);

        printf("Boot Transitions (%s):\n", (mode == 0U) ? "NORMAL SW_UART=HiZ" : "JIG SW_UART=LOW");
        printf("  Phase A (0..1300ms)   : GP2=%" PRIu32 " GP3=%" PRIu32 " GP4=%" PRIu32 " GP5=%" PRIu32 "\n",
               phase_a.rxd, phase_a.nsw5v, phase_a.txd, phase_a.xreset);
        printf("  Phase B (1300..3500ms): GP2=%" PRIu32 " GP3=%" PRIu32 " GP4=%" PRIu32 " GP5=%" PRIu32 "\n",
               phase_b.rxd, phase_b.nsw5v, phase_b.txd, phase_b.xreset);
    }

    picofw_hw_isolate_bus_hiz();
    if (aborted) {
        picofw_cli_set_result("ERR", "EABORT");
    }
    printf("BUSDIAG: OK%s\n", aborted ? " (aborted=YES)" : "");
}
