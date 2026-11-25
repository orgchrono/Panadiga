/**
 * @file picofw_drv_pana.c
 * @brief Runtime `DRVPANA` Matshita R6-MV4 Drive Factory CLI dispatcher over PIO1.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_drv_pana.h"
#include "cli/picofw_cli.h"
#include "uart/picofw_drv_pana_codec.h"
#include "uart/picofw_pio_loop.h"
#include "uart/picofw_uart_pio_engine.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"
#include "hal/picofw_watchdog.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>

static void drvpana_on_byte(void *ctx, uint8_t b, uint32_t rel_us, bool *out_stop) {
    (void)ctx;
    (void)rel_us;
    (void)out_stop;
    putchar((int)b);
}

static picofw_pio_loop_metrics_t drvpana_drain_ms(uint32_t wait_ms) {
    const picofw_pio_loop_cfg_t cfg = {
        .window_ms = wait_ms,
        .boot_action = PICOFW_BOOT_ACTION_PASSIVE,
        .jig_hold_ms = 0u,
        .manage_session = false,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = drvpana_on_byte,
        .on_tick = NULL,
        .on_sw_rel = NULL,
        .ctx = NULL,
    };
    picofw_pio_loop_metrics_t m;
    picofw_pio_run_session(&cfg, &m);
    stdio_flush();
    return m;
}

static void drvpana_send_cmd_line(
    const char *line,
    size_t len,
    uint32_t wait_ms,
    uint32_t *rx_total,
    uint32_t *ferr_total,
    bool *aborted
) {
    if (*aborted) {
        return;
    }
    picofw_wdt_set_stage(PICOFW_WDT_STAGE_TX_WAIT);
    picofw_pio_tx_bytes((const uint8_t *)line, len, false);
    const picofw_pio_loop_metrics_t m = drvpana_drain_ms(wait_ms);
    *rx_total += m.rx_bytes;
    *ferr_total += m.framing_errors;
    *aborted = m.aborted;
}

void picofw_cmd_drv_pana(const char *arg) {
    picofw_drv_pana_spec_t spec;
    if (!picofw_drv_pana_parse_args(arg, &spec)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("DRVPANA: ERROR (Usage: DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <DIAG|cmd...>)\n");
        return;
    }

    char cmd_line[104];
    const size_t cmd_len = picofw_drv_pana_format_cmd(&spec, cmd_line, sizeof(cmd_line));
    if (cmd_len == 0u) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("DRVPANA: ERROR (command length overflow)\n");
        return;
    }

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RESET_SYNC);
    picofw_hw_enforce_nsw5v_hiz();
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    printf("=== DRVPANA PIO1 (baud=%" PRIu32 " ms=%" PRIu32 " boot=%d hold=%" PRIu32 " pass=%s diag=%s cmd=\"%s\") ===\n",
           picofw_pio_get_baud(),
           spec.timeout_ms,
           (int)spec.boot_action,
           spec.jig_hold_ms,
           spec.send_pass ? "AUTO" : "SKIP",
           spec.diag_macro ? "YES" : "NO",
           spec.cmd);

    picofw_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    uint32_t rx_count = 0u;
    uint32_t ferr_count = 0u;
    bool aborted = false;

    if (spec.boot_action != PICOFW_BOOT_ACTION_PASSIVE) {
        const uint32_t settle_ms = (spec.jig_hold_ms > 250u) ? spec.jig_hold_ms : 250u;
        const picofw_pio_loop_metrics_t m1 = drvpana_drain_ms(settle_ms);
        rx_count += m1.rx_bytes;
        ferr_count += m1.framing_errors;
        aborted = m1.aborted;
    }

    if (spec.send_pass) {
        static const char k_pass_cmd[] = PICOFW_DRVPANA_PASS_LINE;
        drvpana_send_cmd_line(
            k_pass_cmd, sizeof(k_pass_cmd) - 1u, 120u, &rx_count, &ferr_count, &aborted
        );
    }

    if (spec.diag_macro) {
        static const char *const k_diag_cmds[4] = { "VER", "ERR", "APC", "TRAY" };
        for (size_t i = 0u; (i < 4u) && !aborted; ++i) {
            printf("\n@EVT kind=DRVPANA_STEP step=%u/4 cmd=%s\n", (unsigned)(i + 1u), k_diag_cmds[i]);
            char step_line[16];
            const int n = snprintf(step_line, sizeof(step_line), "%s\r\n", k_diag_cmds[i]);
            if (n > 0) {
                drvpana_send_cmd_line(
                    step_line, (size_t)n, spec.timeout_ms, &rx_count, &ferr_count, &aborted
                );
            }
        }
    } else {
        drvpana_send_cmd_line(
            cmd_line, cmd_len, spec.timeout_ms, &rx_count, &ferr_count, &aborted
        );
    }

    const bool fifo_ovf = picofw_pio_had_rx_overflow();
    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf("\nDRVPANA: OK (rx=%" PRIu32 " bytes, ferr=%" PRIu32 ", pass=%s, ovf=%s, aborted=%s, cmd=\"%s\")\n",
           rx_count,
           ferr_count,
           spec.send_pass ? "YES" : "NO",
           fifo_ovf ? "YES" : "NO",
           aborted ? "YES" : "NO",
           spec.cmd);
}
