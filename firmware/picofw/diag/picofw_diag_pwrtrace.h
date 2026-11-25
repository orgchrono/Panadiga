/**
 * @file picofw_diag_pwrtrace.h
 * @brief P7501 boot power-rail and digital line glitch monitor (`PWRTRACE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_DIAG_PWRTRACE_H
#define PICOFW_DIAG_PWRTRACE_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include "common/picofw_hex.h"

/** Running GP26 SW_UART integer millivolt bounds, separating active Pico pull-down samples from Hi-Z ADC0 readings. */
typedef struct {
    uint32_t min_mv;
    uint32_t max_mv;
    uint32_t hiz_min_mv;
    uint32_t hiz_max_mv;
    uint32_t low_samples;
    uint32_t hiz_samples;
} picofw_adc0_trace_t;

/** Trace SW_UART ADC0 voltage min/max and NSW5V/RXD/TXD transitions during boot (`PWRTRACE [ms] [boot]`). */
void picofw_cmd_pwrtrace(const char *arg);

/** Initialize an ADC0/SW_UART voltage trace accumulator. */
void picofw_diag_adc0_trace_init(picofw_adc0_trace_t *trace);

/** Sample GP26 SW_UART: records 0 mV while actively pulled LOW by Pico, or ADC0 mV while Hi-Z. */
void picofw_diag_adc0_trace_sample(picofw_adc0_trace_t *trace);

/** Clamp unpopulated min bounds before printing trace summary. */
void picofw_diag_adc0_trace_finalize(picofw_adc0_trace_t *trace);

/** Format finalized ADC0 trace summary (`min=..V max=..V (low=.. hiz=..V n=..)`). */
void picofw_diag_adc0_trace_format(const picofw_adc0_trace_t *trace, char *out_buf, size_t cap);

/** Sample GP26 ADC0/SW_UART millivolts and update running min/max bounds. */
void picofw_diag_sample_adc0_minmax(uint32_t *min_mv, uint32_t *max_mv);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_DIAG_PWRTRACE_H */
