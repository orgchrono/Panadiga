/**
 * @file picofw_fast_stub_io.h
 * @brief PIO1 transport, progress telemetry, and `0x9043A000` MN103S fast-stub lifecycle helpers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_FAST_STUB_IO_H
#define PICOFW_FAST_STUB_IO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Emit structured `@PICO_PROG` event-driven progress line with live VDD sample. */
void picofw_fast_emit_prog_push(
    const char *phase,
    uint16_t sa,
    uint32_t done,
    uint32_t total,
    uint32_t crc32
);

/** Read exactly `n` bytes from PIO1 RX within `timeout_ms` while kicking WDT and checking NSW5V. */
bool picofw_fast_pio_read_exact_ms(uint8_t *dst, size_t n, uint32_t timeout_ms);

/** Transmit NUL-terminated ASCII string paced for MN103S SC0 while draining synchronous echo. */
void picofw_fast_pio_tx_paced_ascii(const char *str);

/** Transmit raw binary packet bytes over PIO1 TX with inter-byte pacing and WDT kick. */
void picofw_fast_pio_tx_paced_bin(const uint8_t *buf, size_t len);

/** Probe whether the `0x9043A000` fast-BGO stub is responsive (`'!'` -> `'?'`, `'P'` -> `'P'`). */
bool picofw_fast_stub_ping_open(void);

/** Exit the `0x9043A000` stub (`'Q'`) and wait for the `PANAMON1>` prompt return. */
bool picofw_fast_stub_exit_open(void);

/** Jump from `PANAMON1>` to `0x9043A000` (`J 9043A000`) and verify stub ping response. */
bool picofw_fast_stub_jump_and_ping_open(void);

/** Upload `k_picofw_fast_bgo_stub` to `0x9043A000` if absent and enter stub execution mode. */
bool picofw_fast_stub_upload_and_enter_open(bool *out_uploaded);

/** Read one 256-byte NOR page via the active `0x9043A000` stub with sum8 verification. */
bool picofw_fast_stub_read_page_open(uint32_t page_va, uint8_t *out256, bool *out_blank);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_FAST_STUB_IO_H */
