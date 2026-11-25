/**
 * @file picofw_multicore.c
 * @brief RP2040/RP2350 Core-1 passive GPIO/telemetry SPSC feeder (`pico_multicore`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_multicore.h"
#include "common/picofw_spsc_ring.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"

#include "hardware/gpio.h"
#include "hardware/sync.h"
#include "pico/multicore.h"
#include "pico/platform.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#define PICOFW_CORE1_RING_CAP          1024u
#define PICOFW_CORE1_LOCKOUT_TIMEOUT_US 50000ULL

static uint8_t s_core1_storage[PICOFW_CORE1_RING_CAP];
static picofw_spsc_ring_t s_core1_ring;
static volatile bool s_core1_running = false;
static volatile bool s_core1_lockout_ready = false;
static volatile uint32_t s_core1_ticks = 0u;
static volatile uint32_t s_core1_edges = 0u;

static void __not_in_flash_func(core1_worker_loop_ram)(void) {
    uint8_t prev_mask = (uint8_t)((gpio_get_all() >> P7501_PIN_RXD) & 0x0Fu);
    for (;;) {
        s_core1_ticks += 1u;
        const uint8_t cur_mask = (uint8_t)((gpio_get_all() >> P7501_PIN_RXD) & 0x0Fu);
        if (cur_mask != prev_mask) {
            s_core1_edges += 1u;
            (void)picofw_spsc_push(&s_core1_ring, cur_mask);
            prev_mask = cur_mask;
        }
        tight_loop_contents();
    }
}

static void core1_worker_entry(void) {
    /* First call on Core-1: arm inter-core FIFO XIP flash lockout handler safely. */
    multicore_lockout_victim_init();
    __dmb();
    s_core1_lockout_ready = true;
    __dmb();
    core1_worker_loop_ram();
}

void picofw_multicore_start_worker(void) {
    if (s_core1_running) {
        return;
    }
    (void)picofw_spsc_init(&s_core1_ring, s_core1_storage, sizeof(s_core1_storage));
    s_core1_ticks = 0u;
    s_core1_edges = 0u;
    s_core1_lockout_ready = false;
    s_core1_running = true;
    __dmb();
    multicore_launch_core1(core1_worker_entry);

    const uint64_t deadline = time_us_64() + 200000ULL;
    while (!s_core1_lockout_ready && (time_us_64() < deadline)) {
        picofw_wdt_kick();
        sleep_us(200u);
    }
    __dmb();
}

bool picofw_multicore_lockout_begin(void) {
    if (!s_core1_running) {
        return true;
    }
    if (!s_core1_lockout_ready || !multicore_lockout_victim_is_initialized(1u)) {
        return false;
    }
    return multicore_lockout_start_timeout_us(PICOFW_CORE1_LOCKOUT_TIMEOUT_US);
}

void picofw_multicore_lockout_end(void) {
    if (s_core1_running && s_core1_lockout_ready && multicore_lockout_victim_is_initialized(1u)) {
        (void)multicore_lockout_end_timeout_us(PICOFW_CORE1_LOCKOUT_TIMEOUT_US);
    }
}

bool picofw_multicore_pop_byte(uint8_t *out_byte) {
    return s_core1_running && picofw_spsc_pop(&s_core1_ring, out_byte);
}

void picofw_multicore_stats(uint32_t *out_ticks, uint32_t *out_dropped) {
    if (out_ticks != NULL) {
        *out_ticks = s_core1_ticks;
    }
    if (out_dropped != NULL) {
        *out_dropped = s_core1_ring.dropped;
    }
}

uint32_t picofw_multicore_edge_count(void) {
    return s_core1_edges;
}
