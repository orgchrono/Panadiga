/**
 * @file picofw_watchdog.c
 * @brief Imperative RP2040/RP2350 hardware watchdog HAL, loop guard, and command traceability.
 *
 * Strictly layered inside `hal/` with zero dependency on upper `uart/` drivers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_watchdog.h"
#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"

#include "hardware/structs/watchdog.h"
#include "hardware/watchdog.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <stdio.h>

static picofw_wdt_postmortem_ring_t s_pm_ring __attribute__((section(".uninitialized_data")));

typedef struct picofw_wdt_drv {
    picofw_wdt_telemetry_t telemetry;
    uint64_t active_cmd_start_us;
    uint8_t active_cmd_id;
    uint8_t active_stage;
    const char *active_cmd_name;
} picofw_wdt_drv_t;

static picofw_wdt_drv_t s_wdt_drv = {
    .telemetry = {
        .caused_reboot = false,
        .recoveries = 0u,
        .last_fault_cmd_id = PICOFW_WDT_CMD_IDLE,
        .last_fault_stage = PICOFW_WDT_STAGE_IDLE,
        .last_flags = 0u,
        .cmd_seq = 0u,
        .last_cmd_name = "NONE",
        .last_cmd_elapsed_us = 0ULL,
    },
    .active_cmd_start_us = 0ULL,
    .active_cmd_id = PICOFW_WDT_CMD_IDLE,
    .active_stage = PICOFW_WDT_STAGE_IDLE,
    .active_cmd_name = "NONE",
};

static void write_scratch(uint8_t cmd_id, uint8_t flags, uint8_t stage) {
    watchdog_hw->scratch[0] = picofw_wdt_pack_scratch0(cmd_id, flags, s_wdt_drv.telemetry.recoveries);
    watchdog_hw->scratch[1] = PICOFW_WDT_SCRATCH_MAGIC;
    watchdog_hw->scratch[2] = s_wdt_drv.telemetry.cmd_seq;
    watchdog_hw->scratch[3] = picofw_wdt_pack_scratch3(stage, to_ms_since_boot(get_absolute_time()));
}

void picofw_wdt_init(void) {
    const uint32_t s0 = watchdog_hw->scratch[0];
    const uint32_t s1 = watchdog_hw->scratch[1];
    const uint32_t s2 = watchdog_hw->scratch[2];
    const uint32_t s3 = watchdog_hw->scratch[3];

    uint8_t prev_cmd = PICOFW_WDT_CMD_IDLE;
    uint8_t prev_flags = 0u;
    uint16_t prev_recoveries = 0u;
    uint8_t prev_stage = PICOFW_WDT_STAGE_IDLE;
    uint32_t prev_uptime_ms = 0u;
    const bool valid_magic = (s1 == PICOFW_WDT_SCRATCH_MAGIC) &&
                             picofw_wdt_unpack_scratch0(s0, &prev_cmd, &prev_flags, &prev_recoveries);
    (void)picofw_wdt_unpack_scratch3(s3, &prev_stage, &prev_uptime_ms);

    picofw_wdt_pm_init_or_recover(&s_pm_ring);
    if (picofw_wdt_is_genuine_timeout_recovery(
            watchdog_caused_reboot(),
            watchdog_enable_caused_reboot(),
            valid_magic)) {
        picofw_hw_isolate_bus_hiz();
        s_wdt_drv.telemetry.caused_reboot = true;
        s_wdt_drv.telemetry.recoveries = (uint16_t)(prev_recoveries + 1u);
        s_wdt_drv.telemetry.last_fault_cmd_id = prev_cmd;
        s_wdt_drv.telemetry.last_fault_stage = prev_stage;
        s_wdt_drv.telemetry.last_flags = (uint8_t)(prev_flags | PICOFW_WDT_FLAG_RECOVERED);
        s_wdt_drv.telemetry.cmd_seq = s2;
        picofw_wdt_pm_record(
            &s_pm_ring, s2, prev_uptime_ms, 0u, prev_cmd, prev_stage, s_wdt_drv.telemetry.last_flags
        );
    } else if (valid_magic) {
        s_wdt_drv.telemetry.recoveries = prev_recoveries;
        s_wdt_drv.telemetry.cmd_seq = s2;
    }

    s_wdt_drv.active_stage = PICOFW_WDT_STAGE_IDLE;
    write_scratch(PICOFW_WDT_CMD_IDLE, s_wdt_drv.telemetry.last_flags, PICOFW_WDT_STAGE_IDLE);
    watchdog_enable(PICOFW_WDT_TIMEOUT_MS, true);
    watchdog_update();
}

void picofw_wdt_prepare_bootsel_reset(void) {
    watchdog_hw->scratch[0] = 0u;
    watchdog_hw->scratch[1] = 0u;
    watchdog_hw->scratch[3] = 0u;
    watchdog_update();
}

void picofw_wdt_kick(void) {
    picofw_hw_enforce_nsw5v_hiz();
    watchdog_update();
    picofw_led_tick();
}

void picofw_wdt_sleep_ms(uint32_t total_ms) {
    uint32_t remaining = total_ms;
    while (remaining > 0u) {
        const uint32_t slice = (remaining > PICOFW_WDT_SLEEP_SLICE_MS)
                                   ? PICOFW_WDT_SLEEP_SLICE_MS
                                   : remaining;
        sleep_ms(slice);
        picofw_wdt_kick();
        remaining -= slice;
    }
}

void picofw_wdt_begin_cmd(uint8_t cmd_id, const char *cmd_name) {
    s_wdt_drv.telemetry.cmd_seq += 1u;
    s_wdt_drv.active_cmd_id = cmd_id;
    s_wdt_drv.active_stage = PICOFW_WDT_STAGE_INIT;
    s_wdt_drv.active_cmd_name = (cmd_name != NULL) ? cmd_name : "UNKNOWN";
    s_wdt_drv.active_cmd_start_us = time_us_64();
    write_scratch(cmd_id, PICOFW_WDT_FLAG_IN_CMD, PICOFW_WDT_STAGE_INIT);
    picofw_wdt_kick();
}

void picofw_wdt_set_stage(uint8_t stage) {
    s_wdt_drv.active_stage = stage;
    const uint8_t flags = (s_wdt_drv.active_cmd_id != PICOFW_WDT_CMD_IDLE)
                              ? PICOFW_WDT_FLAG_IN_CMD
                              : s_wdt_drv.telemetry.last_flags;
    write_scratch(s_wdt_drv.active_cmd_id, flags, stage);
    picofw_wdt_kick();
}

bool picofw_wdt_poll_abort(void) {
    picofw_wdt_kick();
    return picofw_wdt_is_abort_char(getchar_timeout_us(0));
}

void picofw_wdt_end_cmd(bool aborted) {
    const uint64_t now_us = time_us_64();
    s_wdt_drv.telemetry.last_cmd_name = s_wdt_drv.active_cmd_name;
    s_wdt_drv.telemetry.last_cmd_elapsed_us = (now_us >= s_wdt_drv.active_cmd_start_us)
                                                  ? (now_us - s_wdt_drv.active_cmd_start_us)
                                                  : 0ULL;
    const uint8_t flags = aborted ? PICOFW_WDT_FLAG_ABORTED : 0u;
    picofw_wdt_pm_record(
        &s_pm_ring,
        s_wdt_drv.telemetry.cmd_seq,
        to_ms_since_boot(get_absolute_time()),
        (uint32_t)s_wdt_drv.telemetry.last_cmd_elapsed_us,
        s_wdt_drv.active_cmd_id,
        s_wdt_drv.active_stage,
        flags
    );
    s_wdt_drv.telemetry.last_flags = flags;
    s_wdt_drv.active_cmd_id = PICOFW_WDT_CMD_IDLE;
    s_wdt_drv.active_stage = PICOFW_WDT_STAGE_IDLE;
    write_scratch(PICOFW_WDT_CMD_IDLE, flags, PICOFW_WDT_STAGE_IDLE);
    picofw_wdt_kick();
}

bool picofw_wdt_loop_step(
    uint64_t start_us,
    uint64_t deadline_us,
    bool allow_cdc_abort,
    bool *out_aborted
) {
    picofw_wdt_kick();
    if (allow_cdc_abort && picofw_wdt_is_abort_char(getchar_timeout_us(0))) {
        if (out_aborted != NULL) {
            *out_aborted = true;
        }
        return false;
    }
    const uint64_t now_us = time_us_64();
    const uint64_t max_ceiling_us =
        start_us + ((uint64_t)PICOFW_WDT_MAX_CMD_WINDOW_MS * 1000ULL);
    const uint64_t effective_deadline =
        (deadline_us < max_ceiling_us) ? deadline_us : max_ceiling_us;
    if (now_us >= effective_deadline) {
        return false;
    }
    tight_loop_contents();
    return true;
}

picofw_wdt_telemetry_t picofw_wdt_snapshot(void) {
    return s_wdt_drv.telemetry;
}

void picofw_wdt_print_summary_line(void) {
    const picofw_wdt_telemetry_t *t = &s_wdt_drv.telemetry;
    printf("[WDT] wdt=ACTIVE(%ums,NON_OVERRIDABLE) seq=%lu"
           " wdt_recoveries=%u reboot_src=%s fault_slot=%u fault_stage=%s last_cmd=%s last_us=%llu\n",
           (unsigned)PICOFW_WDT_TIMEOUT_MS,
           (unsigned long)t->cmd_seq,
           (unsigned)t->recoveries,
           t->caused_reboot ? "WDT_RECOVERY" : "CLEAN",
           (unsigned)t->last_fault_cmd_id,
           picofw_wdt_stage_name(t->last_fault_stage),
           (t->last_cmd_name != NULL) ? t->last_cmd_name : "NONE",
           (unsigned long long)t->last_cmd_elapsed_us);
}

void picofw_cmd_wdt_status(const char *arg) {
    picofw_wdt_kick();
    if ((arg != NULL) && picofw_ascii_ieq(arg, "POSTMORTEM")) {
        const size_t n = picofw_wdt_pm_count(&s_pm_ring);
        printf("[WDT POSTMORTEM] retained_events=%u total_recorded=%lu crc32=0x%08lX\n",
               (unsigned)n,
               (unsigned long)s_pm_ring.total_events,
               (unsigned long)s_pm_ring.crc32);
        for (size_t i = 0u; i < n; ++i) {
            picofw_wdt_pm_entry_t e;
            if (picofw_wdt_pm_get_recent(&s_pm_ring, i, &e)) {
                printf("  #%u seq=%lu up=%lums dur=%luus cmd_id=%u stage=%s flags=0x%02X\n",
                       (unsigned)i, (unsigned long)e.seq, (unsigned long)e.uptime_ms,
                       (unsigned long)e.elapsed_us, (unsigned)e.cmd_id,
                       picofw_wdt_stage_name(e.stage), (unsigned)e.flags);
            }
        }
        printf("WDT: OK (POSTMORTEM)\n");
        return;
    }
    if ((arg != NULL) && (arg[0] != '\0') && !picofw_ascii_ieq(arg, "STATUS")) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("WDT: ERROR (Usage: WDT [STATUS|POSTMORTEM])\n");
        return;
    }
    picofw_wdt_print_summary_line();
    printf("WDT: OK (Hardware auto-recovery active @ %ums; override disabled)\n",
           (unsigned)PICOFW_WDT_TIMEOUT_MS);
}
