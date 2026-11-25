/**
 * @file test_picofw_hw_host.c
 * @brief Host C11 safety verification compiling REAL `hal/picofw_hw.c` against stateful mock GPIO/ADC.
 *
 * Verifies non-negotiable P7501 silicon invariants:
 *   1. GP3 (`P7501_PIN_NSW5V`) stays Hi-Z Input with NO pulls (never output, never driven HIGH).
 *   2. `XRESET` (GP5) and `SW_UART` (GP26) drive strictly Open-Drain (LOW as OUT=0, release as Hi-Z IN).
 *   3. `picofw_hw_isolate_bus_hiz()` restores every P7501 pin to Hi-Z Input with pulls disabled.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_hw.h"
#include "diag/picofw_i2c_codec.h"
#include "hal/picofw_led.h"
#include "hal/picofw_watchdog.h"
#include "gdb/gdb_rsp.h"
#include "hardware/adc.h"
#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

/* ---- Stateful mock backing storage ---- */
uint64_t picofw_mock_now_us = 10000ULL;
static gpio_irq_callback_t s_mock_irq_cb = NULL;
picofw_mock_gpio_pin_t picofw_mock_pins[32];
uint16_t picofw_mock_adc_raw = 0u;
bool picofw_mock_adc_initialized = false;
uint picofw_mock_adc_input = 0u;

static void mock_reset_hw(void) {
    memset(picofw_mock_pins, 0, sizeof(picofw_mock_pins));
    picofw_mock_now_us = 10000ULL;
    s_mock_irq_cb = NULL;
    picofw_mock_adc_raw = 0u;
    picofw_mock_adc_initialized = false;
    picofw_mock_adc_input = 0u;
    /* Simulate RP2350 reset-default pull-down on GP3 so picofw_hw_init MUST clear it */
    picofw_mock_pins[P7501_PIN_NSW5V].pull_down = true;
    /* Pre-latch dangerous output HIGH on GP3/GP5 to verify picofw_hw_init clears it */
    picofw_mock_pins[P7501_PIN_XRESET].dir_out = false;
    picofw_mock_pins[P7501_PIN_XRESET].ext_input_level = true;
}

void gpio_init(uint gpio) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].initialized = true;
        picofw_mock_pins[gpio].dir_out = false;
        picofw_mock_pins[gpio].func = GPIO_FUNC_SIO;
    }
}

void gpio_set_function(uint gpio, uint fn) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].func = fn;
    }
}

void gpio_set_dir(uint gpio, bool out) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].dir_out = out;
        if (out) {
            picofw_mock_pins[gpio].ever_dir_out = true;
            if (picofw_mock_pins[gpio].out_level) {
                picofw_mock_pins[gpio].ever_driven_high = true;
            }
        }
    }
}

bool gpio_is_dir_out(uint gpio) {
    return (gpio < 32u) && picofw_mock_pins[gpio].dir_out;
}

void gpio_put(uint gpio, bool value) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].out_level = value;
        if (value && picofw_mock_pins[gpio].dir_out) {
            picofw_mock_pins[gpio].ever_driven_high = true;
        }
    }
}

bool gpio_get(uint gpio) {
    if (gpio >= 32u) {
        return false;
    }
    return picofw_mock_pins[gpio].dir_out
        ? picofw_mock_pins[gpio].out_level
        : picofw_mock_pins[gpio].ext_input_level;
}

uint32_t gpio_get_all(void) {
    uint32_t mask = 0u;
    for (uint i = 0u; i < 32u; ++i) {
        if (gpio_get(i)) {
            mask |= (1u << i);
        }
    }
    return mask;
}

void gpio_disable_pulls(uint gpio) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].pull_up = false;
        picofw_mock_pins[gpio].pull_down = false;
    }
}

void gpio_pull_up(uint gpio) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].pull_up = true;
        picofw_mock_pins[gpio].pull_down = false;
        picofw_mock_pins[gpio].ever_pulled = true;
    }
}

void gpio_pull_down(uint gpio) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].pull_down = true;
        picofw_mock_pins[gpio].pull_up = false;
        picofw_mock_pins[gpio].ever_pulled = true;
    }
}

void gpio_set_slew_rate(uint gpio, int slew) {
    (void)gpio;
    (void)slew;
}

void gpio_set_irq_enabled_with_callback(
    uint gpio,
    uint32_t event_mask,
    bool enabled,
    gpio_irq_callback_t callback
) {
    (void)event_mask;
    if (gpio == P7501_PIN_NSW5V) {
        s_mock_irq_cb = enabled ? callback : NULL;
    }
}

void adc_init(void) {
    picofw_mock_adc_initialized = true;
}

void adc_gpio_init(uint gpio) {
    if (gpio < 32u) {
        picofw_mock_pins[gpio].dir_out = false;
        picofw_mock_pins[gpio].pull_up = false;
        picofw_mock_pins[gpio].pull_down = false;
    }
}

void adc_select_input(uint input) {
    picofw_mock_adc_input = input;
}

uint16_t adc_read(void) {
    return picofw_mock_adc_raw;
}

/* ---- Minimal stubs for LED / Watchdog / GDB symbols referenced by picofw_hw.c ---- */
static picofw_led_state_t g_mock_led_state = {
    .mode = LED_MODE_HEARTBEAT_IDLE,
    .phase = 0u,
    .manual_lock = false,
    .current_on = false,
};

void picofw_led_set_auto_mode(pico_led_mode_t mode) {
    g_mock_led_state.mode = mode;
}

picofw_led_state_t picofw_led_get_state(void) {
    return g_mock_led_state;
}

void picofw_wdt_sleep_ms(uint32_t total_ms) {
    picofw_mock_now_us += (uint64_t)total_ms * 1000ULL;
}

