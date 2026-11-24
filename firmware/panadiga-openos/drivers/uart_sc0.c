/**
 * @file uart_sc0.c
 * @brief AM34 `SC0` UART0 (`115200 8N1` on `P7501 TO JIGU`) driver for Panadiga OpenOS.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

void openos_uart0_init(void) {
    /* Ensure SC0 transmitter/receiver framing stays enabled in 115200 8N1 mode. */
    (void)am34_mmio_write16(AM34_OPENOS_SC0_CTR, 0x0080U);
}

bool openos_uart0_putc(char ch) {
    uint16_t status = 0u;
    uint32_t spin = 0u;
    while (spin < AM34_OPENOS_SC0_SPIN_LIMIT) {
        if (!am34_mmio_read16(AM34_OPENOS_SC0_STR, &status)) {
            return false;
        }
        if ((status & AM34_OPENOS_SC0_TX_BUSY_MASK) == 0u) {
            break;
        }
        spin += 1u;
    }
    return am34_mmio_write8(AM34_OPENOS_SC0_TXB, (uint8_t)ch);
}

void openos_uart0_puts(const char *str) {
    if (str == NULL) {
        return;
    }
    for (const char *p = str; *p != '\0'; ++p) {
        if (*p == '\n') {
            (void)openos_uart0_putc('\r');
        }
        (void)openos_uart0_putc(*p);
    }
}
