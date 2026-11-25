/**
 * @file picofw_i2c_master.c
 * @brief RP2040/RP2350 I2C Master (`GP6`/`GP7`, 3.3V only) for `I2CSCAN` & `EEPROM` (#15).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_i2c_codec.h"
#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "hal/picofw_wireless_codec.h"

#include "hardware/gpio.h"
#include "hardware/i2c.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#define PICOFW_I2C_TIMEOUT_US 50000u

static bool i2c_target_power_ok(void) {
    return picofw_hw_is_nsw5v_high() || picofw_hw_is_target_powered();
}

static void init_i2c_safe_bus(void) {
    picofw_hw_enforce_nsw5v_hiz();
    (void)i2c_init(i2c1, PICOFW_I2C_HZ);
    gpio_set_function(PICOFW_I2C_SDA_PIN, GPIO_FUNC_I2C);
    gpio_set_function(PICOFW_I2C_SCL_PIN, GPIO_FUNC_I2C);
    gpio_pull_up(PICOFW_I2C_SDA_PIN);
    gpio_pull_up(PICOFW_I2C_SCL_PIN);
}

static void deinit_i2c_safe_bus(void) {
    i2c_deinit(i2c1);
    gpio_set_function(PICOFW_I2C_SDA_PIN, GPIO_FUNC_SIO);
    gpio_set_dir(PICOFW_I2C_SDA_PIN, GPIO_IN);
    gpio_disable_pulls(PICOFW_I2C_SDA_PIN);
    gpio_set_function(PICOFW_I2C_SCL_PIN, GPIO_FUNC_SIO);
    gpio_set_dir(PICOFW_I2C_SCL_PIN, GPIO_IN);
    gpio_disable_pulls(PICOFW_I2C_SCL_PIN);
    picofw_hw_enforce_nsw5v_hiz();
}

void picofw_cmd_i2c_scan(const char *arg) {
    (void)arg;
    if (!i2c_target_power_ok()) {
        picofw_cli_set_result("ERR", "EPWR");
        printf("[I2C_SCAN] ERROR EPWR: Target rail unpowered; refusing 3.3V GP6/GP7 pull-ups to prevent backpowering.\n");
        return;
    }
    init_i2c_safe_bus();
    uint32_t found = 0u;
    bool aborted = false;
    printf("[I2C_SCAN] Scanning I2C1 on GP%u(SDA)/GP%u(SCL) @ 100kHz (3.3V ONLY)...\n",
           PICOFW_I2C_SDA_PIN, PICOFW_I2C_SCL_PIN);
    for (uint8_t addr = 0x08u; addr <= 0x77u; ++addr) {
        if (picofw_wdt_poll_abort()) {
            aborted = true;
            break;
        }
        uint8_t dummy = 0u;
        const int rc = i2c_read_timeout_us(i2c1, addr, &dummy, 1u, false, PICOFW_I2C_TIMEOUT_US);
        if (rc >= 0) {
            const picofw_ssot_eeprom_chip_t *known = picofw_eeprom_lookup_addr(addr);
            printf("  [0x%02X] ACK (%s)\n",
                   (unsigned)addr, known ? known->part_number : "unknown");
            found += 1u;
        }
    }
    deinit_i2c_safe_bus();
    printf("I2C_SCAN: OK (devices=%" PRIu32 "%s)\n", found, aborted ? ", aborted=YES" : "");
}

static size_t encode_eeprom_addr(bool addr_16bit, uint16_t offset, uint8_t out[2]) {
    if (addr_16bit) {
        out[0] = (uint8_t)((offset >> 8u) & 0xFFu);
        out[1] = (uint8_t)(offset & 0xFFu);
        return 2u;
    }
    out[0] = (uint8_t)(offset & 0xFFu);
    return 1u;
}

void picofw_cmd_eeprom(const char *arg) {
    picofw_eeprom_spec_t spec;
    if (!picofw_eeprom_parse_args(arg, &spec)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("EEPROM: ERROR (Usage: EEPROM INFO | READ <0x50..0x57> <off> [len] | WRITE <addr> <off> <hex> CONFIRM)\n");
        return;
    }
    if (spec.op == PICOFW_EEPROM_OP_INFO) {
        for (size_t i = 0u; i < PICOFW_SSOT_EEPROM_COUNT; ++i) {
            const picofw_ssot_eeprom_chip_t *c = &k_picofw_ssot_eeprom_chips[i];
            printf("  %s @ 0x%02X: %luB (page=%u, %umV)\n",
                   c->part_number, c->i2c_addr_7bit, (unsigned long)c->total_bytes,
                   (unsigned)c->page_size, (unsigned)c->voltage_mv);
        }
        printf("EEPROM: OK\n");
        return;
    }
    if ((spec.op == PICOFW_EEPROM_OP_WRITE) &&
        !picofw_wireless_nor_write_allowed(picofw_wireless_global_state())) {
        picofw_cli_set_result("ERR", "E05");
        printf("EEPROM: ERROR E05 (Wireless F14 lockout: authorize via USB BLE_NOR_AUTH CONFIRM)\n");
        return;
    }
    if (!i2c_target_power_ok()) {
        picofw_cli_set_result("ERR", "EPWR");
        printf("EEPROM: ERROR EPWR (Target rail unpowered; refusing 3.3V GP6/GP7 pull-ups)\n");
        return;
    }
    init_i2c_safe_bus();
    bool ok = false;
    if (spec.op == PICOFW_EEPROM_OP_READ) {
        uint8_t addr_buf[2];
        const size_t addr_len = encode_eeprom_addr(spec.addr_16bit, spec.offset, addr_buf);
        uint8_t *const rx = g_picofw_cmd_arena.raw;
        if (i2c_write_timeout_us(i2c1, spec.addr_7bit, addr_buf, addr_len, true, PICOFW_I2C_TIMEOUT_US) < 0 ||
            i2c_read_timeout_us(i2c1, spec.addr_7bit, rx, spec.len, false, PICOFW_I2C_TIMEOUT_US) < 0) {
            picofw_cli_set_result("ERR", "EI2C");
            printf("EEPROM: ERROR (I2C NACK/TIMEOUT @ 0x%02X)\n", (unsigned)spec.addr_7bit);
        } else {
            for (uint16_t i = 0u; i < spec.len; ++i) {
                printf("%02X%c", rx[i], ((i % 16u) == 15u || i + 1u == spec.len) ? '\n' : ' ');
            }
            ok = true;
        }
    } else {
        const picofw_ssot_eeprom_chip_t *chip = picofw_eeprom_lookup_addr(spec.addr_7bit);
        uint16_t page_sz = (chip != NULL && chip->page_size > 0u) ? chip->page_size : 8u;
        if (page_sz > 16u) {
            page_sz = 16u;
        }
        uint16_t done = 0u;
        ok = true;
        while (done < spec.len) {
            if (picofw_wdt_poll_abort()) {
                picofw_cli_set_result("ERR", "EABORT");
                ok = false;
                break;
            }
            const uint16_t cur_off = (uint16_t)(spec.offset + done);
            const uint16_t page_rem = (uint16_t)(page_sz - (cur_off % page_sz));
            const uint16_t left = (uint16_t)(spec.len - done);
            const uint16_t chunk = (left < page_rem) ? left : page_rem;
            uint8_t tx[18];
            const size_t addr_len = encode_eeprom_addr(spec.addr_16bit, cur_off, tx);
            memcpy(&tx[addr_len], &spec.write_data[done], chunk);
            const int wr = i2c_write_timeout_us(
                i2c1, spec.addr_7bit, tx, addr_len + (size_t)chunk, false, PICOFW_I2C_TIMEOUT_US
            );
            picofw_wdt_sleep_ms(10u);
            if (wr < 0) {
                picofw_cli_set_result("ERR", "EI2C");
                printf("EEPROM: ERROR (I2C WRITE NACK/TIMEOUT @ 0x%02X off=0x%04X)\n",
                       (unsigned)spec.addr_7bit, (unsigned)cur_off);
                ok = false;
                break;
            }
            done = (uint16_t)(done + chunk);
        }
        if (ok) {
            picofw_wireless_consume_nor_op(picofw_wireless_global_state());
        }
    }
    deinit_i2c_safe_bus();
    if (ok) {
        printf("EEPROM: OK\n");
    }
}

uint8_t picofw_i2c_probe_addrs_safe(const uint8_t *addrs, size_t count) {
    if ((addrs == NULL) || (count == 0u) || !i2c_target_power_ok()) {
        return 0u;
    }
    init_i2c_safe_bus();
    uint8_t mask = 0u;
    const size_t limit = (count > 8u) ? 8u : count;
    for (size_t i = 0u; i < limit; ++i) {
        uint8_t dummy = 0u;
        if (i2c_read_timeout_us(i2c1, addrs[i], &dummy, 1u, false, PICOFW_I2C_TIMEOUT_US) >= 0) {
            mask |= (uint8_t)(1u << i);
        }
    }
    deinit_i2c_safe_bus();
    return mask;
}
