/**
 * @file picofw_uart_capture.c
 * @brief Shared 16 KB P7501 RX buffer and PIO1 `BOOTCAP`/`UARTMON`/`JIGBOOT` implementation.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_uart_capture.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "picofw_types.h"
#include "uart/picofw_pio_loop.h"
#include "uart/picofw_uart_codec.h"
#include "uart/picofw_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>

typedef struct capture_buf_ctx {
    uint8_t *buf;
    uint32_t len;
    uint32_t max_cap;
} capture_buf_ctx_t;

static void capture_on_byte(void *raw_ctx, uint8_t b, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    (void)out_stop;
    capture_buf_ctx_t *ctx = (capture_buf_ctx_t *)raw_ctx;
    if (ctx->len < ctx->max_cap) {
        ctx->buf[ctx->len++] = b;
    }
}

uint8_t *picofw_uart_rx_buffer(size_t *out_capacity) {
    if (out_capacity != NULL) {
        *out_capacity = sizeof(g_picofw_cmd_arena.uart_rx);
    }
    return g_picofw_cmd_arena.uart_rx;
}

uint32_t picofw_uart_capture_window(uint32_t dur_us, uint32_t max_cap) {
    const uint32_t lim = (max_cap < (uint32_t)sizeof(g_picofw_cmd_arena.uart_rx))
        ? max_cap
        : (uint32_t)sizeof(g_picofw_cmd_arena.uart_rx);
    capture_buf_ctx_t ctx = { g_picofw_cmd_arena.uart_rx, 0u, lim };
    const uint32_t window_ms = (dur_us >= 1000UL) ? (dur_us / 1000UL) : 1u;

    const picofw_pio_loop_cfg_t cfg = {
        .window_ms = window_ms,
        .boot_action = PICOFW_BOOT_ACTION_PASSIVE,
        .jig_hold_ms = 0u,
        .manage_session = false,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = capture_on_byte,
        .on_tick = NULL,
        .on_sw_rel = NULL,
        .ctx = &ctx,
    };
    picofw_pio_run_session(&cfg, NULL);
    return ctx.len;
}

void picofw_cmd_boot_capture(const char *arg, bool do_reset, bool is_jig, const char *cmd_name) {
    uint32_t duration_ms = PICOFW_CAPTURE_DEFAULT_MS;
    uint32_t jig_hold_ms = 0U;
    char tok1[24] = {0};
    char tok2[24] = {0};
    const char *cur = picofw_next_token(arg, tok1, sizeof(tok1));
    if (tok1[0] != '\0') {
        duration_ms = picofw_parse_uint32_clamped(
            tok1, PICOFW_CAPTURE_DEFAULT_MS, 100U, PICOFW_TRACE_MAX_MS
        );
    }
    if (cur != NULL && picofw_next_token(cur, tok2, sizeof(tok2)) != NULL) {
        jig_hold_ms = picofw_parse_uint32_clamped(tok2, 0U, 0U, PICOFW_TRACE_MAX_MS);
    }
    if (is_jig && jig_hold_ms == 0U) {
        jig_hold_ms = (duration_ms > PICOFW_JIG_DEFAULT_HOLD_MS)
            ? PICOFW_JIG_DEFAULT_HOLD_MS
            : duration_ms;
    }

    const picofw_boot_action_t action = is_jig
        ? PICOFW_BOOT_ACTION_JIG
        : (do_reset ? PICOFW_BOOT_ACTION_NORMAL : PICOFW_BOOT_ACTION_PASSIVE);

    capture_buf_ctx_t ctx = {
        g_picofw_cmd_arena.uart_rx,
        0u,
        (uint32_t)(sizeof(g_picofw_cmd_arena.uart_rx) - 1U),
    };
    const picofw_pio_loop_cfg_t cfg = {
        .window_ms = duration_ms,
        .boot_action = action,
        .jig_hold_ms = jig_hold_ms,
        .manage_session = true,
        .auto_release_serial_boot = is_jig,
        .allow_cdc_abort = true,
        .on_rx_byte = capture_on_byte,
        .on_tick = NULL,
        .on_sw_rel = NULL,
        .ctx = &ctx,
    };
    picofw_pio_loop_metrics_t m;
    picofw_pio_run_session(&cfg, &m);

    printf(
        "\n=== %s: PIO1 %" PRIu32 " %s UART CAPTURE ON GP2 (%" PRIu32 " bytes, %" PRIu32 " ms, ferr=%" PRIu32 ", perr=%" PRIu32 ", ovf=%s%s) ===\n",
        cmd_name,
        picofw_pio_get_baud(),
        picofw_framing_name(picofw_pio_get_framing()),
        ctx.len,
        duration_ms,
        m.framing_errors,
        m.parity_errors,
        m.overflow ? "YES" : "NO",
        is_jig ? (m.sw_uart_released ? ", sw_rel=YES" : ", sw_rel=END") : ""
    );
    picofw_uart_print_ascii_stream(g_picofw_cmd_arena.uart_rx, ctx.len);
    printf("\n%s: OK (Bus kept in Hi-Z with pull-up so target continues running)\n", cmd_name);
}
