/**
 * @file picofw_identify.h
 * @brief Runtime 4-stage `IDENTIFY`/`AUTOPROBE` command driver (`@PICO_IDENT`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_IDENTIFY_H
#define PICOFW_IDENTIFY_H

#include "diag/picofw_identify_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Execute 4-stage hardware identification and populate `out_rep` (optional). */
void picofw_identify_run_hw(const char *arg, picofw_ident_report_t *out_rep);

/** CLI command `IDENTIFY [PASSIVE|NORMAL|JIG] [ms]` (and alias `AUTOPROBE`). */
void picofw_cmd_identify(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_IDENTIFY_H */
