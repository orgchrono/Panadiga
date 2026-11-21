/**
 * @file p7501_uart_autobaud.c
 * @brief Dual-polarity pulse cluster histogram with isolated glitch immunity for `BAUDSCAN`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_autobaud.h"
#include "uart/p7501_uart_codec.h"

#include <limits.h>
#include <stddef.h>
#include <string.h>

static const uint32_t k_std_rates[P7501_AUTOBAUD_CANDIDATES] = {
    460800u, 230400u, 115200u, 57600u, 38400u, 31250u, 19200u, 9600u, 4800u, 2400u, 1200u
};

static uint32_t rate_nominal_bit_us(uint32_t baud) {
    return (1000000u + (baud / 2u)) / baud;
}

static uint32_t u32_abs_diff(uint32_t a, uint32_t b) {
    return (a > b) ? (a - b) : (b - a);
}

void p7501_baud_hist_init(p7501_baud_hist_t *hist) {
    if (hist == NULL) {
        return;
    }
    memset(hist, 0, sizeof(*hist));
    hist->min_low_us = UINT32_MAX;
    hist->min_high_us = UINT32_MAX;
}

void p7501_baud_hist_push_pulse(p7501_baud_hist_t *hist, uint32_t width_us, bool is_high) {
    if (hist == NULL) {
        return;
    }
    if (width_us < 2u || width_us > 2000u) {
        if (width_us < 2u) {
            ++hist->glitches_rejected;
        }
        return;
    }
    if (is_high) {
        ++hist->high_pulses;
        if (width_us < hist->min_high_us) {
            hist->min_high_us = width_us;
        }
        if (width_us > hist->max_high_us) {
            hist->max_high_us = width_us;
        }
    } else {
        ++hist->low_pulses;
        if (width_us < hist->min_low_us) {
            hist->min_low_us = width_us;
        }
        if (width_us > hist->max_low_us) {
            hist->max_low_us = width_us;
        }
    }

    uint32_t raw_bps = 0u;
    const uint32_t snapped = p7501_snap_standard_baud(width_us, &raw_bps);
    if (snapped == 0u) {
        return;
    }
    for (size_t i = 0u; i < P7501_AUTOBAUD_CANDIDATES; ++i) {
        if (k_std_rates[i] == snapped) {
            ++hist->cluster_hits[i];
            break;
        }
    }
}

void p7501_baud_hist_finish(const p7501_baud_hist_t *hist, p7501_baud_estimate_t *out_est) {
    if (out_est == NULL) {
        return;
    }
    memset(out_est, 0, sizeof(*out_est));
    if (hist == NULL) {
        return;
    }

    const uint32_t total_valid = hist->low_pulses + hist->high_pulses;
    if (total_valid == 0u) {
        return;
    }

    /* Require >=3 corroborating 1T pulses when total activity is rich; prevents power-on spikes from forcing 460800 */
    const uint32_t min_cluster = (total_valid >= 6u) ? 3u : 1u;
    size_t best_idx = P7501_AUTOBAUD_CANDIDATES;
    uint32_t best_score = 0u;

    for (size_t i = 0u; i < P7501_AUTOBAUD_CANDIDATES; ++i) {
        const uint32_t hits = hist->cluster_hits[i];
        if (hits < min_cluster) {
            continue;
        }
        /* Select dominant cluster (tie-break toward faster baud as i increases from fast to slow). */
        if (best_idx == P7501_AUTOBAUD_CANDIDATES || hits > best_score) {
            best_score = hits;
            best_idx = i;
        }
    }

    /* Second pass: check if a faster corroborated baud (>=3 hits) matches genuine 1T bits behind multi-bit runs */
    for (size_t i = 0u; i < best_idx && i < P7501_AUTOBAUD_CANDIDATES; ++i) {
        const uint32_t hits = hist->cluster_hits[i];
        if (hits >= 3u && (hits * 4u) >= total_valid) {
            best_idx = i;
            best_score = hits;
            break;
        }
        /* Promote 1T over a 2T harmonic only when supported by both >=3 hits and >= best_score / 8 */
        if (best_idx < P7501_AUTOBAUD_CANDIDATES && hits >= 3u && hits >= (best_score / 8u)) {
            const uint32_t t_fast = rate_nominal_bit_us(k_std_rates[i]);
            const uint32_t t_best = rate_nominal_bit_us(k_std_rates[best_idx]);
            if (u32_abs_diff(t_fast * 2u, t_best) <= 3u) {
                best_idx = i;
                best_score = hits;
                break;
            }
        }
    }

    if (best_idx == P7501_AUTOBAUD_CANDIDATES) {
        const uint32_t fallback_us = (hist->min_low_us <= hist->min_high_us)
            ? hist->min_low_us
            : hist->min_high_us;
        out_est->snapped_baud = p7501_snap_standard_baud(fallback_us, &out_est->raw_bps);
        out_est->min_cluster_us = fallback_us;
        out_est->best_cluster_hits = 1u;
        out_est->confidence_pct = (total_valid == 1u) ? 40u : 25u;
        return;
    }

    out_est->snapped_baud = k_std_rates[best_idx];
    out_est->min_cluster_us = rate_nominal_bit_us(out_est->snapped_baud);
    out_est->raw_bps = (hist->min_low_us < UINT32_MAX &&
                        p7501_snap_standard_baud(hist->min_low_us, NULL) == out_est->snapped_baud)
        ? (1000000u / hist->min_low_us)
        : ((out_est->min_cluster_us > 0u) ? (1000000u / out_est->min_cluster_us) : out_est->snapped_baud);
    out_est->best_cluster_hits = best_score;
    const uint32_t raw_pct = (best_score * 100u) / total_valid;
    out_est->confidence_pct = (raw_pct < 35u) ? 50u : (raw_pct > 100u ? 100u : raw_pct);
}
