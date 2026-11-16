/**
 * @file p7501_uart_pio_tx.c
 * @brief PIO1 TX transmission and safe break signaling on P7501 pin 6 / TXD.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_pio_internal.h"

#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "p7501_uart_duplex.pio.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"

void p7501_pio_internal_release_tx_hiz(void) {
    pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_TX_SM, false);
    p7501_hw_release_txd_hiz(true);
}

static void engage_tx_pio(const p7501_pio_tx_snapshot_t *snap) {
    p7501_uart_tx_fast_init(
        P7501_PIO_UART_INST,
        P7501_PIO_TX_SM,
        snap->tx_offset,
        P7501_PIN_TXD,
        snap->clkdiv,
        snap->bit_loops
    );
    pio_sm_set_pins_with_mask(
        P7501_PIO_UART_INST,
        P7501_PIO_TX_SM,
        (1u << P7501_PIN_TXD),
        (1u << P7501_PIN_TXD)
    );
    pio_sm_set_consecutive_pindirs(
        P7501_PIO_UART_INST,
        P7501_PIO_TX_SM,
        P7501_PIN_TXD,
        1u,
        true
    );
    pio_gpio_init(P7501_PIO_UART_INST, P7501_PIN_TXD);
    pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_TX_SM, true);
}

static bool pio_tx_put_word_bounded(uint32_t word) {
    const uint64_t wait_t0 = time_us_64();
    while (pio_sm_is_tx_fifo_full(P7501_PIO_UART_INST, P7501_PIO_TX_SM)) {
        p7501_wdt_kick();
        if ((time_us_64() - wait_t0) >= P7501_TX_FIFO_TIMEOUT_US) {
            return false;
        }
        tight_loop_contents();
    }
    pio_sm_put(P7501_PIO_UART_INST, P7501_PIO_TX_SM, word);
    return true;
}

void p7501_pio_tx_bytes(const uint8_t *data, size_t len, bool force_sw_uart_low) {
    if (!data || len == 0u) {
        return;
    }
    p7501_pio_tx_snapshot_t snap;
    p7501_pio_internal_prepare_tx(&snap);
    const uint32_t drain_us = ((p7501_baud_to_bit_ns(snap.baud) * 13u) / 1000u) + 60u;

    p7501_hw_enforce_nsw5v_hiz();
    p7501_hw_sw_uart_set_low(force_sw_uart_low);
    sleep_us(200u);

    engage_tx_pio(&snap);
    for (size_t i = 0u; i < len; ++i) {
        if (!pio_tx_put_word_bounded(p7501_uart_encode_tx_word(data[i], snap.framing))) {
            p7501_pio_internal_release_tx_hiz();
            return;
        }
    }
    const uint64_t drain_t0 = time_us_64();
    while (!pio_sm_is_tx_fifo_empty(P7501_PIO_UART_INST, P7501_PIO_TX_SM)) {
        p7501_wdt_kick();
        if ((time_us_64() - drain_t0) >= P7501_TX_FIFO_TIMEOUT_US) {
            p7501_pio_internal_release_tx_hiz();
            return;
        }
        tight_loop_contents();
    }
    sleep_us(drain_us);
    p7501_pio_internal_release_tx_hiz();
}

void p7501_pio_tx_break_ms(uint32_t break_ms, bool force_sw_uart_low) {
    p7501_hw_enforce_nsw5v_hiz();
    p7501_hw_sw_uart_set_low(force_sw_uart_low);
    p7501_pio_internal_release_tx_hiz();

    p7501_hw_drive_txd_low();
    p7501_wdt_sleep_ms(break_ms > 0u ? break_ms : 120u);
    p7501_hw_release_txd_hiz(true);
}
