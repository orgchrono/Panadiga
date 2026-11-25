/**
 * @file picofw_pico_main.c
 * @brief Slim RP2040 / RP2350 entrypoint for the Panadiga P7501 UART Bench Probe.
 *
 * Boot safety ordering:
 *  1. `picofw_hw_init()` runs FIRST to clear RP2040/RP2350 reset-default pull-downs
 *     on `GP3 (NSW5V)` and float `GP2/GP4/GP5/GP26` Hi-Z before USB enumeration.
 *  2. `stdio_set_translate_crlf(&stdio_usb, false)` disables binary `0x0A -> 0x0D 0x0A`
 *     corruption globally so `PANADMP1`/`PANASUM1`/`PANAJID1` streams are byte-exact.
 *  3. `picofw_ble_nus_init()` runs BEFORE `picofw_led_init()` so CYW43439 is initialized
 *     before LED GPIO writes on Pico W / Pico 2 W.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include "pico/stdlib.h"
#include "pico/stdio_usb.h"

#include "cli/picofw_cli.h"
#include "cli/picofw_cli_sys.h"
#include "common/picofw_wdt_logic.h"
#include "diag/picofw_tbus_codec.h"
#include "flash/picofw_flash.h"
#include "flash/picofw_qspi_storage.h"
#include "hal/picofw_ble_nus.h"
#include "hal/picofw_bootsel_btn.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"
#include "hal/picofw_multicore.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "gdb/gdb_rsp.h"

static void process_cli_char(
    char c,
    char *line_buf,
    size_t *line_idx,
    size_t line_cap,
    bool *last_was_cr,
    bool *line_overflow
) {
    if (c == (char)PICOFW_CDC_ABORT_CTRL_C) {
        *line_idx = 0U;
        *line_overflow = false;
        *last_was_cr = false;
        printf("^C\nPICO> ");
        fflush(stdout);
        return;
    }

    if ((c == '\r') || (c == '\n')) {
        if ((c == '\n') && (*last_was_cr)) {
            *last_was_cr = false;
            return;
        }
        *last_was_cr = (c == '\r');
        const bool had_overflow = *line_overflow;
        *line_overflow = false;
        line_buf[*line_idx] = '\0';
        *line_idx = 0U;

        if (had_overflow) {
            printf("[CLI] ERROR: command line exceeded %u bytes (E2BIG).\n"
                   "@PICO_RES cmd=CLI status=ERR code=E2BIG elapsed_us=0\nPICO> ",
                   (unsigned)(line_cap - 1U));
            fflush(stdout);
            return;
        }

        picofw_cli_process_command(line_buf);
        if (!gdb_rsp_is_active()) {
            printf("PICO> ");
            fflush(stdout);
        }
        return;
    }

    *last_was_cr = false;
    if ((c == '\b') || (c == 0x7FU)) {
        if ((*line_idx > 0U) && !(*line_overflow)) {
            (*line_idx)--;
        }
    } else if (*line_idx < (line_cap - 1U)) {
        line_buf[(*line_idx)++] = c;
    } else {
        *line_overflow = true;
    }
}

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
/* Bare-metal SSP canary and fail-closed fault handler isolating all P7501 lines Hi-Z. */
uintptr_t __stack_chk_guard = (uintptr_t)0x50414E41u;

__attribute__((noreturn)) void __stack_chk_fail(void) {
    picofw_hw_isolate_bus_hiz();
    for (;;) {
        tight_loop_contents();
    }
}
#endif

int main(void) {
    /* 1. Hardware safety FIRST: clear GP3 NSW5V pull-down & float all P7501 pins Hi-Z. */
    (void)picofw_hw_init();

    /* 2. USB CDC stdio with binary-safe raw LF output (no 0x0A -> 0x0D 0x0A mangling). */
    stdio_init_all();
    stdio_set_translate_crlf(&stdio_usb, false);

    /* 3. Radio before LED so CYW43439 is initialized prior to Pico W LED drive. */
    picofw_ble_nus_init();
    picofw_led_init();
    picofw_target_bus_init_pio();
    picofw_wdt_init();
    uint32_t fw_crc_bytes = 0u;
    const uint32_t fw_crc = picofw_qspi_firmware_crc32(&fw_crc_bytes);
    if ((fw_crc_bytes == 0u) || (fw_crc == 0u) || (fw_crc == 0xFFFFFFFFu)) {
        picofw_hw_isolate_bus_hiz();
        picofw_led_set_mode(LED_MODE_BUS_FAULT);
    }
    picofw_multicore_start_worker();
    gdb_rsp_init();

    picofw_wdt_sleep_ms(250U);
    picofw_cli_print_banner();
    printf("PICO> ");
    fflush(stdout);

    static char s_line_buf[PICOFW_SSOT_CLI_LINE_MAX];
    size_t line_idx = 0U;
    bool last_was_cr = false;
    bool line_overflow = false;

    for (;;) {
        picofw_wdt_kick();
        picofw_ble_nus_poll();
        if (!gdb_rsp_is_active() && (line_idx == 0U)) {
            picofw_bootsel_poll();
            picofw_hw_poll_rail_recovery();
            (void)picofw_probe_dispatch_event(PICOFW_EV_TICK, false);
            const picofw_ps_t ps = picofw_probe_get_state();
            if ((ps == PICOFW_PS_ENGAGED) || (ps == PICOFW_PS_ACTIVE_OP)) {
                picofw_tbus_keepalive_poll();
            }
        }

        const int ch = getchar_timeout_us(0);
        if (ch == PICO_ERROR_TIMEOUT) {
            tight_loop_contents();
            continue;
        }

        const char c = (char)ch;
        if (gdb_rsp_is_active()) {
            gdb_rsp_process_char(c);
            if (!gdb_rsp_is_active()) {
                (void)picofw_probe_dispatch_event(PICOFW_EV_GDB_DETACH, false);
                picofw_wdt_end_cmd(false);
                picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
                printf("\n[GDB RSP] Detached. Returning to CLI.\n@PICO_RES cmd=GDB status=OK code=0 elapsed_us=%llu\nPICO> ",
                       (unsigned long long)picofw_wdt_snapshot().last_cmd_elapsed_us);
                fflush(stdout);
            }
        } else {
            process_cli_char(
                c,
                s_line_buf,
                &line_idx,
                sizeof(s_line_buf),
                &last_was_cr,
                &line_overflow
            );
        }
    }

    return 0;
}
