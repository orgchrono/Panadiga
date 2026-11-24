/**
 * @file p7501_uart_pio_engine.h
 * @brief Multi-baud PIO1 RX/TX engine, Safe XRESET/SW_UART session control, and GP2 timing.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_UART_PIO_ENGINE_H
#define P7501_UART_PIO_ENGINE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include "uart/p7501_uart_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

uint32_t p7501_pio_get_baud(void);
bool p7501_pio_set_baud(uint32_t baud);
p7501_uart_framing_t p7501_pio_get_framing(void);
bool p7501_pio_set_framing(p7501_uart_framing_t framing);

void p7501_pio_enforce_nsw5v_hiz(void);
void p7501_pio_set_sw_uart(bool drive_low);
void p7501_pio_pulse_xreset_ms(uint32_t hold_low_ms);

void p7501_pio_begin_rx_session(p7501_boot_action_t action);
void p7501_pio_begin_rx_session_ex(p7501_boot_action_t action, uint32_t jig_hold_ms);
void p7501_pio_begin_rx_session_cfg(
    p7501_boot_action_t action,
    uint32_t jig_hold_ms,
    bool no_xreset
);
void p7501_pio_release_sw_uart_now(void);
bool p7501_pio_poll_sw_uart_release(void);
bool p7501_pio_poll_jig_serial_release(
    uint8_t *ring,
    size_t *ring_len,
    size_t ring_cap,
    uint8_t rx_byte
);
void p7501_pio_end_session(void);

bool p7501_pio_pop_rx_byte(uint8_t *out_byte);
bool p7501_pio_pop_rx_byte_ex(uint8_t *out_byte, bool *out_parity_err);
size_t p7501_pio_pop_rx_bounded(uint8_t *buf, size_t cap, size_t cur_len, bool slide_on_full);
uint32_t p7501_pio_get_and_clear_parity_errors(void);
void p7501_pio_drain_rx(void);
bool p7501_pio_poll_and_clear_framing_err(void);
bool p7501_pio_had_rx_overflow(void);
bool p7501_pio_is_rx_dma_active(void);

typedef void (*p7501_pio_rx_drain_cb_t)(void *ctx, uint8_t rx_byte);

void p7501_pio_tx_bytes(const uint8_t *data, size_t len, bool force_sw_uart_low);
void p7501_pio_tx_bytes_duplex(
    const uint8_t *data,
    size_t len,
    bool force_sw_uart_low,
    bool skip_sw_uart_settle,
    p7501_pio_rx_drain_cb_t on_rx_byte,
    void *rx_ctx
);
void p7501_pio_tx_break_ms(uint32_t break_ms, bool force_sw_uart_low);

#ifdef __cplusplus
}
#endif

#endif /* P7501_UART_PIO_ENGINE_H */
