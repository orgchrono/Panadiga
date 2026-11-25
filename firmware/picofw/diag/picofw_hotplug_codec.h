/**
 * @file picofw_hotplug_codec.h
 * @brief Pure C11 debounced P7501 hotplug & NSW5V/VDD auto-heal FSM (`HOTPLUG`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_HOTPLUG_CODEC_H
#define PICOFW_HOTPLUG_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_HOTPLUG_DEBOUNCE_SAMPLES 5u

typedef enum picofw_hotplug_state {
    PICOFW_HP_DETACHED = 0,
    PICOFW_HP_STANDBY = 1,
    PICOFW_HP_DEBOUNCING = 2,
    PICOFW_HP_ONLINE_AWAKE = 3,
    PICOFW_HP_DROPOUT_FAULT = 4
} picofw_hotplug_state_t;

typedef struct picofw_hotplug_fsm {
    picofw_hotplug_state_t state;
    picofw_hotplug_state_t prev_state;
    picofw_hotplug_state_t candidate;
    uint8_t stable_count;
    uint32_t transitions;
    uint32_t recoveries;
    uint32_t last_vdd_mv;
    bool last_nsw5v;
    bool push_enabled;
} picofw_hotplug_fsm_t;

/** Initialize hotplug FSM (boot default: `push_enabled = false`, idle state `DETACHED`). */
void picofw_hotplug_fsm_init(picofw_hotplug_fsm_t *fsm);

/** Return canonical uppercase string name for a hotplug state. */
const char *picofw_hotplug_state_name(picofw_hotplug_state_t st);

/**
 * @brief Step the 5-sample debounced P7501 hotplug / auto-heal state machine.
 *
 * Sets `*out_changed = true` only on debounced committed transitions (or immediate dropout faults),
 * and sets `*out_clear_fault_latch = true` once `ONLINE_AWAKE` stabilizes after a latched fault.
 */
void picofw_hotplug_fsm_step(
    picofw_hotplug_fsm_t *fsm,
    uint32_t vdd_mv,
    bool nsw5v_high,
    bool fault_latched,
    bool *out_changed,
    bool *out_clear_fault_latch
);

/** Format canonical `@PICO_HOTPLUG ...` KV payload into `out[0..cap)`. */
size_t picofw_hotplug_format_kv(
    const picofw_hotplug_fsm_t *fsm,
    bool fault_latched,
    uint32_t dropouts,
    char *out,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_HOTPLUG_CODEC_H */
