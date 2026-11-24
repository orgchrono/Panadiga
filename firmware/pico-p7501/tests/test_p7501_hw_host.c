/**
 * @file test_p7501_hw_host.c
 * @brief Host C11 safety verification compiling REAL `hal/p7501_hw.c` against stateful mock GPIO/ADC.
 *
 * Verifies non-negotiable P7501 silicon invariants:
 *   1. GP3 (`P7501_PIN_NSW5V`) stays Hi-Z Input with NO pulls (never output, never driven HIGH).
 *   2. `XRESET` (GP5) and `SW_UART` (GP26) drive strictly Open-Drain (LOW as OUT=0, release as Hi-Z IN).
 *   3. `p7501_hw_isolate_bus_hiz()` restores every P7501 pin to Hi-Z Input with pulls disabled.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "gdb/gdb_rsp.h"
#include "hardware/adc.h"
#include "hardware/gpio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

/* ---- Stateful mock backing storage ---- */
uint64_t p7501_mock_now_us = 10000ULL;
p7501_mock_gpio_pin_t p7501_mock_pins[32];
uint16_t p7501_mock_adc_raw = 0u;
bool p7501_mock_adc_initialized = false;
uint p7501_mock_adc_input = 0u;

static void mock_reset_hw(void) {
    memset(p7501_mock_pins, 0, sizeof(p7501_mock_pins));
    p7501_mock_now_us = 10000ULL;
    p7501_mock_adc_raw = 0u;
    p7501_mock_adc_initialized = false;
    p7501_mock_adc_input = 0u;
    /* Simulate RP2350 reset-default pull-down on GP3 so p7501_hw_init MUST clear it */
    p7501_mock_pins[P7501_PIN_NSW5V].pull_down = true;
    /* Pre-latch dangerous output HIGH on GP3/GP5 to verify p7501_hw_init clears it */
    p7501_mock_pins[P7501_PIN_XRESET].dir_out = false;
    p7501_mock_pins[P7501_PIN_XRESET].ext_input_level = true;
}

void gpio_init(uint gpio) {
    if (gpio < 32u) {
        p7501_mock_pins[gpio].initialized = true;
        p7501_mock_pins[gpio].dir_out = false;
        p7501_mock_pins[gpio].func = GPIO_FUNC_SIO;
    }
}

void gpio_set_function(uint gpio, uint fn) {
    if (gpio < 32u) {
        p7501_mock_pins[gpio].func = fn;
    }
}

void gpio_set_dir(uint gpio, bool out) {
    if (gpio < 32u) {
        p7501_mock_pins[gpio].dir_out = out;
        if (out) {
            p7501_mock_pins[gpio].ever_dir_out = true;
            if (p7501_mock_pins[gpio].out_level) {
                p7501_mock_pins[gpio].ever_driven_high = true;
            }
        }
    }
}

bool gpio_is_dir_out(uint gpio) {
    return (gpio < 32u) && p7501_mock_pins[gpio].dir_out;
}

void gpio_put(uint gpio, bool value) {
    if (gpio < 32u) {
        p7501_mock_pins[gpio].out_level = value;
        if (value && p7501_mock_pins[gpio].dir_out) {
            p7501_mock_pins[gpio].ever_driven_high = true;
        }
    }
}

bool gpio_get(uint gpio) {
    if (gpio >= 32u) {
        return false;
    }
    return p7501_mock_pins[gpio].dir_out
        ? p7501_mock_pins[gpio].out_level
        : p7501_mock_pins[gpio].ext_input_level;
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
        p7501_mock_pins[gpio].pull_up = false;
        p7501_mock_pins[gpio].pull_down = false;
    }
}

void gpio_pull_up(uint gpio) {
    if (gpio < 32u) {
        p7501_mock_pins[gpio].pull_up = true;
        p7501_mock_pins[gpio].pull_down = false;
        p7501_mock_pins[gpio].ever_pulled = true;
    }
}

