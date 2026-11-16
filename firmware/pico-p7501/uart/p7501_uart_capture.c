/**
 * @file p7501_uart_capture.c
 * @brief Shared 16 KB P7501 RX buffer and PIO1 `BOOTCAP`/`UARTMON`/`JIGBOOT` implementation.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_capture.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_bitbang.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>

typedef struct p7501_capture_drv {
    /** Single shared RX capture buffer (16 KB) reused across all UART/FMU probes. */
    uint8_t rx_buf[16384];
} p7501_capture_drv_t;

static p7501_capture_drv_t s_capture_drv;

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

uint8_t *p7501_uart_rx_buffer(size_t *out_capacity) {
    if (out_capacity != NULL) {
        *out_capacity = sizeof(s_capture_drv.rx_buf);
    }
    return s_capture_drv.rx_buf;
}

bool p7501_uart_poll_capture_byte(bool *prev_high, uint32_t *rx_len, uint32_t max_cap) {
    uint8_t rx_byte = 0U;
    if (!p7501_uart_try_sample_byte(prev_high, &rx_byte, NULL)) {
        return false;
    }
    if (*rx_len < max_cap) {
        s_capture_drv.rx_buf[(*rx_len)++] = rx_byte;
    }
    return true;
}

uint32_t p7501_uart_capture_window(uint32_t dur_us, uint32_t max_cap) {
    const uint32_t lim = (max_cap < (uint32_t)sizeof(s_capture_drv.rx_buf))
        ? max_cap
        : (uint32_t)sizeof(s_capture_drv.rx_buf);
    capture_buf_ctx_t ctx = { s_capture_drv.rx_buf, 0u, lim };
    const uint32_t window_ms = (dur_us >= 1000UL) ? (dur_us / 1000UL) : 1u;

    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = window_ms,
        .boot_action = P7501_BOOT_ACTION_PASSIVE,
        .jig_hold_ms = 0u,
        .manage_session = false,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = capture_on_byte,
        .on_tick = NULL,
        .on_sw_rel = NULL,
        .ctx = &ctx,
    };
    p7501_pio_run_session(&cfg, NULL);
    return ctx.len;
}

void p7501_cmd_boot_capture(const char *arg, bool do_reset, bool is_jig, const char *cmd_name) {
    uint32_t duration_ms = P7501_CAPTURE_DEFAULT_MS;
    uint32_t jig_hold_ms = 0U;
    char tok1[24] = {0};
    char tok2[24] = {0};
    const char *cur = p7501_next_token(arg, tok1, sizeof(tok1));
    if (tok1[0] != '\0') {
        duration_ms = p7501_parse_uint32_clamped(
            tok1, P7501_CAPTURE_DEFAULT_MS, 100U, P7501_TRACE_MAX_MS
        );
    }
    if (cur != NULL && p7501_next_token(cur, tok2, sizeof(tok2)) != NULL) {
        jig_hold_ms = p7501_parse_uint32_clamped(tok2, 0U, 0U, P7501_TRACE_MAX_MS);
    }
    if (is_jig && jig_hold_ms == 0U) {
        jig_hold_ms = (duration_ms > P7501_JIG_DEFAULT_HOLD_MS)
            ? P7501_JIG_DEFAULT_HOLD_MS
            : duration_ms;
    }

    const p7501_boot_action_t action = is_jig
        ? P7501_BOOT_ACTION_JIG
        : (do_reset ? P7501_BOOT_ACTION_NORMAL : P7501_BOOT_ACTION_PASSIVE);

    capture_buf_ctx_t ctx = {
        s_capture_drv.rx_buf,
        0u,
        (uint32_t)(sizeof(s_capture_drv.rx_buf) - 1U),
    };
    const p7501_pio_loop_cfg_t cfg = {
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
    p7501_pio_loop_metrics_t m;
    p7501_pio_run_session(&cfg, &m);

    printf(
        "\n=== %s: PIO1 %" PRIu32 " %s UART CAPTURE ON GP2 (%" PRIu32 " bytes, %" PRIu32 " ms, ferr=%" PRIu32 ", perr=%" PRIu32 ", ovf=%s%s) ===\n",
        cmd_name,
        p7501_pio_get_baud(),
        p7501_framing_name(p7501_pio_get_framing()),
        ctx.len,
        duration_ms,
        m.framing_errors,
        m.parity_errors,
        m.overflow ? "YES" : "NO",
        is_jig ? (m.sw_uart_released ? ", sw_rel=YES" : ", sw_rel=END") : ""
    );
    p7501_uart_print_ascii_stream(s_capture_drv.rx_buf, ctx.len);
    printf("\n%s: OK (Bus kept in Hi-Z with pull-up so target continues running)\n", cmd_name);
}
