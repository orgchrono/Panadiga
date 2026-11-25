/**
 * @file picofw_uart_pio_tx.c
 * @brief PIO1 TX transmission and safe break signaling on P7501 pin 6 / TXD.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_uart_pio_internal.h"

#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "picofw_uart_duplex.pio.h"

#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

void picofw_pio_internal_release_tx_hiz(void) {
    pio_sm_set_enabled(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM, false);
    picofw_hw_release_txd_hiz(true);
}

static void engage_tx_pio(const picofw_pio_tx_snapshot_t *snap) {
    picofw_uart_tx_fast_init(
        PICOFW_PIO_UART_INST,
        PICOFW_PIO_TX_SM,
        snap->tx_offset,
        P7501_PIN_TXD,
        snap->div_int,
        snap->div_frac,
        snap->bit_loops
    );
    pio_sm_set_pins_with_mask(
        PICOFW_PIO_UART_INST,
        PICOFW_PIO_TX_SM,
        (1u << P7501_PIN_TXD),
        (1u << P7501_PIN_TXD)
    );
    pio_sm_set_consecutive_pindirs(
        PICOFW_PIO_UART_INST,
        PICOFW_PIO_TX_SM,
        P7501_PIN_TXD,
        1u,
        true
    );
    pio_gpio_init(PICOFW_PIO_UART_INST, P7501_PIN_TXD);
    pio_sm_set_enabled(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM, true);
}

static void drain_rx_during_tx(picofw_pio_rx_drain_cb_t on_rx_byte, void *rx_ctx) {
    if (on_rx_byte == NULL) {
        return;
    }
    uint8_t rx_byte = 0u;
    while (picofw_pio_pop_rx_byte(&rx_byte)) {
        on_rx_byte(rx_ctx, rx_byte);
    }
}

static bool pio_tx_put_word_bounded_duplex(
    uint32_t word,
    picofw_pio_rx_drain_cb_t on_rx_byte,
    void *rx_ctx
) {
    const uint64_t wait_t0 = time_us_64();
    while (pio_sm_is_tx_fifo_full(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM)) {
        drain_rx_during_tx(on_rx_byte, rx_ctx);
        picofw_wdt_kick();
        if ((time_us_64() - wait_t0) >= PICOFW_TX_FIFO_TIMEOUT_US) {
            return false;
        }
        tight_loop_contents();
    }
    pio_sm_put(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM, word);
    return true;
}

void picofw_pio_tx_bytes_duplex(
    const uint8_t *data,
    size_t len,
    bool force_sw_uart_low,
    bool skip_sw_uart_settle,
    picofw_pio_rx_drain_cb_t on_rx_byte,
    void *rx_ctx
) {
    if (!data || len == 0u) {
        return;
    }
    picofw_pio_tx_snapshot_t snap;
    picofw_pio_internal_prepare_tx(&snap);
    const uint32_t drain_us = ((picofw_baud_to_bit_ns(snap.baud) * 13u) / 1000u) + 60u;

    /* Always enforce Pin 5 NSW5V (GP3) Hi-Z Input before engaging GP4 TX. */
    picofw_hw_enforce_nsw5v_hiz();
    if (!skip_sw_uart_settle) {
        picofw_hw_sw_uart_set_low(force_sw_uart_low);
        sleep_us(200u);
    }
    drain_rx_during_tx(on_rx_byte, rx_ctx);

    engage_tx_pio(&snap);
    for (size_t i = 0u; i < len; ++i) {
        drain_rx_during_tx(on_rx_byte, rx_ctx);
        if (!pio_tx_put_word_bounded_duplex(
                picofw_uart_encode_tx_word(data[i], snap.framing), on_rx_byte, rx_ctx)) {
            picofw_pio_internal_release_tx_hiz();
            drain_rx_during_tx(on_rx_byte, rx_ctx);
            return;
        }
    }
    const uint64_t drain_t0 = time_us_64();
    while (!pio_sm_is_tx_fifo_empty(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM)) {
        drain_rx_during_tx(on_rx_byte, rx_ctx);
        picofw_wdt_kick();
        if ((time_us_64() - drain_t0) >= PICOFW_TX_FIFO_TIMEOUT_US) {
            picofw_pio_internal_release_tx_hiz();
            drain_rx_during_tx(on_rx_byte, rx_ctx);
            return;
        }
        tight_loop_contents();
    }
    if (on_rx_byte != NULL) {
        const uint64_t tail_t0 = time_us_64();
        while ((time_us_64() - tail_t0) < (uint64_t)drain_us) {
            drain_rx_during_tx(on_rx_byte, rx_ctx);
            tight_loop_contents();
        }
    } else {
        sleep_us(drain_us);
    }
    picofw_pio_internal_release_tx_hiz();
    drain_rx_during_tx(on_rx_byte, rx_ctx);
}

void picofw_pio_tx_bytes(const uint8_t *data, size_t len, bool force_sw_uart_low) {
    picofw_pio_tx_bytes_duplex(data, len, force_sw_uart_low, false, NULL, NULL);
}

void picofw_pio_tx_break_ms(uint32_t break_ms, bool force_sw_uart_low) {
    picofw_hw_enforce_nsw5v_hiz();
    picofw_hw_sw_uart_set_low(force_sw_uart_low);
    picofw_pio_internal_release_tx_hiz();

    picofw_hw_drive_txd_low();
    picofw_wdt_sleep_ms(break_ms > 0u ? break_ms : 120u);
    picofw_hw_release_txd_hiz(true);
}
