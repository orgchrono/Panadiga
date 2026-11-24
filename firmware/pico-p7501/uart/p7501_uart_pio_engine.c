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
#include "hardware/dma.h"
#include "hardware/gpio.h"
#include "hardware/pio.h"
#include "pico/stdlib.h"
#include "p7501_uart_duplex.pio.h"

#include <string.h>

#define P7501_RX_DMA_WORDS          4096u
#define P7501_RX_DMA_RING_BITS      14u
/* 28-bit max transfer count (`MODE=0`), required on RP2350 where bits [31:28]=0xF selects ENDLESS mode. */
#define P7501_RX_DMA_TRANSFER_COUNT 0x0FFFFFFFUL

static uint32_t s_rx_dma_ring[P7501_RX_DMA_WORDS] __attribute__((aligned(16384)));

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
    int rx_dma_chan;
    bool rx_dma_claimed;
    bool rx_dma_active;
    bool rx_dma_overflow;
    uint32_t rx_dma_total_read;
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
    .rx_dma_chan = -1,
    .rx_dma_claimed = false,
    .rx_dma_active = false,
    .rx_dma_overflow = false,
    .rx_dma_total_read = 0u,
};

static uint framing_bit_loop_count(void) {
    return (s_pio_drv.active_framing == P7501_FRAMING_8N1) ? 7u : 8u;
}

static void compute_pio_clkdiv_int_frac(uint32_t baud, uint16_t *out_int, uint8_t *out_frac) {
    const uint32_t sys_hz = clock_get_hz(clk_sys);
    p7501_calc_pio_clkdiv_fp16(sys_hz, baud, out_int, out_frac);
}

static void ensure_programs_loaded(void) {
    if (!pio_sm_is_claimed(P7501_PIO_UART_INST, P7501_PIO_RX_SM)) {
        pio_sm_claim(P7501_PIO_UART_INST, P7501_PIO_RX_SM);
    }
    if (!pio_sm_is_claimed(P7501_PIO_UART_INST, P7501_PIO_TX_SM)) {
        pio_sm_claim(P7501_PIO_UART_INST, P7501_PIO_TX_SM);
    }
    if (!s_pio_drv.rx_loaded) {
        s_pio_drv.rx_offset = pio_add_program(P7501_PIO_UART_INST, &p7501_uart_rx_fast_program);
        s_pio_drv.rx_loaded = true;
    }
    if (!s_pio_drv.tx_loaded) {
        s_pio_drv.tx_offset = pio_add_program(P7501_PIO_UART_INST, &p7501_uart_tx_fast_program);
        s_pio_drv.tx_loaded = true;
    }
}

static void clear_pio_status_flags(void) {
    P7501_PIO_UART_INST->fdebug = (1u << (PIO_FDEBUG_RXSTALL_LSB + P7501_PIO_RX_SM));
    pio_interrupt_clear(P7501_PIO_UART_INST, 4u);
    s_pio_drv.parity_err_count = 0u;
    s_pio_drv.rx_dma_overflow = false;
}

static void clear_rx_diagnostics(void) {
    pio_sm_clear_fifos(P7501_PIO_UART_INST, P7501_PIO_RX_SM);
    clear_pio_status_flags();
    s_pio_drv.rx_dma_total_read = 0u;
}

static void start_rx_dma_ring(void) {
    if (!s_pio_drv.rx_dma_claimed) {
        const int ch = dma_claim_unused_channel(false);
        if (ch >= 0) {
            s_pio_drv.rx_dma_chan = ch;
            s_pio_drv.rx_dma_claimed = true;
        }
    }
    if (!s_pio_drv.rx_dma_claimed) {
        s_pio_drv.rx_dma_active = false;
        return;
    }
    const uint ch = (uint)s_pio_drv.rx_dma_chan;
    dma_channel_abort(ch);
    dma_channel_config cfg = dma_channel_get_default_config(ch);
    channel_config_set_transfer_data_size(&cfg, DMA_SIZE_32);
    channel_config_set_read_increment(&cfg, false);
    channel_config_set_write_increment(&cfg, true);
    channel_config_set_ring(&cfg, true, P7501_RX_DMA_RING_BITS);
    channel_config_set_dreq(&cfg, pio_get_dreq(P7501_PIO_UART_INST, P7501_PIO_RX_SM, false));

    s_pio_drv.rx_dma_total_read = 0u;
    s_pio_drv.rx_dma_overflow = false;
    s_pio_drv.rx_dma_active = true;
    dma_channel_configure(
        ch,
        &cfg,
        s_rx_dma_ring,
        &P7501_PIO_UART_INST->rxf[P7501_PIO_RX_SM],
        P7501_RX_DMA_TRANSFER_COUNT,
        true
    );
}

void p7501_pio_internal_prepare_tx(p7501_pio_tx_snapshot_t *out_snap) {
    ensure_programs_loaded();
    out_snap->baud = s_pio_drv.active_baud;
    out_snap->framing = s_pio_drv.active_framing;
    compute_pio_clkdiv_int_frac(s_pio_drv.active_baud, &out_snap->div_int, &out_snap->div_frac);
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
    uint16_t div_i = 0u;
    uint8_t div_f = 0u;
    compute_pio_clkdiv_int_frac(s_pio_drv.active_baud, &div_i, &div_f);
    if (s_pio_drv.rx_loaded) {
        pio_sm_set_clkdiv_int_frac(P7501_PIO_UART_INST, P7501_PIO_RX_SM, div_i, div_f);
    }
    if (s_pio_drv.tx_loaded) {
        pio_sm_set_clkdiv_int_frac(P7501_PIO_UART_INST, P7501_PIO_TX_SM, div_i, div_f);
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
        pio_sm_exec(P7501_PIO_UART_INST, P7501_PIO_RX_SM, pio_encode_jmp(s_pio_drv.rx_offset));
        if (was_enabled) {
            start_rx_dma_ring();
            pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_RX_SM, true);
        }
    }
    if (s_pio_drv.tx_loaded) {
        pio_sm_exec(P7501_PIO_UART_INST, P7501_PIO_TX_SM, pio_encode_set(pio_y, loops));
    }
    return true;
}

