/**
 * @file picofw_uart_interactive.c
 * @brief Interactive P7501 PIO1 TX/RX and BREAK probes (`UARTSEND`, `UARTPROBE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_uart_interactive.h"
#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "flash/picofw_bus_policy.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "uart/picofw_pio_loop.h"
#include "uart/picofw_uart_capture.h"
#include "uart/picofw_uart_codec.h"
#include "uart/picofw_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pico/stdlib.h"
#include "pico/time.h"

typedef struct uartprobe_ctx {
    uint32_t pmode;
    bool sw_uart_low;
    bool sw_released;
    uint32_t sw_rel_ms;
    bool break_released;
    uint8_t probe_idx;
    absolute_time_t next_probe_t;
    uint8_t *rx_buf;
    uint32_t rx_cap;
    uint32_t rx_len;
    uint8_t ring[24];
    size_t ring_len;
} uartprobe_ctx_t;

void picofw_cmd_uart_send(const char *arg) {
    bool sw_uart_out_low = false;
    const char *payload = (arg != NULL) ? arg : "";
    if (picofw_ascii_iprefix(payload, "LOW:")) {
        sw_uart_out_low = true;
        payload += 4;
    }

    picofw_bus_deny_t deny = PICOFW_BUS_ALLOW;
    if (!picofw_bus_check_uart_tx_payload((const uint8_t *)payload, strlen(payload), false, &deny)) {
        picofw_cli_set_result("ERR", "E05");
        printf("[UARTSEND] BLOCKED (%s)\n", picofw_bus_deny_name(deny));
        return;
    }

    char tx_str[128];
    if (*payload != '\0') {
        snprintf(tx_str, sizeof(tx_str), "%s\r\n", payload);
    } else {
        snprintf(tx_str, sizeof(tx_str), "\r\n");
    }

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_TX_WAIT);
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    picofw_pio_drain_rx();
    picofw_pio_tx_bytes((const uint8_t *)tx_str, strlen(tx_str), sw_uart_out_low);

    uint8_t *rx_buf = picofw_uart_rx_buffer(NULL);
    const uint32_t rx_len = picofw_uart_capture_window(2500000UL, 4096U);
    const uint32_t perr = picofw_pio_get_and_clear_parity_errors();
    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf(
        "\n=== UARTSEND PIO1 REPLY (%" PRIu32 " bytes, %s @ %" PRIu32 ", SW_UART=%s, perr=%" PRIu32 ") ===\n",
        rx_len,
        picofw_framing_name(picofw_pio_get_framing()),
        picofw_pio_get_baud(),
        sw_uart_out_low ? "LOW" : "Hi-Z",
        perr
    );
    picofw_uart_print_ascii_stream(rx_buf, rx_len);
    printf("\nUARTSEND: OK\n");
}

static uint32_t parse_probe_mode(const char *arg) {
    uint32_t val = 5U;
    if ((arg == NULL) || (*arg == '\0') || !picofw_parse_u32_strict(arg, &val) || (val > 255U)) {
        return 5U;
    }
    return val;
}

static void uartprobe_on_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)out_stop;
    uartprobe_ctx_t *ctx = (uartprobe_ctx_t *)raw_ctx;
    if (!ctx->sw_released &&
        picofw_rolling_match_push(ctx->ring, &ctx->ring_len, sizeof(ctx->ring), byte, "serial boot mode")) {
        picofw_pio_release_sw_uart_now();
        ctx->sw_released = true;
        ctx->sw_rel_ms = rel_us / 1000u;
    }
    if ((ctx->rx_len < (ctx->rx_cap - 1U)) && (ctx->rx_len < 8192U)) {
        ctx->rx_buf[ctx->rx_len++] = byte;
    }
}

static void uartprobe_on_tick(void *raw_ctx, uint32_t rel_us, bool *out_stop) {
    (void)out_stop;
    static const uint8_t probe_chars[8] = { '\r', '\n', ' ', '?', 0x1BU, 0x03U, 'd', '\r' };
    uartprobe_ctx_t *ctx = (uartprobe_ctx_t *)raw_ctx;
    const uint32_t elapsed_ms = rel_us / 1000u;

    if (!ctx->sw_released && elapsed_ms >= 350U) {
        picofw_pio_release_sw_uart_now();
        ctx->sw_released = true;
        ctx->sw_rel_ms = elapsed_ms;
    }
    if (!ctx->break_released && elapsed_ms >= 550U) {
        picofw_hw_release_txd_hiz(true);
        ctx->break_released = true;
    }
    if ((ctx->pmode == 6U) && ctx->sw_released && time_reached(ctx->next_probe_t)) {
        const uint8_t ch = probe_chars[ctx->probe_idx % 8U];
        picofw_pio_tx_bytes(&ch, 1U, false);
        ctx->probe_idx++;
        ctx->next_probe_t = make_timeout_time_ms(80U);
    }
}

void picofw_cmd_uart_probe(const char *arg) {
    const uint32_t pmode = parse_probe_mode(arg);
    const bool sw_uart_low = ((pmode == 5U) || (pmode == 6U) || (pmode == 7U));
    const bool start_sdata_low = ((pmode == 7U) || (pmode == 8U) || (pmode == 2U));

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RESET_SYNC);
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    picofw_hw_sw_uart_set_low(sw_uart_low);
    if (start_sdata_low) {
        picofw_hw_drive_txd_low();
    }
    picofw_wdt_sleep_ms(PICOFW_RESET_POST_SETTLE_MS);
    picofw_hw_pulse_xreset_ms(PICOFW_RESET_DIAG_ASSERT_MS);

    size_t rx_cap = 0U;
    uint8_t *rx_buf = picofw_uart_rx_buffer(&rx_cap);
    uartprobe_ctx_t ctx = {
        .pmode = pmode,
        .sw_uart_low = sw_uart_low,
        .sw_released = !sw_uart_low,
        .sw_rel_ms = 0u,
        .break_released = !start_sdata_low,
        .probe_idx = 0u,
        .next_probe_t = make_timeout_time_ms(150U),
        .rx_buf = rx_buf,
        .rx_cap = (uint32_t)rx_cap,
        .rx_len = 0u,
        .ring_len = 0u,
    };

    const picofw_pio_loop_cfg_t loop_cfg = {
        .window_ms = PICOFW_UART_PROBE_WINDOW_MS,
        .boot_action = PICOFW_BOOT_ACTION_PASSIVE,
        .jig_hold_ms = 0u,
        .manage_session = false,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = uartprobe_on_byte,
        .on_tick = uartprobe_on_tick,
        .on_sw_rel = NULL,
        .ctx = &ctx,
    };
    picofw_pio_loop_metrics_t m;
    picofw_pio_run_session(&loop_cfg, &m);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_CLEANUP);
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf("\n=== UARTPROBE PIO1 MODE %" PRIu32 " (bytes=%" PRIu32 ", sw_rel=%" PRIu32 "ms, ferr=%" PRIu32 ", perr=%" PRIu32 ", ovf=%s) ===\n",
           pmode, ctx.rx_len, ctx.sw_rel_ms, m.framing_errors, m.parity_errors, m.overflow ? "YES" : "NO");
    picofw_uart_print_ascii_stream(rx_buf, ctx.rx_len);
    printf("\nUARTPROBE: OK\n");
}
