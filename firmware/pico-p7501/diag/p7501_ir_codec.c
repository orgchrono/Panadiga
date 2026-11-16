/**
 * @file p7501_ir_codec.c
 * @brief Pure ISO C11 Panasonic Kaseikyo IR frame encoder, named service LUT, and GPIO safety validator.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_ir_codec.h"
#include "common/p7501_hex.h"
#include "uart/p7501_pana_rsp_internal.h"
#include "uart/p7501_uart_codec.h"

#include <stdio.h>
#include <string.h>

typedef struct ir_preset_entry {
    const char *name;
    uint8_t dev;
    uint8_t sub;
    uint8_t cmd;
} ir_preset_entry_t;

static const ir_preset_entry_t k_ir_presets[] = {
    { "POWER",   P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x3Du },
    { "STOP",    P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x00u },
    { "EJECT",   P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x01u },
    { "OPEN",    P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x01u },
    { "REC",     P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x08u },
    { "PLAY",    P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x0Au },
    { "AUDIO",   P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x33u },
    { "CHUP",    P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x34u },
    { "CHDN",    P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x35u },
    { "STATUS",  P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x5Fu },
    { "OK",      P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x82u },
    { "RETURN",  P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x81u },
    { "DISPLAY", P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x92u },
    { "SERV",    P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0xF2u },
    { "INIT",    P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0xF6u },
    { "ACCEPT",  P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0xFDu },
    { "BLUE",    P7501_IR_DEV_DIGA_RECORDER, 0x01u, 0x40u },
    { "RED",     P7501_IR_DEV_DIGA_RECORDER, 0x01u, 0x41u },
    { "GREEN",   P7501_IR_DEV_DIGA_RECORDER, 0x01u, 0x42u },
    { "YELLOW",  P7501_IR_DEV_DIGA_RECORDER, 0x01u, 0x43u },
    { "DISC",    P7501_IR_DEV_ND_SERVICE,    0x01u, 0x80u },
    { "ND1",     P7501_IR_DEV_ND_SERVICE,    0x01u, 0x81u },
    { "ND2",     P7501_IR_DEV_ND_SERVICE,    0x01u, 0x82u },
    { "0",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x19u },
    { "1",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x10u },
    { "2",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x11u },
    { "3",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x12u },
    { "4",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x13u },
    { "5",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x14u },
    { "6",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x15u },
    { "7",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x16u },
    { "8",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x17u },
    { "9",       P7501_IR_DEV_DIGA_RECORDER, 0x00u, 0x18u },
};

bool p7501_ir_is_safe_gpio(uint32_t gpio) {
    return ((gpio >= 6u) && (gpio <= 9u)) || ((gpio >= 16u) && (gpio <= 22u));
}

void p7501_ir_encode_kaseikyo(
    uint8_t dev_byte,
    uint8_t sub_byte,
    uint8_t cmd_byte,
    uint8_t out_frame[P7501_IR_KASEIKYO_FRAME_BYTES]
) {
    if (out_frame == NULL) {
        return;
    }
    const uint8_t sub_masked = (uint8_t)(sub_byte & 0x0Fu);
    out_frame[0] = P7501_IR_VENDOR_LSB0;
    out_frame[1] = P7501_IR_VENDOR_LSB1;
    out_frame[2] = dev_byte;
    out_frame[3] = sub_masked;
    out_frame[4] = cmd_byte;
    out_frame[5] = (uint8_t)(dev_byte ^ sub_masked ^ cmd_byte);
}

static bool parse_ir_command_token(const char *tok, p7501_ir_spec_t *out) {
    for (size_t i = 0u; i < (sizeof(k_ir_presets) / sizeof(k_ir_presets[0])); ++i) {
        if (p7501_ascii_ieq(tok, k_ir_presets[i].name)) {
            p7501_ir_encode_kaseikyo(
                k_ir_presets[i].dev, k_ir_presets[i].sub, k_ir_presets[i].cmd, out->frame
            );
            (void)snprintf(out->label, sizeof(out->label), "%s", k_ir_presets[i].name);
            return true;
        }
    }

    const char *colon = strchr(tok, ':');
    uint32_t dev = P7501_IR_DEV_DIGA_RECORDER;
    uint32_t sub = 0u;
    uint32_t cmd = 0u;
    if (colon != NULL) {
        if ((colon == tok) || (colon[1] == '\0')) {
            return false;
        }
        uint32_t lhs = 0u;
        if (!p7501_rsp_parse_hex_u32_span(tok, (size_t)(colon - tok), &lhs) ||
            !p7501_rsp_parse_hex_u32(colon + 1, 4u, &cmd) ||
            (cmd > 0xFFu)) {
            return false;
        }
        if (lhs <= 0x0Fu) {
            sub = lhs;
        } else if (lhs <= 0xFFFFu) {
            dev = (lhs >> 8) & 0xFFu;
            sub = lhs & 0x0Fu;
        } else {
            return false;
        }
    } else {
        if (!p7501_rsp_parse_hex_u32(tok, 6u, &cmd) || (cmd > 0x7FFu)) {
            return false;
        }
        sub = (cmd >> 8) & 0x07u;
        cmd &= 0xFFu;
    }

    p7501_ir_encode_kaseikyo((uint8_t)dev, (uint8_t)sub, (uint8_t)cmd, out->frame);
    (void)snprintf(
        out->label, sizeof(out->label), "HEX_%02X_%02X%02X",
        (unsigned)dev, (unsigned)sub, (unsigned)cmd
    );
    return true;
}

bool p7501_ir_parse_args(const char *arg, p7501_ir_spec_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->gpio = P7501_IR_DEFAULT_GPIO;
    out->repeat = P7501_IR_DEFAULT_REPEAT;

    if ((arg == NULL) || (arg[0] == '\0')) {
        return parse_ir_command_token("RED", out);
    }

    char tok1[32] = {0};
    char tok2[24] = {0};
    char tok3[24] = {0};
    const char *cur = p7501_next_token(arg, tok1, sizeof(tok1));
    if ((cur == NULL) || (tok1[0] == '\0') || !parse_ir_command_token(tok1, out)) {
        return false;
    }

    cur = p7501_next_token(cur, tok2, sizeof(tok2));
    if (tok2[0] != '\0') {
        const char *pin_str = p7501_ascii_iprefix(tok2, "GP") ? (tok2 + 2) : tok2;
        if (!p7501_ascii_is_digits(pin_str)) {
            return false;
        }
        const uint32_t gpio = p7501_parse_uint32_clamped(pin_str, P7501_IR_DEFAULT_GPIO, 0u, 29u);
        if (!p7501_ir_is_safe_gpio(gpio)) {
            return false;
        }
        out->gpio = gpio;

        if (p7501_next_token(cur, tok3, sizeof(tok3)) != NULL && tok3[0] != '\0') {
            if (!p7501_ascii_is_digits(tok3)) {
                return false;
            }
            out->repeat = p7501_parse_uint32_clamped(
                tok3, P7501_IR_DEFAULT_REPEAT, 1u, P7501_IR_MAX_REPEAT
            );
        }
    }
    return true;
}
