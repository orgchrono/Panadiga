/**
 * @file p7501_uart_pio_internal.h
 * @brief Internal shared PIO1 constants and TX/RX driver snapshot contract.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_UART_PIO_INTERNAL_H
#define P7501_UART_PIO_INTERNAL_H

#include "hardware/pio.h"
#include "uart/p7501_uart_pio_engine.h"

#define P7501_PIO_UART_INST pio1
#define P7501_PIO_RX_SM 0u
#define P7501_PIO_TX_SM 1u

typedef struct p7501_pio_tx_snapshot {
    uint32_t baud;
    p7501_uart_framing_t framing;
    uint16_t div_int;
    uint8_t div_frac;
    uint bit_loops;
    uint tx_offset;
} p7501_pio_tx_snapshot_t;

void p7501_pio_internal_prepare_tx(p7501_pio_tx_snapshot_t *out_snap);
void p7501_pio_internal_release_tx_hiz(void);

#endif /* P7501_UART_PIO_INTERNAL_H */
