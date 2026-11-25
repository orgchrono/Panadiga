/**
 * @file picofw_probe_fsm.c
 * @brief Pure C11 Probe State Machine implementation (zero I/O, zero heap).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/picofw_probe_fsm.h"

#include <stddef.h>

#define FAULT_ISOLATE_EFFECTS \
    (PICOFW_EFF_HIZ_ALL | PICOFW_EFF_PIO_END | PICOFW_EFF_KEEPALIVE_OFF | PICOFW_EFF_UPPER_NOR_LOCK)

static bool rail_vdd_ok(const picofw_rail_snap_t *snap) {
    return (snap != NULL) &&
           (snap->vdd_mv >= PICOFW_SSOT_VDD_MIN_MV) &&
           (snap->vdd_mv < PICOFW_SSOT_VDD_FLOATING_CEIL_MV);
}

static bool rail_pins_ok(const picofw_rail_snap_t *snap) {
    return (snap != NULL) &&
           snap->nsw5v_high &&
           (snap->jig_strapped_low || rail_vdd_ok(snap));
}

bool picofw_rail_is_latched(const picofw_rail_snap_t *snap) {
    return (snap == NULL) || (snap->dropouts != snap->acked_dropouts);
}

bool picofw_rail_is_healthy(const picofw_rail_snap_t *snap) {
    return !picofw_rail_is_latched(snap) && rail_pins_ok(snap);
}

static bool is_active_bus_state(picofw_ps_t st) {
    return (st == PICOFW_PS_ENGAGED) ||
           (st == PICOFW_PS_BRIDGE) ||
           (st == PICOFW_PS_GDB) ||
           (st == PICOFW_PS_ACTIVE_OP);
}

picofw_step_res_t picofw_probe_step(
    picofw_ps_t state,
    picofw_ev_t ev,
    const picofw_rail_snap_t *snap
) {
    static const picofw_ps_t k_enter_next[PICOFW_EV_COUNT] = {
        [PICOFW_EV_TICK] = PICOFW_PS_ISOLATED,
        [PICOFW_EV_ISOLATE] = PICOFW_PS_ISOLATED,
        [PICOFW_EV_ENGAGE] = PICOFW_PS_ENGAGED,
        [PICOFW_EV_BRIDGE_ENTER] = PICOFW_PS_BRIDGE,
        [PICOFW_EV_BRIDGE_EXIT] = PICOFW_PS_ISOLATED,
        [PICOFW_EV_GDB_ENTER] = PICOFW_PS_GDB,
        [PICOFW_EV_GDB_DETACH] = PICOFW_PS_ISOLATED,
        [PICOFW_EV_OP_BEGIN] = PICOFW_PS_ACTIVE_OP,
        [PICOFW_EV_OP_END] = PICOFW_PS_ISOLATED,
        [PICOFW_EV_FAULT_ACK] = PICOFW_PS_ISOLATED,
    };

    const picofw_ps_t cur = ((unsigned)state < (unsigned)PICOFW_PS_COUNT) ? state : PICOFW_PS_FAULT;
    if ((snap == NULL) || ((unsigned)ev >= (unsigned)PICOFW_EV_COUNT)) {
        return (picofw_step_res_t){
            .next = PICOFW_PS_FAULT,
            .effects = FAULT_ISOLATE_EFFECTS | ((cur != PICOFW_PS_FAULT) ? PICOFW_EFF_EMIT_FAULT : 0u),
            .allowed = false,
        };
    }

    const bool latched = picofw_rail_is_latched(snap);
    const bool pins_ok = rail_pins_ok(snap);

    if (ev == PICOFW_EV_FAULT_ACK) {
        if (snap->nsw5v_high && !snap->jig_strapped_low && rail_vdd_ok(snap)) {
            return (picofw_step_res_t){
                .next = PICOFW_PS_ISOLATED,
                .effects = PICOFW_EFF_ACK_FAULT | PICOFW_EFF_HIZ_ALL,
                .allowed = true,
            };
        }
        return (picofw_step_res_t){
            .next = latched ? PICOFW_PS_FAULT : cur,
            .effects = FAULT_ISOLATE_EFFECTS,
            .allowed = false,
        };
    }

    if (latched) {
        const uint32_t emit = (cur != PICOFW_PS_FAULT) ? PICOFW_EFF_EMIT_FAULT : 0u;
        const bool benign_teardown = (ev == PICOFW_EV_ISOLATE) ||
                                     (ev == PICOFW_EV_BRIDGE_EXIT) ||
                                     (ev == PICOFW_EV_GDB_DETACH) ||
                                     (ev == PICOFW_EV_OP_END);
        return (picofw_step_res_t){
            .next = PICOFW_PS_FAULT,
            .effects = FAULT_ISOLATE_EFFECTS | emit,
            .allowed = benign_teardown,
        };
    }

    switch (ev) {
        case PICOFW_EV_TICK:
            if (is_active_bus_state(cur) && !snap->nsw5v_high) {
                return (picofw_step_res_t){
                    .next = PICOFW_PS_ISOLATED,
                    .effects = FAULT_ISOLATE_EFFECTS,
                    .allowed = false,
                };
            }
            return (picofw_step_res_t){
                .next = (cur == PICOFW_PS_FAULT) ? PICOFW_PS_ISOLATED : cur,
                .effects = (cur == PICOFW_PS_FAULT) ? PICOFW_EFF_HIZ_ALL : PICOFW_EFF_NONE,
                .allowed = true,
            };

        case PICOFW_EV_ISOLATE:
            return (picofw_step_res_t){
                .next = PICOFW_PS_ISOLATED,
                .effects = FAULT_ISOLATE_EFFECTS,
                .allowed = true,
            };

        case PICOFW_EV_GDB_DETACH:
            return (picofw_step_res_t){
                .next = PICOFW_PS_ISOLATED,
                .effects = FAULT_ISOLATE_EFFECTS,
                .allowed = true,
            };

        case PICOFW_EV_BRIDGE_EXIT:
            return (picofw_step_res_t){
                .next = PICOFW_PS_ISOLATED,
                .effects = PICOFW_EFF_PIO_END | PICOFW_EFF_HIZ_ALL,
                .allowed = true,
            };

        case PICOFW_EV_OP_END:
            return (picofw_step_res_t){
                .next = PICOFW_PS_ISOLATED,
                .effects = PICOFW_EFF_HIZ_ALL,
                .allowed = true,
            };

        case PICOFW_EV_ENGAGE:
        case PICOFW_EV_BRIDGE_ENTER:
        case PICOFW_EV_GDB_ENTER:
        case PICOFW_EV_OP_BEGIN:
            if (!pins_ok) {
                return (picofw_step_res_t){
                    .next = PICOFW_PS_ISOLATED,
                    .effects = FAULT_ISOLATE_EFFECTS,
                    .allowed = false,
                };
            }
            return (picofw_step_res_t){
                .next = k_enter_next[ev],
                .effects = (ev == PICOFW_EV_ENGAGE)
                    ? PICOFW_EFF_ENGAGE_TX
                    : ((ev == PICOFW_EV_BRIDGE_ENTER) ? PICOFW_EFF_PIO_BEGIN : PICOFW_EFF_NONE),
                .allowed = true,
            };

        default:
            return (picofw_step_res_t){
                .next = PICOFW_PS_FAULT,
                .effects = FAULT_ISOLATE_EFFECTS,
                .allowed = false,
            };
    }
}

const char *picofw_probe_state_name(picofw_ps_t state) {
    static const char *const k_names[PICOFW_PS_COUNT] = {
        [PICOFW_PS_ISOLATED] = "ISOLATED",
        [PICOFW_PS_ENGAGED] = "ENGAGED",
        [PICOFW_PS_BRIDGE] = "BRIDGE",
        [PICOFW_PS_GDB] = "GDB",
        [PICOFW_PS_ACTIVE_OP] = "ACTIVE_OP",
        [PICOFW_PS_FAULT] = "FAULT",
    };
    return ((unsigned)state < (unsigned)PICOFW_PS_COUNT) ? k_names[state] : "FAULT";
}
