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

/** Execute `BOOTTRACE [ms] [NORMAL|JIG[:ms]|NOPULL]` correlated UART + control/rail trace. */
void p7501_cmd_boottrace(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DIAG_BOOTTRACE_H */
