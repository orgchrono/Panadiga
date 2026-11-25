/**
 * @file hardware/adc.h
 * @brief Stateful host-test mock for RP2040/RP2350 ADC helpers.
 */

#ifndef PICOFW_MOCK_HARDWARE_ADC_H
#define PICOFW_MOCK_HARDWARE_ADC_H

#include "pico/stdlib.h"
#include <stdint.h>

extern uint16_t picofw_mock_adc_raw;
extern bool picofw_mock_adc_initialized;
extern uint picofw_mock_adc_input;

void adc_init(void);
void adc_gpio_init(uint gpio);
void adc_select_input(uint input);
uint16_t adc_read(void);

#endif /* PICOFW_MOCK_HARDWARE_ADC_H */
