/**
 * @file picofw_target_bus_panamon.c
 * @brief `PANAMON1>[B/R/W/D/F/J/E/U/Q]` PIO1 target-bus exchange, hex-line read, and bytewise write helpers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_target_bus_panamon.h"
#include "common/picofw_hex.h"
#include "flash/picofw_bus_policy.h"
#include "flash/picofw_flash.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "uart/picofw_uart_pio_engine.h"

#include "pico/stdlib.h"
#include "pico/time.h"

#include <stdio.h>
#include <string.h>

#define PIO_PANAMON_TIMEOUT_MS 250u

bool picofw_bus_panamon1_has_prompt(const uint8_t *buf, size_t len) {
    static const char k_tag[] = "PANAMON1>";
    const size_t tag_len = sizeof(k_tag) - 1u;
    if ((buf == NULL) || (len < tag_len)) {
        return false;
    }
    for (size_t i = 0u; i + tag_len <= len; ++i) {
        if (memcmp(&buf[i], k_tag, tag_len) == 0) {
            for (size_t j = i + tag_len; j < len; ++j) {
                if (buf[j] == (uint8_t)']') {
                    return true;
                }
            }
        }
    }
    return false;
}

void picofw_bus_panamon1_drain_tail(uint32_t quiet_ms) {
    uint64_t last_rx = time_us_64();
    const uint64_t hard_end = last_rx + 120000ULL;
    while (time_us_64() < hard_end) {
        picofw_wdt_kick();
        uint8_t b = 0u;
        bool got = false;
        for (uint32_t k = 0u; (k < 4096u) && picofw_pio_pop_rx_byte(&b); ++k) {
            if ((k & 63u) == 63u) {
                picofw_wdt_kick();
            }
            got = true;
        }
        const uint64_t now = time_us_64();
        if (got) {
            last_rx = now;
        } else if ((now - last_rx) >= ((uint64_t)quiet_ms * 1000ULL)) {
            break;
        }
    }
}

static bool panamon1_exchange_open(
    const char *cmd,
    uint32_t timeout_ms,
    bool allow_abort,
    char *out_buf,
    size_t out_cap,
    size_t *out_len,
    bool *io_panamon_mode
) {
    if ((cmd == NULL) || (out_buf == NULL) || (out_cap < 2u) || picofw_hw_is_nsw5v_fault_latched()) {
        return false;
    }
    picofw_pio_drain_rx();

    size_t len = 0u;
    const size_t cmd_len = strlen(cmd);
    /* MN103S PANAMON1 uses a 1-byte SC0RXB with polled synchronous TX echo; pace per byte to prevent overrun. */
    for (size_t i = 0u; i < cmd_len; ++i) {
        picofw_wdt_kick();
        if (picofw_hw_is_nsw5v_fault_latched()) {
            if (io_panamon_mode != NULL) {
                *io_panamon_mode = false;
            }
            return false;
        }
        picofw_pio_tx_bytes((const uint8_t *)&cmd[i], 1u, false);
        sleep_us(350u);
        uint8_t echo = 0u;
        for (uint32_t k = 0u; (k < 4096u) && picofw_pio_pop_rx_byte(&echo); ++k) {
            if ((len + 1u) < out_cap) {
                out_buf[len++] = (char)echo;
            }
        }
    }

    const uint64_t t0 = time_us_64();
    const uint64_t deadline = t0 + ((uint64_t)timeout_ms * 1000ULL);
    bool aborted = false;
    bool has_prompt = picofw_bus_panamon1_has_prompt((const uint8_t *)out_buf, len);

    while (!has_prompt && picofw_wdt_loop_step(t0, deadline, allow_abort, &aborted)) {
        if (picofw_hw_is_nsw5v_fault_latched()) {
            if (io_panamon_mode != NULL) {
                *io_panamon_mode = false;
            }
            return false;
        }
        len = picofw_pio_pop_rx_bounded(
            (uint8_t *)out_buf, (out_cap > 0u) ? (out_cap - 1u) : 0u, len, false
        );
        if (picofw_bus_panamon1_has_prompt((const uint8_t *)out_buf, len)) {
            has_prompt = true;
            break;
        }
    }

    sleep_us(300u);
    len = picofw_pio_pop_rx_bounded(
        (uint8_t *)out_buf, (out_cap > 0u) ? (out_cap - 1u) : 0u, len, false
    );
    out_buf[len] = '\0';
    if (out_len != NULL) {
        *out_len = len;
    }
    if (aborted || !has_prompt) {
        if (io_panamon_mode != NULL) {
            *io_panamon_mode = false;
        }
        return false;
    }
    if (io_panamon_mode != NULL) {
        *io_panamon_mode = true;
    }
    return true;
}

