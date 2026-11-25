/**
 * @file picofw_jigsweep_codec.h
 * @brief Pure ISO C11 argument parser and outcome classifier for `JIGSWEEP`.
 *
 * Supports `HOLD`, `DELAY`, and `PREBOOT` timing sweeps around `XRESET` and `DDR CHECK: OK`
 * to pinpoint the hardware `SW_UART` latch window and probe pre-DDR boot handshakes.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_JIGSWEEP_CODEC_H
#define PICOFW_JIGSWEEP_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_JIGSWEEP_MAX_POINTS 16u

typedef enum picofw_jigsweep_mode {
    PICOFW_JIGSWEEP_HOLD = 0,
    PICOFW_JIGSWEEP_DELAY = 1,
    PICOFW_JIGSWEEP_PREBOOT = 2
} picofw_jigsweep_mode_t;

typedef enum picofw_jigsweep_outcome {
    PICOFW_SWEEP_OUTCOME_SILENT = 0,
    PICOFW_SWEEP_OUTCOME_NORMAL = 1,
    PICOFW_SWEEP_OUTCOME_SERIAL = 2,
    PICOFW_SWEEP_OUTCOME_EARLY_OTHER = 3
} picofw_jigsweep_outcome_t;

typedef struct picofw_jigsweep_spec {
    picofw_jigsweep_mode_t mode;
    uint32_t points_ms[PICOFW_JIGSWEEP_MAX_POINTS];
    size_t point_count;
} picofw_jigsweep_spec_t;

/** Parse `JIGSWEEP [HOLD|DELAY|PREBOOT] [ms1 [ms2 ...]]`. */
bool picofw_jigsweep_parse_args(const char *arg, picofw_jigsweep_spec_t *out);

/** Classify captured boot bytes into SILENT / NORMAL / SERIAL / EARLY_OTHER. */
picofw_jigsweep_outcome_t picofw_jigsweep_classify_rx(const uint8_t *buf, size_t len);

/** Return human-readable outcome label. */
const char *picofw_jigsweep_outcome_name(picofw_jigsweep_outcome_t outcome);

/** CLI command `JIGSWEEP`. */
void picofw_cmd_jigsweep(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_JIGSWEEP_CODEC_H */
