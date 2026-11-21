/**
 * @file p7501_diag_bus.h
 * @brief AM34 P7501 boot profile trigger, monitor probe sweep, and memory dump commands.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_DIAG_BUS_H
#define P7501_DIAG_BUS_H

#include "p7501_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Handle `DUMP [addr] [cnt]` CLI command via async monitor aperture reads. */
void p7501_diag_cmd_dump(const char *arg);

/** Handle `BREAK_IN [0..3]` P7501 boot/reset profile trigger CLI command. */
void p7501_diag_cmd_boot_profile(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DIAG_BUS_H */
