/**
 * @file picofw_diag_watch.h
 * @brief Autonomous PIO1 boot watchdog & pre-MAIN recovery dispatcher (`DIAGWATCH`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_DIAG_WATCH_H
#define PICOFW_DIAG_WATCH_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Monitor SC0 boot stream, auto-release `SW_UART` -> Hi-Z on `DDR CHECK: OK`,
 *        classify SDDIAG/CDDIAG/BGO events, and auto-trigger `PANAMON1` recovery actions.
 */
void picofw_cmd_diag_watch(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_DIAG_WATCH_H */
