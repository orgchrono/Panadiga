/**
 * @file p7501_ir_kaseikyo.h
 * @brief Runtime 38 kHz hardware PWM Kaseikyo IR transmitter (`IRTX`) for RP2040/RP2350.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_IR_KASEIKYO_H
#define P7501_IR_KASEIKYO_H

#include "diag/p7501_ir_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Transmit a parsed IR spec on hardware 38 kHz PWM (with WDT & Hi-Z safety restore). */
void p7501_ir_transmit_spec(const p7501_ir_spec_t *spec);

/** CLI command handler for `IRTX <preset|sub:cmd|cmd> [GPn|gpio] [repeat]`. */
void p7501_cmd_irtx(const char *arg);

/** CLI command handler for `IRRX [ms] [GPn]` (TSOP38238 active-LOW Kaseikyo decoder). */
void p7501_cmd_irrx(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_IR_KASEIKYO_H */
