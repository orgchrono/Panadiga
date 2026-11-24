/**
 * @file p7501_hw.h
 * @brief Hardware Abstraction Layer (HAL) for Panasonic DMR-EH55 P7501 interface.
 *
 * Electrical Safety Guarantees:
 *   - GP3 (Pin 5 NSW5V / X_SW_5R2V) is strictly locked as Hi-Z Input (never driven).
 *   - GP5 (Pin 2 XRESET) is open-drain active-LOW only (never driven push-pull 3.3V).
 *   - GP2 (Pin 6 UARTP2M_T) is always input (115200 8N1 RX from SoC).
 *   - GP26 (Pin 1 SW_UART / ADC0) clears SIO OE before switching back to Hi-Z ADC0.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_HW_H
#define P7501_HW_H

#include "p7501_types.h"
#include "common/p7501_probe_fsm.h"
#include "diag/p7501_hotplug_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Target debug/memory transport VTable decoupling core logic from physical wire protocol.
 *
 * All callbacks default to fail-closed asynchronous/offline responders (`OPEN_HIGH`)
 * or register the PIO1 async UART `$m`/`$M` monitor backend (`p7501_target_bus_pio.c`).
 */
typedef struct p7501_target_bus_ops {
    uint32_t (*read_mem32)(void *ctx, uint32_t addr);
    void     (*write_mem32)(void *ctx, uint32_t addr, uint32_t data);
    bool     (*is_online)(void *ctx);
    bool     (*is_halted)(void *ctx);
    uint32_t (*read_dcsr)(void *ctx);
    void     (*set_pc)(void *ctx, uint32_t pc_val);
    void     (*resume_cpu)(void *ctx);
    void     *ctx;
    bool     (*read_mem32_checked)(void *ctx, uint32_t addr, uint32_t *out_word);
    bool     (*read_block_checked)(
        void *ctx,
        uint32_t addr,
        uint8_t *out_buf,
        uint32_t len,
        bool allow_abort
    );
} p7501_target_bus_ops_t;

void p7501_target_bus_register_ops(const p7501_target_bus_ops_t *ops);
void p7501_target_bus_reset_default_ops(void);
void p7501_target_bus_init_pio(void);
uint32_t p7501_hw_read_mem32(uint32_t addr);
bool p7501_hw_read_mem32_checked(uint32_t addr, uint32_t *out_word);
bool p7501_hw_read_block_checked(uint32_t addr, uint8_t *out_buf, uint32_t len, bool allow_abort);
void p7501_hw_write_mem32(uint32_t addr, uint32_t data);
bool p7501_hw_is_target_online(void);
bool p7501_hw_is_cpu_halted(void);
uint32_t p7501_hw_read_dcsr(void);
void p7501_hw_set_pc(uint32_t pc_val);
void p7501_hw_resume_cpu(void);

/** Initialize ADC0, safe Hi-Z P7501 GPIOs, and NSW5V falling-edge anti-backpowering IRQ. */
p7501_status_t p7501_hw_init(void);

/** Enforce strict electrical Hi-Z invariant on Pin 5 NSW5V (GP3) with pulls disabled. */
void p7501_hw_enforce_nsw5v_hiz(void);

/** Return number of NSW5V (`GP3`) falling-edge dropouts observed since boot. */
uint32_t p7501_hw_get_nsw5v_dropouts(void);

/** Return true if NSW5V anti-backpowering Hi-Z latch fired since last clear. */
bool p7501_hw_is_nsw5v_fault_latched(void);

/** Clear NSW5V fault latch after host acknowledgment when rail is verified healthy; returns true on ack. */
bool p7501_hw_ack_fault(void);

/** Legacy void wrapper delegating to `p7501_hw_ack_fault()`. */
void p7501_hw_clear_nsw5v_fault_latch(void);

/** Capture one consistent snapshot of rail voltages, NSW5V, and dropout/ack counters. */
void p7501_hw_rail_snapshot(p7501_rail_snap_t *out, bool active_vdd_sample);

/** True when `!fault_latched && nsw5v_high && target_powered` (strict rail safety gate). */
bool p7501_hw_is_rail_healthy(void);

