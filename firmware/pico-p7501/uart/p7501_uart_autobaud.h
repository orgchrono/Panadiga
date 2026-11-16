/**
 * @file p7501_uart_autobaud.h
 * @brief Glitch-immune dual-polarity pulse-width cluster autobaud estimator for P7501 RXD.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_UART_AUTOBAUD_H
#define P7501_UART_AUTOBAUD_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_AUTOBAUD_CANDIDATES 11u

/** Accumulates dual-polarity (LOW + HIGH) bit-width clusters while rejecting isolated glitches. */
typedef struct p7501_baud_hist {
    uint32_t min_low_us;
    uint32_t max_low_us;
    uint32_t min_high_us;
    uint32_t max_high_us;
    uint32_t low_pulses;
    uint32_t high_pulses;
    uint32_t glitches_rejected;
    uint32_t cluster_hits[P7501_AUTOBAUD_CANDIDATES];
} p7501_baud_hist_t;

/** Result summary of a clustered pulse-width + PIO framing verification pass. */
typedef struct p7501_baud_estimate {
    uint32_t snapped_baud;
    uint32_t raw_bps;
    uint32_t min_cluster_us;
    uint32_t best_cluster_hits;
    uint32_t confidence_pct;
} p7501_baud_estimate_t;

void p7501_baud_hist_init(p7501_baud_hist_t *hist);

/** Records one measured LOW (`is_high=false`) or HIGH (`is_high=true`) interval in microseconds. */
void p7501_baud_hist_push_pulse(p7501_baud_hist_t *hist, uint32_t width_us, bool is_high);

/** Selects the highest corroborated standard baud cluster (rejecting single-pulse spikes). */
void p7501_baud_hist_finish(const p7501_baud_hist_t *hist, p7501_baud_estimate_t *out_est);

#ifdef __cplusplus
}
#endif

#endif /* P7501_UART_AUTOBAUD_H */
