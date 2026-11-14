/**
 * @file p7501_diag_watch.h
 * @brief Autonomous PIO1 boot watchdog & pre-MAIN recovery dispatcher (`DIAGWATCH`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_DIAG_WATCH_H
#define P7501_DIAG_WATCH_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Monitor SC0 boot stream, auto-release `SW_UART` -> Hi-Z on `DDR CHECK: OK`,
 *        classify SDDIAG/CDDIAG/BGO events, and auto-trigger `PANAMON1` recovery actions.
 */
void p7501_cmd_diag_watch(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DIAG_WATCH_H */
