/**
 * @file gdb_rsp_codec.c
 * @brief Explicit-context GDB RSP packet framing state machine, pure hex/checksum codecs, and wrappers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "gdb/gdb_rsp_internal.h"
#include "common/p7501_hex.h"
#include <stdio.h>
#include <string.h>

static gdb_rsp_ctx_t s_default_gdb_ctx;

#if defined(PICO_HOST_TEST) && (PICO_HOST_TEST != 0)
static void gdb_rsp_test_record_response(gdb_rsp_ctx_t *ctx, const char *payload) {
    if ((ctx != NULL) && (payload != NULL)) {
        snprintf(ctx->test_last_resp, sizeof(ctx->test_last_resp), "%s", payload);
    }
}

void gdb_rsp_ctx_test_get_last_response(const gdb_rsp_ctx_t *ctx, char *out, size_t max_len) {
    if ((ctx != NULL) && (out != NULL) && (max_len > 0U)) {
        snprintf(out, max_len, "%s", ctx->test_last_resp);
    }
}

void gdb_rsp_ctx_test_inject_packet_len(gdb_rsp_ctx_t *ctx, const uint8_t *payload, size_t len) {
    if ((ctx == NULL) || (payload == NULL)) {
        return;
    }
    gdb_rsp_ctx_process_char(ctx, '$');
    uint8_t csum = 0U;
    for (size_t i = 0U; i < len; i++) {
        const uint8_t b = payload[i];
        csum = (uint8_t)(csum + b);
        gdb_rsp_ctx_process_char(ctx, (char)b);
    }
    gdb_rsp_ctx_process_char(ctx, '#');
    gdb_rsp_ctx_process_char(ctx, gdb_nibble_to_hex_char((uint8_t)(csum >> 4)));
    gdb_rsp_ctx_process_char(ctx, gdb_nibble_to_hex_char((uint8_t)(csum & 0x0FU)));
}

void gdb_rsp_ctx_test_inject_packet(gdb_rsp_ctx_t *ctx, const char *payload) {
    if (payload == NULL) {
        return;
    }
    gdb_rsp_ctx_test_inject_packet_len(ctx, (const uint8_t *)payload, strlen(payload));
}

void gdb_rsp_test_get_last_response(char *out, size_t max_len) {
    gdb_rsp_ctx_test_get_last_response(&s_default_gdb_ctx, out, max_len);
}

void gdb_rsp_test_inject_packet_len(const uint8_t *payload, size_t len) {
    gdb_rsp_ctx_test_inject_packet_len(&s_default_gdb_ctx, payload, len);
}

void gdb_rsp_test_inject_packet(const char *payload) {
    gdb_rsp_ctx_test_inject_packet(&s_default_gdb_ctx, payload);
}
#endif

int gdb_hex_char_to_nibble(char c) {
    return p7501_hex_char_to_nibble(c);
}

char gdb_nibble_to_hex_char(uint8_t n) {
    return p7501_nibble_to_hex_char(n);
}

uint8_t gdb_compute_checksum(const char *payload) {
    return p7501_rsp_checksum_str(payload);
}

const char *gdb_parse_hex_u32(const char *str, uint32_t *out_val, char delim) {
    uint32_t val = 0U;
    size_t digits = 0U;
    if ((str == NULL) || (out_val == NULL)) {
        return NULL;
    }

    while ((*str != '\0') && (*str != delim)) {
        const int nibble = gdb_hex_char_to_nibble(*str);
        if (nibble < 0) {
            return NULL;
        }
        val = (val << 4) | (uint32_t)nibble;
        str++;
        digits++;
    }
    if ((digits == 0U) || (*str != delim)) {
        return NULL;
    }
    *out_val = val;
    if ((*str == delim) && (delim != '\0')) {
        str++;
    }
    return str;
}

const char *gdb_parse_addr_and_len(
    const char *str,
    uint32_t *out_addr,
    uint32_t *out_len,
    char end_delim
) {
    const char *p = gdb_parse_hex_u32(str, out_addr, ',');
    if (p == NULL) {
        return NULL;
    }
    p = gdb_parse_hex_u32(p, out_len, end_delim);
    if ((p == NULL) || (*out_len == 0U) || (*out_len > 512U)) {
        return NULL;
    }
    return p;
}

void gdb_rsp_ctx_init(gdb_rsp_ctx_t *ctx) {
    if (ctx == NULL) {
        return;
    }
    memset(ctx, 0, sizeof(*ctx));
    ctx->parser_state = GDB_STATE_IDLE;
    ctx->flash_size_bytes = 0x00800000UL;
    gdb_cpu_init_registers(&ctx->regs);
    gdb_rsp_ctx_invalidate_cache(ctx);
}

bool gdb_rsp_ctx_is_active(const gdb_rsp_ctx_t *ctx) {
    return (ctx != NULL) && ctx->active;
}

void gdb_rsp_ctx_set_active(gdb_rsp_ctx_t *ctx, bool active) {
    if (ctx == NULL) {
        return;
    }
    ctx->active = active;
    if (active) {
        ctx->parser_state = GDB_STATE_IDLE;
        ctx->rx_len = 0U;
        gdb_rsp_ctx_invalidate_cache(ctx);
        gdb_cpu_sync_registers(&ctx->regs);
    }
}

void gdb_rsp_ctx_send_packet(gdb_rsp_ctx_t *ctx, const char *payload) {
    if ((ctx == NULL) || (payload == NULL)) {
        return;
    }

#if defined(PICO_HOST_TEST) && (PICO_HOST_TEST != 0)
    gdb_rsp_test_record_response(ctx, payload);
#endif

    const uint8_t csum = gdb_compute_checksum(payload);
    snprintf(ctx->last_tx_packet, sizeof(ctx->last_tx_packet), "%s", payload);

    putchar('$');
    const char *p = payload;
    while (*p != '\0') {
        putchar(*p++);
    }
    putchar('#');
    putchar(gdb_nibble_to_hex_char((uint8_t)(csum >> 4)));
    putchar(gdb_nibble_to_hex_char((uint8_t)(csum & 0x0FU)));
}

void gdb_rsp_ctx_process_char(gdb_rsp_ctx_t *ctx, char ch) {
    if (ctx == NULL) {
        return;
    }
    if ((ch == 0x03) && !((ctx->parser_state == GDB_STATE_DATA) && ctx->rx_escaped)) {
        gdb_cpu_handle_ctrl_c(ctx);
        return;
    }

    switch (ctx->parser_state) {
        case GDB_STATE_IDLE:
            if (ch == '$') {
                ctx->parser_state = GDB_STATE_DATA;
                ctx->rx_len = 0U;
                ctx->rx_escaped = false;
                ctx->calc_checksum = 0U;
            } else if (ch == '-') {
                if (ctx->last_tx_packet[0] != '\0') {
                    gdb_rsp_ctx_send_packet(ctx, ctx->last_tx_packet);
                }
            } else if (ch == '+') {
                /* ACK received */
            }
            break;

        case GDB_STATE_DATA:
            if ((ch == '#') && !ctx->rx_escaped) {
                ctx->parser_state = GDB_STATE_CHKSUM_MSB;
            } else if (ctx->rx_len < (GDB_RSP_MAX_PACKET_LEN - 1U)) {
                ctx->rx_buffer[ctx->rx_len++] = ch;
                ctx->calc_checksum = (uint8_t)(ctx->calc_checksum + (uint8_t)ch);
                ctx->rx_escaped = !ctx->rx_escaped && ((uint8_t)ch == 0x7DU);
            }
            break;

        case GDB_STATE_CHKSUM_MSB: {
            const int nibble = gdb_hex_char_to_nibble(ch);
            if (nibble >= 0) {
                ctx->received_checksum = (uint8_t)(nibble << 4);
                ctx->parser_state = GDB_STATE_CHKSUM_LSB;
            } else {
                putchar('-');
                ctx->parser_state = GDB_STATE_IDLE;
            }
            break;
        }

        case GDB_STATE_CHKSUM_LSB: {
            const int nibble = gdb_hex_char_to_nibble(ch);
            if (nibble >= 0) {
                ctx->received_checksum = (uint8_t)(ctx->received_checksum | (uint8_t)nibble);
                if (ctx->received_checksum == ctx->calc_checksum) {
                    putchar('+');
                    ctx->rx_buffer[ctx->rx_len] = '\0';
                    gdb_cpu_dispatch_command(ctx, ctx->rx_buffer);
                } else {
                    putchar('-');
                }
            } else {
                putchar('-');
            }
            ctx->parser_state = GDB_STATE_IDLE;
            break;
        }
    }
}

void gdb_rsp_init(void) { gdb_rsp_ctx_init(&s_default_gdb_ctx); }
bool gdb_rsp_is_active(void) { return gdb_rsp_ctx_is_active(&s_default_gdb_ctx); }
void gdb_rsp_set_active(bool active) { gdb_rsp_ctx_set_active(&s_default_gdb_ctx, active); }
void gdb_rsp_process_char(char ch) { gdb_rsp_ctx_process_char(&s_default_gdb_ctx, ch); }
void gdb_rsp_send_packet(const char *payload) { gdb_rsp_ctx_send_packet(&s_default_gdb_ctx, payload); }
void gdb_rsp_invalidate_cache(void) { gdb_rsp_ctx_invalidate_cache(&s_default_gdb_ctx); }
const char *gdb_rsp_get_status_info(void) { return GDB_TARGET_CHIP_LABEL; }
bool gdb_rsp_set_flash_size(uint32_t flash_size_bytes) {
    return gdb_rsp_ctx_set_flash_size(&s_default_gdb_ctx, flash_size_bytes);
}
uint32_t gdb_rsp_get_flash_size(void) { return gdb_rsp_ctx_get_flash_size(&s_default_gdb_ctx); }
