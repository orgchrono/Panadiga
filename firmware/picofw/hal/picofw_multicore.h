/**
 * @file picofw_multicore.h
 * @brief RP2040/RP2350 Core-1 background telemetry worker wrapper (Feature #11).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_MULTICORE_H
#define PICOFW_MULTICORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Start Core-1 passive SPSC RX/GPIO edge telemetry worker safely (idempotent). */
void picofw_multicore_start_worker(void);

/**
 * Suspend Core-1 XIP execution before erasing/programming QSPI flash with bounded timeout.
 * Returns false (fail-closed) if Core-1 is running and failed to acknowledge lockout in time.
 */
bool picofw_multicore_lockout_begin(void);

/** Resume Core-1 execution after exiting QSPI critical section. */
void picofw_multicore_lockout_end(void);

/** Pop one telemetry byte queued by Core-1 worker without blocking Core-0. */
bool picofw_multicore_pop_byte(uint8_t *out_byte);

/** Return Core-1 worker heartbeat counter + dropped byte count. */
void picofw_multicore_stats(uint32_t *out_ticks, uint32_t *out_dropped);

/** Return observed Core-1 GPIO edge transitions on P7501 monitor pins. */
uint32_t picofw_multicore_edge_count(void);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_MULTICORE_H */
