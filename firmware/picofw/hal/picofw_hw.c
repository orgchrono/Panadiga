/**
 * @file picofw_hw.c
 * @brief Safe Hardware Abstraction Layer implementation for Panasonic P7501 bus.
 *
 * Implements Hi-Z safe defaults, GP26 ADC0 restoration (clearing SIO OE before
 * switching pad mux), Open-Drain XRESET control, NSW5V falling-edge anti-backpowering
 * IRQ guard on GP3 (kept strictly Hi-Z Input with pulls disabled), and integer mV
 * ADC measurement. PIO1 dual-SM UART is managed exclusively by `picofw_uart_pio_engine`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_hw.h"
#include "diag/picofw_i2c_codec.h"
#include "gdb/gdb_rsp.h"
#include "hal/picofw_led.h"
#include "hal/picofw_watchdog.h"
#include "pico/stdlib.h"
#include "pico/time.h"
#include "hardware/adc.h"
#include "hardware/gpio.h"

#include <inttypes.h>
#include <stdio.h>

static bool s_initialized = false;
static volatile uint32_t s_nsw5v_dropouts = 0u;
static uint32_t s_nsw5v_acked_dropouts = 0u;
static uint32_t s_nsw5v_recoveries = 0u;
static uint32_t s_last_rail_poll_ms = 0u;
static picofw_hotplug_fsm_t s_hp_fsm;
static bool s_hp_fsm_init = false;

static void picofw_nsw5v_falling_irq(uint gpio, uint32_t event_mask) {
    if ((gpio == P7501_PIN_NSW5V) && ((event_mask & GPIO_IRQ_EDGE_FALL) != 0u)) {
        /* Always keep GP3 NSW5V strictly Hi-Z Input with pulls disabled */
        gpio_disable_pulls(P7501_PIN_NSW5V);
        gpio_set_function(P7501_PIN_NSW5V, GPIO_FUNC_SIO);
        gpio_set_dir(P7501_PIN_NSW5V, GPIO_IN);

        /* Anti-backpowering guard (UNCONDITIONAL): reclaim SIO from PIO1/ADC and float all pins. */
        gpio_set_function(P7501_PIN_TXD, GPIO_FUNC_SIO);
        gpio_set_dir(P7501_PIN_TXD, GPIO_IN);
        gpio_disable_pulls(P7501_PIN_TXD);

        gpio_set_function(P7501_PIN_RXD, GPIO_FUNC_SIO);
        gpio_set_dir(P7501_PIN_RXD, GPIO_IN);
        gpio_disable_pulls(P7501_PIN_RXD);

        gpio_set_function(P7501_PIN_XRESET, GPIO_FUNC_SIO);
        gpio_set_dir(P7501_PIN_XRESET, GPIO_IN);
        gpio_disable_pulls(P7501_PIN_XRESET);

        gpio_set_function(P7501_PIN_SW_UART_ADC, GPIO_FUNC_SIO);
        gpio_set_dir(P7501_PIN_SW_UART_ADC, GPIO_IN);
        gpio_disable_pulls(P7501_PIN_SW_UART_ADC);

        /* Also isolate auxiliary I2C/IR pins (GP6/GP7) so active 3.3V pull-ups cannot backpower. */
        gpio_set_function(PICOFW_I2C_SDA_PIN, GPIO_FUNC_SIO);
        gpio_set_dir(PICOFW_I2C_SDA_PIN, GPIO_IN);
        gpio_disable_pulls(PICOFW_I2C_SDA_PIN);

        gpio_set_function(PICOFW_I2C_SCL_PIN, GPIO_FUNC_SIO);
        gpio_set_dir(PICOFW_I2C_SCL_PIN, GPIO_IN);
        gpio_disable_pulls(PICOFW_I2C_SCL_PIN);

        s_nsw5v_dropouts += 1u;
    }
}

uint32_t picofw_hw_get_nsw5v_dropouts(void) {
    return s_nsw5v_dropouts;
}

bool picofw_hw_is_nsw5v_fault_latched(void) {
    return s_nsw5v_dropouts != s_nsw5v_acked_dropouts;
}

