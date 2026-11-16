/**
 * @file p7501_uart_pio_engine.c
 * @brief Multi-baud PIO1 RX/TX engine delegating GPIO control strictly to `p7501_hw`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_pio_engine.h"
#include "uart/p7501_uart_pio_internal.h"

#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"

#include "hardware/clocks.h"
#include "hardware/gpio.h"
#include "hardware/pio.h"
#include "pico/stdlib.h"
#include "p7501_uart_duplex.pio.h"

typedef struct p7501_pio_engine_drv {
    uint32_t active_baud;
    p7501_uart_framing_t active_framing;
    uint32_t parity_err_count;
    bool rx_loaded;
    bool sw_uart_timed_hold;
    absolute_time_t sw_uart_release_deadline;
    uint rx_offset;
    bool tx_loaded;
    uint tx_offset;
} p7501_pio_engine_drv_t;

static p7501_pio_engine_drv_t s_pio_drv = {
    .active_baud = P7501_UART_DEFAULT_BAUD,
    .active_framing = P7501_FRAMING_8N1,
    .parity_err_count = 0u,
    .rx_loaded = false,
    .sw_uart_timed_hold = false,
    .rx_offset = 0u,
    .tx_loaded = false,
    .tx_offset = 0u,
};

static uint framing_bit_loop_count(void) {
    return (s_pio_drv.active_framing == P7501_FRAMING_8N1) ? 7u : 8u;
}

static float compute_pio_clkdiv(uint32_t baud) {
    uint16_t div_i = 0u;
    uint8_t div_f = 0u;
    const uint32_t sys_hz = clock_get_hz(clk_sys);

    p7501_calc_pio_clkdiv_fp16(sys_hz, baud, &div_i, &div_f);
    return (float)div_i + ((float)div_f / 256.0f);
}

static void ensure_programs_loaded(void) {
    if (!s_pio_drv.rx_loaded) {
        s_pio_drv.rx_offset = pio_add_program(P7501_PIO_UART_INST, &p7501_uart_rx_fast_program);
        s_pio_drv.rx_loaded = true;
    }
    if (!s_pio_drv.tx_loaded) {
        s_pio_drv.tx_offset = pio_add_program(P7501_PIO_UART_INST, &p7501_uart_tx_fast_program);
        s_pio_drv.tx_loaded = true;
    }
}

static void clear_rx_diagnostics(void) {
    pio_sm_clear_fifos(P7501_PIO_UART_INST, P7501_PIO_RX_SM);
    P7501_PIO_UART_INST->fdebug = (1u << (PIO_FDEBUG_RXSTALL_LSB + P7501_PIO_RX_SM));
    pio_interrupt_clear(P7501_PIO_UART_INST, 4u);
    s_pio_drv.parity_err_count = 0u;
}

void p7501_pio_internal_prepare_tx(p7501_pio_tx_snapshot_t *out_snap) {
    ensure_programs_loaded();
    out_snap->baud = s_pio_drv.active_baud;
    out_snap->framing = s_pio_drv.active_framing;
    out_snap->clkdiv = compute_pio_clkdiv(s_pio_drv.active_baud);
    out_snap->bit_loops = framing_bit_loop_count();
    out_snap->tx_offset = s_pio_drv.tx_offset;
}

void p7501_pio_enforce_nsw5v_hiz(void) {
    p7501_hw_enforce_nsw5v_hiz();
}

void p7501_pio_set_sw_uart(bool drive_low) {
    p7501_hw_sw_uart_set_low(drive_low);
}

void p7501_pio_pulse_xreset_ms(uint32_t hold_low_ms) {
    p7501_hw_pulse_xreset_ms(hold_low_ms);
}

uint32_t p7501_pio_get_baud(void) {
    return s_pio_drv.active_baud;
}

bool p7501_pio_set_baud(uint32_t baud) {
    if (!p7501_baud_is_valid(baud)) {
        return false;
    }
    s_pio_drv.active_baud = baud;
    if (s_pio_drv.rx_loaded) {
        pio_sm_set_clkdiv(P7501_PIO_UART_INST, P7501_PIO_RX_SM, compute_pio_clkdiv(s_pio_drv.active_baud));
    }
    if (s_pio_drv.tx_loaded) {
        pio_sm_set_clkdiv(P7501_PIO_UART_INST, P7501_PIO_TX_SM, compute_pio_clkdiv(s_pio_drv.active_baud));
    }
    return true;
}

p7501_uart_framing_t p7501_pio_get_framing(void) {
    return s_pio_drv.active_framing;
}

bool p7501_pio_set_framing(p7501_uart_framing_t framing) {
    if (framing != P7501_FRAMING_8N1 &&
        framing != P7501_FRAMING_8E1 &&
        framing != P7501_FRAMING_8O1) {
        return false;
    }
    s_pio_drv.active_framing = framing;
    s_pio_drv.parity_err_count = 0u;
    const uint loops = framing_bit_loop_count();
    if (s_pio_drv.rx_loaded) {
        const bool was_enabled = ((P7501_PIO_UART_INST->ctrl >> P7501_PIO_RX_SM) & 1u) != 0u;
        pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_RX_SM, false);
        clear_rx_diagnostics();
        pio_sm_exec(P7501_PIO_UART_INST, P7501_PIO_RX_SM, pio_encode_set(pio_y, loops));
        pio_sm_restart(P7501_PIO_UART_INST, P7501_PIO_RX_SM);
        if (was_enabled) {
            pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_RX_SM, true);
        }
    }
    if (s_pio_drv.tx_loaded) {
        pio_sm_exec(P7501_PIO_UART_INST, P7501_PIO_TX_SM, pio_encode_set(pio_y, loops));
    }
    return true;
}

void p7501_pio_begin_rx_session(p7501_boot_action_t action) {
    p7501_pio_begin_rx_session_ex(action, 0u);
}

void p7501_pio_begin_rx_session_ex(p7501_boot_action_t action, uint32_t jig_hold_ms) {
    const float div = compute_pio_clkdiv(s_pio_drv.active_baud);

    s_pio_drv.sw_uart_timed_hold = false;
    ensure_programs_loaded();
    p7501_hw_enforce_nsw5v_hiz();
    p7501_pio_internal_release_tx_hiz();

    /* Hold target in reset BEFORE enabling RX SM so no stale pre-reset bytes pollute FIFO. */
    if (action != P7501_BOOT_ACTION_PASSIVE) {
        p7501_hw_sw_uart_set_low(action == P7501_BOOT_ACTION_JIG);
        p7501_wdt_sleep_ms(P7501_RESET_POST_SETTLE_MS);
        p7501_hw_reset_assert();
        p7501_wdt_sleep_ms(P7501_RESET_DIAG_ASSERT_MS);
    }

    p7501_uart_rx_fast_init(
        P7501_PIO_UART_INST,
        P7501_PIO_RX_SM,
        s_pio_drv.rx_offset,
        P7501_PIN_RXD,
        div,
        framing_bit_loop_count()
    );
    clear_rx_diagnostics();

    if (action != P7501_BOOT_ACTION_PASSIVE) {
        p7501_hw_reset_release();
        if (action == P7501_BOOT_ACTION_JIG && jig_hold_ms > 0u) {
            s_pio_drv.sw_uart_timed_hold = true;
            s_pio_drv.sw_uart_release_deadline = make_timeout_time_ms(jig_hold_ms);
        }
    }
}