bool gdb_rsp_is_active(void) {
    return false;
}

static void assert_nsw5v_strictly_hiz(void) {
    const picofw_mock_gpio_pin_t *p = &picofw_mock_pins[P7501_PIN_NSW5V];
    assert(!p->dir_out);
    assert(!p->pull_up);
    assert(!p->pull_down);
    assert(!p->ever_dir_out);
    assert(!p->ever_driven_high);
    assert(!p->ever_pulled);
}

int main(void) {
    mock_reset_hw();
    picofw_mock_pins[P7501_PIN_NSW5V].ext_input_level = true;
    picofw_hw_init();

    assert_nsw5v_strictly_hiz();
    assert(picofw_mock_adc_initialized);
    assert(!picofw_mock_pins[P7501_PIN_XRESET].dir_out);
    assert(!picofw_mock_pins[P7501_PIN_TXD].dir_out);
    assert(!picofw_mock_pins[P7501_PIN_RXD].dir_out);
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);

    /* Open-drain XRESET pulse must never drive HIGH and must restore Hi-Z Input afterwards */
    picofw_hw_pulse_xreset_ms(50u);
    assert(picofw_mock_pins[P7501_PIN_XRESET].ever_dir_out);
    assert(!picofw_mock_pins[P7501_PIN_XRESET].ever_driven_high);
    assert(!picofw_mock_pins[P7501_PIN_XRESET].dir_out);
    assert(!picofw_mock_pins[P7501_PIN_XRESET].pull_up);
    assert(!picofw_mock_pins[P7501_PIN_XRESET].pull_down);

    /* Open-drain SW_UART set LOW and release back to Hi-Z ADC input */
    picofw_hw_sw_uart_set_low(true);
    assert(picofw_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].out_level);
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].ever_driven_high);
    picofw_hw_sw_uart_set_low(false);
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].pull_up);
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].pull_down);

    /* ADC read path: ~4095 raw ~= 3300 mV and restores Hi-Z + no pulls */
    picofw_mock_adc_raw = 4095u;
    const uint32_t mv = picofw_hw_read_vdd_mv();
    assert((mv >= 3290u) && (mv <= 3310u));
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].pull_down);

    /* S01: Pure getters NEVER mutate fault latch or dropout counters */
    const uint32_t drops_before = picofw_hw_get_nsw5v_dropouts();
    assert(!picofw_hw_is_nsw5v_fault_latched());
    picofw_mock_pins[P7501_PIN_NSW5V].ext_input_level = false;
    picofw_mock_adc_raw = 0u;
    assert(!picofw_hw_is_nsw5v_high());
    assert(picofw_hw_read_vdd_mv() == 0u);
    assert(picofw_hw_get_nsw5v_dropouts() == drops_before);
    assert(!picofw_hw_is_nsw5v_fault_latched());

    /* Real GPIO falling-edge ISR triggers latch; poll keeps fault latched until explicit ACK (S08) */
    assert(s_mock_irq_cb != NULL);
    s_mock_irq_cb(P7501_PIN_NSW5V, GPIO_IRQ_EDGE_FALL);
    assert(picofw_hw_get_nsw5v_dropouts() == (drops_before + 1u));
    assert(picofw_hw_is_nsw5v_fault_latched());
    assert(!picofw_hw_is_rail_healthy());

    /* Cannot ACK while rail is LOW or while JIG is strapped LOW */
    assert(!picofw_hw_ack_fault());
    picofw_mock_pins[P7501_PIN_NSW5V].ext_input_level = true;
    picofw_mock_adc_raw = 4095u;
    picofw_mock_now_us += 60000ULL;
    picofw_hw_poll_rail_recovery();
    assert(picofw_hw_is_nsw5v_fault_latched());

    picofw_hw_sw_uart_set_low(true);
    assert(!picofw_hw_ack_fault());
    picofw_hw_sw_uart_set_low(false);

    /* ACK succeeds once unstrapped + NSW5V=1 + VDD 3.3V */
    assert(picofw_hw_ack_fault());
    assert(!picofw_hw_is_nsw5v_fault_latched());
    assert(picofw_hw_is_rail_healthy());

    /* Active operation with JIG strapped LOW reports jig_strapped_low=true and stays healthy while NSW5V=1 */
    picofw_hw_sw_uart_set_low(true);
    picofw_rail_snap_t jig_snap;
    picofw_hw_rail_snapshot(&jig_snap, true);
    assert(jig_snap.nsw5v_high && jig_snap.jig_strapped_low && (jig_snap.vdd_mv == 0u));
    assert(picofw_hw_is_rail_healthy());

    /* S02: Isolate bus restores strict Hi-Z with zero pulls on all P7501 + GP6/GP7 I2C pins */
    picofw_hw_isolate_bus_hiz();
    assert(!picofw_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert(!picofw_mock_pins[PICOFW_I2C_SDA_PIN].dir_out &&
           !picofw_mock_pins[PICOFW_I2C_SDA_PIN].pull_up &&
           !picofw_mock_pins[PICOFW_I2C_SDA_PIN].pull_down);
    assert(!picofw_mock_pins[PICOFW_I2C_SCL_PIN].dir_out &&
           !picofw_mock_pins[PICOFW_I2C_SCL_PIN].pull_up &&
           !picofw_mock_pins[PICOFW_I2C_SCL_PIN].pull_down);
    assert_nsw5v_strictly_hiz();

    printf("[HOST-TEST] test_picofw_hw_host (GP3 NSW5V Hi-Z, S01/S02/S08 IRQ+ACK+JIG, OD XRESET/SW_UART, isolate) passed.\n");
    return 0;
}
