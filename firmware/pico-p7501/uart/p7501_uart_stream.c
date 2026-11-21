/**
 * @file p7501_uart_stream.c
 * @brief Real-time PIO1 UART byte streamer (`UARTSTREAM`) and delta-us hex frame capture (`HEXCAP`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_stream.h"

#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>

#define HEXCAP_RAM_BUF_CAP P7501_ARENA_HEXCAP_CAP

typedef struct stream_ctx {
    uint32_t col;
    bool print_release_note;
} stream_ctx_t;

typedef struct hexcap_ctx {
    uint8_t *bytes;
    uint32_t *gaps_us;
    size_t buf_len;
    uint32_t last_byte_us;
} hexcap_ctx_t;

static void stream_on_byte(void *raw_ctx, uint8_t b, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    (void)out_stop;
    stream_ctx_t *ctx = (stream_ctx_t *)raw_ctx;
    /* CDC1 mirroring is already performed once per RX byte inside p7501_pio_run_session(). */
    if (b == '\r') {
        return;
    }
    p7501_print_ascii_byte(b, '[', ']');
    if (b == '\n') {
        ctx->col = 0u;
        return;
    }
    if (++ctx->col >= 96u) {
        ctx->col = 0u;
    }
}

static void stream_on_sw_rel(void *raw_ctx, uint32_t rel_us, const char *reason) {
    (void)rel_us;
    (void)reason;
    stream_ctx_t *ctx = (stream_ctx_t *)raw_ctx;
    if (ctx->print_release_note) {
        printf("\n[SW_UART] Timed release -> Hi-Z ADC0 (Q7514 TX ready)\n");
        fflush(stdout);
    }
}

static void flush_hexcap_row(const uint8_t *row, size_t len, uint32_t delta_us) {
    if (len == 0u) {
        return;
    }
    printf("[+%7" PRIu32 "us] ", delta_us);
    for (size_t i = 0u; i < 16u; ++i) {
        if (i < len) printf("%02X ", (unsigned)row[i]);
        else printf("   ");
    }
    printf("| ");
    for (size_t i = 0u; i < len; ++i) {
        putchar((row[i] >= 0x20u && row[i] <= 0x7Eu) ? (char)row[i] : '.');
    }
    putchar('\n');
}

static void flush_hexcap_buffer(const uint8_t *bytes, const uint32_t *gaps_us, size_t count) {
    uint8_t row[16];
    size_t row_len = 0u;
    uint32_t row_delta_us = 0u;

    for (size_t i = 0u; i < count; ++i) {
        if (row_len > 0u && gaps_us[i] >= 5000u) {
            flush_hexcap_row(row, row_len, row_delta_us);
            row_len = 0u;
        }
        if (row_len == 0u) {
            row_delta_us = gaps_us[i];
        }
        row[row_len++] = bytes[i];
        if (row_len >= sizeof(row)) {
            flush_hexcap_row(row, row_len, row_delta_us);
            row_len = 0u;
        }
    }
    flush_hexcap_row(row, row_len, row_delta_us);
}

static void hexcap_on_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)out_stop;
    hexcap_ctx_t *ctx = (hexcap_ctx_t *)raw_ctx;
    const uint32_t gap_us = rel_us - ctx->last_byte_us;
    ctx->last_byte_us = rel_us;
    if (ctx->buf_len >= HEXCAP_RAM_BUF_CAP) {
        flush_hexcap_buffer(ctx->bytes, ctx->gaps_us, ctx->buf_len);
        ctx->buf_len = 0u;
    }
    ctx->bytes[ctx->buf_len] = byte;
    ctx->gaps_us[ctx->buf_len] = gap_us;
    ++ctx->buf_len;
}

void p7501_cmd_uart_stream(const char *arg) {
    p7501_window_boot_spec_t spec;
    p7501_parse_window_boot_args(arg, 10000u, 200u, P7501_CAPTURE_MAX_MS, &spec);

    printf("=== UARTSTREAM: PIO1 %" PRIu32 " %s (window=%" PRIu32 " ms, hold=%" PRIu32 " ms) ===\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()),
           spec.window_ms,
           spec.jig_hold_ms);
    fflush(stdout);

    stream_ctx_t ctx = { .col = 0u, .print_release_note = true };
    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = spec.window_ms,
        .boot_action = spec.boot_action,
        .jig_hold_ms = spec.jig_hold_ms,
        .manage_session = true,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = stream_on_byte,
        .on_tick = NULL,
        .on_sw_rel = stream_on_sw_rel,
        .ctx = &ctx,
    };
    p7501_pio_loop_metrics_t m;
    p7501_pio_run_session(&cfg, &m);

    if (ctx.col > 0u) {
        putchar('\n');
    }
    printf("UARTSTREAM: OK (%" PRIu32 " bytes, framing_err=%" PRIu32 ", overflow=%s%s)\n",
           m.rx_bytes, m.framing_errors, m.overflow ? "YES" : "NO", m.aborted ? ", aborted=YES" : "");
}

void p7501_cmd_hexcap(const char *arg) {
    uint8_t *const hexcap_bytes = g_p7501_cmd_arena.hexcap.bytes;
    uint32_t *const hexcap_gaps_us = g_p7501_cmd_arena.hexcap.gaps_us;

    p7501_window_boot_spec_t spec;
    p7501_parse_window_boot_args(arg, 8000u, 200u, P7501_CAPTURE_MAX_MS, &spec);

    printf("=== HEXCAP: PIO1 %" PRIu32 " %s (window=%" PRIu32 " ms, hold=%" PRIu32 " ms) ===\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()),
           spec.window_ms,
           spec.jig_hold_ms);
    fflush(stdout);

    hexcap_ctx_t ctx = { hexcap_bytes, hexcap_gaps_us, 0u, 0u };
    const p7501_pio_loop_cfg_t cfg = {
        .window_ms = spec.window_ms,
        .boot_action = spec.boot_action,
        .jig_hold_ms = spec.jig_hold_ms,
        .manage_session = true,
        .auto_release_serial_boot = false,
        .allow_cdc_abort = true,
        .on_rx_byte = hexcap_on_byte,
        .on_tick = NULL,
        .on_sw_rel = NULL,
        .ctx = &ctx,
    };
    p7501_pio_loop_metrics_t m;
    p7501_pio_run_session(&cfg, &m);

    flush_hexcap_buffer(hexcap_bytes, hexcap_gaps_us, ctx.buf_len);
    fflush(stdout);
    printf("HEXCAP: OK (%" PRIu32 " bytes, framing_err=%" PRIu32 ", overflow=%s%s)\n",
           m.rx_bytes, m.framing_errors, m.overflow ? "YES" : "NO", m.aborted ? ", aborted=YES" : "");
}
