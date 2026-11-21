/**
 * @file p7501_pico_main.c
 * @brief Slim RP2040 / RP2350 entrypoint for the Panadiga P7501 UART Bench Probe.
 *
 * Boot safety ordering:
 *  1. `p7501_hw_init()` runs FIRST to clear RP2040/RP2350 reset-default pull-downs
 *     on `GP3 (NSW5V)` and float `GP2/GP4/GP5/GP26` Hi-Z before USB enumeration.
 *  2. `stdio_set_translate_crlf(&stdio_usb, false)` disables binary `0x0A -> 0x0D 0x0A`
 *     corruption globally so `PANADMP1`/`PANASUM1`/`PANAJID1` streams are byte-exact.
 *  3. `p7501_ble_nus_init()` runs BEFORE `p7501_led_init()` so CYW43439 is initialized
 *     before LED GPIO writes on Pico W / Pico 2 W.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include "pico/stdlib.h"
#include "pico/stdio_usb.h"

#include "cli/p7501_cli.h"
#include "common/p7501_wdt_logic.h"
#include "diag/p7501_tbus_codec.h"
#include "flash/p7501_flash.h"
#include "flash/p7501_qspi_storage.h"
#include "hal/p7501_ble_nus.h"
#include "hal/p7501_bootsel_btn.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_multicore.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "gdb/gdb_rsp.h"

static void process_cli_char(
    char c,
    char *line_buf,
    size_t *line_idx,
    size_t line_cap,
    bool *last_was_cr,
    bool *line_overflow
) {
    if (c == (char)P7501_CDC_ABORT_CTRL_C) {
        *line_idx = 0U;
        *line_overflow = false;
        *last_was_cr = false;
        printf("^C\nPANADIGA> ");
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
                   "@P7501_RES cmd=CLI status=ERR code=E2BIG elapsed_us=0\nPANADIGA> ",
                   (unsigned)(line_cap - 1U));
            fflush(stdout);
            return;
        }

        p7501_cli_process_command(line_buf);
        if (!gdb_rsp_is_active()) {
            printf("PANADIGA> ");
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

int main(void) {
    /* 1. Hardware safety FIRST: clear GP3 NSW5V pull-down & float all P7501 pins Hi-Z. */
    (void)p7501_hw_init();

    /* 2. USB CDC stdio with binary-safe raw LF output (no 0x0A -> 0x0D 0x0A mangling). */
    stdio_init_all();
    stdio_set_translate_crlf(&stdio_usb, false);

    /* 3. Radio before LED so CYW43439 is initialized prior to Pico W LED drive. */
    p7501_ble_nus_init();
    p7501_led_init();
    p7501_target_bus_init_pio();
    p7501_wdt_init();
    uint32_t fw_crc_bytes = 0u;
    const uint32_t fw_crc = p7501_qspi_firmware_crc32(&fw_crc_bytes);
    if ((fw_crc_bytes == 0u) || (fw_crc == 0u) || (fw_crc == 0xFFFFFFFFu)) {
        p7501_hw_isolate_bus_hiz();
        p7501_led_set_mode(LED_MODE_BUS_FAULT);
    }
    p7501_multicore_start_worker();
    gdb_rsp_init();

    p7501_wdt_sleep_ms(250U);
    p7501_cli_print_banner();
    printf("PANADIGA> ");
    fflush(stdout);

    static char s_line_buf[P7501_SSOT_CLI_LINE_MAX];
    size_t line_idx = 0U;
    bool last_was_cr = false;
    bool line_overflow = false;

    for (;;) {
        p7501_wdt_kick();
        p7501_ble_nus_poll();
        if (!gdb_rsp_is_active() && (line_idx == 0U)) {
            p7501_bootsel_poll();
            p7501_tbus_keepalive_poll();
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
                p7501_flash_set_upper_nor_unlocked(false);
                p7501_wdt_end_cmd(false);
                p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
                printf("\n[GDB RSP] Detached. Returning to CLI.\n@P7501_RES cmd=GDB status=OK code=0 elapsed_us=%llu\nPANADIGA> ",
                       (unsigned long long)p7501_wdt_snapshot().last_cmd_elapsed_us);
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
