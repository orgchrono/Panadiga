/**
 * @file picofw_bootsel_logic.h
 * @brief Pure ISO C11 BOOTSEL button short-press (`IDENTIFY`+`BLACKBOX`) / long-press (`VAULT_BACKUP`) FSM.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_BOOTSEL_LOGIC_H
#define PICOFW_BOOTSEL_LOGIC_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_BOOTSEL_DEBOUNCE_MS    50u
#define PICOFW_BOOTSEL_SHORT_MAX_MS   1500u
#define PICOFW_BOOTSEL_LONG_MIN_MS    3000u

typedef enum picofw_bootsel_event {
    PICOFW_BOOTSEL_EVT_NONE = 0,
    PICOFW_BOOTSEL_EVT_SHORT_IDENTIFY = 1,
    PICOFW_BOOTSEL_EVT_LONG_VAULT_BACKUP = 2
} picofw_bootsel_event_t;

typedef struct picofw_bootsel_fsm {
    bool is_pressed;
    bool long_fired;
    uint32_t press_start_ms;
} picofw_bootsel_fsm_t;

/** Initialize BOOTSEL press state machine. */
void picofw_bootsel_fsm_init(picofw_bootsel_fsm_t *fsm);

/**
 * Step the BOOTSEL state machine with current button state (`pressed`) and timestamp `now_ms`.
 * Fires `PICOFW_BOOTSEL_EVT_LONG_VAULT_BACKUP` as soon as held >= 3000 ms, or
 * `PICOFW_BOOTSEL_EVT_SHORT_IDENTIFY` on release after 50..1500 ms.
 */
picofw_bootsel_event_t picofw_bootsel_fsm_step(
    picofw_bootsel_fsm_t *fsm,
    bool pressed,
    uint32_t now_ms
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_BOOTSEL_LOGIC_H */
