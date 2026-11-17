/**
 * @file p7501_diag_watch.c
 * @brief Runtime PIO1 boot recovery watchdog (`DIAGWATCH`) with zero-overflow buffered RX.
 *
 * Decouples tight PIO1 FIFO draining (storing bytes/event timestamps in RAM first)
 * from USB CDC `printf()` calls so multi-line SD/BGO diagnostics never cause RX FIFO stalls.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_diag_watch.h"
#include "uart/p7501_diag_watch_codec.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_capture.h"
#include "uart/p7501_uart_pio_engine.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

typedef struct diagwatch_ctx {
    const p7501_diag_watch_spec_t *spec;
    p7501_diag_watch_state_t fsm;
    uint8_t *buf;
    size_t buf_cap;
    size_t total_len;
    size_t flushed_len;
    uint32_t pending_alerts;
    uint32_t rel_sw_ms;
    uint32_t rel_hit_flags;
    uint32_t sd_ok_ms;
    uint32_t fat_fail_ms;
    uint32_t load_fail_ms;
    uint32_t uncomp_fail_ms;
    uint32_t diag_exec_ms;
    uint32_t sd_err_ms;
    uint32_t odd_tray_ms;
    uint32_t cd_apc_ms;
    uint32_t bgo_fail_ms;
    uint32_t panamon_ms;
    char injected_tx[36];
    size_t injected_len;
    bool tx_dispatched;
} diagwatch_ctx_t;

static void record_alert_timestamp(diagwatch_ctx_t *dc, uint32_t hit, uint32_t ms) {
    if ((hit & P7501_DIAG_EVT_SD_HW_OK) != 0u) dc->sd_ok_ms = ms;
    if ((hit & P7501_DIAG_EVT_FAT_FAIL) != 0u) dc->fat_fail_ms = ms;
    if ((hit & P7501_DIAG_EVT_SD_DIAG_LOAD_FAIL) != 0u) dc->load_fail_ms = ms;
    if ((hit & P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL) != 0u) dc->uncomp_fail_ms = ms;
    if ((hit & P7501_DIAG_EVT_DIAG_EXEC) != 0u) dc->diag_exec_ms = ms;
    if ((hit & P7501_DIAG_EVT_SD_ERR) != 0u) dc->sd_err_ms = ms;
    if ((hit & P7501_DIAG_EVT_ODD_TRAY_FAIL) != 0u) dc->odd_tray_ms = ms;
    if ((hit & P7501_DIAG_EVT_CD_APC_ERR) != 0u) dc->cd_apc_ms = ms;
    if ((hit & P7501_DIAG_EVT_BGO_FAIL) != 0u) dc->bgo_fail_ms = ms;
    dc->pending_alerts |= hit;
}

static void diagwatch_on_byte(void *raw_ctx, uint8_t b, uint32_t rel_us, bool *out_stop) {
    (void)out_stop;
    diagwatch_ctx_t *dc = (diagwatch_ctx_t *)raw_ctx;
    const uint32_t ms = rel_us / 1000u;

    if (dc->total_len < dc->buf_cap) {
        dc->buf[dc->total_len++] = b;
    }

    char tx_ch = '\0';
    bool release_now = false;
    const uint32_t hit = p7501_diag_watch_feed_byte(&dc->fsm, dc->spec, b, &tx_ch, &release_now);
    if (release_now) {
        p7501_pio_release_sw_uart_now();
        dc->rel_sw_ms = ms;
        dc->rel_hit_flags = hit;
        dc->pending_alerts |= P7501_DIAG_EVT_SERIAL_BOOT;
    }
    record_alert_timestamp(dc, hit, ms);

    if (tx_ch != '\0') {
        size_t tx_len = 0u;
        if (tx_ch == 'U') {
            tx_len = sizeof(P7501_DIAG_WATCH_UNLOCK_TX) - 1u;
            memcpy(dc->injected_tx, P7501_DIAG_WATCH_UNLOCK_TX, tx_len + 1u);
            p7501_pio_tx_bytes((const uint8_t *)dc->injected_tx, tx_len, false);
        } else {
            tx_len = p7501_diag_watch_format_tx(dc->spec, dc->injected_tx, sizeof(dc->injected_tx));
            if (tx_len <= 1u) {
                const uint8_t tx_u8 = (uint8_t)tx_ch;
                p7501_pio_tx_bytes(&tx_u8, 1u, false);
                dc->injected_tx[0] = tx_ch;
                tx_len = 1u;
            } else {
                p7501_pio_tx_bytes((const uint8_t *)dc->injected_tx, tx_len, false);
            }
        }
        dc->injected_len = tx_len;
        dc->panamon_ms = ms;
        dc->tx_dispatched = true;
    }
}

static void flush_pending_rx_bytes(diagwatch_ctx_t *dc) {
    while (dc->flushed_len < dc->total_len) {
        const uint8_t b = dc->buf[dc->flushed_len++];
        if (b == '\r') {
            continue;
        }
        if ((b == '\n') || (b >= 0x20u && b <= 0x7Eu)) {
            putchar((int)b);
        } else {
            printf("[%02X]", (unsigned)b);
        }
    }
}

static void flush_pending_alerts(diagwatch_ctx_t *dc) {
    const uint32_t flags = dc->pending_alerts;
    if (flags == 0u && !dc->tx_dispatched) {
        return;
    }
    dc->pending_alerts = 0u;
    if (dc->rel_hit_flags != 0u) {
        const uint32_t rhit = dc->rel_hit_flags;
        dc->rel_hit_flags = 0u;
        printf("\n[DIAGWATCH +%" PRIu32 "ms] %s -> SW_UART released Hi-Z (Q7514 TX ready)\n",
               dc->rel_sw_ms,
               ((rhit & P7501_DIAG_EVT_SERIAL_BOOT) != 0u) ? "serial boot mode" : "DDR CHECK: OK");
    }
    if ((flags & P7501_DIAG_EVT_SD_HW_OK) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] STAGE: SD hardware 4-bit init OK (... SD init end)\n", dc->sd_ok_ms);
    if ((flags & P7501_DIAG_EVT_FAT_FAIL) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: FAT16 Mount/Open error emitted by bootloader (-10010/-10020/-10025)\n", dc->fat_fail_ms);
    if ((flags & P7501_DIAG_EVT_SD_DIAG_LOAD_FAIL) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: SD diag load failed[0] (if no FAT Mount/Open error: fat_read 0x20000 failed / <128KiB)\n", dc->load_fail_ms);
    if ((flags & P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: DIGADIAG uncompress/header error detected\n", dc->uncomp_fail_ms);
    if ((flags & P7501_DIAG_EVT_DIAG_EXEC) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] STAGE: DIGADIAG.FRM decompressed & jumped\n", dc->diag_exec_ms);
    if ((flags & P7501_DIAG_EVT_SD_ERR) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: SDDIAG 4-bit status failure detected (-91006/-90002/-90001)\n", dc->sd_err_ms);
    if ((flags & P7501_DIAG_EVT_ODD_TRAY_FAIL) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: ODD tray close error detected (** can not close tray ** / 80045300)\n", dc->odd_tray_ms);
    if ((flags & P7501_DIAG_EVT_CD_APC_ERR) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: CDDIAG laser APC error detected (80040904 / 0x83A7)\n", dc->cd_apc_ms);
    if ((flags & P7501_DIAG_EVT_BGO_FAIL) != 0u)
        printf("\n[DIAGWATCH +%" PRIu32 "ms] ALERT: BGO driver init failure detected\n", dc->bgo_fail_ms);
    if (dc->tx_dispatched) {
        dc->tx_dispatched = false;
        if (dc->injected_len <= 1u) {
            printf("\n[DIAGWATCH +%" PRIu32 "ms] PANAMON1> matched -> injected '%c'\n",
                   dc->panamon_ms, dc->injected_tx[0]);
        } else {
            printf("\n[DIAGWATCH +%" PRIu32 "ms] PANAMON1> matched -> injected %.*s\n",
                   dc->panamon_ms, (int)(dc->injected_len - 1u), dc->injected_tx);
        }
    }
    stdio_flush();
}

static void diagwatch_on_tick(void *raw_ctx, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    (void)out_stop;
    diagwatch_ctx_t *dc = (diagwatch_ctx_t *)raw_ctx;
    flush_pending_rx_bytes(dc);
    flush_pending_alerts(dc);
}

void p7501_cmd_diag_watch(const char *arg) {
    p7501_diag_watch_spec_t spec;
    if (!p7501_diag_watch_parse_args(arg, &spec)) {
        printf("DIAGWATCH: ERROR (Usage: DIAGWATCH [ms] [NORMAL|JIG[:ms]|PREJIG[:ms]|NOPULL] [AUTO|BYPASS|READ[:a[,n]]|WRITE:<a,v>|FORMAT|DUMP|JUMP:<a>|ERASE:<a_nvram>|QUIT|NONE] [RELEASE|HOLD] [WAKE[:ms]])\n");
        return;
    }

    p7501_hw_enforce_nsw5v_hiz();
    if (p7501_pio_get_baud() != P7501_UART_DEFAULT_BAUD) {
        (void)p7501_pio_set_baud(P7501_UART_DEFAULT_BAUD);
    }
    if (p7501_pio_get_framing() != P7501_FRAMING_8N1) {
        (void)p7501_pio_set_framing(P7501_FRAMING_8N1);
    }

    if (spec.wait_for_wake) {
        float wake_v = 0.0f;
        uint32_t wake_ms = 0u;
        const bool prestrap = spec.no_xreset && (spec.boot_action == P7501_BOOT_ACTION_JIG);
        if (!p7501_pio_wait_standby_wake("DIAGWATCH", spec.wake_timeout_ms, prestrap, &wake_v, &wake_ms)) {
            printf("DIAGWATCH: OK (rx=0 bytes, ferr=0, events=0x0000, sw_rel=NO, tx=NO, ovf=NO, aborted=YES)\n");
            printf("[DIAGWATCH] Notice: Standby wake not observed within %" PRIu32 " ms (VDD=%.2fV)\n",
                   wake_ms, (double)wake_v);
            return;
        }
        printf("[DIAGWATCH +%" PRIu32 "ms] WAKE edge detected (VDD=%.2fV) -> starting capture\n",
               wake_ms, (double)wake_v);
    }

    size_t buf_cap = 0u;
    diagwatch_ctx_t dc = {
        .spec = &spec,
        .buf = p7501_uart_rx_buffer(&buf_cap),
        .buf_cap = buf_cap,
    };
    p7501_diag_watch_init(&dc.fsm);

    printf("=== DIAGWATCH PIO1 (baud=%" PRIu32 " framing=%s ms=%" PRIu32 " boot=%d hold=%" PRIu32 " act=%d rel_ddr=%s no_xrst=%s) ===\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()),
           spec.timeout_ms,
           (int)spec.boot_action,
           spec.jig_hold_ms,
           (int)spec.action,
           spec.release_on_ddr ? "YES" : "NO",
           spec.no_xreset ? "YES" : "NO");
    fflush(stdout);

    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = spec.timeout_ms,
        .boot_action = spec.boot_action,
        .jig_hold_ms = spec.jig_hold_ms,
        .manage_session = true,
        .no_xreset = spec.no_xreset,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = diagwatch_on_byte,
        .on_tick = diagwatch_on_tick,
        .on_sw_rel = NULL,
        .ctx = &dc,
    };
    p7501_pio_loop_metrics_t m;
    p7501_pio_run_session(&cfg, &m);
    flush_pending_rx_bytes(&dc);
    flush_pending_alerts(&dc);
    stdio_flush();

    if (m.rx_bytes == 0u) {
        const float end_v = p7501_hw_read_vdd();
        if (end_v < P7501_VDD_THRESHOLD_V) {
            printf("\n[DIAGWATCH] Notice: Target P7501 rails OFF / Standby (VDD=%.2fV) — wake unit or use WAKE:30000\n",
                   end_v);
        }
    }

    printf("\nDIAGWATCH: OK (rx=%" PRIu32 " bytes, ferr=%" PRIu32 ", events=0x%04" PRIX32 ", sw_rel=%s, tx=%s, ovf=%s, aborted=%s)\n",
           m.rx_bytes,
           m.framing_errors,
           dc.fsm.seen_events,
           dc.fsm.sw_uart_released ? "YES" : "NO",
           dc.fsm.tx_dispatched ? "YES" : "NO",
           m.overflow ? "YES" : "NO",
           m.aborted ? "YES" : "NO");
    printf("[DIAGWATCH] Diagnosis: %s\n", p7501_diag_watch_diagnosis(dc.fsm.seen_events));
}
