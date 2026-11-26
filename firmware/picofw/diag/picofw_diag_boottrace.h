/**
 * @file picofw_diag_boottrace.h
 * @brief Correlated PIO1 UART + GP3/GP4/GP5 control-edge + GP26 ADC0 boot profiler (`BOOTTRACE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_DIAG_BOOTTRACE_H
#define PICOFW_DIAG_BOOTTRACE_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct picofw_boot_classifier picofw_boot_classifier_t;

/** Print `@PICO_BOOT_CLASS`, `@PICO_DIAG_DEEP`, and push `@PICO_FRAME type=DIAG_DEEP`. */
void picofw_boottrace_emit_reports(const picofw_boot_classifier_t *cls);

/** Execute `BOOTTRACE [ms] [NORMAL|JIG[:ms]|NOPULL]` correlated UART + control/rail trace. */
void picofw_cmd_boottrace(const char *arg);

/**
 * Execute `BOOTFOLLOW [idle_ms]`: passive (never drives XRESET/SW_UART), unbounded boot trace.
 * Cycles restart the relative clock on real XRESET/wake/`DDR CHECK: OK` events. Stop with Ctrl+C.
 */
void picofw_cmd_bootfollow(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_DIAG_BOOTTRACE_H */
