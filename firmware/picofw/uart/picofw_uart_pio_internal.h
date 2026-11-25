/**
 * @file picofw_uart_pio_internal.h
 * @brief Internal shared PIO1 constants and TX/RX driver snapshot contract.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_UART_PIO_INTERNAL_H
#define PICOFW_UART_PIO_INTERNAL_H

#include "hardware/pio.h"
#include "uart/picofw_uart_pio_engine.h"

#define PICOFW_PIO_UART_INST pio1
#define PICOFW_PIO_RX_SM 0u
#define PICOFW_PIO_TX_SM 1u

typedef struct picofw_pio_tx_snapshot {
    uint32_t baud;
    picofw_uart_framing_t framing;
    uint16_t div_int;
    uint8_t div_frac;
    uint bit_loops;
    uint tx_offset;
} picofw_pio_tx_snapshot_t;

void picofw_pio_internal_prepare_tx(picofw_pio_tx_snapshot_t *out_snap);
void picofw_pio_internal_release_tx_hiz(void);

#endif /* PICOFW_UART_PIO_INTERNAL_H */
