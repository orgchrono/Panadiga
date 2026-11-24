/**
 * @file pico/time.h
 * @brief Minimal host-test mock stub for Pico SDK time helpers.
 */

#ifndef PANADIGA_MOCK_PICO_TIME_H
#define PANADIGA_MOCK_PICO_TIME_H

#include <stdbool.h>
#include <stdint.h>

typedef uint64_t absolute_time_t;

extern uint64_t p7501_mock_now_us;

static inline uint64_t time_us_64(void) {
    return p7501_mock_now_us;
}

static inline uint32_t to_ms_since_boot(absolute_time_t t) {
    return (uint32_t)(t / 1000ULL);
}

static inline absolute_time_t get_absolute_time(void) {
    return p7501_mock_now_us;
}

static inline void sleep_us(uint64_t us) {
    p7501_mock_now_us += us;
}

static inline void busy_wait_us_32(uint32_t us) {
    p7501_mock_now_us += (uint64_t)us;
}

static inline void busy_wait_us(uint64_t us) {
    p7501_mock_now_us += us;
}

static inline void sleep_ms(uint32_t ms) {
    p7501_mock_now_us += (uint64_t)ms * 1000ULL;
}

#endif /* PANADIGA_MOCK_PICO_TIME_H */