/** Return total automatic NSW5V fault-latch recoveries performed in idle loop. */
uint32_t p7501_hw_get_nsw5v_recoveries(void);

/** Access the probe-wide debounced P7501 hotplug FSM state. */
p7501_hotplug_fsm_t *p7501_hw_hotplug_fsm(void);

/** Poll Hi-Z NSW5V/ADC0 at >=50ms cadence and auto-heal fault latch + LED Fault mode. */
void p7501_hw_poll_rail_recovery(void);

/** Return true if a LOW-drive pin contention fault (`ECONTENTION`) was detected on GP4/GP26. */
bool p7501_hw_is_contention_latched(void);

/** Clear the electrical contention latch after fault resolution. */
void p7501_hw_clear_contention_latch(void);

/** Assert XRESET (GP5) active LOW via Open-Drain emulation (0V output). */
void p7501_hw_reset_assert(void);

/** Release XRESET (GP5) back to High-Z Input with internal pulls disabled. */
void p7501_hw_reset_release(void);

/** Execute open-drain XRESET pulse of `hold_low_ms` (`P7501_RESET_DIAG_ASSERT_MS` if 0) safely. */
void p7501_hw_pulse_xreset_ms(uint32_t hold_low_ms);

/** Drive TXD (GP4) LOW for UART Break / pre-boot strap pulses (releasing pulls). */
void p7501_hw_drive_txd_low(void);

/** Release TXD (GP4) back to idle input with optional weak pull-up. */
void p7501_hw_release_txd_hiz(bool with_pull_up);

/** Safely control GP26 (Pin 1 SW_UART): LOW outputs 0V to enter XFWE serial boot; false restores Hi-Z ADC0. */
void p7501_hw_sw_uart_set_low(bool assert_low);

/** Restore GP26 cleanly from SIO digital mode back to High-Z ADC0 without sticky OE latch. */
void p7501_hw_restore_sw_uart_adc0(void);

/** Sample Pin 1 (SW_UART / GP26) in integer millivolts via ADC0 with selectable internal pull state. */
uint32_t p7501_hw_sample_sw_uart_mv(bool pull_down, bool pull_up);

/** Sample Pin 1 (SW_UART / GP26) in integer millivolts with active pull-down floating check. */
uint32_t p7501_hw_read_vdd_mv(void);

/** Check whether P7501 Pin 1 voltage exceeds P7501_SSOT_VDD_MIN_MV (2500 mV). */
bool p7501_hw_is_target_powered(void);

/** Passive zero-glitch digital check of GP3 (`NSW5V` Hi-Z Input) without pulsing GP26 pull-down. */
bool p7501_hw_is_nsw5v_high(void);

/** Return true when `GP26 (SW_UART)` is currently driven LOW (`JIGPIN LOW`). */
bool p7501_hw_is_sw_uart_forced_low(void);

/** Check whether P7501 hardware HAL has been initialized. */
bool p7501_hw_is_initialized(void);

/** Execute a discrete P7501 SW_UART + XRESET boot/reset profile (Modes 0..3). */
void p7501_hw_boot_sequence(p7501_boot_mode_t mode);

/** Place all 5 digital/analog P7501 pins safely into High-Z Isolation (NSW5V Hi-Z, XRESET Hi-Z, GP26 ADC0). */
void p7501_hw_isolate_bus_hiz(void);

typedef struct p7501_bus_pin_sample {
    bool jig_latched;
    bool nsw5v_high;
    bool rxd_high;
    bool txd_high;
    bool xreset_high;
    uint32_t vdd_mv;
    uint32_t sw_pu_mv;
} p7501_bus_pin_sample_t;

/** Capture live Hi-Z P7501 electrical pin levels with anti-backpower pull-up guard. */
void p7501_hw_sample_bus_pins(p7501_bus_pin_sample_t *out);

/** Engage P7501 TX pin (GP4 UARTM2P_T) idle HIGH for active UART transmission, keeping NSW5V (GP3) Hi-Z. */
void p7501_hw_engage_uart_bus(void);

#ifdef __cplusplus
}
#endif

#endif /* P7501_HW_H */
