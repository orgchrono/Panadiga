/**
 * @file p7501_probe_fsm.h
 * @brief Pure C11 deterministic Probe State Machine (`step(state, ev, snap) -> (next, effects)`).
 *
 * Host-testable with zero heap, zero float, zero `volatile`, and zero Pico SDK dependencies.
 * All hardware I/O lives in the imperative shell (`hal/p7501_hw.c`, `cli/p7501_cli_sys.c`,
 * `uart/p7501_uart_tx.c`, `p7501_pico_main.c`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PROBE_FSM_H
#define P7501_PROBE_FSM_H

#include "generated/p7501_ssot.h"

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum p7501_ps {
    P7501_PS_ISOLATED = 0,
    P7501_PS_ENGAGED = 1,
    P7501_PS_BRIDGE = 2,
    P7501_PS_GDB = 3,
    P7501_PS_ACTIVE_OP = 4,
    P7501_PS_FAULT = 5,
    P7501_PS_COUNT = 6
} p7501_ps_t;

typedef enum p7501_ev {
    P7501_EV_TICK = 0,
    P7501_EV_ISOLATE = 1,
    P7501_EV_ENGAGE = 2,
    P7501_EV_BRIDGE_ENTER = 3,
    P7501_EV_BRIDGE_EXIT = 4,
    P7501_EV_GDB_ENTER = 5,
    P7501_EV_GDB_DETACH = 6,
    P7501_EV_OP_BEGIN = 7,
    P7501_EV_OP_END = 8,
    P7501_EV_FAULT_ACK = 9,
    P7501_EV_COUNT = 10
} p7501_ev_t;

typedef struct p7501_rail_snap {
    uint32_t vdd_mv;
    uint32_t dropouts;
    uint32_t acked_dropouts;
    bool nsw5v_high;
    bool jig_strapped_low;
} p7501_rail_snap_t;

enum p7501_eff_bit {
    P7501_EFF_NONE = 0u,
    P7501_EFF_HIZ_ALL = (1u << 0),
    P7501_EFF_PIO_END = (1u << 1),
    P7501_EFF_KEEPALIVE_OFF = (1u << 2),
    P7501_EFF_ENGAGE_TX = (1u << 3),
    P7501_EFF_PIO_BEGIN = (1u << 4),
    P7501_EFF_UPPER_NOR_LOCK = (1u << 5),
    P7501_EFF_EMIT_FAULT = (1u << 6),
    P7501_EFF_ACK_FAULT = (1u << 7)
};

typedef struct p7501_step_res {
    p7501_ps_t next;
    uint32_t effects;
    bool allowed;
} p7501_step_res_t;

/** Pure fault-latch predicate (`dropouts != acked_dropouts`, rollover-safe). */
bool p7501_rail_is_latched(const p7501_rail_snap_t *snap);

/** Pure target rail safety predicate (`!latched && nsw5v_high && vdd_mv in [MIN, CEIL)`). */
bool p7501_rail_is_healthy(const p7501_rail_snap_t *snap);

/** Pure state transition function `step(state, ev, snap) -> (next, effects, allowed)`. */
p7501_step_res_t p7501_probe_step(
    p7501_ps_t state,
    p7501_ev_t ev,
    const p7501_rail_snap_t *snap
);

/** Human-readable static name for `p7501_ps_t`. */
const char *p7501_probe_state_name(p7501_ps_t state);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PROBE_FSM_H */
