/**
 * @file p7501_pieos_xray.h
 * @brief Pure functional 12-stage PiE-OS boot-chain & metabolism X-Ray logger.
 *
 * Host-testable ISO C11 (`C11_HOST_SOURCES`), zero heap allocation, zero floats.
 * Attributes observed UART lines to PiE-OS stages (`S0..S11`), calibrated SSOT
 * function/caller VAs (`k_p7501_ssot_pieos_anchors`), integer ms timestamps/deltas,
 * and optional ANSI color level tags (disabled by default for raw log hygiene).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PIEOS_XRAY_H
#define P7501_PIEOS_XRAY_H

#include "diag/p7501_math_gps.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_XRAY_STAGE_COUNT 12u

typedef enum p7501_xray_stage {
    P7501_XRAY_S0_POWER = 0,
    P7501_XRAY_S1_IPL_DDR = 1,
    P7501_XRAY_S2_LOADER = 2,
    P7501_XRAY_S3_HDD_BRIDGE = 3,
    P7501_XRAY_S4_SD_DIAG_FVU = 4,
    P7501_XRAY_S5_PIE_CRT0 = 5,
    P7501_XRAY_S6_SYS_BOOT = 6,
    P7501_XRAY_S7_BGO_NFROM = 7,
    P7501_XRAY_S8_AVIN_AVOUT = 8,
    P7501_XRAY_S9_DRIVE_HDD = 9,
    P7501_XRAY_S10_VFD_VV = 10,
    P7501_XRAY_S11_STEADY_FAULT = 11
} p7501_xray_stage_t;

typedef enum p7501_xray_level {
    P7501_XRAY_LVL_INFO = 0,
    P7501_XRAY_LVL_OK = 1,
    P7501_XRAY_LVL_WARN = 2,
    P7501_XRAY_LVL_ERR = 3,
    P7501_XRAY_LVL_CRIT = 4
} p7501_xray_level_t;

typedef struct p7501_xray_event {
    uint64_t ts_us;
    uint64_t delta_us;
    uint32_t resolved_va;
    int8_t anchor_idx; /* -1 when not directly bound to an SSOT anchor */
    p7501_xray_stage_t stage;
    p7501_xray_level_t level;
    const char *subsys;
    char excerpt[96];
} p7501_xray_event_t;

typedef struct p7501_xray_trace {
    uint64_t first_ts_us;
    uint64_t last_ts_us;
    uint64_t stage_first_us[P7501_XRAY_STAGE_COUNT];
    uint16_t stage_seen_mask;
    uint8_t max_stage;
    int8_t fault_stage; /* -1 if no ERR/CRIT event occurred */
    uint32_t events_seen;
    uint32_t errors_seen;
    bool has_started;
    bool color_enabled;
} p7501_xray_trace_t;

/** Initialize PiE-OS X-Ray trace accumulator (preserving optional color preference if `keep_color`). */
void p7501_xray_init(p7501_xray_trace_t *tr, bool color_enabled);

/** Canonical short stage name (`S0_POWER`..`S11_STEADY_FAULT`). */
const char *p7501_xray_stage_name(p7501_xray_stage_t st);

/** Canonical 3-char severity tag (`INF`, `OK `, `WRN`, `ERR`, `CRT`). */
const char *p7501_xray_level_name(p7501_xray_level_t lvl);

/**
 * @brief Feed one clean UART log line at probe microsecond timestamp `ts_us`.
 *
 * Returns `true` and fills `*out_ev` whenever the line advances or annotates the PiE-OS chain.
 */
bool p7501_xray_feed_line(
    p7501_xray_trace_t *tr,
    const p7501_gps_env_t *env,
    uint64_t ts_us,
    const char *line,
    p7501_xray_event_t *out_ev
);

/** Format a single X-Ray metabolism line (optional ANSI colors when `ansi_color == true`). */
size_t p7501_xray_format_event(
    const p7501_xray_event_t *ev,
    bool ansi_color,
    char *out,
    size_t cap
);

/** Format machine-readable `@P7501_XRAY ...` summary envelope into `out[0..cap)`. */
size_t p7501_xray_format_summary(
    const p7501_xray_trace_t *tr,
    const p7501_gps_env_t *env,
    char *out,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PIEOS_XRAY_H */
