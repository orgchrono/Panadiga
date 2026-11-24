/**
 * @file p7501_probe_fsm.c
 * @brief Pure C11 Probe State Machine implementation (zero I/O, zero heap).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_probe_fsm.h"

#include <stddef.h>

#define FAULT_ISOLATE_EFFECTS \
    (P7501_EFF_HIZ_ALL | P7501_EFF_PIO_END | P7501_EFF_KEEPALIVE_OFF | P7501_EFF_UPPER_NOR_LOCK)

static bool rail_vdd_ok(const p7501_rail_snap_t *snap) {
    return (snap != NULL) &&
           (snap->vdd_mv >= P7501_SSOT_VDD_MIN_MV) &&
           (snap->vdd_mv < P7501_SSOT_VDD_FLOATING_CEIL_MV);
}

static bool rail_pins_ok(const p7501_rail_snap_t *snap) {
    return (snap != NULL) &&
           snap->nsw5v_high &&
           (snap->jig_strapped_low || rail_vdd_ok(snap));
}

bool p7501_rail_is_latched(const p7501_rail_snap_t *snap) {
    return (snap == NULL) || (snap->dropouts != snap->acked_dropouts);
}

bool p7501_rail_is_healthy(const p7501_rail_snap_t *snap) {
    return !p7501_rail_is_latched(snap) && rail_pins_ok(snap);
}

static bool is_active_bus_state(p7501_ps_t st) {
    return (st == P7501_PS_ENGAGED) ||
           (st == P7501_PS_BRIDGE) ||
           (st == P7501_PS_GDB) ||
           (st == P7501_PS_ACTIVE_OP);
}

p7501_step_res_t p7501_probe_step(
    p7501_ps_t state,
    p7501_ev_t ev,
    const p7501_rail_snap_t *snap
) {
    static const p7501_ps_t k_enter_next[P7501_EV_COUNT] = {
        [P7501_EV_TICK] = P7501_PS_ISOLATED,
        [P7501_EV_ISOLATE] = P7501_PS_ISOLATED,
        [P7501_EV_ENGAGE] = P7501_PS_ENGAGED,
        [P7501_EV_BRIDGE_ENTER] = P7501_PS_BRIDGE,
        [P7501_EV_BRIDGE_EXIT] = P7501_PS_ISOLATED,
        [P7501_EV_GDB_ENTER] = P7501_PS_GDB,
        [P7501_EV_GDB_DETACH] = P7501_PS_ISOLATED,
        [P7501_EV_OP_BEGIN] = P7501_PS_ACTIVE_OP,
        [P7501_EV_OP_END] = P7501_PS_ISOLATED,
        [P7501_EV_FAULT_ACK] = P7501_PS_ISOLATED,
    };

    const p7501_ps_t cur = ((unsigned)state < (unsigned)P7501_PS_COUNT) ? state : P7501_PS_FAULT;
    if ((snap == NULL) || ((unsigned)ev >= (unsigned)P7501_EV_COUNT)) {
        return (p7501_step_res_t){
            .next = P7501_PS_FAULT,
            .effects = FAULT_ISOLATE_EFFECTS | ((cur != P7501_PS_FAULT) ? P7501_EFF_EMIT_FAULT : 0u),
            .allowed = false,
        };
    }

    const bool latched = p7501_rail_is_latched(snap);
    const bool pins_ok = rail_pins_ok(snap);

    if (ev == P7501_EV_FAULT_ACK) {
        if (snap->nsw5v_high && !snap->jig_strapped_low && rail_vdd_ok(snap)) {
            return (p7501_step_res_t){
                .next = P7501_PS_ISOLATED,
                .effects = P7501_EFF_ACK_FAULT | P7501_EFF_HIZ_ALL,
                .allowed = true,
            };
        }
        return (p7501_step_res_t){
            .next = latched ? P7501_PS_FAULT : cur,
            .effects = FAULT_ISOLATE_EFFECTS,
            .allowed = false,
        };
    }

    if (latched) {
        const uint32_t emit = (cur != P7501_PS_FAULT) ? P7501_EFF_EMIT_FAULT : 0u;
        const bool benign_teardown = (ev == P7501_EV_ISOLATE) ||
                                     (ev == P7501_EV_BRIDGE_EXIT) ||
                                     (ev == P7501_EV_GDB_DETACH) ||
                                     (ev == P7501_EV_OP_END);
        return (p7501_step_res_t){
            .next = P7501_PS_FAULT,
            .effects = FAULT_ISOLATE_EFFECTS | emit,
            .allowed = benign_teardown,
        };
    }

    switch (ev) {
        case P7501_EV_TICK:
            if (is_active_bus_state(cur) && !snap->nsw5v_high) {
                return (p7501_step_res_t){
                    .next = P7501_PS_ISOLATED,
                    .effects = FAULT_ISOLATE_EFFECTS,
                    .allowed = false,
                };
            }
            return (p7501_step_res_t){
                .next = (cur == P7501_PS_FAULT) ? P7501_PS_ISOLATED : cur,
                .effects = (cur == P7501_PS_FAULT) ? P7501_EFF_HIZ_ALL : P7501_EFF_NONE,
                .allowed = true,
            };

        case P7501_EV_ISOLATE:
            return (p7501_step_res_t){
                .next = P7501_PS_ISOLATED,
                .effects = FAULT_ISOLATE_EFFECTS,
                .allowed = true,
            };

        case P7501_EV_GDB_DETACH:
            return (p7501_step_res_t){
                .next = P7501_PS_ISOLATED,
                .effects = FAULT_ISOLATE_EFFECTS,
                .allowed = true,
            };

        case P7501_EV_BRIDGE_EXIT:
            return (p7501_step_res_t){
                .next = P7501_PS_ISOLATED,
                .effects = P7501_EFF_PIO_END | P7501_EFF_HIZ_ALL,
                .allowed = true,
            };

        case P7501_EV_OP_END:
            return (p7501_step_res_t){
                .next = P7501_PS_ISOLATED,
                .effects = P7501_EFF_HIZ_ALL,
                .allowed = true,
            };

        case P7501_EV_ENGAGE:
        case P7501_EV_BRIDGE_ENTER:
        case P7501_EV_GDB_ENTER:
        case P7501_EV_OP_BEGIN:
            if (!pins_ok) {
                return (p7501_step_res_t){
                    .next = P7501_PS_ISOLATED,
                    .effects = FAULT_ISOLATE_EFFECTS,
                    .allowed = false,
                };
            }
            return (p7501_step_res_t){
                .next = k_enter_next[ev],
                .effects = (ev == P7501_EV_ENGAGE)
                    ? P7501_EFF_ENGAGE_TX
                    : ((ev == P7501_EV_BRIDGE_ENTER) ? P7501_EFF_PIO_BEGIN : P7501_EFF_NONE),
                .allowed = true,
            };

        default:
            return (p7501_step_res_t){
                .next = P7501_PS_FAULT,
                .effects = FAULT_ISOLATE_EFFECTS,
                .allowed = false,
            };
    }
}

const char *p7501_probe_state_name(p7501_ps_t state) {
    static const char *const k_names[P7501_PS_COUNT] = {
        [P7501_PS_ISOLATED] = "ISOLATED",
        [P7501_PS_ENGAGED] = "ENGAGED",
        [P7501_PS_BRIDGE] = "BRIDGE",
        [P7501_PS_GDB] = "GDB",
        [P7501_PS_ACTIVE_OP] = "ACTIVE_OP",
        [P7501_PS_FAULT] = "FAULT",
    };
    return ((unsigned)state < (unsigned)P7501_PS_COUNT) ? k_names[state] : "FAULT";
}
