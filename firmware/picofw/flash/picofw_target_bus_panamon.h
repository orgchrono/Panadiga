/**
 * @file picofw_target_bus_panamon.h
 * @brief `PANAMON1>` UART0 fallback transport helpers for the Pico PIO1 target-bus VTable.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_TARGET_BUS_PANAMON_H
#define PICOFW_TARGET_BUS_PANAMON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_PIO_BUS_BLOCK_CHUNK_MAX 64u

bool picofw_bus_panamon1_has_prompt(const uint8_t *buf, size_t len);
void picofw_bus_panamon1_drain_tail(uint32_t quiet_ms);
bool picofw_bus_panamon1_probe_open(bool allow_abort, bool *io_panamon_mode);
bool picofw_bus_panamon1_cmd_open(
    const char *cmd,
    uint32_t timeout_ms,
    bool allow_abort,
    char *out_buf,
    size_t out_cap,
    bool *io_panamon_mode
);
bool picofw_bus_panamon1_cmd_raw_open(
    const char *cmd,
    uint32_t timeout_ms,
    bool allow_abort,
    uint8_t *out_buf,
    size_t out_cap,
    size_t *out_len,
    bool *io_panamon_mode
);
bool picofw_bus_panamon1_write_buf_open(
    uint32_t addr,
    const uint8_t *buf,
    size_t len,
    bool allow_abort,
    bool *io_panamon_mode
);
bool picofw_bus_panamon1_buf_matches_open(
    uint32_t addr,
    const uint8_t *buf,
    size_t len,
    bool allow_abort,
    bool *io_panamon_mode
);
bool picofw_bus_panamon1_read_chunk_open(
    uint32_t addr,
    uint8_t *out_buf,
    uint32_t chunk,
    bool allow_abort,
    bool *io_panamon_mode
);
bool picofw_bus_panamon1_write_u32(uint32_t addr, uint32_t data, bool *io_panamon_mode);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_TARGET_BUS_PANAMON_H */
