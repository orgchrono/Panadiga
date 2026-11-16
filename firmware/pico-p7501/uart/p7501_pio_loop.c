/**
 * @file p7501_pio_loop.c
 * @brief Shared functional PIO1 RX/WDT/SW_UART session runner implementation.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pio_loop.h"

#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_uart_pio_engine.h"

#include "pico/stdlib.h"
#include "pico/time.h"

#include <string.h>

void p7501_pio_run_session(
    const p7501_pio_loop_cfg_t *cfg,
    p7501_pio_loop_metrics_t *out_metrics
) {
    p7501_pio_loop_metrics_t local;
    memset(&local, 0, sizeof(local));
    if (cfg == NULL) {
        if (out_metrics != NULL) {
            *out_metrics = local;
        }
        return;
    }

    if (cfg->manage_session) {
        p7501_wdt_set_stage(P7501_WDT_STAGE_RESET_SYNC);
        p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
        p7501_pio_begin_rx_session_ex(cfg->boot_action, cfg->jig_hold_ms);
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_RX_WAIT);
    const uint64_t t0_us = time_us_64();
    const uint64_t deadline_us = t0_us + ((uint64_t)cfg->window_ms * 1000ULL);
    uint8_t jig_ring[24];
    size_t jig_ring_len = 0u;
    bool stop = false;

    while (!stop && p7501_wdt_loop_step(t0_us, deadline_us, cfg->allow_cdc_abort, &local.aborted)) {
        const uint32_t rel_us = (uint32_t)(time_us_64() - t0_us);

        if (p7501_pio_poll_sw_uart_release()) {
            local.sw_uart_released = true;
            if (cfg->on_sw_rel != NULL) {
                cfg->on_sw_rel(cfg->ctx, rel_us, "TIMER");
            }
        }
        if (p7501_pio_poll_and_clear_framing_err()) {
            local.framing_errors += 1u;
        }

        uint8_t byte = 0u;
        while (p7501_pio_pop_rx_byte(&byte)) {
            const uint32_t b_rel_us = (uint32_t)(time_us_64() - t0_us);
            local.rx_bytes += 1u;

            if (cfg->auto_release_serial_boot &&
                (cfg->boot_action == P7501_BOOT_ACTION_JIG) &&
                !local.sw_uart_released &&
                p7501_pio_poll_jig_serial_release(jig_ring, &jig_ring_len, sizeof(jig_ring), byte)) {
                local.sw_uart_released = true;
                if (cfg->on_sw_rel != NULL) {
                    cfg->on_sw_rel(cfg->ctx, b_rel_us, "SERIAL_BOOT");
                }
            }

            if (cfg->on_rx_byte != NULL) {
                cfg->on_rx_byte(cfg->ctx, byte, b_rel_us, &stop);
                if (stop) {
                    break;
                }
            }
        }

        if (!stop && (cfg->on_tick != NULL)) {
            cfg->on_tick(cfg->ctx, rel_us, &stop);
        }
    }

    local.elapsed_ms = (uint32_t)((time_us_64() - t0_us) / 1000ULL);
    local.parity_errors = p7501_pio_get_and_clear_parity_errors();
    local.overflow = p7501_pio_had_rx_overflow();

    if (cfg->manage_session) {
        p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
        p7501_pio_end_session();
        p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    }

    if (out_metrics != NULL) {
        *out_metrics = local;
    }
}
