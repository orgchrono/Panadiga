/**
 * @file p7501_hw.c
 * @brief Safe Hardware Abstraction Layer implementation for Panasonic P7501 bus.
 *
 * Implements Hi-Z safe defaults, GP26 ADC0 restoration (clearing SIO OE before
 * switching pad mux), Open-Drain XRESET control, NSW5V falling-edge anti-backpowering
 * IRQ guard on GP3 (kept strictly Hi-Z Input with pulls disabled), and integer mV
 * ADC measurement. PIO1 dual-SM UART is managed exclusively by `p7501_uart_pio_engine`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/p7501_hw.h"
#include "diag/p7501_i2c_codec.h"
#include "hal/p7501_watchdog.h"
#include "pico/stdlib.h"
#include "hardware/adc.h"
#include "hardware/gpio.h"

static bool s_initialized = false;
static volatile uint32_t s_nsw5v_dropouts = 0u;
static volatile bool s_nsw5v_fault_latched = false;
static volatile bool s_nsw5v_power_seen = false;

static void p7501_nsw5v_falling_irq(uint gpio, uint32_t event_mask) {
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
        gpio_set_function(P7501_I2C_SDA_PIN, GPIO_FUNC_SIO);
        gpio_set_dir(P7501_I2C_SDA_PIN, GPIO_IN);
        gpio_disable_pulls(P7501_I2C_SDA_PIN);

        gpio_set_function(P7501_I2C_SCL_PIN, GPIO_FUNC_SIO);
        gpio_set_dir(P7501_I2C_SCL_PIN, GPIO_IN);
        gpio_disable_pulls(P7501_I2C_SCL_PIN);

        s_nsw5v_power_seen = false;
        s_nsw5v_dropouts += 1u;
        s_nsw5v_fault_latched = true;
    }
}

uint32_t p7501_hw_get_nsw5v_dropouts(void) {
    return s_nsw5v_dropouts;
}

bool p7501_hw_is_nsw5v_fault_latched(void) {
    return s_nsw5v_fault_latched;
}

void p7501_hw_clear_nsw5v_fault_latch(void) {
    s_nsw5v_fault_latched = false;
}

static volatile bool s_contention_latched = false;

bool p7501_hw_is_contention_latched(void) {
    return s_contention_latched;
}

void p7501_hw_clear_contention_latch(void) {
    s_contention_latched = false;
}

static bool p7501_hw_verify_low_no_contention(uint pin) {
    busy_wait_us_32(8u);
    if (gpio_get(pin)) {
        gpio_set_dir(pin, GPIO_IN);
        gpio_disable_pulls(pin);
        s_contention_latched = true;
        return false;
    }
    return true;
}

void p7501_hw_enforce_nsw5v_hiz(void) {
    /* CRITICAL SAFETY INVARIANT: Pin 5 NSW5V (GP3) must ALWAYS stay Hi-Z Input with no pulls */
    gpio_set_function(P7501_PIN_NSW5V, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_NSW5V, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_NSW5V);
}

