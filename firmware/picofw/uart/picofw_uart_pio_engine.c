/**
 * @file picofw_uart_pio_engine.c
 * @brief Multi-baud PIO1 RX/TX engine delegating GPIO control strictly to `picofw_hw`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_uart_pio_engine.h"
#include "uart/picofw_uart_pio_internal.h"

#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"

#include "hardware/clocks.h"
#include "hardware/dma.h"
#include "hardware/gpio.h"
#include "hardware/pio.h"
#include "pico/stdlib.h"
#include "picofw_uart_duplex.pio.h"

#include <string.h>

#define PICOFW_RX_DMA_WORDS          4096u
#define PICOFW_RX_DMA_RING_BITS      14u
/* 28-bit max transfer count (`MODE=0`), required on RP2350 where bits [31:28]=0xF selects ENDLESS mode. */
#define PICOFW_RX_DMA_TRANSFER_COUNT 0x0FFFFFFFUL

static uint32_t s_rx_dma_ring[PICOFW_RX_DMA_WORDS] __attribute__((aligned(16384)));

typedef struct picofw_pio_engine_drv {
    uint32_t active_baud;
    picofw_uart_framing_t active_framing;
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
} picofw_pio_engine_drv_t;

static picofw_pio_engine_drv_t s_pio_drv = {
    .active_baud = PICOFW_UART_DEFAULT_BAUD,
    .active_framing = PICOFW_FRAMING_8N1,
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
    return (s_pio_drv.active_framing == PICOFW_FRAMING_8N1) ? 7u : 8u;
}

static void compute_pio_clkdiv_int_frac(uint32_t baud, uint16_t *out_int, uint8_t *out_frac) {
    const uint32_t sys_hz = clock_get_hz(clk_sys);
    picofw_calc_pio_clkdiv_fp16(sys_hz, baud, out_int, out_frac);
}

static void ensure_programs_loaded(void) {
    if (!pio_sm_is_claimed(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM)) {
        pio_sm_claim(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM);
    }
    if (!pio_sm_is_claimed(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM)) {
        pio_sm_claim(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM);
    }
    if (!s_pio_drv.rx_loaded) {
        s_pio_drv.rx_offset = pio_add_program(PICOFW_PIO_UART_INST, &picofw_uart_rx_fast_program);
        s_pio_drv.rx_loaded = true;
    }
    if (!s_pio_drv.tx_loaded) {
        s_pio_drv.tx_offset = pio_add_program(PICOFW_PIO_UART_INST, &picofw_uart_tx_fast_program);
        s_pio_drv.tx_loaded = true;
    }
}

static void clear_pio_status_flags(void) {
    PICOFW_PIO_UART_INST->fdebug = (1u << (PIO_FDEBUG_RXSTALL_LSB + PICOFW_PIO_RX_SM));
    pio_interrupt_clear(PICOFW_PIO_UART_INST, 4u);
    s_pio_drv.parity_err_count = 0u;
    s_pio_drv.rx_dma_overflow = false;
}

static void clear_rx_diagnostics(void) {
    pio_sm_clear_fifos(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM);
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
    channel_config_set_ring(&cfg, true, PICOFW_RX_DMA_RING_BITS);
    channel_config_set_dreq(&cfg, pio_get_dreq(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, false));

    s_pio_drv.rx_dma_total_read = 0u;
    s_pio_drv.rx_dma_overflow = false;
    s_pio_drv.rx_dma_active = true;
    dma_channel_configure(
        ch,
        &cfg,
        s_rx_dma_ring,
        &PICOFW_PIO_UART_INST->rxf[PICOFW_PIO_RX_SM],
        PICOFW_RX_DMA_TRANSFER_COUNT,
        true
    );
}

void picofw_pio_internal_prepare_tx(picofw_pio_tx_snapshot_t *out_snap) {
    ensure_programs_loaded();
    out_snap->baud = s_pio_drv.active_baud;
    out_snap->framing = s_pio_drv.active_framing;
    compute_pio_clkdiv_int_frac(s_pio_drv.active_baud, &out_snap->div_int, &out_snap->div_frac);
    out_snap->bit_loops = framing_bit_loop_count();
    out_snap->tx_offset = s_pio_drv.tx_offset;
}

void picofw_pio_enforce_nsw5v_hiz(void) {
    picofw_hw_enforce_nsw5v_hiz();
}

void picofw_pio_set_sw_uart(bool drive_low) {
    picofw_hw_sw_uart_set_low(drive_low);
}

void picofw_pio_pulse_xreset_ms(uint32_t hold_low_ms) {
    picofw_hw_pulse_xreset_ms(hold_low_ms);
}

uint32_t picofw_pio_get_baud(void) {
    return s_pio_drv.active_baud;
}