bool picofw_hw_ack_fault(void) {
    const uint32_t snap = s_nsw5v_dropouts;
    if (!gpio_get(P7501_PIN_NSW5V)) {
        return false;
    }
    const uint32_t vdd_mv = picofw_hw_read_vdd_mv();
    if ((vdd_mv < PICOFW_SSOT_VDD_MIN_MV) || (vdd_mv >= PICOFW_SSOT_VDD_FLOATING_CEIL_MV)) {
        return false;
    }
    if (s_nsw5v_dropouts == snap) {
        s_nsw5v_acked_dropouts = snap;
        return true;
    }
    return false;
}

void picofw_hw_clear_nsw5v_fault_latch(void) {
    (void)picofw_hw_ack_fault();
}

uint32_t picofw_hw_get_nsw5v_recoveries(void) {
    return s_nsw5v_recoveries;
}

picofw_hotplug_fsm_t *picofw_hw_hotplug_fsm(void) {
    if (!s_hp_fsm_init) {
        picofw_hotplug_fsm_init(&s_hp_fsm);
        s_hp_fsm_init = true;
    }
    return &s_hp_fsm;
}

void picofw_hw_poll_rail_recovery(void) {
    if (!s_initialized || gpio_is_dir_out(P7501_PIN_SW_UART_ADC)) {
        return;
    }
    const uint32_t now_ms = to_ms_since_boot(get_absolute_time());
    if ((uint32_t)(now_ms - s_last_rail_poll_ms) < 50u) {
        return;
    }
    s_last_rail_poll_ms = now_ms;

    const uint32_t drops_snap = s_nsw5v_dropouts;
    picofw_hw_enforce_nsw5v_hiz();
    const uint32_t vdd_mv = picofw_hw_sample_sw_uart_mv(false, false);
    const bool nsw_h = gpio_get(P7501_PIN_NSW5V);
    const bool fault = (drops_snap != s_nsw5v_acked_dropouts);

    bool changed = false;
    bool clear_fault = false;
    picofw_hotplug_fsm_t *const fsm = picofw_hw_hotplug_fsm();
    picofw_hotplug_fsm_step(fsm, vdd_mv, nsw_h, fault, &changed, &clear_fault);

    if (clear_fault && (s_nsw5v_dropouts == drops_snap)) {
        s_nsw5v_acked_dropouts = drops_snap;
        s_nsw5v_recoveries += 1u;
    }
    if (!gdb_rsp_is_active()) {
        if ((fsm->state == PICOFW_HP_ONLINE_AWAKE) &&
            (picofw_led_get_state().mode == LED_MODE_BUS_FAULT)) {
            picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
        } else if (changed && (fsm->state == PICOFW_HP_DROPOUT_FAULT)) {
            picofw_led_set_auto_mode(LED_MODE_BUS_FAULT);
        }
    }
    if (changed && fsm->push_enabled) {
        char kv[160];
        if (picofw_hotplug_format_kv(fsm, picofw_hw_is_nsw5v_fault_latched(), s_nsw5v_dropouts, kv, sizeof(kv)) > 0u) {
            printf("@PICO_HOTPLUG %s\n", kv);
            fflush(stdout);
        }
    }
}

static volatile bool s_contention_latched = false;

bool picofw_hw_is_contention_latched(void) {
    return s_contention_latched;
}

void picofw_hw_clear_contention_latch(void) {
    s_contention_latched = false;
}

static bool picofw_hw_verify_low_no_contention(uint pin) {
    busy_wait_us_32(8u);
    if (gpio_get(pin)) {
        gpio_set_dir(pin, GPIO_IN);
        gpio_disable_pulls(pin);
        s_contention_latched = true;
        return false;
    }
    return true;
}

void picofw_hw_enforce_nsw5v_hiz(void) {
    /* CRITICAL SAFETY INVARIANT: Pin 5 NSW5V (GP3) must ALWAYS stay Hi-Z Input with no pulls */
    gpio_set_function(P7501_PIN_NSW5V, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_NSW5V, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_NSW5V);
}

