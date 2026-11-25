/**
 * @file picofw_i2c_codec.c
 * @brief Pure C11 I2C EEPROM address lookup & argument validation (`k_picofw_ssot_eeprom_chips`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_i2c_codec.h"
#include "common/picofw_hex.h"
#include "uart/picofw_uart_codec.h"
#include <string.h>

const picofw_ssot_eeprom_chip_t *picofw_eeprom_lookup_addr(uint8_t addr_7bit) {
    for (size_t i = 0u; i < PICOFW_SSOT_EEPROM_COUNT; ++i) {
        if (k_picofw_ssot_eeprom_chips[i].i2c_addr_7bit == addr_7bit) {
            return &k_picofw_ssot_eeprom_chips[i];
        }
    }
    return NULL;
}

bool picofw_eeprom_parse_args(const char *arg, picofw_eeprom_spec_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->op = PICOFW_EEPROM_OP_INFO;
    out->addr_7bit = 0x50u;
    out->len = 16u;

    if ((arg == NULL) || (*arg == '\0')) {
        return true;
    }
    char tok[48];
    const char *cur = picofw_next_token(arg, tok, sizeof(tok));
    if (picofw_ascii_ieq(tok, "INFO")) {
        return true;
    }
    if (picofw_ascii_ieq(tok, "READ")) {
        out->op = PICOFW_EEPROM_OP_READ;
    } else if (picofw_ascii_ieq(tok, "WRITE")) {
        out->op = PICOFW_EEPROM_OP_WRITE;
    } else {
        return false;
    }

    uint32_t addr = 0x50u, off = 0u, len = 16u;
    if ((cur == NULL) || (picofw_next_token(cur, tok, sizeof(tok)) == NULL) ||
        !picofw_parse_u32_auto(tok, &addr) || (addr < 0x50u) || (addr > 0x57u)) {
        return false;
    }
    /* Move past addr token */
    cur = arg;
    for (int skip = 0; skip < 2 && cur != NULL; ++skip) {
        cur = picofw_next_token(cur, tok, sizeof(tok));
    }
    if ((cur != NULL) && (picofw_next_token(cur, tok, sizeof(tok)) != NULL) && (tok[0] != '\0')) {
        if (!picofw_parse_u32_auto(tok, &off) || (off > 8192u)) {
            return false;
        }
        cur = picofw_next_token(cur, tok, sizeof(tok));
    }
    if (out->op == PICOFW_EEPROM_OP_READ) {
        if ((cur != NULL) && (picofw_next_token(cur, tok, sizeof(tok)) != NULL) && (tok[0] != '\0')) {
            if (!picofw_parse_u32_auto(tok, &len) || (len == 0u) || (len > 256u)) {
                return false;
            }
        }
    } else {
        if ((cur == NULL) || (picofw_next_token(cur, tok, sizeof(tok)) == NULL) || (tok[0] == '\0')) {
            return false;
        }
        size_t wlen = 0u;
        if (!picofw_decode_hex_bytes(tok, out->write_data, sizeof(out->write_data), &wlen)) {
            return false;
        }
        len = (uint32_t)wlen;
        cur = picofw_next_token(cur, tok, sizeof(tok));
        if ((cur != NULL) && (picofw_next_token(cur, tok, sizeof(tok)) != NULL) &&
            picofw_ascii_ieq(tok, "CONFIRM")) {
            out->write_confirmed = true;
        }
        if (!out->write_confirmed) {
            return false;
        }
    }

    const picofw_ssot_eeprom_chip_t *chip = picofw_eeprom_lookup_addr((uint8_t)addr);
    if ((chip != NULL) && (chip->voltage_mv > PICOFW_SSOT_VDD_FLOATING_CEIL_MV)) {
        return false;
    }
    const uint32_t max_bytes = (chip != NULL) ? chip->total_bytes : 8192u;
    if ((off + len) > max_bytes) {
        return false;
    }
    out->addr_7bit = (uint8_t)addr;
    out->offset = (uint16_t)off;
    out->len = (uint16_t)len;
    out->addr_16bit = (chip != NULL) ? (chip->total_bytes > 2048u) : (off > 0xFFu);
    return true;
}
