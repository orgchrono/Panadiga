/**
 * @file picofw_pwrosc_codec.h
 * @brief Pure C11 integer mV power-rail oscilloscope & die-temp telemetry analyzer (`PWROSC`, #13).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PWROSC_CODEC_H
#define PICOFW_PWROSC_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct picofw_pwrosc_stats {
    uint32_t sample_count;
    uint32_t min_mv;
    uint32_t max_mv;
    uint32_t sum_mv;
    uint32_t avg_mv;
    uint32_t ripple_mv;
    uint32_t brownouts;
    int32_t die_temp_mc;
} picofw_pwrosc_stats_t;

void picofw_pwrosc_stats_init(picofw_pwrosc_stats_t *st, int32_t die_temp_mc);
void picofw_pwrosc_stats_feed_mv(picofw_pwrosc_stats_t *st, uint32_t sample_mv);
void picofw_pwrosc_stats_finalize(picofw_pwrosc_stats_t *st);

/** Convert RP2 12-bit internal temp ADC reading (`0..4095`) to milli-Celsius cleanly. */
int32_t picofw_adc_raw_to_temp_mc(uint16_t raw_adc);

/** CLI command `PWROSC [ms]`. */
void picofw_cmd_pwrosc(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PWROSC_CODEC_H */
