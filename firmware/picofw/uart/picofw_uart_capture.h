/**
 * @file picofw_uart_capture.h
 * @brief Shared 32 KiB P7501 UART RX capture engine & passive boot monitor (`BOOTCAP`/`UARTMON`/`JIGBOOT`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_UART_CAPTURE_H
#define PICOFW_UART_CAPTURE_H

#include <stddef.h>
#include "picofw_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Access the single shared 16 KB P7501 UART RX capture buffer.
 *
 * @param[out] out_capacity Optional pointer receiving buffer size in bytes.
 * @return Pointer to shared capture buffer.
 */
uint8_t *picofw_uart_rx_buffer(size_t *out_capacity);

/**
 * @brief Capture 115200 8N1 serial traffic into the shared RX buffer for `dur_us` microseconds.
 */
uint32_t picofw_uart_capture_window(uint32_t dur_us, uint32_t max_cap);

/**
 * @brief Execute passive or reset-synchronized 115200 UART capture (`BOOTCAP`/`UARTMON`/`JIGBOOT`).
 */
void picofw_cmd_boot_capture(const char *arg, bool do_reset, bool is_jig, const char *cmd_name);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_UART_CAPTURE_H */