bool picofw_bus_panamon1_probe_open(bool allow_abort, bool *io_panamon_mode) {
    char reply[96];
    return panamon1_exchange_open(
        "?\r", 150u, allow_abort, reply, sizeof(reply), NULL, io_panamon_mode
    );
}

bool picofw_bus_panamon1_cmd_open(
    const char *cmd,
    uint32_t timeout_ms,
    bool allow_abort,
    char *out_buf,
    size_t out_cap,
    bool *io_panamon_mode
) {
    return panamon1_exchange_open(
        cmd, timeout_ms, allow_abort, out_buf, out_cap, NULL, io_panamon_mode
    );
}

bool picofw_bus_panamon1_cmd_raw_open(
    const char *cmd,
    uint32_t timeout_ms,
    bool allow_abort,
    uint8_t *out_buf,
    size_t out_cap,
    size_t *out_len,
    bool *io_panamon_mode
) {
    return panamon1_exchange_open(
        cmd, timeout_ms, allow_abort, (char *)out_buf, out_cap, out_len, io_panamon_mode
    );
}

bool picofw_bus_panamon1_write_buf_open(
    uint32_t addr,
    const uint8_t *buf,
    size_t len,
    bool allow_abort,
    bool *io_panamon_mode
) {
    if ((buf == NULL) || (len == 0u)) {
        return false;
    }
    if (picofw_bus_check_access(
            picofw_session_default_ctx(),
            picofw_flash_get_active_size(),
            addr,
            len,
            true,
            false,
            picofw_flash_is_upper_nor_unlocked()) != PICOFW_BUS_ALLOW) {
        return false;
    }
    char req[24];
    char rx_buf[128];
    for (size_t i = 0u; i < len; ++i) {
        snprintf(req, sizeof(req), "W %08lX,%02X\r", (unsigned long)(addr + (uint32_t)i), (unsigned)buf[i]);
        bool byte_ok = false;
        for (uint8_t attempt = 0u; attempt < 2u; ++attempt) {
            if (panamon1_exchange_open(
                    req, PIO_PANAMON_TIMEOUT_MS, allow_abort, rx_buf, sizeof(rx_buf), NULL, io_panamon_mode
                ) &&
                (strstr(rx_buf, "OK:WRITE") != NULL)) {
                byte_ok = true;
                break;
            }
        }
        if (!byte_ok) {
            return false;
        }
    }
    return true;
}

bool picofw_bus_panamon1_buf_matches_open(
    uint32_t addr,
    const uint8_t *buf,
    size_t len,
    bool allow_abort,
    bool *io_panamon_mode
) {
    if ((buf == NULL) || (len == 0u) || picofw_bus_span_wraps_u32(addr, len)) {
        return false;
    }
    uint8_t read_chunk[PICOFW_PIO_BUS_BLOCK_CHUNK_MAX];
    size_t off = 0u;
    while (off < len) {
        picofw_wdt_kick();
        const size_t rem = len - off;
        const size_t chunk = (rem > PICOFW_PIO_BUS_BLOCK_CHUNK_MAX)
            ? PICOFW_PIO_BUS_BLOCK_CHUNK_MAX
            : rem;
        if (!picofw_bus_panamon1_read_chunk_open(
                addr + (uint32_t)off, read_chunk, (uint32_t)chunk, allow_abort, io_panamon_mode
            ) ||
            (memcmp(read_chunk, &buf[off], chunk) != 0)) {
            return false;
        }
        off += chunk;
    }
    return true;
}

