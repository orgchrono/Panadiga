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

#include <stdbool.h>
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

enum p7501_arena_owner {
    P7501_ARENA_FREE = 0u,
    P7501_ARENA_OWNER_FAST_STREAM = 1u,
    P7501_ARENA_OWNER_HOTPLUG_MAP = 2u,
    P7501_ARENA_OWNER_FLASH = 3u,
    P7501_ARENA_OWNER_CLI = 4u,
    P7501_ARENA_OWNER_CRYPTO = 5u,
    P7501_ARENA_OWNER_DIAG = 6u
};

typedef union p7501_cmd_arena {
    uint64_t align_u64;
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

_Static_assert(sizeof(p7501_cmd_arena_t) == P7501_CMD_ARENA_BYTES, "arena size must equal 32 KiB");
_Static_assert(_Alignof(p7501_cmd_arena_t) >= _Alignof(uint64_t), "arena must stay 8-byte aligned");

extern p7501_cmd_arena_t g_p7501_cmd_arena;

/** Non-reentrant exclusive lease guard for `g_p7501_cmd_arena`. */
bool p7501_arena_try_acquire(uint8_t owner_id);

/** Release exclusive lease only if currently held by `owner_id`. */
void p7501_arena_release(uint8_t owner_id);

/** Return current arena owner id (`P7501_ARENA_FREE` when available). */
uint8_t p7501_arena_current_owner(void);

#ifdef __cplusplus
}
#endif

#endif /* P7501_CMD_ARENA_H */
