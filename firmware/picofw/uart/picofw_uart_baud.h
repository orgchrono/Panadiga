/**
 * @file picofw_uart_baud.h
 * @brief PIO1 multi-baud runtime configuration and edge-width autobaud scanner (`BAUD`, `BAUDSCAN`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_UART_BAUD_H
#define PICOFW_UART_BAUD_H

#ifdef __cplusplus
extern "C" {
#endif

/** Query or set PIO1 RX/TX baudrate (`BAUD [bps]`). */
void picofw_cmd_baud(const char *arg);

/** Query or set PIO1 RX/TX parity framing (`FRAMING [8N1|8E1|8O1]`). */
void picofw_cmd_framing(const char *arg);

/** Measure GP2 pulse widths during boot/window and snap to standard baudrate (`BAUDSCAN [ms] [boot] [APPLY]`). */
void picofw_cmd_baudscan(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_UART_BAUD_H */
