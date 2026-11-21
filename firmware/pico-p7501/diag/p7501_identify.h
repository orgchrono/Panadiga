/**
 * @file p7501_identify.h
 * @brief Runtime 4-stage `IDENTIFY`/`AUTOPROBE` command driver (`@P7501_IDENT`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_IDENTIFY_H
#define P7501_IDENTIFY_H

#include "diag/p7501_identify_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Execute 4-stage hardware identification and populate `out_rep` (optional). */
void p7501_identify_run_hw(const char *arg, p7501_ident_report_t *out_rep);

/** CLI command `IDENTIFY [PASSIVE|NORMAL|JIG] [ms]` (and alias `AUTOPROBE`). */
void p7501_cmd_identify(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_IDENTIFY_H */
