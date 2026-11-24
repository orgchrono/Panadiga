/**
 * @file hardware/gpio.h
 * @brief Stateful host-test mock for RP2040/RP2350 GPIO direction, pulls, and Open-Drain verification.
 */

#ifndef PANADIGA_MOCK_HARDWARE_GPIO_H
#define PANADIGA_MOCK_HARDWARE_GPIO_H

#include "pico/stdlib.h"
#include <stdbool.h>
#include <stdint.h>

#define GPIO_IN 0
#define GPIO_OUT 1

#define GPIO_FUNC_SIO 5u
#define GPIO_IRQ_EDGE_FALL 0x4u
#define GPIO_SLEW_RATE_SLOW 0

typedef void (*gpio_irq_callback_t)(uint gpio, uint32_t events);

typedef struct p7501_mock_gpio_pin {
    bool initialized;
    bool dir_out;
    bool out_level;
    bool ext_input_level;
    bool pull_up;
    bool pull_down;
    uint func;
    bool ever_dir_out;
    bool ever_driven_high;
    bool ever_pulled;
} p7501_mock_gpio_pin_t;

extern p7501_mock_gpio_pin_t p7501_mock_pins[32];

void gpio_init(uint gpio);
void gpio_set_function(uint gpio, uint fn);
void gpio_set_dir(uint gpio, bool out);
bool gpio_is_dir_out(uint gpio);
void gpio_put(uint gpio, bool value);
bool gpio_get(uint gpio);
uint32_t gpio_get_all(void);
void gpio_disable_pulls(uint gpio);
void gpio_pull_up(uint gpio);
void gpio_pull_down(uint gpio);
void gpio_set_slew_rate(uint gpio, int slew);
void gpio_set_irq_enabled_with_callback(
    uint gpio,
    uint32_t event_mask,
    bool enabled,
    gpio_irq_callback_t callback
);

#endif /* PANADIGA_MOCK_HARDWARE_GPIO_H */
