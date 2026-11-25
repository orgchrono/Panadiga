/**
 * @file picofw_i2c_codec.h
 * @brief Pure C11 SSOT I2C EEPROM catalog & safety codec (`I2CSCAN` / `EEPROM`, Feature #15).
 *
 * Host-testable pure C11 (consumes `k_picofw_ssot_eeprom_chips[]`, strictly 3.3V GP6/GP7).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_I2C_CODEC_H
#define PICOFW_I2C_CODEC_H

#include "generated/picofw_ssot.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_I2C_SDA_PIN 6u
#define PICOFW_I2C_SCL_PIN 7u
#define PICOFW_I2C_HZ      100000u

typedef enum picofw_eeprom_op {
    PICOFW_EEPROM_OP_INFO = 0,
    PICOFW_EEPROM_OP_READ = 1,
    PICOFW_EEPROM_OP_WRITE = 2
} picofw_eeprom_op_t;

typedef struct picofw_eeprom_spec {
    picofw_eeprom_op_t op;
    uint8_t addr_7bit;
    uint16_t offset;
    uint16_t len;
    uint8_t write_data[16];
    bool addr_16bit;
    bool write_confirmed;
} picofw_eeprom_spec_t;

/** Look up an SSOT EEPROM descriptor by 7-bit I2C address (`0x50..0x57`). */
const picofw_ssot_eeprom_chip_t *picofw_eeprom_lookup_addr(uint8_t addr_7bit);

/** Validate 3.3V-safe EEPROM operation and bounded offset/page length. */
bool picofw_eeprom_parse_args(const char *arg, picofw_eeprom_spec_t *out);

/** Probe up to 8 candidate 7-bit I2C addresses safely when target rail is powered; returns bitmask. */
uint8_t picofw_i2c_probe_addrs_safe(const uint8_t *addrs, size_t count);

void picofw_cmd_i2c_scan(const char *arg);
void picofw_cmd_eeprom(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_I2C_CODEC_H */
