/**
 * @file p7501_diag_boottrace.h
 * @brief Correlated PIO1 UART + GP3/GP4/GP5 control-edge + GP26 ADC0 boot profiler (`BOOTTRACE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_DIAG_BOOTTRACE_H
#define P7501_DIAG_BOOTTRACE_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct p7501_boot_classifier p7501_boot_classifier_t;

/** Print `@P7501_BOOT_CLASS`, `@P7501_DIAG_DEEP`, and push `@P7501_FRAME type=DIAG_DEEP`. */
void p7501_boottrace_emit_reports(const p7501_boot_classifier_t *cls);

/** Execute `BOOTTRACE [ms] [NORMAL|JIG[:ms]|NOPULL]` correlated UART + control/rail trace. */
void p7501_cmd_boottrace(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DIAG_BOOTTRACE_H */
