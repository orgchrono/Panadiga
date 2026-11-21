/**
 * @file p7501_spsc_ring.c
 * @brief Portable lock-free SPSC ring implementation with RP2 memory barrier hooks.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_spsc_ring.h"
#include <string.h>

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
#include "hardware/sync.h"
#include "pico/platform.h"
#define P7501_MEM_BARRIER() __dmb()
#define P7501_RAM_FN(fn)    __not_in_flash_func(fn)
#else
#define P7501_MEM_BARRIER() do {} while (0)
#define P7501_RAM_FN(fn)    fn
#endif

bool p7501_spsc_init(p7501_spsc_ring_t *r, uint8_t *storage, size_t cap_pow2) {
    if ((r == NULL) || (storage == NULL) || (cap_pow2 < 2u) ||
        ((cap_pow2 & (cap_pow2 - 1u)) != 0u)) {
        return false;
    }
    r->buf = storage;
    r->mask = (uint32_t)(cap_pow2 - 1u);
    r->head = 0u;
    r->tail = 0u;
    r->dropped = 0u;
    return true;
}

bool P7501_RAM_FN(p7501_spsc_push)(p7501_spsc_ring_t *r, uint8_t byte) {
    if ((r == NULL) || (r->buf == NULL)) {
        return false;
    }
    const uint32_t h = r->head;
    const uint32_t next = (h + 1u) & r->mask;
    if (next == r->tail) {
        r->dropped += 1u;
        return false;
    }
    r->buf[h] = byte;
    P7501_MEM_BARRIER();
    r->head = next;
    return true;
}

bool p7501_spsc_pop(p7501_spsc_ring_t *r, uint8_t *out_byte) {
    if ((r == NULL) || (r->buf == NULL) || (out_byte == NULL)) {
        return false;
    }
    const uint32_t t = r->tail;
    if (t == r->head) {
        return false;
    }
    P7501_MEM_BARRIER();
    *out_byte = r->buf[t];
    r->tail = (t + 1u) & r->mask;
    return true;
}

size_t p7501_spsc_available(const p7501_spsc_ring_t *r) {
    if ((r == NULL) || (r->buf == NULL)) {
        return 0u;
    }
    return (size_t)((r->head - r->tail) & r->mask);
}
