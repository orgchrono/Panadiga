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

bool p7501_ir_parse_gpio_token(const char *tok, uint32_t *out_gpio) {
    if ((tok == NULL) || (tok[0] == '\0') || (out_gpio == NULL)) {
        return false;
    }
    const char *pin_str = p7501_ascii_iprefix(tok, "GP") ? (tok + 2) : tok;
    if (!p7501_ascii_is_digits(pin_str)) {
        return false;
    }
    const uint32_t gpio = p7501_parse_uint32_clamped(pin_str, P7501_IR_DEFAULT_GPIO, 0u, 29u);
    if (!p7501_ir_is_safe_gpio(gpio)) {
        return false;
    }
    *out_gpio = gpio;
    return true;
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
    /* Preserve upper bits of sub_byte (0x00..0x3F for DIGA Remote Modes 1/2/3). */
    out_frame[0] = P7501_IR_VENDOR_LSB0;
    out_frame[1] = P7501_IR_VENDOR_LSB1;
    out_frame[2] = dev_byte;
    out_frame[3] = sub_byte;
    out_frame[4] = cmd_byte;
    out_frame[5] = (uint8_t)(dev_byte ^ sub_byte ^ cmd_byte);
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
        if (lhs <= 0xFFu) {
            sub = lhs;
        } else if (lhs <= 0xFFFFu) {
            dev = (lhs >> 8) & 0xFFu;
            sub = lhs & 0xFFu;
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
        if (!p7501_ir_parse_gpio_token(tok2, &out->gpio)) {
            return false;
        }

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

static bool ir_match_us(uint32_t actual_us, uint32_t target_us) {
    const uint32_t lo = (target_us * 65u) / 100u;
    const uint32_t hi = (target_us * 140u) / 100u;
    return (actual_us >= lo) && (actual_us <= hi);
}

const char *p7501_ir_lookup_preset_name(uint8_t dev_byte, uint8_t sub_byte, uint8_t cmd_byte) {
    for (size_t i = 0u; i < (sizeof(k_ir_presets) / sizeof(k_ir_presets[0])); ++i) {
        if ((k_ir_presets[i].dev == dev_byte) &&
            (k_ir_presets[i].sub == sub_byte) &&
            (k_ir_presets[i].cmd == cmd_byte)) {
            return k_ir_presets[i].name;
        }
    }
    return NULL;
}

size_t p7501_ir_build_pulse_table(
    const uint8_t frame[P7501_IR_KASEIKYO_FRAME_BYTES],
    uint32_t *out_durations_us,
    size_t cap
) {
    if ((frame == NULL) || (out_durations_us == NULL) || (cap < P7501_IR_PULSE_COUNT)) {
        return 0u;
    }
    size_t idx = 0u;
    out_durations_us[idx++] = P7501_IR_LEADER_MARK_US;
    out_durations_us[idx++] = P7501_IR_LEADER_SPACE_US;
    for (size_t b = 0u; b < P7501_IR_KASEIKYO_FRAME_BYTES; ++b) {
        const uint8_t val = frame[b];
        for (uint8_t bit = 0u; bit < 8u; ++bit) {
            out_durations_us[idx++] = P7501_IR_UNIT_US;
            out_durations_us[idx++] = (((val >> bit) & 1u) != 0u)
                ? P7501_IR_BIT_ONE_SPACE_US
                : P7501_IR_BIT_ZERO_SPACE_US;
        }
    }
    out_durations_us[idx++] = P7501_IR_UNIT_US;
    return idx;
}

bool p7501_ir_decode_pulses(
    const uint32_t *durations_us,
    size_t count,
    p7501_ir_decode_result_t *out_res
) {
    if ((durations_us == NULL) || (out_res == NULL) || (count < (P7501_IR_PULSE_COUNT - 1u))) {
        return false;
    }
    memset(out_res, 0, sizeof(*out_res));
    if (!ir_match_us(durations_us[0], P7501_IR_LEADER_MARK_US) ||
        !ir_match_us(durations_us[1], P7501_IR_LEADER_SPACE_US)) {
        return false;
    }
    size_t idx = 2u;
    for (size_t b = 0u; b < P7501_IR_KASEIKYO_FRAME_BYTES; ++b) {
        uint8_t val = 0u;
        for (uint8_t bit = 0u; bit < 8u; ++bit) {
            const uint32_t mark_us = durations_us[idx++];
            const uint32_t space_us = durations_us[idx++];
            if (!ir_match_us(mark_us, P7501_IR_UNIT_US)) {
                return false;
            }
            if (ir_match_us(space_us, P7501_IR_BIT_ONE_SPACE_US)) {
                val |= (uint8_t)(1u << bit);
            } else if (!ir_match_us(space_us, P7501_IR_BIT_ZERO_SPACE_US)) {
                return false;
            }
        }
        out_res->frame[b] = val;
    }
    out_res->dev = out_res->frame[2];
    out_res->sub = out_res->frame[3];
    out_res->cmd = out_res->frame[4];
    out_res->xor_rx = out_res->frame[5];
    out_res->xor_expected = (uint8_t)(out_res->dev ^ out_res->sub ^ out_res->cmd);
    out_res->vendor_ok = (out_res->frame[0] == P7501_IR_VENDOR_LSB0) &&
                         (out_res->frame[1] == P7501_IR_VENDOR_LSB1);
    out_res->parity_ok = (out_res->xor_rx == out_res->xor_expected);
    out_res->valid = out_res->vendor_ok && out_res->parity_ok;
    return true;
}

bool p7501_ir_parse_rx_args(const char *arg, uint32_t *out_ms, uint32_t *out_gpio) {
    if ((out_ms == NULL) || (out_gpio == NULL)) {
        return false;
    }
    *out_ms = 5000u;
    *out_gpio = P7501_IR_DEFAULT_RX_GPIO;
    if (arg == NULL) {
        return true;
    }

    char tok[24];
    const char *cur = arg;
    while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
        const char *pin_str = p7501_ascii_iprefix(tok, "GP") ? (tok + 2) : NULL;
        if (pin_str != NULL) {
            if (!p7501_ascii_is_digits(pin_str)) {
                return false;
            }
            const uint32_t gpio = p7501_parse_uint32_clamped(pin_str, P7501_IR_DEFAULT_RX_GPIO, 0u, 29u);
            if (!p7501_ir_is_safe_gpio(gpio)) {
                return false;
            }
            *out_gpio = gpio;
        } else if (p7501_ascii_is_digits(tok)) {
            const uint32_t val = p7501_parse_uint32_clamped(tok, 5000u, 100u, 30000u);
            *out_ms = val;
        } else {
            return false;
        }
    }
    return true;
}
