/**
 * @file p7501_drv_pana.c
 * @brief Runtime `DRVPANA` Matshita R6-MV4 Drive Factory CLI dispatcher over PIO1.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_drv_pana.h"
#include "uart/p7501_drv_pana_codec.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_pio_engine.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>

static void drvpana_on_byte(void *ctx, uint8_t b, uint32_t rel_us, bool *out_stop) {
    (void)ctx;
    (void)rel_us;
    (void)out_stop;
    putchar((int)b);
}

static p7501_pio_loop_metrics_t drvpana_drain_ms(uint32_t wait_ms) {
    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = wait_ms,
        .boot_action = P7501_BOOT_ACTION_PASSIVE,
        .jig_hold_ms = 0u,
        .manage_session = false,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = drvpana_on_byte,
        .on_tick = NULL,
        .on_sw_rel = NULL,
        .ctx = NULL,
    };
    p7501_pio_loop_metrics_t m;
    p7501_pio_run_session(&cfg, &m);
    stdio_flush();
    return m;
}

void p7501_cmd_drv_pana(const char *arg) {
    p7501_drv_pana_spec_t spec;
    if (!p7501_drv_pana_parse_args(arg, &spec)) {
        printf("DRVPANA: ERROR (Usage: DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd...>)\n");
        return;
    }

    char cmd_line[104];
    const size_t cmd_len = p7501_drv_pana_format_cmd(&spec, cmd_line, sizeof(cmd_line));
    if (cmd_len == 0u) {
        printf("DRVPANA: ERROR (command length overflow)\n");
        return;
    }

    p7501_wdt_set_stage(P7501_WDT_STAGE_RESET_SYNC);
    p7501_hw_enforce_nsw5v_hiz();
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    printf("=== DRVPANA PIO1 (baud=%" PRIu32 " ms=%" PRIu32 " boot=%d hold=%" PRIu32 " pass=%s cmd=\"%s\") ===\n",
           p7501_pio_get_baud(),
           spec.timeout_ms,
           (int)spec.boot_action,
           spec.jig_hold_ms,
           spec.send_pass ? "AUTO" : "SKIP",
           spec.cmd);

    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    uint32_t rx_count = 0u;
    uint32_t ferr_count = 0u;
    bool aborted = false;

    if (spec.boot_action != P7501_BOOT_ACTION_PASSIVE) {
        const uint32_t settle_ms = (spec.jig_hold_ms > 250u) ? spec.jig_hold_ms : 250u;
        const p7501_pio_loop_metrics_t m1 = drvpana_drain_ms(settle_ms);
        rx_count += m1.rx_bytes;
        ferr_count += m1.framing_errors;
        aborted = m1.aborted;
    }

    if (!aborted && spec.send_pass) {
        p7501_wdt_set_stage(P7501_WDT_STAGE_TX_WAIT);
        static const char k_pass_cmd[] = P7501_DRVPANA_PASS_LINE;
        p7501_pio_tx_bytes((const uint8_t *)k_pass_cmd, sizeof(k_pass_cmd) - 1u, false);
        const p7501_pio_loop_metrics_t m2 = drvpana_drain_ms(120u);
        rx_count += m2.rx_bytes;
        ferr_count += m2.framing_errors;
        aborted = m2.aborted;
    }

    if (!aborted) {
        p7501_wdt_set_stage(P7501_WDT_STAGE_TX_WAIT);
        p7501_pio_tx_bytes((const uint8_t *)cmd_line, cmd_len, false);
        const p7501_pio_loop_metrics_t m3 = drvpana_drain_ms(spec.timeout_ms);
        rx_count += m3.rx_bytes;
        ferr_count += m3.framing_errors;
        aborted = m3.aborted;
    }

    const bool fifo_ovf = p7501_pio_had_rx_overflow();
    p7501_wdt_set_stage(P7501_WDT_STAGE_CLEANUP);
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf("\nDRVPANA: OK (rx=%" PRIu32 " bytes, ferr=%" PRIu32 ", pass=%s, ovf=%s, aborted=%s, cmd=\"%s\")\n",
           rx_count,
           ferr_count,
           spec.send_pass ? "YES" : "NO",
           fifo_ovf ? "YES" : "NO",
           aborted ? "YES" : "NO",
           spec.cmd);
}
