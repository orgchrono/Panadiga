/**
 * @file p7501_ir_kaseikyo.c
 * @brief Hardware 38 kHz carrier PWM Kaseikyo IR transmitter (`IRTX`) and receiver (`IRRX`) on safe spare GPIOs.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_ir_kaseikyo.h"
#include "cli/p7501_cli.h"
#include "diag/p7501_ir_codec.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"

#include "hardware/clocks.h"
#include "hardware/gpio.h"
#include "hardware/pwm.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

static void ir_mark(uint32_t slice, uint32_t chan, uint16_t level_on, uint32_t us) {
    pwm_set_chan_level(slice, chan, level_on);
    busy_wait_us_32(us);
    pwm_set_chan_level(slice, chan, 0u);
}

static void ir_space(uint32_t us) {
    busy_wait_us_32(us);
}

static void transmit_kaseikyo_frame(
    uint32_t slice,
    uint32_t chan,
    uint16_t level_on,
    const uint8_t frame[P7501_IR_KASEIKYO_FRAME_BYTES]
) {
    uint32_t durations[P7501_IR_PULSE_COUNT];
    const size_t n = p7501_ir_build_pulse_table(frame, durations, P7501_IR_PULSE_COUNT);
    for (size_t i = 0u; i < n; ++i) {
        if ((i & 1u) == 0u) {
            ir_mark(slice, chan, level_on, durations[i]);
        } else {
            ir_space(durations[i]);
        }
    }
}

void p7501_ir_transmit_spec(const p7501_ir_spec_t *spec) {
    if (spec == NULL) {
        return;
    }
    p7501_hw_enforce_nsw5v_hiz();
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_wdt_set_stage(P7501_WDT_STAGE_TX_WAIT);

    const uint32_t sys_hz = clock_get_hz(clk_sys);
    uint16_t wrap = (uint16_t)((sys_hz / P7501_IR_CARRIER_HZ) - 1UL);
    if (wrap < 100u) {
        wrap = 3288u;
    }
    const uint16_t level_on = (uint16_t)(wrap / 3u);

    const uint32_t slice = pwm_gpio_to_slice_num(spec->gpio);
    const uint32_t chan = pwm_gpio_to_channel(spec->gpio);

    gpio_set_function(spec->gpio, GPIO_FUNC_PWM);
    pwm_config cfg = pwm_get_default_config();
    pwm_config_set_wrap(&cfg, wrap);
    pwm_init(slice, &cfg, false);
    pwm_set_chan_level(slice, chan, 0u);
    pwm_set_enabled(slice, true);

    for (uint32_t r = 0u; r < spec->repeat; ++r) {
        if (r > 0u) {
            ir_space(P7501_IR_FRAME_GAP_US);
        }
        transmit_kaseikyo_frame(slice, chan, level_on, spec->frame);
        p7501_wdt_kick();
    }

    pwm_set_chan_level(slice, chan, 0u);
    pwm_set_enabled(slice, false);
    gpio_set_function(spec->gpio, GPIO_FUNC_SIO);
    gpio_set_dir(spec->gpio, GPIO_IN);
    gpio_disable_pulls(spec->gpio);
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
}

void p7501_cmd_irtx(const char *arg) {
    p7501_ir_spec_t spec;
    if (!p7501_ir_parse_args(arg, &spec)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("IRTX: ERROR (Usage: IRTX <preset|sub:cmd|cmd> [GP6..9|16..22] [repeat])\n");
        return;
    }
    p7501_ir_transmit_spec(&spec);
    printf("IRTX: OK (cmd=%s gpio=GP%" PRIu32 " repeat=%" PRIu32 " frame=%02X%02X%02X%02X%02X%02X)\n",
           spec.label,
           spec.gpio,
           spec.repeat,
           (unsigned)spec.frame[0],
           (unsigned)spec.frame[1],
           (unsigned)spec.frame[2],
           (unsigned)spec.frame[3],
           (unsigned)spec.frame[4],
           (unsigned)spec.frame[5]);
}

void p7501_cmd_irrx(const char *arg) {
    uint32_t window_ms = 5000u;
    uint32_t gpio = P7501_IR_DEFAULT_RX_GPIO;
    if (!p7501_ir_parse_rx_args(arg, &window_ms, &gpio)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("IRRX: ERROR (Usage: IRRX [ms] [GP6..9|16..22])\n");
        return;
    }

    p7501_hw_enforce_nsw5v_hiz();
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_wdt_set_stage(P7501_WDT_STAGE_RX_WAIT);

    gpio_init(gpio);
    gpio_set_dir(gpio, GPIO_IN);
    gpio_pull_up(gpio);

    uint32_t pulses[P7501_IR_PULSE_COUNT];
    size_t count = 0u;
    bool in_frame = false;
    bool prev_mark = !gpio_get(gpio);
    uint64_t last_edge_us = time_us_64();

    const uint64_t t0_us = last_edge_us;
    const uint64_t deadline_us = t0_us + ((uint64_t)window_ms * 1000ULL);
    while (p7501_wdt_loop_step(t0_us, deadline_us, true, NULL)) {
        const bool now_mark = !gpio_get(gpio);
        const uint64_t now_us = time_us_64();
        if (now_mark != prev_mark) {
            const uint32_t dt_us = (uint32_t)(now_us - last_edge_us);
            last_edge_us = now_us;
            prev_mark = now_mark;
            if (!in_frame) {
                if (!now_mark && (dt_us >= 2200u) && (dt_us <= 4800u)) {
                    in_frame = true;
                    pulses[0] = dt_us;
                    count = 1u;
                }
            } else if (count < P7501_IR_PULSE_COUNT) {
                pulses[count++] = dt_us;
                if (count >= P7501_IR_PULSE_COUNT) {
                    break;
                }
            }
        } else if (in_frame && ((now_us - last_edge_us) > 15000ULL)) {
            if (count >= (P7501_IR_PULSE_COUNT - 1u)) {
                break;
            }
            in_frame = false;
            count = 0u;
        }
    }

    gpio_set_dir(gpio, GPIO_IN);
    gpio_disable_pulls(gpio);
    p7501_hw_enforce_nsw5v_hiz();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    p7501_ir_decode_result_t res;
    if (!p7501_ir_decode_pulses(pulses, count, &res)) {
        p7501_cli_set_result("ERR", "ETIMEOUT");
        printf("IRRX: TIMEOUT (gpio=GP%" PRIu32 " pulses=%u)\n", gpio, (unsigned)count);
        return;
    }

    const char *preset = p7501_ir_lookup_preset_name(res.dev, res.sub, res.cmd);
    printf("IRRX: OK (gpio=GP%" PRIu32 " pulses=%u valid=%s parity=%s dev=0x%02X sub=0x%02X cmd=0x%02X preset=%s frame=%02X%02X%02X%02X%02X%02X)\n",
           gpio,
           (unsigned)count,
           res.valid ? "YES" : "NO",
           res.parity_ok ? "OK" : "BAD",
           (unsigned)res.dev,
           (unsigned)res.sub,
           (unsigned)res.cmd,
           (preset != NULL) ? preset : "CUSTOM",
           (unsigned)res.frame[0],
           (unsigned)res.frame[1],
           (unsigned)res.frame[2],
           (unsigned)res.frame[3],
           (unsigned)res.frame[4],
           (unsigned)res.frame[5]);
}
