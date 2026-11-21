/**
 * @file p7501_pio_loop.h
 * @brief Shared functional-style PIO1 UART capture/monitor runner with deterministic WDT safety.
 *
 * Eliminates duplicated RX/WDT/release/framing loops across `uart/` and `diag/` commands.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PIO_LOOP_H
#define P7501_PIO_LOOP_H

#include "uart/p7501_uart_codec.h"

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*p7501_pio_loop_byte_fn)(void *ctx, uint8_t byte, uint32_t rel_us, bool *out_stop);
typedef void (*p7501_pio_loop_tick_fn)(void *ctx, uint32_t rel_us, bool *out_stop);
typedef void (*p7501_pio_loop_sw_rel_fn)(void *ctx, uint32_t rel_us, const char *reason);

typedef struct p7501_pio_loop_cfg {
    uint32_t window_ms;
    p7501_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    bool manage_session;
    bool no_xreset;
    bool auto_release_serial_boot;
    bool allow_cdc_abort;
    p7501_pio_loop_byte_fn on_rx_byte;
    p7501_pio_loop_tick_fn on_tick;
    p7501_pio_loop_sw_rel_fn on_sw_rel;
    void *ctx;
} p7501_pio_loop_cfg_t;

typedef struct p7501_pio_loop_metrics {
    uint32_t rx_bytes;
    uint32_t framing_errors;
    uint32_t parity_errors;
    uint32_t elapsed_ms;
    bool overflow;
    bool aborted;
    bool sw_uart_released;
} p7501_pio_loop_metrics_t;

/**
 * @brief Wait up to `wait_ms` for Standby -> Awake edge (`NSW5V` high or `SW_UART` VDD >= threshold),
 *        optionally holding `SW_UART` LOW in advance (`PREJIG`) without pulsing `XRESET`.
 */
bool p7501_pio_wait_standby_wake(
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
void p7501_pio_run_session(
    const p7501_pio_loop_cfg_t *cfg,
    p7501_pio_loop_metrics_t *out_metrics
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PIO_LOOP_H */
