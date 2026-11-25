/**
 * @file picofw_uart_stream.h
 * @brief PIO1 real-time UART streaming (`UARTSTREAM`) and microsecond hex capture (`HEXCAP`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_UART_STREAM_H
#define PICOFW_UART_STREAM_H

#ifdef __cplusplus
extern "C" {
#endif

/** Stream real-time decoded target UART RX bytes on GP2 (`UARTSTREAM [ms] [boot]`). */
void picofw_cmd_uart_stream(const char *arg);

/** Capture microsecond-timestamped hex + ASCII frames on GP2 (`HEXCAP [ms] [boot]`). */
void picofw_cmd_hexcap(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_UART_STREAM_H */