static bool panamon1_extract_hex_line(
    const char *reply,
    size_t expected_hex_chars,
    uint8_t *out_buf,
    size_t chunk
) {
    if ((reply == NULL) || (out_buf == NULL) ||
        (expected_hex_chars > (PICOFW_PIO_BUS_BLOCK_CHUNK_MAX * 2u))) {
        return false;
    }
    const char *p = reply;
    while (*p != '\0') {
        while ((*p == '\r') || (*p == '\n') || (*p == ' ') || (*p == '\t')) {
            ++p;
        }
        if (*p == '\0') {
            break;
        }
        const char *line = p;
        while ((*p != '\0') && (*p != '\r') && (*p != '\n')) {
            ++p;
        }
        size_t line_len = (size_t)(p - line);
        while ((line_len > 0u) && ((line[line_len - 1u] == ' ') || (line[line_len - 1u] == '\t'))) {
            --line_len;
        }
        const bool is_meta = (line_len == 0u) ||
                             ((line_len >= 8u) && (memcmp(line, "PANAMON1", 8u) == 0)) ||
                             ((line_len >= 2u) && (memcmp(line, "R ", 2u) == 0)) ||
                             ((line_len >= 2u) && (memcmp(line, "W ", 2u) == 0)) ||
                             ((line_len >= 3u) && (memcmp(line, "OK:", 3u) == 0)) ||
                             ((line_len >= 4u) && (memcmp(line, "ERR:", 4u) == 0));
        if (!is_meta && (line_len == expected_hex_chars)) {
            char hex_line[(PICOFW_PIO_BUS_BLOCK_CHUNK_MAX * 2u) + 1u];
            memcpy(hex_line, line, line_len);
            hex_line[line_len] = '\0';
            size_t blen = 0u;
            if (picofw_decode_hex_bytes(hex_line, out_buf, chunk, &blen) && (blen == chunk)) {
                return true;
            }
        }
    }
    return false;
}

bool picofw_bus_panamon1_read_chunk_open(
    uint32_t addr,
    uint8_t *out_buf,
    uint32_t chunk,
    bool allow_abort,
    bool *io_panamon_mode
) {
    char req[32];
    char rx_buf[(PICOFW_PIO_BUS_BLOCK_CHUNK_MAX * 2u) + 128u];
    snprintf(req, sizeof(req), "R %08lX,%lX\r", (unsigned long)addr, (unsigned long)chunk);
    for (uint8_t attempt = 0u; attempt < 2u; ++attempt) {
        size_t rx_len = 0u;
        if (panamon1_exchange_open(
                req, PIO_PANAMON_TIMEOUT_MS, allow_abort, rx_buf, sizeof(rx_buf), &rx_len, io_panamon_mode
            ) &&
            panamon1_extract_hex_line(rx_buf, (size_t)chunk * 2u, out_buf, (size_t)chunk)) {
            return true;
        }
    }
    return false;
}

bool picofw_bus_panamon1_write_u32(uint32_t addr, uint32_t data, bool *io_panamon_mode) {
    if (!picofw_hw_is_nsw5v_high() || picofw_hw_is_nsw5v_fault_latched()) {
        return false;
    }
    uint8_t bytes[4];
    picofw_write_u32_le(bytes, data);
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    const bool ok = picofw_bus_panamon1_write_buf_open(addr, bytes, sizeof(bytes), true, io_panamon_mode);
    picofw_pio_end_session();
    return ok;
}
