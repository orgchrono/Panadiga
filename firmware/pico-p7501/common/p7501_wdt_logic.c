/**
 * @file p7501_wdt_logic.c
 * @brief Pure ISO C11 watchdog scratch word encoding, window clamping, and CDC abort helpers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_wdt_logic.h"
#include "common/p7501_hex.h"

#include <stddef.h>
#include <string.h>

uint32_t p7501_wdt_pack_scratch0(uint8_t cmd_id, uint8_t flags, uint16_t recoveries) {
    return (((uint32_t)cmd_id) << 24u) |
           (((uint32_t)flags) << 16u) |
           ((uint32_t)recoveries);
}

uint32_t p7501_wdt_pack_scratch3(uint8_t stage, uint32_t uptime_ms) {
    return (((uint32_t)stage) << 24u) | (uptime_ms & 0x00FFFFFFu);
}

bool p7501_wdt_unpack_scratch3(uint32_t word, uint8_t *out_stage, uint32_t *out_uptime_ms) {
    if ((out_stage == NULL) || (out_uptime_ms == NULL)) {
        return false;
    }
    *out_stage = (uint8_t)((word >> 24u) & 0xFFu);
    *out_uptime_ms = word & 0x00FFFFFFu;
    return true;
}

const char *p7501_wdt_stage_name(uint8_t stage) {
    switch (stage) {
        case P7501_WDT_STAGE_IDLE:        return "IDLE";
        case P7501_WDT_STAGE_INIT:        return "INIT";
        case P7501_WDT_STAGE_RESET_SYNC:  return "RESET_SYNC";
        case P7501_WDT_STAGE_RX_WAIT:     return "RX_WAIT";
        case P7501_WDT_STAGE_TX_WAIT:     return "TX_WAIT";
        case P7501_WDT_STAGE_PROBE_SWEEP: return "PROBE_SWEEP";
        case P7501_WDT_STAGE_BRIDGE:      return "BRIDGE";
        case P7501_WDT_STAGE_CLEANUP:     return "CLEANUP";
        case P7501_WDT_STAGE_GDB_RSP:     return "GDB_RSP";
        default:                          return "UNKNOWN";
    }
}

bool p7501_wdt_unpack_scratch0(
    uint32_t word,
    uint8_t *out_cmd_id,
    uint8_t *out_flags,
    uint16_t *out_recoveries
) {
    if ((out_cmd_id == NULL) || (out_flags == NULL) || (out_recoveries == NULL)) {
        return false;
    }
    *out_cmd_id = (uint8_t)((word >> 24u) & 0xFFu);
    *out_flags = (uint8_t)((word >> 16u) & 0xFFu);
    *out_recoveries = (uint16_t)(word & 0xFFFFu);
    return true;
}

uint32_t p7501_wdt_clamp_window_ms(uint32_t requested_ms, uint32_t default_ms, uint32_t min_ms) {
    uint32_t eff_min = (min_ms == 0u) ? 1u : min_ms;
    if (eff_min > P7501_WDT_MAX_CMD_WINDOW_MS) {
        eff_min = P7501_WDT_MAX_CMD_WINDOW_MS;
    }
    uint32_t val = (requested_ms == 0u) ? default_ms : requested_ms;
    if (val < eff_min) {
        return eff_min;
    }
    if (val > P7501_WDT_MAX_CMD_WINDOW_MS) {
        return P7501_WDT_MAX_CMD_WINDOW_MS;
    }
    return val;
}

bool p7501_wdt_is_abort_char(int ch) {
    return ch == P7501_CDC_ABORT_CTRL_C;
}

bool p7501_wdt_is_genuine_timeout_recovery(
    bool wdt_caused_reboot,
    bool wdt_enable_caused_reboot,
    bool valid_magic
) {
    return wdt_caused_reboot && wdt_enable_caused_reboot && valid_magic;
}

_Static_assert(sizeof(p7501_wdt_postmortem_ring_t) == 4096u, "Postmortem ring must be exactly 4 KiB");

static uint32_t wdt_pm_crc(const p7501_wdt_postmortem_ring_t *ring) {
    return p7501_crc32_ieee((const uint8_t *)ring, offsetof(p7501_wdt_postmortem_ring_t, crc32));
}

void p7501_wdt_pm_init_or_recover(p7501_wdt_postmortem_ring_t *ring) {
    if (ring == NULL) {
        return;
    }
    if ((ring->magic == P7501_WDT_PM_MAGIC) && (ring->crc32 == wdt_pm_crc(ring)) &&
        (ring->write_idx < P7501_WDT_PM_ENTRY_CAP)) {
        return;
    }
    memset(ring, 0, sizeof(*ring));
    ring->magic = P7501_WDT_PM_MAGIC;
    ring->crc32 = wdt_pm_crc(ring);
}

void p7501_wdt_pm_record(
    p7501_wdt_postmortem_ring_t *ring,
    uint32_t seq,
    uint32_t uptime_ms,
    uint32_t elapsed_us,
    uint8_t cmd_id,
    uint8_t stage,
    uint8_t flags
) {
    if (ring == NULL) {
        return;
    }
    p7501_wdt_pm_init_or_recover(ring);
    const uint32_t slot = ring->write_idx % P7501_WDT_PM_ENTRY_CAP;
    p7501_wdt_pm_entry_t *e = &ring->entries[slot];
    e->seq = seq;
    e->uptime_ms = uptime_ms;
    e->elapsed_us = elapsed_us;
    e->cmd_id = cmd_id;
    e->stage = stage;
    e->flags = flags;
    e->reserved = 0u;
    ring->write_idx = (slot + 1u) % P7501_WDT_PM_ENTRY_CAP;
    ring->total_events += 1u;
    ring->crc32 = wdt_pm_crc(ring);
}

size_t p7501_wdt_pm_count(const p7501_wdt_postmortem_ring_t *ring) {
    if ((ring == NULL) || (ring->magic != P7501_WDT_PM_MAGIC) || (ring->crc32 != wdt_pm_crc(ring))) {
        return 0u;
    }
    return (ring->total_events < P7501_WDT_PM_ENTRY_CAP)
        ? (size_t)ring->total_events
        : P7501_WDT_PM_ENTRY_CAP;
}

bool p7501_wdt_pm_get_recent(
    const p7501_wdt_postmortem_ring_t *ring,
    size_t back_idx,
    p7501_wdt_pm_entry_t *out
) {
    const size_t cnt = p7501_wdt_pm_count(ring);
    if ((out == NULL) || (back_idx >= cnt)) {
        return false;
    }
    const size_t newest = (ring->write_idx == 0u) ? (P7501_WDT_PM_ENTRY_CAP - 1u) : (ring->write_idx - 1u);
    const size_t idx = (newest + P7501_WDT_PM_ENTRY_CAP - (back_idx % P7501_WDT_PM_ENTRY_CAP)) % P7501_WDT_PM_ENTRY_CAP;
    *out = ring->entries[idx];
    return true;
}
