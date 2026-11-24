/**
 * @file p7501_hotplug_codec.h
 * @brief Pure C11 debounced P7501 hotplug & NSW5V/VDD auto-heal FSM (`HOTPLUG`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_HOTPLUG_CODEC_H
#define P7501_HOTPLUG_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_HOTPLUG_DEBOUNCE_SAMPLES 5u

typedef enum p7501_hotplug_state {
    P7501_HP_DETACHED = 0,
    P7501_HP_STANDBY = 1,
    P7501_HP_DEBOUNCING = 2,
    P7501_HP_ONLINE_AWAKE = 3,
    P7501_HP_DROPOUT_FAULT = 4
} p7501_hotplug_state_t;

typedef struct p7501_hotplug_fsm {
    p7501_hotplug_state_t state;
    p7501_hotplug_state_t prev_state;
    p7501_hotplug_state_t candidate;
    uint8_t stable_count;
    uint32_t transitions;
    uint32_t recoveries;
    uint32_t last_vdd_mv;
    bool last_nsw5v;
    bool push_enabled;
} p7501_hotplug_fsm_t;

/** Initialize hotplug FSM (boot default: `push_enabled = false`, idle state `DETACHED`). */
void p7501_hotplug_fsm_init(p7501_hotplug_fsm_t *fsm);

/** Return canonical uppercase string name for a hotplug state. */
const char *p7501_hotplug_state_name(p7501_hotplug_state_t st);

/**
 * @brief Step the 5-sample debounced P7501 hotplug / auto-heal state machine.
 *
 * Sets `*out_changed = true` only on debounced committed transitions (or immediate dropout faults),
 * and sets `*out_clear_fault_latch = true` once `ONLINE_AWAKE` stabilizes after a latched fault.
 */
void p7501_hotplug_fsm_step(
    p7501_hotplug_fsm_t *fsm,
    uint32_t vdd_mv,
    bool nsw5v_high,
    bool fault_latched,
    bool *out_changed,
    bool *out_clear_fault_latch
);

/** Format canonical `@P7501_HOTPLUG ...` KV payload into `out[0..cap)`. */
size_t p7501_hotplug_format_kv(
    const p7501_hotplug_fsm_t *fsm,
    bool fault_latched,
    uint32_t dropouts,
    char *out,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_HOTPLUG_CODEC_H */
