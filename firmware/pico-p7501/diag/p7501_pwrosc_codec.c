/**
 * @file p7501_pwrosc_codec.c
 * @brief Pure C11 integer mV power oscilloscope & RP2 die temperature converter.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_pwrosc_codec.h"
#include "p7501_types.h"
#include <string.h>

void p7501_pwrosc_stats_init(p7501_pwrosc_stats_t *st, int32_t die_temp_mc) {
    if (st == NULL) {
        return;
    }
    memset(st, 0, sizeof(*st));
    st->min_mv = 0xFFFFFFFFUL;
    st->die_temp_mc = die_temp_mc;
}

void p7501_pwrosc_stats_feed_mv(p7501_pwrosc_stats_t *st, uint32_t sample_mv) {
    if (st == NULL) {
        return;
    }
    st->sample_count += 1u;
    st->sum_mv += sample_mv;
    if (sample_mv < st->min_mv) {
        st->min_mv = sample_mv;
    }
    if (sample_mv > st->max_mv) {
        st->max_mv = sample_mv;
    }
    if (sample_mv < P7501_SSOT_VDD_MIN_MV) {
        st->brownouts += 1u;
    }
}

void p7501_pwrosc_stats_finalize(p7501_pwrosc_stats_t *st) {
    if (st == NULL) {
        return;
    }
    if (st->sample_count == 0u) {
        st->min_mv = 0u;
        st->avg_mv = 0u;
        st->max_mv = 0u;
        st->ripple_mv = 0u;
        return;
    }
    st->avg_mv = st->sum_mv / st->sample_count;
    st->ripple_mv = (st->max_mv >= st->min_mv) ? (st->max_mv - st->min_mv) : 0u;
}

int32_t p7501_adc_raw_to_temp_mc(uint16_t raw_adc) {
    /* V_be = raw * 3300 / 4096 mV; T = 27.0C - (V_be - 0.706V) / 0.001721 */
    const int64_t v_mv = ((int64_t)raw_adc * (int64_t)P7501_SSOT_VDD_NOMINAL_MV) / 4096LL;
    return 27000 - (int32_t)(((v_mv - 706LL) * 1000000LL) / 1721LL);
}
