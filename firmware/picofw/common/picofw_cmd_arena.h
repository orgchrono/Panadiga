/**
 * @file picofw_cmd_arena.h
 * @brief Shared 32 KiB mutually-exclusive SRAM workspace union (`g_picofw_cmd_arena`).
 *
 * Unifies non-concurrent command capture buffers across UART capture, HEXCAP,
 * PIOLOGIC, PANARSP, and JIGSWEEP so RP2040/RP2350 saves ~40 KiB of `.bss` while
 * simultaneously expanding per-command capture capacities.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_CMD_ARENA_H
#define PICOFW_CMD_ARENA_H

#include "diag/picofw_piologic_codec.h"
#include "uart/picofw_pana_rsp_report.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_CMD_ARENA_BYTES       32768u
#define PICOFW_ARENA_HEXCAP_CAP      4096u
#define PICOFW_ARENA_PIOLOGIC_EDGES  4096u
#define PICOFW_ARENA_JIGSWEEP_CAP    16384u
#define PICOFW_ARENA_PWROSC_CAP      8192u

enum picofw_arena_owner {
    PICOFW_ARENA_FREE = 0u,
    PICOFW_ARENA_OWNER_FAST_STREAM = 1u,
    PICOFW_ARENA_OWNER_HOTPLUG_MAP = 2u,
    PICOFW_ARENA_OWNER_FLASH = 3u,
    PICOFW_ARENA_OWNER_CLI = 4u,
    PICOFW_ARENA_OWNER_CRYPTO = 5u,
    PICOFW_ARENA_OWNER_DIAG = 6u
};

typedef union picofw_cmd_arena {
    uint64_t align_u64;
    uint8_t raw[PICOFW_CMD_ARENA_BYTES];
    uint8_t uart_rx[PICOFW_CMD_ARENA_BYTES];
    uint16_t adc_samples[PICOFW_ARENA_PWROSC_CAP];
    struct {
        uint8_t bytes[PICOFW_ARENA_HEXCAP_CAP];
        uint32_t gaps_us[PICOFW_ARENA_HEXCAP_CAP];
    } hexcap;
    picofw_logic_edge_t piologic_edges[PICOFW_ARENA_PIOLOGIC_EDGES];
    panarsp_rx_cap_t panarsp;
    uint8_t jigsweep_buf[PICOFW_ARENA_JIGSWEEP_CAP];
} picofw_cmd_arena_t;

_Static_assert(sizeof(picofw_cmd_arena_t) == PICOFW_CMD_ARENA_BYTES, "arena size must equal 32 KiB");
_Static_assert(_Alignof(picofw_cmd_arena_t) >= _Alignof(uint64_t), "arena must stay 8-byte aligned");

extern picofw_cmd_arena_t g_picofw_cmd_arena;

/** Non-reentrant exclusive lease guard for `g_picofw_cmd_arena`. */
bool picofw_arena_try_acquire(uint8_t owner_id);

/** Release exclusive lease only if currently held by `owner_id`. */
void picofw_arena_release(uint8_t owner_id);

/** Return current arena owner id (`PICOFW_ARENA_FREE` when available). */
uint8_t picofw_arena_current_owner(void);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_CMD_ARENA_H */
