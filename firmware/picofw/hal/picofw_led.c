/**
 * @file picofw_led.c
 * @brief Hardware LED driver with IRQ-safe 100ms repeating timer and CYW43 support.
 *
 * Fixes three real-world failure modes on the live P7501 bench:
 * 1. Immediate pin application when switching modes (`IDLE`, `GDB`, `FAULT`, `OFF`).
 * 2. Manual lock so 2-second Desktop `STATUS` telemetry never clobbers user `LED <mode>`.
 * 3. Non-blocking 10 Hz `repeating_timer` tick on RP2040/RP2350 GP25 (and chunked sleep
 *    fallback on CYW43 Pico W / Pico 2 W) so blocking UART captures never freeze the LED.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_led.h"
#include "hal/picofw_watchdog.h"

#include "hardware/gpio.h"
#include "hardware/sync.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#if defined(PICOFW_BLE_ENABLED) && (PICOFW_BLE_ENABLED != 0)
#include "hal/picofw_ble_nus.h"
#include "pico/cyw43_arch.h"
#endif

typedef struct picofw_led_drv {
    picofw_led_state_t state;
    absolute_time_t next_poll_tick;
#if !defined(PICOFW_BLE_ENABLED) || (PICOFW_BLE_ENABLED == 0)
    struct repeating_timer timer;
    bool timer_active;
#endif
} picofw_led_drv_t;

static picofw_led_drv_t s_led_drv;

static void drive_physical_led(bool on) {
#if defined(PICOFW_BLE_ENABLED) && (PICOFW_BLE_ENABLED != 0)
    if (picofw_ble_is_radio_ready()) {
        cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, on ? 1 : 0);
    }
#elif defined(PICO_DEFAULT_LED_PIN)
    gpio_put(PICO_DEFAULT_LED_PIN, on ? 1 : 0);
#else
    (void)on;
#endif
}

#if !defined(PICOFW_BLE_ENABLED) || (PICOFW_BLE_ENABLED == 0)
static bool led_timer_callback(struct repeating_timer *rt) {
    (void)rt;
    const uint32_t irq_state = save_and_disable_interrupts();
    const bool on = picofw_led_state_step_10hz(&s_led_drv.state);
    drive_physical_led(on);
    restore_interrupts(irq_state);
    return true;
}
#endif

void picofw_led_init(void) {
    const uint32_t irq_state = save_and_disable_interrupts();
    picofw_led_state_init(&s_led_drv.state);
#if (!defined(PICOFW_BLE_ENABLED) || (PICOFW_BLE_ENABLED == 0)) && defined(PICO_DEFAULT_LED_PIN)
    gpio_init(PICO_DEFAULT_LED_PIN);
    gpio_set_dir(PICO_DEFAULT_LED_PIN, GPIO_OUT);
#endif
    const bool initial_on = s_led_drv.state.current_on;
    restore_interrupts(irq_state);
    /* Drive physical LED outside IRQ-disabled section so CYW43 SPI never deadlocks. */
    drive_physical_led(initial_on);

    s_led_drv.next_poll_tick = make_timeout_time_ms(100u);

#if !defined(PICOFW_BLE_ENABLED) || (PICOFW_BLE_ENABLED == 0)
    if (!s_led_drv.timer_active) {
        s_led_drv.timer_active = add_repeating_timer_ms(-100, led_timer_callback, NULL, &s_led_drv.timer);
    }
#endif
}

void picofw_led_set_mode(pico_led_mode_t mode) {
    const uint32_t irq_state = save_and_disable_interrupts();
    picofw_led_state_set_manual(&s_led_drv.state, mode);
    const bool on = s_led_drv.state.current_on;
    restore_interrupts(irq_state);
    drive_physical_led(on);
    s_led_drv.next_poll_tick = make_timeout_time_ms(100u);
}

void picofw_led_set_auto_mode(pico_led_mode_t mode) {
    const uint32_t irq_state = save_and_disable_interrupts();
    const bool changed = picofw_led_state_set_auto(&s_led_drv.state, mode);
    const bool on = s_led_drv.state.current_on;
    restore_interrupts(irq_state);
    if (changed) {
        drive_physical_led(on);
        s_led_drv.next_poll_tick = make_timeout_time_ms(100u);
    }
}

void picofw_led_unlock_auto(pico_led_mode_t fallback_mode) {
    const uint32_t irq_state = save_and_disable_interrupts();
    picofw_led_state_unlock_auto(&s_led_drv.state, fallback_mode);
    const bool on = s_led_drv.state.current_on;
    restore_interrupts(irq_state);
    drive_physical_led(on);
    s_led_drv.next_poll_tick = make_timeout_time_ms(100u);
}

void picofw_led_notify_activity(void) {
    const uint32_t irq_state = save_and_disable_interrupts();
    picofw_led_state_pulse_activity(&s_led_drv.state);
    const bool on = s_led_drv.state.current_on;
    restore_interrupts(irq_state);
    drive_physical_led(on);
}

void picofw_led_tick(void) {
#if defined(PICOFW_BLE_ENABLED) && (PICOFW_BLE_ENABLED != 0)
    if (!time_reached(s_led_drv.next_poll_tick)) {
        return;
    }
    s_led_drv.next_poll_tick = make_timeout_time_ms(100u);
    const bool on = picofw_led_state_step_10hz(&s_led_drv.state);
    drive_physical_led(on);
#else
    if (!s_led_drv.timer_active && time_reached(s_led_drv.next_poll_tick)) {
        s_led_drv.next_poll_tick = make_timeout_time_ms(100u);
        const uint32_t irq_state = save_and_disable_interrupts();
        const bool on = picofw_led_state_step_10hz(&s_led_drv.state);
        drive_physical_led(on);
        restore_interrupts(irq_state);
    }
#endif
}

void picofw_led_sleep_ms(uint32_t delay_ms) {
    picofw_wdt_sleep_ms(delay_ms);
}

picofw_led_state_t picofw_led_get_state(void) {
    const uint32_t irq_state = save_and_disable_interrupts();
    const picofw_led_state_t snap = s_led_drv.state;
    restore_interrupts(irq_state);
    return snap;
}

const char *picofw_led_backend_label(void) {
#if defined(PICOFW_BLE_ENABLED) && (PICOFW_BLE_ENABLED != 0)
    return "CYW43_WL_GPIO0 (Pico W / Pico 2 W)";
#else
    return "GP25 (Standard Pico / RP2040 / RP2350)";
#endif
}
