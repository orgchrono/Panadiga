/**
 * @file p7501_surg_internal.h
 * @brief Private shared bridge between offline `SURG` dispatcher (`p7501_surg_cli.c`)
 *        and live SDRAM/CPU execution shell (`p7501_surg_live.c`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_SURG_INTERNAL_H
#define P7501_SURG_INTERNAL_H

#include "diag/p7501_microsurgery.h"

#ifdef __cplusplus
extern "C" {
#endif

p7501_surg_journal_t *p7501_surg_global_journal(void);
bool p7501_surg_resolve_va_or_anchor(const char *tok, uint32_t *out_va);
void p7501_surg_handle_bit_live(uint32_t target_va, p7501_bit_op_t bop, uint32_t a1, uint32_t a2, uint32_t a3);
void p7501_surg_handle_patch_timeout_hook(char *toks[], size_t n);
void p7501_surg_handle_time_and_cpu(char *toks[], size_t n);
void p7501_surg_handle_hdd_unlock(char *toks[], size_t n);

#ifdef __cplusplus
}
#endif

#endif /* P7501_SURG_INTERNAL_H */
