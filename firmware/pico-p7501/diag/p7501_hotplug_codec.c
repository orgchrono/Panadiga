/**
 * @file p7501_hotplug_codec.c
 * @brief Pure C11 5-sample debounced P7501 hotplug & NSW5V self-healing state machine.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_hotplug_codec.h"
#include "generated/p7501_ssot.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

void p7501_hotplug_fsm_init(p7501_hotplug_fsm_t *fsm) {
    if (fsm == NULL) {
        return;
    }
    memset(fsm, 0, sizeof(*fsm));
    fsm->state = P7501_HP_DETACHED;
    fsm->prev_state = P7501_HP_DETACHED;
    fsm->candidate = P7501_HP_DETACHED;
    fsm->push_enabled = false;
}

const char *p7501_hotplug_state_name(p7501_hotplug_state_t st) {
    switch (st) {
        case P7501_HP_DETACHED:      return "DETACHED";
        case P7501_HP_STANDBY:       return "STANDBY";
        case P7501_HP_DEBOUNCING:    return "DEBOUNCING";
        case P7501_HP_ONLINE_AWAKE:  return "ONLINE_AWAKE";
        case P7501_HP_DROPOUT_FAULT: return "DROPOUT_FAULT";
        default:                     return "UNKNOWN";
    }
}

static p7501_hotplug_state_t classify_instant(uint32_t vdd_mv, bool nsw5v_high, bool fault_latched) {
    if (fault_latched && !nsw5v_high) {
        return P7501_HP_DROPOUT_FAULT;
    }
    if (nsw5v_high &&
        (vdd_mv >= P7501_SSOT_VDD_WAKE_MIN_MV) &&
        (vdd_mv < P7501_SSOT_VDD_FLOATING_CEIL_MV)) {
        return P7501_HP_ONLINE_AWAKE;
    }
    if (vdd_mv >= P7501_SSOT_VDD_PULLDOWN_MIN_MV) {
        return P7501_HP_STANDBY;
    }
    return P7501_HP_DETACHED;
}

void p7501_hotplug_fsm_step(
    p7501_hotplug_fsm_t *fsm,
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

    const p7501_hotplug_state_t raw = classify_instant(vdd_mv, nsw5v_high, fault_latched);

    /* Instantaneous lockout/dropout fault transitions immediately without waiting 5 polls. */
    if (raw == P7501_HP_DROPOUT_FAULT) {
        fsm->candidate = raw;
        fsm->stable_count = P7501_HOTPLUG_DEBOUNCE_SAMPLES;
        if (fsm->state != P7501_HP_DROPOUT_FAULT) {
            fsm->prev_state = fsm->state;
            fsm->state = P7501_HP_DROPOUT_FAULT;
            fsm->transitions += 1u;
            if (out_changed != NULL) {
                *out_changed = true;
            }
        }
        return;
    }

    if (raw == fsm->state) {
        fsm->candidate = raw;
        fsm->stable_count = P7501_HOTPLUG_DEBOUNCE_SAMPLES;
        if (fault_latched && (raw == P7501_HP_ONLINE_AWAKE)) {
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

    if (fsm->stable_count < P7501_HOTPLUG_DEBOUNCE_SAMPLES) {
        fsm->stable_count += 1u;
    }
    if (fsm->stable_count < P7501_HOTPLUG_DEBOUNCE_SAMPLES) {
        return;
    }

    const p7501_hotplug_state_t prev = fsm->state;
    fsm->prev_state = prev;
    fsm->state = raw;
    fsm->transitions += 1u;
    if ((raw == P7501_HP_ONLINE_AWAKE) && (fault_latched || (prev == P7501_HP_DROPOUT_FAULT))) {
        fsm->recoveries += 1u;
        if (out_clear_fault_latch != NULL) {
            *out_clear_fault_latch = true;
        }
    }
    if (out_changed != NULL) {
        *out_changed = true;
    }
}

size_t p7501_hotplug_format_kv(
    const p7501_hotplug_fsm_t *fsm,
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
        p7501_hotplug_state_name(fsm->state),
        p7501_hotplug_state_name(fsm->prev_state),
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
