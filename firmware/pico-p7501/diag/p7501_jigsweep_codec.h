/**
 * @file p7501_jigsweep_codec.h
 * @brief Pure ISO C11 argument parser and outcome classifier for `JIGSWEEP`.
 *
 * Supports `HOLD`, `DELAY`, and `PREBOOT` timing sweeps around `XRESET` and `DDR CHECK: OK`
 * to pinpoint the hardware `SW_UART` latch window and probe pre-DDR boot handshakes.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_JIGSWEEP_CODEC_H
#define P7501_JIGSWEEP_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_JIGSWEEP_MAX_POINTS 16u

typedef enum p7501_jigsweep_mode {
    P7501_JIGSWEEP_HOLD = 0,
    P7501_JIGSWEEP_DELAY = 1,
    P7501_JIGSWEEP_PREBOOT = 2
} p7501_jigsweep_mode_t;

typedef enum p7501_jigsweep_outcome {
    P7501_SWEEP_OUTCOME_SILENT = 0,
    P7501_SWEEP_OUTCOME_NORMAL = 1,
    P7501_SWEEP_OUTCOME_SERIAL = 2,
    P7501_SWEEP_OUTCOME_EARLY_OTHER = 3
} p7501_jigsweep_outcome_t;

typedef struct p7501_jigsweep_spec {
    p7501_jigsweep_mode_t mode;
    uint32_t points_ms[P7501_JIGSWEEP_MAX_POINTS];
    size_t point_count;
} p7501_jigsweep_spec_t;

/** Parse `JIGSWEEP [HOLD|DELAY|PREBOOT] [ms1 [ms2 ...]]`. */
bool p7501_jigsweep_parse_args(const char *arg, p7501_jigsweep_spec_t *out);

/** Classify captured boot bytes into SILENT / NORMAL / SERIAL / EARLY_OTHER. */
p7501_jigsweep_outcome_t p7501_jigsweep_classify_rx(const uint8_t *buf, size_t len);

/** Return human-readable outcome label. */
const char *p7501_jigsweep_outcome_name(p7501_jigsweep_outcome_t outcome);

/** CLI command `JIGSWEEP`. */
void p7501_cmd_jigsweep(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_JIGSWEEP_CODEC_H */
