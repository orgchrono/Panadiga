/**
 * @file p7501_fmu_internal.h
 * @brief Internal shared phase helpers for FMU hardware probes (`FMUSCAN`, `FMUTEST`, `FMUBATCH`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_FMU_INTERNAL_H
#define P7501_FMU_INTERNAL_H

#include "uart/p7501_fmu_codec.h"

void p7501_fmu_start_phase_session(p7501_fmu_phase_state_t *out_st);
bool p7501_fmu_pop_rx_byte(uint8_t *rx_buf, uint32_t *rx_len, uint32_t max_cap);

#endif /* P7501_FMU_INTERNAL_H */