bool picofw_pio_set_baud(uint32_t baud) {
    if (!picofw_baud_is_valid(baud)) {
        return false;
    }
    s_pio_drv.active_baud = baud;
    uint16_t div_i = 0u;
    uint8_t div_f = 0u;
    compute_pio_clkdiv_int_frac(s_pio_drv.active_baud, &div_i, &div_f);
    if (s_pio_drv.rx_loaded) {
        pio_sm_set_clkdiv_int_frac(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, div_i, div_f);
    }
    if (s_pio_drv.tx_loaded) {
        pio_sm_set_clkdiv_int_frac(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM, div_i, div_f);
    }
    return true;
}

picofw_uart_framing_t picofw_pio_get_framing(void) {
    return s_pio_drv.active_framing;
}

bool picofw_pio_set_framing(picofw_uart_framing_t framing) {
    if (framing != PICOFW_FRAMING_8N1 &&
        framing != PICOFW_FRAMING_8E1 &&
        framing != PICOFW_FRAMING_8O1) {
        return false;
    }
    s_pio_drv.active_framing = framing;
    s_pio_drv.parity_err_count = 0u;
    const uint loops = framing_bit_loop_count();
    if (s_pio_drv.rx_loaded) {
        const bool was_enabled = ((PICOFW_PIO_UART_INST->ctrl >> PICOFW_PIO_RX_SM) & 1u) != 0u;
        pio_sm_set_enabled(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, false);
        clear_rx_diagnostics();
        pio_sm_exec(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, pio_encode_set(pio_y, loops));
        pio_sm_restart(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM);
        pio_sm_exec(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, pio_encode_jmp(s_pio_drv.rx_offset));
        if (was_enabled) {
            start_rx_dma_ring();
            pio_sm_set_enabled(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, true);
        }
    }
    if (s_pio_drv.tx_loaded) {
        pio_sm_exec(PICOFW_PIO_UART_INST, PICOFW_PIO_TX_SM, pio_encode_set(pio_y, loops));
    }
    return true;
}

void picofw_pio_begin_rx_session(picofw_boot_action_t action) {
    picofw_pio_begin_rx_session_cfg(action, 0u, false);
}

void picofw_pio_begin_rx_session_ex(picofw_boot_action_t action, uint32_t jig_hold_ms) {
    picofw_pio_begin_rx_session_cfg(action, jig_hold_ms, false);
}

void picofw_pio_begin_rx_session_cfg(
    picofw_boot_action_t action,
    uint32_t jig_hold_ms,
    bool no_xreset
) {
    uint16_t div_i = 0u;
    uint8_t div_f = 0u;
    compute_pio_clkdiv_int_frac(s_pio_drv.active_baud, &div_i, &div_f);

    s_pio_drv.sw_uart_timed_hold = false;
    ensure_programs_loaded();
    picofw_hw_enforce_nsw5v_hiz();
    picofw_pio_internal_release_tx_hiz();

    /* Hold target in reset BEFORE enabling RX SM unless `no_xreset` (PREJIG standby strap). */
    if (action != PICOFW_BOOT_ACTION_PASSIVE) {
        picofw_hw_sw_uart_set_low(action == PICOFW_BOOT_ACTION_JIG);
        if (!no_xreset) {
            picofw_wdt_sleep_ms(PICOFW_RESET_POST_SETTLE_MS);
            picofw_hw_reset_assert();
            picofw_wdt_sleep_ms(PICOFW_RESET_DIAG_ASSERT_MS);
        }
    }

    picofw_uart_rx_fast_init(
        PICOFW_PIO_UART_INST,
        PICOFW_PIO_RX_SM,
        s_pio_drv.rx_offset,
        P7501_PIN_RXD,
        div_i,
        div_f,
        framing_bit_loop_count()
    );
    pio_sm_set_enabled(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, false);
    clear_rx_diagnostics();
    start_rx_dma_ring();
    pio_sm_set_enabled(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, true);

    if (action != PICOFW_BOOT_ACTION_PASSIVE) {
        if (!no_xreset) {
            picofw_hw_reset_release();
        }
        if (action == PICOFW_BOOT_ACTION_JIG && jig_hold_ms > 0u) {
            s_pio_drv.sw_uart_timed_hold = true;
            s_pio_drv.sw_uart_release_deadline = make_timeout_time_ms(jig_hold_ms);
        }
    }
}

void picofw_pio_release_sw_uart_now(void) {
    s_pio_drv.sw_uart_timed_hold = false;
    picofw_hw_restore_sw_uart_adc0();
}