void p7501_hw_restore_sw_uart_adc0(void) {
    /* Clear SIO output enable BEFORE adc_gpio_init() so GP26 can never stay latched LOW */
    gpio_set_function(P7501_PIN_SW_UART_ADC, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_SW_UART_ADC, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
    adc_gpio_init(P7501_PIN_SW_UART_ADC);
}

p7501_status_t p7501_hw_init(void) {
    /* Pin 5 NSW5V (GP3): Clear reset-default PDE pull-down BEFORE gpio_init() (RP2350 ISO safety). */
    gpio_disable_pulls(P7501_PIN_NSW5V);
    gpio_init(P7501_PIN_NSW5V);
    p7501_hw_enforce_nsw5v_hiz();
    gpio_set_irq_enabled_with_callback(
        P7501_PIN_NSW5V, GPIO_IRQ_EDGE_FALL, true, &p7501_nsw5v_falling_irq
    );

    adc_init();
    p7501_hw_restore_sw_uart_adc0();

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
    p7501_hw_reset_release();

    gpio_set_slew_rate(P7501_PIN_TXD, GPIO_SLEW_RATE_SLOW);
    gpio_set_slew_rate(P7501_PIN_XRESET, GPIO_SLEW_RATE_SLOW);
    gpio_set_slew_rate(P7501_PIN_SW_UART_ADC, GPIO_SLEW_RATE_SLOW);

    s_nsw5v_dropouts = 0u;
    s_nsw5v_fault_latched = false;
    s_contention_latched = false;
    s_nsw5v_power_seen = gpio_get(P7501_PIN_NSW5V);
    s_initialized = true;
    return P7501_OK;
}

void p7501_hw_reset_assert(void) {
    gpio_set_function(P7501_PIN_XRESET, GPIO_FUNC_SIO);
    gpio_put(P7501_PIN_XRESET, false);
    gpio_set_dir(P7501_PIN_XRESET, GPIO_OUT);
}

void p7501_hw_reset_release(void) {
    gpio_set_function(P7501_PIN_XRESET, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_XRESET, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_XRESET);
}

void p7501_hw_pulse_xreset_ms(uint32_t hold_low_ms) {
    p7501_hw_enforce_nsw5v_hiz();
    p7501_hw_reset_assert();
    p7501_wdt_sleep_ms((hold_low_ms > 0U) ? hold_low_ms : P7501_RESET_DIAG_ASSERT_MS);
    p7501_hw_reset_release();
}

void p7501_hw_drive_txd_low(void) {
    gpio_set_function(P7501_PIN_TXD, GPIO_FUNC_SIO);
    gpio_disable_pulls(P7501_PIN_TXD);
    gpio_put(P7501_PIN_TXD, false);
    gpio_set_dir(P7501_PIN_TXD, GPIO_OUT);
    (void)p7501_hw_verify_low_no_contention(P7501_PIN_TXD);
}

void p7501_hw_release_txd_hiz(bool with_pull_up) {
    gpio_set_function(P7501_PIN_TXD, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_TXD, GPIO_IN);
    if (with_pull_up) {
        gpio_pull_up(P7501_PIN_TXD);
    } else {
        gpio_disable_pulls(P7501_PIN_TXD);
    }
}

void p7501_hw_sw_uart_set_low(bool assert_low) {
    if (assert_low) {
        gpio_set_function(P7501_PIN_SW_UART_ADC, GPIO_FUNC_SIO);
        gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
        gpio_put(P7501_PIN_SW_UART_ADC, false);
        gpio_set_dir(P7501_PIN_SW_UART_ADC, GPIO_OUT);
        (void)p7501_hw_verify_low_no_contention(P7501_PIN_SW_UART_ADC);
    } else {
        p7501_hw_restore_sw_uart_adc0();
    }
}

uint32_t p7501_hw_sample_sw_uart_mv(bool pull_down, bool pull_up) {
    p7501_hw_restore_sw_uart_adc0();
    adc_select_input(P7501_ADC_CH_VDD);

    if (pull_down) {
        gpio_pull_down(P7501_PIN_SW_UART_ADC);
    } else if (pull_up) {
        gpio_pull_up(P7501_PIN_SW_UART_ADC);
    } else {
        gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
    }
    sleep_us(P7501_ADC_PULLDOWN_SETTLE_US);

    /* Discard first conversion after mux/pull switch before averaging */
    (void)adc_read();
    uint32_t accumulator = 0U;
    for (uint8_t i = 0U; i < P7501_ADC_SAMPLE_COUNT; ++i) {
        accumulator += (uint32_t)adc_read();
    }

    gpio_disable_pulls(P7501_PIN_SW_UART_ADC);
    const uint32_t denom = 4096UL * (uint32_t)P7501_ADC_SAMPLE_COUNT;
    return (uint32_t)((((uint64_t)accumulator * (uint64_t)P7501_SSOT_VDD_NOMINAL_MV) + (denom / 2ULL)) / denom);
}

bool p7501_hw_is_nsw5v_high(void) {
    const bool high = gpio_get(P7501_PIN_NSW5V);
    if (high) {
        s_nsw5v_power_seen = true;
        s_nsw5v_fault_latched = false;
    }
    return high;
}

bool p7501_hw_is_sw_uart_forced_low(void) {
    return gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
}

uint32_t p7501_hw_read_vdd_mv(void) {
    /* Never clobber an active JIGPIN LOW assertion on GP26! */
    if (gpio_is_dir_out(P7501_PIN_SW_UART_ADC)) {
        return 0u;
    }
    /* Brief pull-down discriminates an open floating pin (~0mV) from connected P7501 (~2680mV) */
    const uint32_t mv_pd = p7501_hw_sample_sw_uart_mv(true, false);
    if (mv_pd < P7501_SSOT_VDD_PULLDOWN_MIN_MV) {
        return mv_pd;
    }
    const uint32_t mv = p7501_hw_sample_sw_uart_mv(false, false);
    if ((mv >= P7501_SSOT_VDD_MIN_MV) && (mv < P7501_SSOT_VDD_FLOATING_CEIL_MV) &&
        gpio_get(P7501_PIN_NSW5V)) {
        s_nsw5v_power_seen = true;
        s_nsw5v_fault_latched = false;
    }
    return mv;
}

bool p7501_hw_is_target_powered(void) {
    const uint32_t mv = p7501_hw_read_vdd_mv();
    return (mv >= P7501_SSOT_VDD_MIN_MV) && (mv < P7501_SSOT_VDD_FLOATING_CEIL_MV);
}

bool p7501_hw_is_initialized(void) {
    return s_initialized;
}

void p7501_hw_boot_sequence(p7501_boot_mode_t mode) {
    const bool use_serial_boot =
        (mode == P7501_BOOT_MODE_SERIAL_LOW_RELEASE) ||
        (mode == P7501_BOOT_MODE_SERIAL_LOW_HOLD);

    p7501_hw_isolate_bus_hiz();
    p7501_hw_sw_uart_set_low(use_serial_boot);

    p7501_hw_reset_assert();
    p7501_wdt_sleep_ms(P7501_RESET_ASSERT_MS);
    p7501_hw_reset_release();

    for (uint32_t wait_us = 0U; wait_us < 2000U; wait_us += 10U) {
        if (gpio_get(P7501_PIN_XRESET)) {
            break;
        }
        sleep_us(10U);
    }

    if (mode == P7501_BOOT_MODE_SERIAL_LOW_RELEASE) {
        sleep_us(P7501_RESET_SETTLE_US);
        p7501_hw_sw_uart_set_low(false);
    } else if (mode == P7501_BOOT_MODE_HIZ_EXTENDED) {
        p7501_wdt_sleep_ms(P7501_RESET_POST_SETTLE_MS);
    } else {
        sleep_us(P7501_POST_RESET_DELAY_US);
    }

    if (mode != P7501_BOOT_MODE_SERIAL_LOW_HOLD) {
        p7501_hw_restore_sw_uart_adc0();
    }
    p7501_hw_enforce_nsw5v_hiz();
}

void p7501_hw_isolate_bus_hiz(void) {
    p7501_hw_reset_release();
    p7501_hw_enforce_nsw5v_hiz();
    p7501_hw_release_txd_hiz(false);

    gpio_set_function(P7501_PIN_RXD, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_RXD, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_RXD);

    p7501_hw_restore_sw_uart_adc0();
}

void p7501_hw_engage_uart_bus(void) {
    p7501_hw_enforce_nsw5v_hiz();

    gpio_set_function(P7501_PIN_RXD, GPIO_FUNC_SIO);
    gpio_set_dir(P7501_PIN_RXD, GPIO_IN);
    gpio_pull_up(P7501_PIN_RXD);

    gpio_set_function(P7501_PIN_TXD, GPIO_FUNC_SIO);
    gpio_put(P7501_PIN_TXD, true);
    gpio_set_dir(P7501_PIN_TXD, GPIO_OUT);

    p7501_hw_restore_sw_uart_adc0();
}