void p7501_pio_begin_rx_session(p7501_boot_action_t action) {
    p7501_pio_begin_rx_session_cfg(action, 0u, false);
}

void p7501_pio_begin_rx_session_ex(p7501_boot_action_t action, uint32_t jig_hold_ms) {
    p7501_pio_begin_rx_session_cfg(action, jig_hold_ms, false);
}

void p7501_pio_begin_rx_session_cfg(
    p7501_boot_action_t action,
    uint32_t jig_hold_ms,
    bool no_xreset
) {
    uint16_t div_i = 0u;
    uint8_t div_f = 0u;
    compute_pio_clkdiv_int_frac(s_pio_drv.active_baud, &div_i, &div_f);

    s_pio_drv.sw_uart_timed_hold = false;
    ensure_programs_loaded();
    p7501_hw_enforce_nsw5v_hiz();
    p7501_pio_internal_release_tx_hiz();

    /* Hold target in reset BEFORE enabling RX SM unless `no_xreset` (PREJIG standby strap). */
    if (action != P7501_BOOT_ACTION_PASSIVE) {
        p7501_hw_sw_uart_set_low(action == P7501_BOOT_ACTION_JIG);
        if (!no_xreset) {
            p7501_wdt_sleep_ms(P7501_RESET_POST_SETTLE_MS);
            p7501_hw_reset_assert();
            p7501_wdt_sleep_ms(P7501_RESET_DIAG_ASSERT_MS);
        }
    }

    p7501_uart_rx_fast_init(
        P7501_PIO_UART_INST,
        P7501_PIO_RX_SM,
        s_pio_drv.rx_offset,
        P7501_PIN_RXD,
        div_i,
        div_f,
        framing_bit_loop_count()
    );
    pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_RX_SM, false);
    clear_rx_diagnostics();
    start_rx_dma_ring();
    pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_RX_SM, true);

    if (action != P7501_BOOT_ACTION_PASSIVE) {
        if (!no_xreset) {
            p7501_hw_reset_release();
        }
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
    if (s_pio_drv.rx_dma_active && s_pio_drv.rx_dma_claimed) {
        dma_channel_abort((uint)s_pio_drv.rx_dma_chan);
        s_pio_drv.rx_dma_active = false;
    }
    pio_sm_set_enabled(P7501_PIO_UART_INST, P7501_PIO_RX_SM, false);
    p7501_pio_internal_release_tx_hiz();
    p7501_hw_isolate_bus_hiz();
}

bool p7501_pio_pop_rx_byte_ex(uint8_t *out_byte, bool *out_parity_err) {
    uint32_t raw = 0u;
    if (s_pio_drv.rx_dma_active && s_pio_drv.rx_dma_claimed) {
        const uint32_t rem = dma_channel_hw_addr((uint)s_pio_drv.rx_dma_chan)->transfer_count & P7501_RX_DMA_TRANSFER_COUNT;
        const uint32_t total_written = P7501_RX_DMA_TRANSFER_COUNT - rem;
        const uint32_t pending = total_written - s_pio_drv.rx_dma_total_read;
        if (pending == 0u) {
            return false;
        }
        if (pending > P7501_RX_DMA_WORDS) {
            s_pio_drv.rx_dma_overflow = true;
            s_pio_drv.rx_dma_total_read = total_written - (P7501_RX_DMA_WORDS - 1u);
        }
        raw = s_rx_dma_ring[s_pio_drv.rx_dma_total_read & (P7501_RX_DMA_WORDS - 1u)];
        s_pio_drv.rx_dma_total_read += 1u;
    } else {
        if (pio_sm_is_rx_fifo_empty(P7501_PIO_UART_INST, P7501_PIO_RX_SM)) {
            return false;
        }
        raw = pio_sm_get(P7501_PIO_UART_INST, P7501_PIO_RX_SM);
    }
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

size_t p7501_pio_pop_rx_bounded(uint8_t *buf, size_t cap, size_t cur_len, bool slide_on_full) {
    if ((buf == NULL) || (cap == 0u)) {
        return cur_len;
    }
    uint8_t b = 0u;
    for (uint32_t k = 0u; (k < 4096u) && p7501_pio_pop_rx_byte(&b); ++k) {
        if ((k & 63u) == 63u) {
            p7501_wdt_kick();
        }
        if (cur_len >= cap) {
            if (!slide_on_full || (cap < 32u)) {
                continue;
            }
            memmove(buf, &buf[cap - 32u], 32u);
            cur_len = 32u;
        }
        buf[cur_len++] = b;
    }
    return cur_len;
}

uint32_t p7501_pio_get_and_clear_parity_errors(void) {
    const uint32_t count = s_pio_drv.parity_err_count;
    s_pio_drv.parity_err_count = 0u;
    return count;
}

void p7501_pio_drain_rx(void) {
    uint8_t dummy = 0u;
    for (uint32_t k = 0u; (k < 4096u) && p7501_pio_pop_rx_byte(&dummy); ++k) {
        if ((k & 63u) == 63u) {
            p7501_wdt_kick();
        }
    }
    clear_pio_status_flags();
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
    return s_pio_drv.rx_dma_overflow || ((P7501_PIO_UART_INST->fdebug & mask) != 0u);
}

bool p7501_pio_is_rx_dma_active(void) {
    return s_pio_drv.rx_dma_active;
}