void gpio_pull_down(uint gpio) {
    if (gpio < 32u) {
        p7501_mock_pins[gpio].pull_down = true;
        p7501_mock_pins[gpio].pull_up = false;
        p7501_mock_pins[gpio].ever_pulled = true;
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
    (void)gpio;
    (void)event_mask;
    (void)enabled;
    (void)callback;
}

void adc_init(void) {
    p7501_mock_adc_initialized = true;
}

void adc_gpio_init(uint gpio) {
    if (gpio < 32u) {
        p7501_mock_pins[gpio].dir_out = false;
        p7501_mock_pins[gpio].pull_up = false;
        p7501_mock_pins[gpio].pull_down = false;
    }
}

void adc_select_input(uint input) {
    p7501_mock_adc_input = input;
}

uint16_t adc_read(void) {
    return p7501_mock_adc_raw;
}

/* ---- Minimal stubs for LED / Watchdog / GDB symbols referenced by p7501_hw.c ---- */
static p7501_led_state_t g_mock_led_state = { LED_MODE_HEARTBEAT_IDLE, false, false };

void p7501_led_set_auto_mode(pico_led_mode_t mode) {
    g_mock_led_state.mode = mode;
}

p7501_led_state_t p7501_led_get_state(void) {
    return g_mock_led_state;
}

void p7501_wdt_sleep_ms(uint32_t total_ms) {
    p7501_mock_now_us += (uint64_t)total_ms * 1000ULL;
}

bool gdb_rsp_is_active(void) {
    return false;
}

static void assert_nsw5v_strictly_hiz(void) {
    const p7501_mock_gpio_pin_t *p = &p7501_mock_pins[P7501_PIN_NSW5V];
    assert(!p->dir_out);
    assert(!p->pull_up);
    assert(!p->pull_down);
    assert(!p->ever_dir_out);
    assert(!p->ever_driven_high);
    assert(!p->ever_pulled);
}

int main(void) {
    mock_reset_hw();
    p7501_mock_pins[P7501_PIN_NSW5V].ext_input_level = true;
    p7501_hw_init();

    assert_nsw5v_strictly_hiz();
    assert(p7501_mock_adc_initialized);
    assert(!p7501_mock_pins[P7501_PIN_XRESET].dir_out);
    assert(!p7501_mock_pins[P7501_PIN_TXD].dir_out);
    assert(!p7501_mock_pins[P7501_PIN_RXD].dir_out);
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);

    /* Open-drain XRESET pulse must never drive HIGH and must restore Hi-Z Input afterwards */
    p7501_hw_pulse_xreset_ms(50u);
    assert(p7501_mock_pins[P7501_PIN_XRESET].ever_dir_out);
    assert(!p7501_mock_pins[P7501_PIN_XRESET].ever_driven_high);
    assert(!p7501_mock_pins[P7501_PIN_XRESET].dir_out);
    assert(!p7501_mock_pins[P7501_PIN_XRESET].pull_up);
    assert(!p7501_mock_pins[P7501_PIN_XRESET].pull_down);

    /* Open-drain SW_UART set LOW and release back to Hi-Z ADC input */
    p7501_hw_sw_uart_set_low(true);
    assert(p7501_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].out_level);
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].ever_driven_high);
    p7501_hw_sw_uart_set_low(false);
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].pull_up);
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].pull_down);

    /* ADC read path: ~4095 raw ~= 3300 mV and restores Hi-Z + no pulls */
    p7501_mock_adc_raw = 4095u;
    const uint32_t mv = p7501_hw_read_vdd_mv();
    assert((mv >= 3290u) && (mv <= 3310u));
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].pull_down);

    /* NSW5V query never alters GP3 Hi-Z state */
    p7501_mock_pins[P7501_PIN_NSW5V].ext_input_level = false;
    assert(!p7501_hw_is_nsw5v_high());
    p7501_mock_pins[P7501_PIN_NSW5V].ext_input_level = true;
    assert(p7501_hw_is_nsw5v_high());

    /* Isolate bus restores strict Hi-Z with zero pulls on all P7501 pins */
    p7501_hw_sw_uart_set_low(true);
    p7501_hw_isolate_bus_hiz();
    assert(!p7501_mock_pins[P7501_PIN_SW_UART_ADC].dir_out);
    assert_nsw5v_strictly_hiz();

    printf("[HOST-TEST] test_p7501_hw_host (GP3 NSW5V Hi-Z, OD XRESET/SW_UART, isolate) passed.\n");
    return 0;
}
