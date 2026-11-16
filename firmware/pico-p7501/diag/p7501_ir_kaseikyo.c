/**
 * @file p7501_ir_kaseikyo.c
 * @brief Hardware 38 kHz carrier PWM Kaseikyo IR transmitter (`IRTX`) on safe spare GPIOs.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_ir_kaseikyo.h"
#include "diag/p7501_ir_codec.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"

#include "hardware/clocks.h"
#include "hardware/gpio.h"
#include "hardware/pwm.h"
#include "pico/stdlib.h"

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
    ir_mark(slice, chan, level_on, P7501_IR_LEADER_MARK_US);
    ir_space(P7501_IR_LEADER_SPACE_US);

    for (size_t byte_idx = 0u; byte_idx < P7501_IR_KASEIKYO_FRAME_BYTES; ++byte_idx) {
        const uint8_t val = frame[byte_idx];
        for (uint8_t bit = 0u; bit < 8u; ++bit) {
            ir_mark(slice, chan, level_on, P7501_IR_UNIT_US);
            const bool is_one = ((val >> bit) & 1u) != 0u;
            ir_space(is_one ? P7501_IR_BIT_ONE_SPACE_US : P7501_IR_BIT_ZERO_SPACE_US);
        }
    }
    ir_mark(slice, chan, level_on, P7501_IR_UNIT_US);
}

void p7501_cmd_irtx(const char *arg) {
    p7501_ir_spec_t spec;
    if (!p7501_ir_parse_args(arg, &spec)) {
        printf("IRTX: ERROR (Usage: IRTX <RED|GREEN|YELLOW|BLUE|POWER|AUDIO|DISPLAY|SERV|ND2|0..9|sub:cmd> [GP6..9|16..22] [repeat])\n");
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

    const uint32_t slice = pwm_gpio_to_slice_num(spec.gpio);
    const uint32_t chan = pwm_gpio_to_channel(spec.gpio);

    gpio_set_function(spec.gpio, GPIO_FUNC_PWM);
    pwm_config cfg = pwm_get_default_config();
    pwm_config_set_wrap(&cfg, wrap);
    pwm_init(slice, &cfg, false);
    pwm_set_chan_level(slice, chan, 0u);
    pwm_set_enabled(slice, true);

    for (uint32_t r = 0u; r < spec.repeat; ++r) {
        if (r > 0u) {
            ir_space(P7501_IR_FRAME_GAP_US);
        }
        transmit_kaseikyo_frame(slice, chan, level_on, spec.frame);
        p7501_wdt_kick();
    }

    pwm_set_chan_level(slice, chan, 0u);
    pwm_set_enabled(slice, false);
    gpio_set_function(spec.gpio, GPIO_FUNC_SIO);
    gpio_set_dir(spec.gpio, GPIO_IN);
    gpio_disable_pulls(spec.gpio);
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

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
