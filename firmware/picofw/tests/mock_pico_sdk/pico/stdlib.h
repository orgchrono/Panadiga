/**
 * @file pico/stdlib.h
 * @brief Minimal host-test mock stub for Raspberry Pi Pico SDK stdlib/types.
 */

#ifndef PICOFW_MOCK_PICO_STDLIB_H
#define PICOFW_MOCK_PICO_STDLIB_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef unsigned int uint;

#ifndef __time_critical_func
#define __time_critical_func(x) x
#endif

static inline void tight_loop_contents(void) {}

#endif /* PICOFW_MOCK_PICO_STDLIB_H */