bool picofw_pio_poll_sw_uart_release(void) {
    if (!s_pio_drv.sw_uart_timed_hold || !time_reached(s_pio_drv.sw_uart_release_deadline)) {
        return false;
    }
    picofw_pio_release_sw_uart_now();
    return true;
}

bool picofw_pio_poll_jig_serial_release(
    uint8_t *ring,
    size_t *ring_len,
    size_t ring_cap,
    uint8_t rx_byte
) {
    if (!s_pio_drv.sw_uart_timed_hold || (ring == NULL) || (ring_len == NULL)) {
        return false;
    }
    if (!picofw_rolling_match_push(ring, ring_len, ring_cap, rx_byte, "serial boot mode")) {
        return false;
    }
    picofw_pio_release_sw_uart_now();
    return true;
}

void picofw_pio_end_session(void) {
    s_pio_drv.sw_uart_timed_hold = false;
    if (s_pio_drv.rx_dma_active && s_pio_drv.rx_dma_claimed) {
        dma_channel_abort((uint)s_pio_drv.rx_dma_chan);
        s_pio_drv.rx_dma_active = false;
    }
    pio_sm_set_enabled(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM, false);
    picofw_pio_internal_release_tx_hiz();
    picofw_hw_isolate_bus_hiz();
}

bool picofw_pio_pop_rx_byte_ex(uint8_t *out_byte, bool *out_parity_err) {
    uint32_t raw = 0u;
    if (s_pio_drv.rx_dma_active && s_pio_drv.rx_dma_claimed) {
        const uint32_t rem = dma_channel_hw_addr((uint)s_pio_drv.rx_dma_chan)->transfer_count & PICOFW_RX_DMA_TRANSFER_COUNT;
        const uint32_t total_written = PICOFW_RX_DMA_TRANSFER_COUNT - rem;
        const uint32_t pending = total_written - s_pio_drv.rx_dma_total_read;
        if (pending == 0u) {
            return false;
        }
        if (pending > PICOFW_RX_DMA_WORDS) {
            s_pio_drv.rx_dma_overflow = true;
            s_pio_drv.rx_dma_total_read = total_written - (PICOFW_RX_DMA_WORDS - 1u);
        }
        raw = s_rx_dma_ring[s_pio_drv.rx_dma_total_read & (PICOFW_RX_DMA_WORDS - 1u)];
        s_pio_drv.rx_dma_total_read += 1u;
    } else {
        if (pio_sm_is_rx_fifo_empty(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM)) {
            return false;
        }
        raw = pio_sm_get(PICOFW_PIO_UART_INST, PICOFW_PIO_RX_SM);
    }
    bool parity_ok = true;
    picofw_uart_decode_rx_word(raw, s_pio_drv.active_framing, out_byte, &parity_ok);
    if (!parity_ok) {
        ++s_pio_drv.parity_err_count;
    }
    if (out_parity_err != NULL) {
        *out_parity_err = !parity_ok;
    }
    return true;
}

bool picofw_pio_pop_rx_byte(uint8_t *out_byte) {
    return picofw_pio_pop_rx_byte_ex(out_byte, NULL);
}

size_t picofw_pio_pop_rx_bounded(uint8_t *buf, size_t cap, size_t cur_len, bool slide_on_full) {
    if ((buf == NULL) || (cap == 0u)) {
        return cur_len;
    }
    uint8_t b = 0u;
    for (uint32_t k = 0u; (k < 4096u) && picofw_pio_pop_rx_byte(&b); ++k) {
        if ((k & 63u) == 63u) {
            picofw_wdt_kick();
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

uint32_t picofw_pio_get_and_clear_parity_errors(void) {
    const uint32_t count = s_pio_drv.parity_err_count;
    s_pio_drv.parity_err_count = 0u;
    return count;
}

void picofw_pio_drain_rx(void) {
    uint8_t dummy = 0u;
    for (uint32_t k = 0u; (k < 4096u) && picofw_pio_pop_rx_byte(&dummy); ++k) {
        if ((k & 63u) == 63u) {
            picofw_wdt_kick();
        }
    }
    clear_pio_status_flags();
}

bool picofw_pio_poll_and_clear_framing_err(void) {
    if (pio_interrupt_get(PICOFW_PIO_UART_INST, 4u)) {
        pio_interrupt_clear(PICOFW_PIO_UART_INST, 4u);
        return true;
    }
    return false;
}

bool picofw_pio_had_rx_overflow(void) {
    const uint32_t mask = (1u << (PIO_FDEBUG_RXSTALL_LSB + PICOFW_PIO_RX_SM));
    return s_pio_drv.rx_dma_overflow || ((PICOFW_PIO_UART_INST->fdebug & mask) != 0u);
}

bool picofw_pio_is_rx_dma_active(void) {
    return s_pio_drv.rx_dma_active;
}