void p7501_pio_release_sw_uart_now(void) {
    s_pio_drv.sw_uart_timed_hold = false;
    p7501_hw_restore_sw_uart_adc0();
}

bool p7501_pio_poll_sw_uart_release(void) {
    if (!s_pio_drv.sw_uart_timed_hold || !time_reached(s_pio_drv.sw_uart_release_deadline)) {
        return false;
    }
    p7501_pio_release_sw_uart_now();
    return true;
}

bool p7501_pio_poll_jig_serial_release(
    uint8_t *ring,
    size_t *ring_len,
    size_t ring_cap,
    uint8_t rx_byte
) {
    if (!s_pio_drv.sw_uart_timed_hold || (ring == NULL) || (ring_len == NULL)) {
        return false;
    }
    if (!p7501_rolling_match_push(ring, ring_len, ring_cap, rx_byte, "serial boot mode")) {
        return false;
    }
    p7501_pio_release_sw_uart_now();
    return true;
}

void p7501_pio_end_session(void) {
    s_pio_drv.sw_uart_timed_hold = false;
    pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_RX_SM, false);
    p7501_pio_internal_release_tx_hiz();
    p7501_hw_isolate_bus_hiz();
}

bool p7501_pio_pop_rx_byte_ex(uint8_t *out_byte, bool *out_parity_err) {
    if (pio_sm_is_rx_fifo_empty(P7501_PIO_UART_INST, P7501_PIO_RX_SM)) {
        return false;
    }
    const uint32_t raw = pio_sm_get(P7501_PIO_UART_INST, P7501_PIO_RX_SM);
    bool parity_ok = true;
    p7501_uart_decode_rx_word(raw, s_pio_drv.active_framing, out_byte, &parity_ok);
    if (!parity_ok) {
        ++s_pio_drv.parity_err_count;
    }
    if (out_parity_err != NULL) {
        *out_parity_err = !parity_ok;
    }
    return true;
}

bool p7501_pio_pop_rx_byte(uint8_t *out_byte) {
    return p7501_pio_pop_rx_byte_ex(out_byte, NULL);
}

uint32_t p7501_pio_get_and_clear_parity_errors(void) {
    const uint32_t count = s_pio_drv.parity_err_count;
    s_pio_drv.parity_err_count = 0u;
    return count;
}

void p7501_pio_drain_rx(void) {
    uint8_t dummy = 0u;
    while (p7501_pio_pop_rx_byte(&dummy)) {
    }
    clear_rx_diagnostics();
}

bool p7501_pio_poll_and_clear_framing_err(void) {
    if (pio_interrupt_get(P7501_PIO_UART_INST, 4u)) {
        pio_interrupt_clear(P7501_PIO_UART_INST, 4u);
        return true;
    }
    return false;
}

bool p7501_pio_had_rx_overflow(void) {
    const uint32_t mask = (1u << (PIO_FDEBUG_RXSTALL_LSB + P7501_PIO_RX_SM));
    return (P7501_PIO_UART_INST->fdebug & mask) != 0u;
}
