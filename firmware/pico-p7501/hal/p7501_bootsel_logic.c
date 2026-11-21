/**
 * @file p7501_bootsel_logic.c
 * @brief Pure ISO C11 implementation of BOOTSEL short/long press classification.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/p7501_bootsel_logic.h"

#include <stddef.h>

void p7501_bootsel_fsm_init(p7501_bootsel_fsm_t *fsm) {
    if (fsm == NULL) {
        return;
    }
    fsm->is_pressed = false;
    fsm->long_fired = false;
    fsm->press_start_ms = 0u;
}

p7501_bootsel_event_t p7501_bootsel_fsm_step(
    p7501_bootsel_fsm_t *fsm,
    bool pressed,
    uint32_t now_ms
) {
    if (fsm == NULL) {
        return P7501_BOOTSEL_EVT_NONE;
    }
    if (pressed) {
        if (!fsm->is_pressed) {
            fsm->is_pressed = true;
            fsm->long_fired = false;
            fsm->press_start_ms = now_ms;
            return P7501_BOOTSEL_EVT_NONE;
        }
        const uint32_t held_ms = now_ms - fsm->press_start_ms;
        if (!fsm->long_fired && (held_ms >= P7501_BOOTSEL_LONG_MIN_MS)) {
            fsm->long_fired = true;
            return P7501_BOOTSEL_EVT_LONG_VAULT_BACKUP;
        }
        return P7501_BOOTSEL_EVT_NONE;
    }

    if (fsm->is_pressed) {
        const uint32_t held_ms = now_ms - fsm->press_start_ms;
        const bool was_long = fsm->long_fired;
        fsm->is_pressed = false;
        fsm->long_fired = false;
        if (!was_long && (held_ms >= P7501_BOOTSEL_DEBOUNCE_MS) &&
            (held_ms <= P7501_BOOTSEL_SHORT_MAX_MS)) {
            return P7501_BOOTSEL_EVT_SHORT_IDENTIFY;
        }
    }
    return P7501_BOOTSEL_EVT_NONE;
}
