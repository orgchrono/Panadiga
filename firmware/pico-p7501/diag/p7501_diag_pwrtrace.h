/**
 * @file p7501_diag_pwrtrace.h
 * @brief P7501 boot power-rail and digital line glitch monitor (`PWRTRACE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_DIAG_PWRTRACE_H
#define P7501_DIAG_PWRTRACE_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

/** Running GP26 SW_UART voltage bounds, separating active Pico pull-down samples from Hi-Z ADC0 readings. */
typedef struct {
    float min_v;
    float max_v;
    float hiz_min_v;
    float hiz_max_v;
    uint32_t low_samples;
    uint32_t hiz_samples;
} p7501_adc0_trace_t;

/** Trace SW_UART ADC0 voltage min/max and NSW5V/RXD/TXD transitions during boot (`PWRTRACE [ms] [boot]`). */
void p7501_cmd_pwrtrace(const char *arg);

/** Initialize an ADC0/SW_UART voltage trace accumulator. */
void p7501_diag_adc0_trace_init(p7501_adc0_trace_t *trace);

/** Sample GP26 SW_UART: records 0.00V while actively pulled LOW by Pico, or ADC0 voltage while Hi-Z. */
void p7501_diag_adc0_trace_sample(p7501_adc0_trace_t *trace);

/** Clamp unpopulated min bounds before printing trace summary. */
void p7501_diag_adc0_trace_finalize(p7501_adc0_trace_t *trace);

/** Sample GP26 ADC0/SW_UART voltage and update running min/max bounds. */
void p7501_diag_sample_adc0_minmax(float *min_v, float *max_v);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DIAG_PWRTRACE_H */
