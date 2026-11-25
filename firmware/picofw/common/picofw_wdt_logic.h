/**
 * @file picofw_wdt_logic.h
 * @brief Pure ISO C11 functional core for non-overridable hardware watchdog scratch encoding and loop clamping.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_WDT_LOGIC_H
#define PICOFW_WDT_LOGIC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Non-overridable RP2040/RP2350 hardware watchdog timeout in milliseconds (4.0 s). */
#define PICOFW_WDT_TIMEOUT_MS         4000u

/** Persistent scratch[1] marker ("P7WD") confirming Panadiga telemetry validity across resets. */
#define PICOFW_WDT_SCRATCH_MAGIC      0x50375744u

/** Maximum bounded capture/sweep window allowed across all CLI commands (185 s for DUMPNOR). */
#define PICOFW_WDT_MAX_CMD_WINDOW_MS  185000u

/** Cooperative sleep slice ceiling (ms) between hardware watchdog kicks. */
#define PICOFW_WDT_SLEEP_SLICE_MS     200u

/** USB CDC Control-C byte used to abort long blocking captures safely without waiting. */
#define PICOFW_CDC_ABORT_CTRL_C       0x03

/** Command slot sentinel indicating idle CLI loop (no active command executing). */
#define PICOFW_WDT_CMD_IDLE           0xFFu

enum {
    PICOFW_WDT_FLAG_IN_CMD    = (1u << 0),
    PICOFW_WDT_FLAG_ABORTED   = (1u << 1),
    PICOFW_WDT_FLAG_RECOVERED = (1u << 2),
};

/** Execution phase markers stored in watchdog scratch[3] for post-recovery forensics. */
enum picofw_wdt_stage {
    PICOFW_WDT_STAGE_IDLE        = 0u,
    PICOFW_WDT_STAGE_INIT        = 1u,
    PICOFW_WDT_STAGE_RESET_SYNC  = 2u,
    PICOFW_WDT_STAGE_RX_WAIT     = 3u,
    PICOFW_WDT_STAGE_TX_WAIT     = 4u,
    PICOFW_WDT_STAGE_PROBE_SWEEP = 5u,
    PICOFW_WDT_STAGE_BRIDGE      = 6u,
    PICOFW_WDT_STAGE_CLEANUP     = 7u,
    PICOFW_WDT_STAGE_GDB_RSP     = 8u,
};

/** Pack `[cmd_id:8 | flags:8 | recoveries:16]` deterministically into a 32-bit scratch word. */
uint32_t picofw_wdt_pack_scratch0(uint8_t cmd_id, uint8_t flags, uint16_t recoveries);

/** Pack `[stage:8 | uptime_ms_24:24]` deterministically into scratch[3]. */
uint32_t picofw_wdt_pack_scratch3(uint8_t stage, uint32_t uptime_ms);

/** Unpack `[stage:8 | uptime_ms_24:24]` from scratch[3] (NULL-safe). */
bool picofw_wdt_unpack_scratch3(uint32_t word, uint8_t *out_stage, uint32_t *out_uptime_ms);

/** Return canonical human-readable stage token for a `picofw_wdt_stage` value. */
const char *picofw_wdt_stage_name(uint8_t stage);

/** Unpack `[cmd_id:8 | flags:8 | recoveries:16]` from a scratch[0] word (NULL-safe). */
bool picofw_wdt_unpack_scratch0(
    uint32_t word,
    uint8_t *out_cmd_id,
    uint8_t *out_flags,
    uint16_t *out_recoveries
);

/** Clamp requested capture/command window into `[min_ms, PICOFW_WDT_MAX_CMD_WINDOW_MS]`. */
uint32_t picofw_wdt_clamp_window_ms(uint32_t requested_ms, uint32_t default_ms, uint32_t min_ms);

/** Pure predicate identifying USB CDC emergency abort character (`0x03` / Ctrl-C). */
bool picofw_wdt_is_abort_char(int ch);

/**
 * @brief Distinguish genuine hardware watchdog timeout from UF2/BOOTSEL or software reboots.
 *
 * `watchdog_caused_reboot()` is also true after ROM UF2 flashes and `reset_usb_boot()`, whereas
 * `watchdog_enable_caused_reboot()` is true only when an armed watchdog timer actually expired.
 */
bool picofw_wdt_is_genuine_timeout_recovery(
    bool wdt_caused_reboot,
    bool wdt_enable_caused_reboot,
    bool valid_magic
);

/** Retained RAM 4 KiB postmortem ring buffer (`WDT POSTMORTEM`, Feature #9). */
#define PICOFW_WDT_PM_MAGIC         0x504D3039UL /* 'PM09' */
#define PICOFW_WDT_PM_ENTRY_CAP     32u

typedef struct picofw_wdt_pm_entry {
    uint32_t seq;
    uint32_t uptime_ms;
    uint32_t elapsed_us;
    uint8_t cmd_id;
    uint8_t stage;
    uint8_t flags;
    uint8_t reserved;
} picofw_wdt_pm_entry_t;

typedef struct picofw_wdt_postmortem_ring {
    uint32_t magic;
    uint32_t write_idx;
    uint32_t total_events;
    picofw_wdt_pm_entry_t entries[PICOFW_WDT_PM_ENTRY_CAP];
    uint8_t reserved_pad[4096u - 12u - (PICOFW_WDT_PM_ENTRY_CAP * sizeof(picofw_wdt_pm_entry_t)) - 4u];
    uint32_t crc32;
} picofw_wdt_postmortem_ring_t;

void picofw_wdt_pm_init_or_recover(picofw_wdt_postmortem_ring_t *ring);
void picofw_wdt_pm_record(
    picofw_wdt_postmortem_ring_t *ring,
    uint32_t seq,
    uint32_t uptime_ms,
    uint32_t elapsed_us,
    uint8_t cmd_id,
    uint8_t stage,
    uint8_t flags
);
size_t picofw_wdt_pm_count(const picofw_wdt_postmortem_ring_t *ring);
bool picofw_wdt_pm_get_recent(
    const picofw_wdt_postmortem_ring_t *ring,
    size_t back_idx,
    picofw_wdt_pm_entry_t *out
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_WDT_LOGIC_H */
