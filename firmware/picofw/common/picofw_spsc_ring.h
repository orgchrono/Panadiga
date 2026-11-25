/**
 * @file picofw_spsc_ring.h
 * @brief Lock-free single-producer single-consumer (SPSC) byte ring buffer (Feature #11).
 *
 * Host-testable pure C11 (zero dynamic allocation, zero C11 `<stdatomic.h>` MSVC issues).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_SPSC_RING_H
#define PICOFW_SPSC_RING_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct picofw_spsc_ring {
    uint8_t *buf;
    uint32_t mask;
    volatile uint32_t head;
    volatile uint32_t tail;
    volatile uint32_t dropped;
} picofw_spsc_ring_t;

/** Initialize ring over `storage` whose size `cap_pow2` MUST be a power of 2 >= 2. */
bool picofw_spsc_init(picofw_spsc_ring_t *r, uint8_t *storage, size_t cap_pow2);

/** Push one byte (lock-free SPSC producer); returns false and increments `dropped` if full. */
bool picofw_spsc_push(picofw_spsc_ring_t *r, uint8_t byte);

/** Pop one byte (lock-free SPSC consumer); returns false if empty. */
bool picofw_spsc_pop(picofw_spsc_ring_t *r, uint8_t *out_byte);

/** Return number of queued unread bytes cleanly. */
size_t picofw_spsc_available(const picofw_spsc_ring_t *r);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_SPSC_RING_H */
