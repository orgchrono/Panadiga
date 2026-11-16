/**
 * @file p7501_ir_kaseikyo.h
 * @brief Runtime 38 kHz hardware PWM Kaseikyo IR transmitter (`IRTX`) for RP2040/RP2350.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_IR_KASEIKYO_H
#define P7501_IR_KASEIKYO_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI command handler for `IRTX <preset|sub:cmd|cmd> [GPn|gpio] [repeat]`. */
void p7501_cmd_irtx(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_IR_KASEIKYO_H */
