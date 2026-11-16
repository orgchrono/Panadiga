/**
 * @file p7501_watchdog.h
 * @brief Non-overridable RP2040/RP2350 hardware watchdog HAL, loop guard, and command traceability.
 *
 * Guarantees:
 *   - Hardware watchdog (`P7501_WDT_TIMEOUT_MS` = 4000 ms) is enabled unconditionally on boot.
 *   - No CLI command can disable or extend the watchdog timeout.
 *   - On watchdog auto-recovery reboot, all P7501 pins are isolated Hi-Z immediately.
 *   - Scratch registers persist `(last_cmd_id, flags, recovery_count, seq)` across resets.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_WATCHDOG_H
#define P7501_WATCHDOG_H

#include "common/p7501_wdt_logic.h"

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct p7501_wdt_telemetry {
    bool caused_reboot;
    uint16_t recoveries;
    uint8_t last_fault_cmd_id;
    uint8_t last_flags;
    uint32_t cmd_seq;
    const char *last_cmd_name;
    uint64_t last_cmd_elapsed_us;
} p7501_wdt_telemetry_t;

/** Initialize hardware watchdog (`4000 ms`, non-overridable) and recover persistent scratch telemetry. */
void p7501_wdt_init(void);

/** Kick the hardware watchdog, enforce GP3 NSW5V Hi-Z safety, and advance LED heartbeat. */
void p7501_wdt_kick(void);

/** Sleep for `total_ms` in `<= 200 ms` slices while kicking hardware watchdog on each slice. */
void p7501_wdt_sleep_ms(uint32_t total_ms);

/** Record active command entry in watchdog scratch registers for post-crash traceability. */
void p7501_wdt_begin_cmd(uint8_t cmd_id, const char *cmd_name);

/** Record clean command completion and compute command execution duration in microseconds. */
void p7501_wdt_end_cmd(bool aborted);

/**
 * @brief Cooperative capture/wait loop step guarding deadline, hard ceiling, WDT kick, and Ctrl-C abort.
 *
 * @return true while execution should continue; false if `deadline_us`, ceiling, or CDC abort fired.
 */
bool p7501_wdt_loop_step(
    uint64_t start_us,
    uint64_t deadline_us,
    bool allow_cdc_abort,
    bool *out_aborted
);

/** Return read-only snapshot of watchdog state, recovery counters, and command trace telemetry. */
p7501_wdt_telemetry_t p7501_wdt_snapshot(void);

/** Print structured `[WDT]` traceability summary line to USB CDC. */
void p7501_wdt_print_summary_line(void);

/** Handle `WDT` CLI status query (`WDT: OK`, non-overridable). */
void p7501_cmd_wdt_status(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_WATCHDOG_H */
