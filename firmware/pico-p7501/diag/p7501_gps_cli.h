/**
 * @file p7501_gps_cli.h
 * @brief Pico hardware CLI handlers `GPS`, `ROUTE`, and `XRAY` + auto-calibrated environment state.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_GPS_CLI_H
#define P7501_GPS_CLI_H

#include "diag/p7501_math_gps.h"
#include "diag/p7501_pieos_xray.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Probe-wide calibrated Mathematical GPS state singleton. */
p7501_gps_env_t *p7501_gps_global_env(void);

/** Probe-wide PiE-OS 12-stage X-Ray trace accumulator singleton. */
p7501_xray_trace_t *p7501_xray_global_trace(void);

/** Synchronize global GPS environment whenever a boot classifier completes. */
void p7501_gps_sync_from_classifier(const p7501_boot_classifier_t *cls);

/** Print structured `@P7501_*` envelope line and emit matching push-bus frame. */
void p7501_gps_emit_tag(const char *tag, const char *line);

/** Trap-checked, WDT-safe 1..4096 B target memory read helper shared across GPS & SURG CLIs. */
bool p7501_gps_read_window_checked(
    const char *cmd_tag,
    uint32_t va,
    uint32_t len,
    uint8_t *dst
);

/** CLI `GPS <CAL|ENV|AUTOCAL <m+s>|PATTERNS <va> <len>|ENTROPY <va> <len>|CALLS <va> <len>|RESOLVE <name|va>>`. */
void p7501_cmd_gps(const char *arg);

/** CLI `ROUTE [BOOT|VIDEO|BGO|HDD|NVRAM|UNBRICK]`. */
void p7501_cmd_route(const char *arg);

/** CLI `XRAY [ms|BOOT [ms]|SUMMARY|COLOR <ON|OFF>]`. */
void p7501_cmd_xray(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_GPS_CLI_H */
