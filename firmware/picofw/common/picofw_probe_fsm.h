/**
 * @file picofw_probe_fsm.h
 * @brief Pure C11 deterministic Probe State Machine (`step(state, ev, snap) -> (next, effects)`).
 *
 * Host-testable with zero heap, zero float, zero `volatile`, and zero Pico SDK dependencies.
 * All hardware I/O lives in the imperative shell (`hal/picofw_hw.c`, `cli/picofw_cli_sys.c`,
 * `uart/picofw_uart_tx.c`, `picofw_pico_main.c`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PROBE_FSM_H
#define PICOFW_PROBE_FSM_H

#include "generated/picofw_ssot.h"

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum picofw_ps {
    PICOFW_PS_ISOLATED = 0,
    PICOFW_PS_ENGAGED = 1,
    PICOFW_PS_BRIDGE = 2,
    PICOFW_PS_GDB = 3,
    PICOFW_PS_ACTIVE_OP = 4,
    PICOFW_PS_FAULT = 5,
    PICOFW_PS_COUNT = 6
} picofw_ps_t;

typedef enum picofw_ev {
    PICOFW_EV_TICK = 0,
    PICOFW_EV_ISOLATE = 1,
    PICOFW_EV_ENGAGE = 2,
    PICOFW_EV_BRIDGE_ENTER = 3,
    PICOFW_EV_BRIDGE_EXIT = 4,
    PICOFW_EV_GDB_ENTER = 5,
    PICOFW_EV_GDB_DETACH = 6,
    PICOFW_EV_OP_BEGIN = 7,
    PICOFW_EV_OP_END = 8,
    PICOFW_EV_FAULT_ACK = 9,
    PICOFW_EV_COUNT = 10
} picofw_ev_t;

typedef struct picofw_rail_snap {
    uint32_t vdd_mv;
    uint32_t dropouts;
    uint32_t acked_dropouts;
    bool nsw5v_high;
    bool jig_strapped_low;
} picofw_rail_snap_t;

enum picofw_eff_bit {
    PICOFW_EFF_NONE = 0u,
    PICOFW_EFF_HIZ_ALL = (1u << 0),
    PICOFW_EFF_PIO_END = (1u << 1),
    PICOFW_EFF_KEEPALIVE_OFF = (1u << 2),
    PICOFW_EFF_ENGAGE_TX = (1u << 3),
    PICOFW_EFF_PIO_BEGIN = (1u << 4),
    PICOFW_EFF_UPPER_NOR_LOCK = (1u << 5),
    PICOFW_EFF_EMIT_FAULT = (1u << 6),
    PICOFW_EFF_ACK_FAULT = (1u << 7)
};

typedef struct picofw_step_res {
    picofw_ps_t next;
    uint32_t effects;
    bool allowed;
} picofw_step_res_t;

/** Pure fault-latch predicate (`dropouts != acked_dropouts`, rollover-safe). */
bool picofw_rail_is_latched(const picofw_rail_snap_t *snap);

/** Pure target rail safety predicate (`!latched && nsw5v_high && vdd_mv in [MIN, CEIL)`). */
bool picofw_rail_is_healthy(const picofw_rail_snap_t *snap);

/** Pure state transition function `step(state, ev, snap) -> (next, effects, allowed)`. */
picofw_step_res_t picofw_probe_step(
    picofw_ps_t state,
    picofw_ev_t ev,
    const picofw_rail_snap_t *snap
);

/** Human-readable static name for `picofw_ps_t`. */
const char *picofw_probe_state_name(picofw_ps_t state);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PROBE_FSM_H */
