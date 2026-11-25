/**
 * @file picofw_ir_kaseikyo.h
 * @brief Runtime 38 kHz hardware PWM Kaseikyo IR transmitter (`IRTX`) for RP2040/RP2350.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_IR_KASEIKYO_H
#define PICOFW_IR_KASEIKYO_H

#include "diag/picofw_ir_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Transmit a parsed IR spec on hardware 38 kHz PWM (with WDT & Hi-Z safety restore). */
void picofw_ir_transmit_spec(const picofw_ir_spec_t *spec);

/** CLI command handler for `IR <VERIFY <12hex>|TX <...>|RX [ms]>`. */
void picofw_cmd_ir(const char *arg);

/** CLI command handler for `IRTX <preset|sub:cmd|cmd> [GPn|gpio] [repeat]`. */
void picofw_cmd_irtx(const char *arg);

/** CLI command handler for `IRRX [ms] [GPn]` (TSOP38238 active-LOW Kaseikyo decoder). */
void picofw_cmd_irrx(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_IR_KASEIKYO_H */
