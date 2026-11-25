/**
 * @file picofw_fmu_internal.h
 * @brief Internal shared phase helpers for FMU hardware probes (`FMUSCAN`, `FMUTEST`, `FMUBATCH`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_FMU_INTERNAL_H
#define PICOFW_FMU_INTERNAL_H

#include "uart/picofw_fmu_codec.h"

void picofw_fmu_start_phase_session(picofw_fmu_phase_state_t *out_st);
bool picofw_fmu_pop_rx_byte(uint8_t *rx_buf, uint32_t *rx_len, uint32_t max_cap);

#endif /* PICOFW_FMU_INTERNAL_H */
