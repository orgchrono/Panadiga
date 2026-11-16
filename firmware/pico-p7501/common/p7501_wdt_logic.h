/**
 * @file p7501_wdt_logic.h
 * @brief Pure ISO C11 functional core for non-overridable hardware watchdog scratch encoding and loop clamping.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_WDT_LOGIC_H
#define P7501_WDT_LOGIC_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Non-overridable RP2040/RP2350 hardware watchdog timeout in milliseconds (4.0 s). */
#define P7501_WDT_TIMEOUT_MS         4000u

/** Persistent scratch[1] marker ("P7WD") confirming Panadiga telemetry validity across resets. */
#define P7501_WDT_SCRATCH_MAGIC      0x50375744u

/** Maximum bounded capture/sweep window allowed across all CLI commands (185 s for DUMPNOR). */
#define P7501_WDT_MAX_CMD_WINDOW_MS  185000u

/** Cooperative sleep slice ceiling (ms) between hardware watchdog kicks. */
#define P7501_WDT_SLEEP_SLICE_MS     200u

/** USB CDC Control-C byte used to abort long blocking captures safely without waiting. */
#define P7501_CDC_ABORT_CTRL_C       0x03

/** Command slot sentinel indicating idle CLI loop (no active command executing). */
#define P7501_WDT_CMD_IDLE           0xFFu

enum {
    P7501_WDT_FLAG_IN_CMD    = (1u << 0),
    P7501_WDT_FLAG_ABORTED   = (1u << 1),
    P7501_WDT_FLAG_RECOVERED = (1u << 2),
};

/** Execution phase markers stored in watchdog scratch[3] for post-recovery forensics. */
enum p7501_wdt_stage {
    P7501_WDT_STAGE_IDLE        = 0u,
    P7501_WDT_STAGE_INIT        = 1u,
    P7501_WDT_STAGE_RESET_SYNC  = 2u,
    P7501_WDT_STAGE_RX_WAIT     = 3u,
    P7501_WDT_STAGE_TX_WAIT     = 4u,
    P7501_WDT_STAGE_PROBE_SWEEP = 5u,
    P7501_WDT_STAGE_BRIDGE      = 6u,
    P7501_WDT_STAGE_CLEANUP     = 7u,
};

/** Pack `[cmd_id:8 | flags:8 | recoveries:16]` deterministically into a 32-bit scratch word. */
uint32_t p7501_wdt_pack_scratch0(uint8_t cmd_id, uint8_t flags, uint16_t recoveries);

/** Pack `[stage:8 | uptime_ms_24:24]` deterministically into scratch[3]. */
uint32_t p7501_wdt_pack_scratch3(uint8_t stage, uint32_t uptime_ms);

/** Unpack `[stage:8 | uptime_ms_24:24]` from scratch[3] (NULL-safe). */
bool p7501_wdt_unpack_scratch3(uint32_t word, uint8_t *out_stage, uint32_t *out_uptime_ms);

/** Return canonical human-readable stage token for a `p7501_wdt_stage` value. */
const char *p7501_wdt_stage_name(uint8_t stage);

/** Unpack `[cmd_id:8 | flags:8 | recoveries:16]` from a scratch[0] word (NULL-safe). */
bool p7501_wdt_unpack_scratch0(
    uint32_t word,
    uint8_t *out_cmd_id,
    uint8_t *out_flags,
    uint16_t *out_recoveries
);

/** Clamp requested capture/command window into `[min_ms, P7501_WDT_MAX_CMD_WINDOW_MS]`. */
uint32_t p7501_wdt_clamp_window_ms(uint32_t requested_ms, uint32_t default_ms, uint32_t min_ms);

/** Pure predicate identifying USB CDC emergency abort character (`0x03` / Ctrl-C). */
bool p7501_wdt_is_abort_char(int ch);

/**
 * @brief Distinguish genuine hardware watchdog timeout from UF2/BOOTSEL or software reboots.
 *
 * `watchdog_caused_reboot()` is also true after ROM UF2 flashes and `reset_usb_boot()`, whereas
 * `watchdog_enable_caused_reboot()` is true only when an armed watchdog timer actually expired.
 */
bool p7501_wdt_is_genuine_timeout_recovery(
    bool wdt_caused_reboot,
    bool wdt_enable_caused_reboot,
    bool valid_magic
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_WDT_LOGIC_H */
