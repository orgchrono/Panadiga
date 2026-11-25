/**
 * @file picofw_spsc_ring.c
 * @brief Portable lock-free SPSC ring implementation with RP2 memory barrier hooks.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/picofw_spsc_ring.h"
#include <string.h>

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
#include "hardware/sync.h"
#include "pico/platform.h"
#define PICOFW_MEM_BARRIER() __dmb()
#define PICOFW_RAM_FN(fn)    __not_in_flash_func(fn)
#else
#define PICOFW_MEM_BARRIER() do {} while (0)
#define PICOFW_RAM_FN(fn)    fn
#endif

bool picofw_spsc_init(picofw_spsc_ring_t *r, uint8_t *storage, size_t cap_pow2) {
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

bool PICOFW_RAM_FN(picofw_spsc_push)(picofw_spsc_ring_t *r, uint8_t byte) {
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
    PICOFW_MEM_BARRIER();
    r->head = next;
    return true;
}

bool picofw_spsc_pop(picofw_spsc_ring_t *r, uint8_t *out_byte) {
    if ((r == NULL) || (r->buf == NULL) || (out_byte == NULL)) {
        return false;
    }
    const uint32_t t = r->tail;
    if (t == r->head) {
        return false;
    }
    PICOFW_MEM_BARRIER();
    *out_byte = r->buf[t];
    r->tail = (t + 1u) & r->mask;
    return true;
}

size_t picofw_spsc_available(const picofw_spsc_ring_t *r) {
    if ((r == NULL) || (r->buf == NULL)) {
        return 0u;
    }
    return (size_t)((r->head - r->tail) & r->mask);
}
