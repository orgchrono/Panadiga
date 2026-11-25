/**
 * @file picofw_hotplug_codec.c
 * @brief Pure C11 5-sample debounced P7501 hotplug & NSW5V self-healing state machine.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_hotplug_codec.h"
#include "generated/picofw_ssot.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

void picofw_hotplug_fsm_init(picofw_hotplug_fsm_t *fsm) {
    if (fsm == NULL) {
        return;
    }
    memset(fsm, 0, sizeof(*fsm));
    fsm->state = PICOFW_HP_DETACHED;
    fsm->prev_state = PICOFW_HP_DETACHED;
    fsm->candidate = PICOFW_HP_DETACHED;
    fsm->push_enabled = false;
}

const char *picofw_hotplug_state_name(picofw_hotplug_state_t st) {
    switch (st) {
        case PICOFW_HP_DETACHED:      return "DETACHED";
        case PICOFW_HP_STANDBY:       return "STANDBY";
        case PICOFW_HP_DEBOUNCING:    return "DEBOUNCING";
        case PICOFW_HP_ONLINE_AWAKE:  return "ONLINE_AWAKE";
        case PICOFW_HP_DROPOUT_FAULT: return "DROPOUT_FAULT";
        default:                     return "UNKNOWN";
    }
}

static picofw_hotplug_state_t classify_instant(uint32_t vdd_mv, bool nsw5v_high, bool fault_latched) {
    if (fault_latched && !nsw5v_high) {
        return PICOFW_HP_DROPOUT_FAULT;
    }
    if (nsw5v_high &&
        (vdd_mv >= PICOFW_SSOT_VDD_WAKE_MIN_MV) &&
        (vdd_mv < PICOFW_SSOT_VDD_FLOATING_CEIL_MV)) {
        return PICOFW_HP_ONLINE_AWAKE;
    }
    if (vdd_mv >= PICOFW_SSOT_VDD_PULLDOWN_MIN_MV) {
        return PICOFW_HP_STANDBY;
    }
    return PICOFW_HP_DETACHED;
}

void picofw_hotplug_fsm_step(
    picofw_hotplug_fsm_t *fsm,
    uint32_t vdd_mv,
    bool nsw5v_high,
    bool fault_latched,
    bool *out_changed,
    bool *out_clear_fault_latch
) {
    if (out_changed != NULL) {
        *out_changed = false;
    }
    if (out_clear_fault_latch != NULL) {
        *out_clear_fault_latch = false;
    }
    if (fsm == NULL) {
        return;
    }

    fsm->last_vdd_mv = vdd_mv;
    fsm->last_nsw5v = nsw5v_high;

    const picofw_hotplug_state_t raw = classify_instant(vdd_mv, nsw5v_high, fault_latched);

    /* Instantaneous lockout/dropout fault transitions immediately without waiting 5 polls. */
    if (raw == PICOFW_HP_DROPOUT_FAULT) {
        fsm->candidate = raw;
        fsm->stable_count = PICOFW_HOTPLUG_DEBOUNCE_SAMPLES;
        if (fsm->state != PICOFW_HP_DROPOUT_FAULT) {
            fsm->prev_state = fsm->state;
            fsm->state = PICOFW_HP_DROPOUT_FAULT;
            fsm->transitions += 1u;
            if (out_changed != NULL) {
                *out_changed = true;
            }
        }
        return;
    }

    if (raw == fsm->state) {
        fsm->candidate = raw;
        fsm->stable_count = PICOFW_HOTPLUG_DEBOUNCE_SAMPLES;
        if (fault_latched && (raw == PICOFW_HP_ONLINE_AWAKE)) {
            fsm->recoveries += 1u;
            if (out_clear_fault_latch != NULL) {
                *out_clear_fault_latch = true;
            }
        }
        return;
    }

    if (raw != fsm->candidate) {
        fsm->candidate = raw;
        fsm->stable_count = 1u;
        return;
    }

    if (fsm->stable_count < PICOFW_HOTPLUG_DEBOUNCE_SAMPLES) {
        fsm->stable_count += 1u;
    }
    if (fsm->stable_count < PICOFW_HOTPLUG_DEBOUNCE_SAMPLES) {
        return;
    }

    const picofw_hotplug_state_t prev = fsm->state;
    fsm->prev_state = prev;
    fsm->state = raw;
    fsm->transitions += 1u;
    if ((raw == PICOFW_HP_ONLINE_AWAKE) && (fault_latched || (prev == PICOFW_HP_DROPOUT_FAULT))) {
        fsm->recoveries += 1u;
        if (out_clear_fault_latch != NULL) {
            *out_clear_fault_latch = true;
        }
    }
    if (out_changed != NULL) {
        *out_changed = true;
    }
}

size_t picofw_hotplug_format_kv(
    const picofw_hotplug_fsm_t *fsm,
    bool fault_latched,
    uint32_t dropouts,
    char *out,
    size_t cap
) {
    if ((fsm == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    const int n = snprintf(
        out,
        cap,
        "event=%s prev=%s vdd_mv=%" PRIu32 " nsw5v=%u fault_latch=%u dropouts=%" PRIu32
        " recoveries=%" PRIu32 " transitions=%" PRIu32 " push=%s",
        picofw_hotplug_state_name(fsm->state),
        picofw_hotplug_state_name(fsm->prev_state),
        fsm->last_vdd_mv,
        fsm->last_nsw5v ? 1u : 0u,
        fault_latched ? 1u : 0u,
        dropouts,
        fsm->recoveries,
        fsm->transitions,
        fsm->push_enabled ? "ON" : "OFF"
    );
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}
