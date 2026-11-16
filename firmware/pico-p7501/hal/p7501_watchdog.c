/**
 * @file p7501_watchdog.c
 * @brief Imperative RP2040/RP2350 hardware watchdog HAL, loop guard, and command traceability.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/p7501_watchdog.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/structs/watchdog.h"
#include "hardware/watchdog.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

static p7501_wdt_telemetry_t s_wdt_state = {
    .caused_reboot = false,
    .recoveries = 0u,
    .last_fault_cmd_id = P7501_WDT_CMD_IDLE,
    .last_flags = 0u,
    .cmd_seq = 0u,
    .last_cmd_name = "NONE",
    .last_cmd_elapsed_us = 0ULL,
};

static uint64_t s_active_cmd_start_us = 0ULL;
static uint8_t s_active_cmd_id = P7501_WDT_CMD_IDLE;
static const char *s_active_cmd_name = "NONE";

static void write_scratch(uint8_t cmd_id, uint8_t flags) {
    watchdog_hw->scratch[0] = p7501_wdt_pack_scratch0(cmd_id, flags, s_wdt_state.recoveries);
    watchdog_hw->scratch[1] = P7501_WDT_SCRATCH_MAGIC;
    watchdog_hw->scratch[2] = s_wdt_state.cmd_seq;
    watchdog_hw->scratch[3] = to_ms_since_boot(get_absolute_time());
}

void p7501_wdt_init(void) {
    const uint32_t s0 = watchdog_hw->scratch[0];
    const uint32_t s1 = watchdog_hw->scratch[1];
    const uint32_t s2 = watchdog_hw->scratch[2];

    uint8_t prev_cmd = P7501_WDT_CMD_IDLE;
    uint8_t prev_flags = 0u;
    uint16_t prev_recoveries = 0u;
    const bool valid_magic = (s1 == P7501_WDT_SCRATCH_MAGIC) &&
                             p7501_wdt_unpack_scratch0(s0, &prev_cmd, &prev_flags, &prev_recoveries);

    if (watchdog_caused_reboot()) {
        p7501_hw_isolate_bus_hiz();
        s_wdt_state.caused_reboot = true;
        s_wdt_state.recoveries = valid_magic ? (uint16_t)(prev_recoveries + 1u) : 1u;
        s_wdt_state.last_fault_cmd_id = valid_magic ? prev_cmd : P7501_WDT_CMD_IDLE;
        s_wdt_state.last_flags = valid_magic ? (uint8_t)(prev_flags | P7501_WDT_FLAG_RECOVERED)
                                             : P7501_WDT_FLAG_RECOVERED;
        s_wdt_state.cmd_seq = valid_magic ? s2 : 0u;
    } else if (valid_magic) {
        s_wdt_state.recoveries = prev_recoveries;
        s_wdt_state.cmd_seq = s2;
    }

    write_scratch(P7501_WDT_CMD_IDLE, s_wdt_state.last_flags);
    watchdog_enable(P7501_WDT_TIMEOUT_MS, true);
    watchdog_update();
}

void p7501_wdt_kick(void) {
    p7501_pio_enforce_nsw5v_hiz();
    watchdog_update();
    p7501_led_tick();
}

void p7501_wdt_sleep_ms(uint32_t total_ms) {
    uint32_t remaining = total_ms;
    while (remaining > 0u) {
        const uint32_t slice = (remaining > P7501_WDT_SLEEP_SLICE_MS)
                                   ? P7501_WDT_SLEEP_SLICE_MS
                                   : remaining;
        sleep_ms(slice);
        p7501_wdt_kick();
        remaining -= slice;
    }
}

void p7501_wdt_begin_cmd(uint8_t cmd_id, const char *cmd_name) {
    s_wdt_state.cmd_seq += 1u;
    s_active_cmd_id = cmd_id;
    s_active_cmd_name = (cmd_name != NULL) ? cmd_name : "UNKNOWN";
    s_active_cmd_start_us = time_us_64();
    write_scratch(cmd_id, P7501_WDT_FLAG_IN_CMD);
    p7501_wdt_kick();
}

void p7501_wdt_end_cmd(bool aborted) {
    const uint64_t now_us = time_us_64();
    s_wdt_state.last_cmd_name = s_active_cmd_name;
    s_wdt_state.last_cmd_elapsed_us = (now_us >= s_active_cmd_start_us)
                                          ? (now_us - s_active_cmd_start_us)
                                          : 0ULL;
    const uint8_t flags = aborted ? P7501_WDT_FLAG_ABORTED : 0u;
    s_wdt_state.last_flags = flags;
    s_active_cmd_id = P7501_WDT_CMD_IDLE;
    write_scratch(P7501_WDT_CMD_IDLE, flags);
    p7501_wdt_kick();
}

bool p7501_wdt_loop_step(
    uint64_t start_us,
    uint64_t deadline_us,
    bool allow_cdc_abort,
    bool *out_aborted
) {
    p7501_wdt_kick();
    if (allow_cdc_abort) {
        const int ch = getchar_timeout_us(0);
        if (p7501_wdt_is_abort_char(ch)) {
            if (out_aborted != NULL) {
                *out_aborted = true;
            }
            return false;
        }
    }
    const uint64_t now_us = time_us_64();
    const uint64_t max_ceiling_us =
        start_us + ((uint64_t)P7501_WDT_MAX_CMD_WINDOW_MS * 1000ULL);
    const uint64_t effective_deadline =
        (deadline_us < max_ceiling_us) ? deadline_us : max_ceiling_us;
    if (now_us >= effective_deadline) {
        return false;
    }
    tight_loop_contents();
    return true;
}

p7501_wdt_telemetry_t p7501_wdt_snapshot(void) {
    return s_wdt_state;
}

void p7501_wdt_print_summary_line(void) {
    printf("[WDT] wdt=ACTIVE(%ums,NON_OVERRIDABLE) seq=%" PRIu32
           " wdt_recoveries=%u reboot_src=%s fault_slot=%u last_cmd=%s last_us=%" PRIu64 "\n",
           (unsigned)P7501_WDT_TIMEOUT_MS,
           s_wdt_state.cmd_seq,
           (unsigned)s_wdt_state.recoveries,
           s_wdt_state.caused_reboot ? "WDT_RECOVERY" : "CLEAN",
           (unsigned)s_wdt_state.last_fault_cmd_id,
           (s_wdt_state.last_cmd_name != NULL) ? s_wdt_state.last_cmd_name : "NONE",
           s_wdt_state.last_cmd_elapsed_us);
}

void p7501_cmd_wdt_status(const char *arg) {
    (void)arg;
    p7501_wdt_kick();
    p7501_wdt_print_summary_line();
    printf("WDT: OK (Hardware auto-recovery active @ %ums; override disabled)\n",
           (unsigned)P7501_WDT_TIMEOUT_MS);
}
