/**
 * @file picofw_gps_cli.h
 * @brief Pico hardware CLI handlers `GPS`, `ROUTE`, and `XRAY` + auto-calibrated environment state.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_GPS_CLI_H
#define PICOFW_GPS_CLI_H

#include "diag/picofw_math_gps.h"
#include "diag/picofw_pieos_xray.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Probe-wide calibrated Mathematical GPS state singleton. */
picofw_gps_env_t *picofw_gps_global_env(void);

/** Probe-wide PiE-OS 12-stage X-Ray trace accumulator singleton. */
picofw_xray_trace_t *picofw_xray_global_trace(void);

/** Synchronize global GPS environment whenever a boot classifier completes. */
void picofw_gps_sync_from_classifier(const picofw_boot_classifier_t *cls);

/** Print structured `@PICO_*` envelope line and emit matching push-bus frame. */
void picofw_gps_emit_tag(const char *tag, const char *line);

/** Trap-checked, WDT-safe 1..4096 B target memory read helper shared across GPS & SURG CLIs. */
bool picofw_gps_read_window_checked(
    const char *cmd_tag,
    uint32_t va,
    uint32_t len,
    uint8_t *dst
);

/** CLI `GPS <CAL|ENV|AUTOCAL <m+s>|PATTERNS <va> <len>|ENTROPY <va> <len>|CALLS <va> <len>|RESOLVE <name|va>>`. */
void picofw_cmd_gps(const char *arg);

/** CLI `ROUTE [BOOT|VIDEO|BGO|HDD|NVRAM|UNBRICK]`. */
void picofw_cmd_route(const char *arg);

/** CLI `XRAY [ms|BOOT [ms]|SUMMARY|COLOR <ON|OFF>]`. */
void picofw_cmd_xray(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_GPS_CLI_H */
