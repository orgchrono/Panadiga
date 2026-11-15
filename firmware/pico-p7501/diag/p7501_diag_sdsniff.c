/**
 * @file p7501_diag_sdsniff.c
 * @brief Passive Hi-Z PIO0 SD Bus Sniffer (`SDSNIFF`) over GP10..GP15.
 *
 * Captures SD_CLK-synchronized CMD + DAT0..DAT3 frames passively without driving
 * any SD pin (`GPIO_IN` with pulls disabled, never `pio_gpio_init`) and decodes
 * CMD55/ACMD6/ACMD13 and per-lane CRC16 errors via `p7501_sds_codec`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_diag_sdsniff.h"

#include "diag/p7501_sds_codec.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "p7501_sds_sampler.pio.h"
#include "p7501_types.h"

#include "hardware/gpio.h"
#include "hardware/pio.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

static void sdsniff_enforce_hiz_inputs(void) {
    static const uint k_pins[6] = {
        P7501_PIN_SD_CLK,
        P7501_PIN_SD_CMD,
        P7501_PIN_SD_DAT0,
        P7501_PIN_SD_DAT1,
        P7501_PIN_SD_DAT2,
        P7501_PIN_SD_DAT3
    };
    gpio_set_dir(P7501_PIN_NSW5V, GPIO_IN);
    gpio_disable_pulls(P7501_PIN_NSW5V);
    for (size_t i = 0u; i < 6u; ++i) {
        gpio_init(k_pins[i]);
        gpio_set_dir(k_pins[i], GPIO_IN);
        gpio_disable_pulls(k_pins[i]);
    }
}

void p7501_cmd_sdsniff(const char *arg) {
    p7501_sds_spec_t spec;
    if (!p7501_sds_parse_args(arg, &spec)) {
        printf("SDSNIFF: ERROR (usage: SDSNIFF [ms] [NORMAL|JIG[:ms]|NOPULL] [LEN:64|512])\n");
        return;
    }

    sdsniff_enforce_hiz_inputs();
    if (!pio_can_add_program(pio0, &p7501_sds_sampler_program)) {
        printf("SDSNIFF: ERROR (No PIO0 instruction slots left)\n");
        return;
    }
    const int sm_signed = pio_claim_unused_sm(pio0, false);
    if (sm_signed < 0) {
        printf("SDSNIFF: ERROR (No free PIO0 state machine)\n");
        return;
    }

    const uint sm = (uint)sm_signed;
    const uint offset = pio_add_program(pio0, &p7501_sds_sampler_program);
    p7501_sds_sampler_program_init(pio0, sm, offset, P7501_PIN_SD_CMD);

    p7501_sds_state_t sds;
    p7501_sds_init(&sds, spec.default_block_bytes);

    bool hold_jig_active = false;
    absolute_time_t jig_deadline = at_the_end_of_time;
    if (spec.boot.boot_action != P7501_BOOT_ACTION_PASSIVE) {
        if (spec.boot.boot_action == P7501_BOOT_ACTION_JIG) {
            p7501_hw_sw_uart_set_low(true);
            hold_jig_active = (spec.boot.jig_hold_ms > 0u);
        } else {
            p7501_hw_sw_uart_set_low(false);
        }
        sleep_ms(5u);
        p7501_hw_reset_assert();
        sleep_ms(15u);
        p7501_hw_reset_release();
        if (hold_jig_active) {
            jig_deadline = make_timeout_time_ms(spec.boot.jig_hold_ms);
        }
    }

    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    pio_sm_clear_fifos(pio0, sm);
    pio0->fdebug = (1u << (PIO_FDEBUG_RXSTALL_LSB + sm));
    pio_sm_set_enabled(pio0, sm, true);

    const absolute_time_t end_t = make_timeout_time_ms(spec.boot.window_ms);
    while (!time_reached(end_t)) {
        p7501_led_tick();
        if (hold_jig_active && time_reached(jig_deadline)) {
            p7501_hw_sw_uart_set_low(false);
            p7501_hw_restore_sw_uart_adc0();
            hold_jig_active = false;
        }
        while (!pio_sm_is_rx_fifo_empty(pio0, sm)) {
            const uint32_t w = pio_sm_get(pio0, sm);
            for (uint32_t i = 0u; i < 4u; ++i) {
                p7501_sds_feed_sample(&sds, (uint8_t)((w >> (i * 8u)) & 0x1Fu));
            }
        }
    }

    const bool rx_stall = (pio0->fdebug & (1u << (PIO_FDEBUG_RXSTALL_LSB + sm))) != 0u;
    pio_sm_set_enabled(pio0, sm, false);
    pio_remove_program(pio0, &p7501_sds_sampler_program, offset);
    pio_sm_unclaim(pio0, sm);
    p7501_hw_isolate_bus_hiz();
    sdsniff_enforce_hiz_inputs();

    printf("=== SDSNIFF PIO0 (GP10=CLK GP11=CMD GP12..15=DAT0..3, %" PRIu32 " ms) ===\n",
           spec.boot.window_ms);
    printf("Clocks=%" PRIu32 " | fifo_stall=%s | BusWidth=%ubit | CMD55=%s ACMD6_4bit=%s ACMD13=%s\n",
           sds.total_clocks,
           rx_stall ? "YES" : "NO",
           (unsigned)sds.bus_width,
           sds.saw_cmd55 ? "YES" : "NO",
           sds.saw_acmd6_4bit ? "YES" : "NO",
           sds.saw_acmd13 ? "YES" : "NO");
    printf("Transitions: CMD=%" PRIu32 " DAT0=%" PRIu32 " DAT1=%" PRIu32 " DAT2=%" PRIu32 " DAT3=%" PRIu32 "\n",
           sds.transitions[0], sds.transitions[1], sds.transitions[2], sds.transitions[3], sds.transitions[4]);
    printf("CMD ok=%lu resp_ok=%lu cmd_crc_err=%lu CMD17=%lu CMD18=%lu | Data ok=%lu err=%lu | StartBadMask=0x%02X FailMask=0x%02X\n",
           (unsigned long)sds.host_cmds_ok,
           (unsigned long)sds.card_resps_ok,
           (unsigned long)sds.cmd_crc_err,
           (unsigned long)sds.cmd17_count,
           (unsigned long)sds.cmd18_count,
           (unsigned long)sds.data_blocks_ok,
           (unsigned long)sds.data_crc_err,
           (unsigned)sds.start_bit_bad_mask,
           (unsigned)sds.last_fail_lane_mask);
    printf("Per-Lane CRC16 Errors: DAT0=%lu DAT1=%lu DAT2=%lu DAT3=%lu\n",
           (unsigned long)sds.lane_crc_err[0],
           (unsigned long)sds.lane_crc_err[1],
           (unsigned long)sds.lane_crc_err[2],
           (unsigned long)sds.lane_crc_err[3]);
    if (sds.acmd13_captured) {
        printf("ACMD13: crc=%s dat_bus_width=%u sd_card_type=0x%04X head=",
               sds.acmd13_crc_ok ? "OK" : "FAIL",
               (unsigned)sds.acmd13_dat_bus_width,
               (unsigned)sds.acmd13_sd_card_type);
        for (size_t i = 0u; i < 8u; ++i) {
            printf("%02X", (unsigned)sds.acmd13_raw[i]);
        }
        printf("\n");
    }
    if (sds.trace_count > 0u) {
        printf("SD Trace (%u/%lu cmds; SDSC CMD17/18 byte_addr/512 -> LBA):\n",
               (unsigned)sds.trace_count,
               (unsigned long)sds.trace_total);
        for (uint8_t i = 0u; i < sds.trace_count; ++i) {
            const p7501_sds_trace_entry_t *ent = &sds.trace[i];
            if (!ent->is_app && (ent->cmd == 17u || ent->cmd == 18u)) {
                printf("  [%02u @clk=%lu] CMD%u arg=0x%08lX (SDSC_LBA=%lu rem=%lu)\n",
                       (unsigned)i,
                       (unsigned long)ent->clk,
                       (unsigned)ent->cmd,
                       (unsigned long)ent->arg,
                       (unsigned long)(ent->arg / 512u),
                       (unsigned long)(ent->arg % 512u));
            } else {
                printf("  [%02u @clk=%lu] %s%u arg=0x%08lX\n",
                       (unsigned)i,
                       (unsigned long)ent->clk,
                       ent->is_app ? "ACMD" : "CMD",
                       (unsigned)ent->cmd,
                       (unsigned long)ent->arg);
            }
        }
    }
    printf("SDSNIFF: OK\n");
}
