/**
 * @file picofw_surg_internal.h
 * @brief Private shared bridge between offline `SURG` dispatcher (`picofw_surg_cli.c`)
 *        and live SDRAM/CPU execution shell (`picofw_surg_live.c`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_SURG_INTERNAL_H
#define PICOFW_SURG_INTERNAL_H

#include "diag/picofw_microsurgery.h"

#ifdef __cplusplus
extern "C" {
#endif

picofw_surg_journal_t *picofw_surg_global_journal(void);
bool picofw_surg_resolve_va_or_anchor(const char *tok, uint32_t *out_va);
void picofw_surg_handle_bit_live(uint32_t target_va, picofw_bit_op_t bop, uint32_t a1, uint32_t a2, uint32_t a3);
void picofw_surg_handle_patch_timeout_hook(char *toks[], size_t n);
void picofw_surg_handle_time_and_cpu(char *toks[], size_t n);
void picofw_surg_handle_hdd_unlock(char *toks[], size_t n);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_SURG_INTERNAL_H */
