/**
 * @file p7501_cmd_arena.h
 * @brief Shared 32 KiB mutually-exclusive SRAM workspace union (`g_p7501_cmd_arena`).
 *
 * Unifies non-concurrent command capture buffers across UART capture, HEXCAP,
 * PIOLOGIC, PANARSP, and JIGSWEEP so RP2040/RP2350 saves ~40 KiB of `.bss` while
 * simultaneously expanding per-command capture capacities.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_CMD_ARENA_H
#define P7501_CMD_ARENA_H

#include "diag/p7501_piologic_codec.h"
#include "uart/p7501_pana_rsp_report.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_CMD_ARENA_BYTES       32768u
#define P7501_ARENA_HEXCAP_CAP      4096u
#define P7501_ARENA_PIOLOGIC_EDGES  4096u
#define P7501_ARENA_JIGSWEEP_CAP    16384u
#define P7501_ARENA_PWROSC_CAP      8192u

typedef union p7501_cmd_arena {
    uint8_t raw[P7501_CMD_ARENA_BYTES];
    uint8_t uart_rx[P7501_CMD_ARENA_BYTES];
    uint16_t adc_samples[P7501_ARENA_PWROSC_CAP];
    struct {
        uint8_t bytes[P7501_ARENA_HEXCAP_CAP];
        uint32_t gaps_us[P7501_ARENA_HEXCAP_CAP];
    } hexcap;
    p7501_logic_edge_t piologic_edges[P7501_ARENA_PIOLOGIC_EDGES];
    panarsp_rx_cap_t panarsp;
    uint8_t jigsweep_buf[P7501_ARENA_JIGSWEEP_CAP];
} p7501_cmd_arena_t;

typedef int p7501_cmd_arena_size_guard_t[
    (sizeof(p7501_cmd_arena_t) == P7501_CMD_ARENA_BYTES) ? 1 : -1
];

extern p7501_cmd_arena_t g_p7501_cmd_arena;

#ifdef __cplusplus
}
#endif

#endif /* P7501_CMD_ARENA_H */
