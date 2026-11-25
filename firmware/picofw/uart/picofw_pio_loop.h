/**
 * @file picofw_pio_loop.h
 * @brief Shared functional-style PIO1 UART capture/monitor runner with deterministic WDT safety.
 *
 * Eliminates duplicated RX/WDT/release/framing loops across `uart/` and `diag/` commands.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PIO_LOOP_H
#define PICOFW_PIO_LOOP_H

#include "uart/picofw_uart_codec.h"

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*picofw_pio_loop_byte_fn)(void *ctx, uint8_t byte, uint32_t rel_us, bool *out_stop);
typedef void (*picofw_pio_loop_tick_fn)(void *ctx, uint32_t rel_us, bool *out_stop);
typedef void (*picofw_pio_loop_sw_rel_fn)(void *ctx, uint32_t rel_us, const char *reason);

typedef struct picofw_pio_loop_cfg {
    uint32_t window_ms;
    picofw_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    bool manage_session;
    bool no_xreset;
    bool auto_release_serial_boot;
    bool allow_cdc_abort;
    picofw_pio_loop_byte_fn on_rx_byte;
    picofw_pio_loop_tick_fn on_tick;
    picofw_pio_loop_sw_rel_fn on_sw_rel;
    void *ctx;
} picofw_pio_loop_cfg_t;

typedef struct picofw_pio_loop_metrics {
    uint32_t rx_bytes;
    uint32_t framing_errors;
    uint32_t parity_errors;
    uint32_t elapsed_ms;
    bool overflow;
    bool aborted;
    bool sw_uart_released;
} picofw_pio_loop_metrics_t;

/**
 * @brief Wait up to `wait_ms` for Standby -> Awake edge (`NSW5V` high or `SW_UART` VDD >= threshold),
 *        optionally holding `SW_UART` LOW in advance (`PREJIG`) without pulsing `XRESET`.
 */
bool picofw_pio_wait_standby_wake(
    const char *tag,
    uint32_t wait_ms,
    bool prestrap_sw_uart_low,
    uint32_t *out_vdd_mv,
    uint32_t *out_elapsed_ms
);

/**
 * @brief Execute a bounded PIO1 RX session loop with automatic WDT kicks, Ctrl-C abort,
 *        JIG SW_UART auto-release, framing/parity telemetry, and deterministic Hi-Z teardown.
 */
void picofw_pio_run_session(
    const picofw_pio_loop_cfg_t *cfg,
    picofw_pio_loop_metrics_t *out_metrics
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PIO_LOOP_H */