void picofw_hw_restore_sw_uart_adc0(void) {
    /* Clear SIO output enable BEFORE adc_gpio_init() so GP26 can never stay latched LOW */
    gpio_set_function(P7501_PIN_SW_UART_ADC, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_SW_UART_ADC, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
    adc_gpio_init(P7501_PIN_SW_UART_ADC);
}

picofw_status_t picofw_hw_init(void) {
    /* Pin 5 NSW5V (GP3): Clear reset-default PDE pull-down BEFORE gpio_init() (RP2350 ISO safety). */
    gpio_disable_pulls(P7501_PIN_NSW5V);
    gpio_init(P7501_PIN_NSW5V);
    picofw_hw_enforce_nsw5v_hiz();
    gpio_set_irq_enabled_with_callback(
        P7501_PIN_NSW5V, GPIO_IRQ_EDGE_FALL, true, &picofw_nsw5v_falling_irq
    );

    adc_init();
    picofw_hw_restore_sw_uart_adc0();

    /* Pin 6 UARTP2M_T (GP2): Input from Target TX (start Hi-Z without pull-up to avoid backpowering) */
    gpio_disable_pulls(P7501_PIN_RXD);
    gpio_init(P7501_PIN_RXD);
    gpio_set_dir(P7501_PIN_RXD, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_RXD);

    /* Pin 3 UARTM2P_T (GP4): Idle Hi-Z Input until UART TX requested */
    gpio_disable_pulls(P7501_PIN_TXD);
    gpio_init(P7501_PIN_TXD);
    gpio_set_dir(P7501_PIN_TXD, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_TXD);

    /* Pin 2 XRESET (GP5): Open-Drain (latch 0, released to Hi-Z Input) */
    gpio_disable_pulls(P7501_PIN_XRESET);
    gpio_init(P7501_PIN_XRESET);
    gpio_disable_pulls(P7501_PIN_XRESET);
    gpio_put(P7501_PIN_XRESET, false);
    picofw_hw_reset_release();

    gpio_set_slew_rate(P7501_PIN_TXD, GPIO_SLEW_RATE_SLOW);
    gpio_set_slew_rate(P7501_PIN_XRESET, GPIO_SLEW_RATE_SLOW);
    gpio_set_slew_rate(P7501_PIN_SW_UART_ADC, GPIO_SLEW_RATE_SLOW);

    s_nsw5v_dropouts = 0u;
    s_nsw5v_acked_dropouts = 0u;
    s_nsw5v_recoveries = 0u;
    s_last_rail_poll_ms = 0u;
    s_contention_latched = false;
    picofw_hotplug_fsm_init(picofw_hw_hotplug_fsm());
    s_initialized = true;
    return PICOFW_OK;
}

void picofw_hw_reset_assert(void) {
    gpio_set_function(P7501_PIN_XRESET, GPIO_FUNC_SIO);
    gpio_put(P7501_PIN_XRESET, false);
    gpio_set_dir(P7501_PIN_XRESET, GPIO_OUT);
}

void picofw_hw_reset_release(void) {
    gpio_set_function(P7501_PIN_XRESET, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_XRESET, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_XRESET);
}

void picofw_hw_pulse_xreset_ms(uint32_t hold_low_ms) {
    picofw_hw_enforce_nsw5v_hiz();
    picofw_hw_reset_assert();
    picofw_wdt_sleep_ms((hold_low_ms > 0U) ? hold_low_ms : PICOFW_RESET_DIAG_ASSERT_MS);
    picofw_hw_reset_release();
}

void picofw_hw_drive_txd_low(void) {
    gpio_set_function(P7501_PIN_TXD, GPIO_FUNC_SIO);
    gpio_disable_pulls(P7501_PIN_TXD);
    gpio_put(P7501_PIN_TXD, false);
    gpio_set_dir(P7501_PIN_TXD, GPIO_OUT);
    (void)picofw_hw_verify_low_no_contention(P7501_PIN_TXD);
}

void picofw_hw_release_txd_hiz(bool with_pull_up) {
    gpio_set_function(P7501_PIN_TXD, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_TXD, GPIO_IN);
    if (with_pull_up) {
        gpio_pull_up(P7501_PIN_TXD);
    } else {
        gpio_disable_pulls(P7501_PIN_TXD);
    }
}

void picofw_hw_sw_uart_set_low(bool assert_low) {
    if (assert_low) {
        gpio_set_function(P7501_PIN_SW_UART_ADC, GPIO_FUNC_SIO);
        gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
        gpio_put(P7501_PIN_SW_UART_ADC, false);
        gpio_set_dir(P7501_PIN_SW_UART_ADC, GPIO_OUT);
        (void)picofw_hw_verify_low_no_contention(P7501_PIN_SW_UART_ADC);
    } else {
        picofw_hw_restore_sw_uart_adc0();
    }
}

uint32_t picofw_hw_sample_sw_uart_mv(bool pull_down, bool pull_up) {
    picofw_hw_restore_sw_uart_adc0();
    adc_select_input(PICOFW_ADC_CH_VDD);

    if (pull_down) {
        gpio_pull_down(P7501_PIN_SW_UART_ADC);
    } else if (pull_up) {
        gpio_pull_up(P7501_PIN_SW_UART_ADC);
    } else {
        gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
    }
    sleep_us(PICOFW_ADC_PULLDOWN_SETTLE_US);

    /* Discard first conversion after mux/pull switch before averaging */
    (void)adc_read();
    uint32_t accumulator = 0U;
    for (uint8_t i = 0U; i < PICOFW_ADC_SAMPLE_COUNT; ++i) {
        accumulator += (uint32_t)adc_read();
    }

    gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
    const uint32_t denom = 4096UL * (uint32_t)PICOFW_ADC_SAMPLE_COUNT;
    return (uint32_t)((((uint64_t)accumulator * (uint64_t)PICOFW_SSOT_VDD_NOMINAL_MV) + (denom / 2ULL)) / denom);
}

bool picofw_hw_is_nsw5v_high(void) {
    return gpio_get(P7501_PIN_NSW5V);
}

bool picofw_hw_is_sw_uart_forced_low(void) {
    return gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
}

uint32_t picofw_hw_read_vdd_mv(void) {
    /* Never clobber an active JIGPIN LOW assertion on GP26! */
    if (gpio_is_dir_out(P7501_PIN_SW_UART_ADC)) {
        return 0u;
    }
    /* Brief pull-down discriminates an open floating pin (~0mV) from connected P7501 (~2680mV) */
    const uint32_t mv_pd = picofw_hw_sample_sw_uart_mv(true, false);
    if (mv_pd < PICOFW_SSOT_VDD_PULLDOWN_MIN_MV) {
        return mv_pd;
    }
    return picofw_hw_sample_sw_uart_mv(false, false);
}

bool picofw_hw_is_target_powered(void) {
    const uint32_t mv = picofw_hw_read_vdd_mv();
    return (mv >= PICOFW_SSOT_VDD_MIN_MV) && (mv < PICOFW_SSOT_VDD_FLOATING_CEIL_MV);
}

void picofw_hw_rail_snapshot(picofw_rail_snap_t *out, bool active_vdd_sample) {
    if (out == NULL) {
        return;
    }
    const bool jig = gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
    out->dropouts = s_nsw5v_dropouts;
    out->acked_dropouts = s_nsw5v_acked_dropouts;
    out->nsw5v_high = gpio_get(P7501_PIN_NSW5V);
    out->jig_strapped_low = jig;
    if (jig) {
        out->vdd_mv = 0u;
    } else if (active_vdd_sample) {
        out->vdd_mv = picofw_hw_read_vdd_mv();
    } else {
        out->vdd_mv = picofw_hw_hotplug_fsm()->last_vdd_mv;
    }
}

bool picofw_hw_is_rail_healthy(void) {
    picofw_rail_snap_t snap;
    picofw_hw_rail_snapshot(&snap, true);
    return picofw_rail_is_healthy(&snap);
}

bool picofw_hw_is_initialized(void) {
    return s_initialized;
}

void picofw_hw_boot_sequence(picofw_boot_mode_t mode) {
    const bool use_serial_boot =
        (mode == PICOFW_BOOT_MODE_SERIAL_LOW_RELEASE) ||
        (mode == PICOFW_BOOT_MODE_SERIAL_LOW_HOLD);

    picofw_hw_isolate_bus_hiz();
    picofw_hw_sw_uart_set_low(use_serial_boot);

    picofw_hw_reset_assert();
    picofw_wdt_sleep_ms(PICOFW_RESET_ASSERT_MS);
    picofw_hw_reset_release();

    for (uint32_t wait_us = 0U; wait_us < 2000U; wait_us += 10U) {
        if (gpio_get(P7501_PIN_XRESET)) {
            break;
        }
        sleep_us(10U);
    }

    if (mode == PICOFW_BOOT_MODE_SERIAL_LOW_RELEASE) {
        sleep_us(PICOFW_RESET_SETTLE_US);
        picofw_hw_sw_uart_set_low(false);
    } else if (mode == PICOFW_BOOT_MODE_HIZ_EXTENDED) {
        picofw_wdt_sleep_ms(PICOFW_RESET_POST_SETTLE_MS);
    } else {
        sleep_us(PICOFW_POST_RESET_DELAY_US);
    }

    if (mode != PICOFW_BOOT_MODE_SERIAL_LOW_HOLD) {
        picofw_hw_restore_sw_uart_adc0();
    }
    picofw_hw_enforce_nsw5v_hiz();
}

void picofw_hw_isolate_bus_hiz(void) {
    picofw_hw_reset_release();
    picofw_hw_enforce_nsw5v_hiz();
    picofw_hw_release_txd_hiz(false);

    gpio_set_function(P7501_PIN_RXD, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_RXD, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_RXD);

    gpio_set_function(PICOFW_I2C_SDA_PIN, GPIO_FUNC_SIO);
    gpio_set_dir(PICOFW_I2C_SDA_PIN, GPIO_IN);
    gpio_disable_pulls(PICOFW_I2C_SDA_PIN);

    gpio_set_function(PICOFW_I2C_SCL_PIN, GPIO_FUNC_SIO);
    gpio_set_dir(PICOFW_I2C_SCL_PIN, GPIO_IN);
    gpio_disable_pulls(PICOFW_I2C_SCL_PIN);

    picofw_hw_restore_sw_uart_adc0();
}

void picofw_hw_sample_bus_pins(picofw_bus_pin_sample_t *out) {
    if (out == NULL) {
        return;
    }
    picofw_hw_enforce_nsw5v_hiz();
    out->jig_latched = gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
    out->vdd_mv = out->jig_latched ? 0u : picofw_hw_read_vdd_mv();
    out->nsw5v_high = gpio_get(P7501_PIN_NSW5V);
    const bool powered = out->nsw5v_high || (out->vdd_mv >= PICOFW_SSOT_VDD_MIN_MV);
    /* Avoid engaging GP26 internal pull-up into an unpowered or JIG-held board (anti-backpower). */
    out->sw_pu_mv = (out->jig_latched || !powered)
        ? out->vdd_mv
        : picofw_hw_sample_sw_uart_mv(false, true);
    out->rxd_high = gpio_get(P7501_PIN_RXD);
    out->txd_high = gpio_get(P7501_PIN_TXD);
    out->xreset_high = gpio_get(P7501_PIN_XRESET);
}

void picofw_hw_engage_uart_bus(void) {
    picofw_hw_enforce_nsw5v_hiz();

    gpio_set_function(P7501_PIN_RXD, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_RXD, GPIO_IN);
    gpio_pull_up(P7501_PIN_RXD);

    gpio_set_function(P7501_PIN_TXD, GPIO_FUNC_SIO);
    gpio_put(P7501_PIN_TXD, true);
    gpio_set_dir(P7501_PIN_TXD, GPIO_OUT);

    picofw_hw_restore_sw_uart_adc0();
}
