/**
 * @file picofw_fast_stub_io.c
 * @brief PIO1 paced I/O, `@PICO_PROG` push emitter, and `0x9043A000` stub lifecycle transport.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_fast_stub_io.h"
#include "common/picofw_hex.h"
#include "flash/picofw_fast_stream_codec.h"
#include "flash/picofw_flash.h"
#include "flash/picofw_target_bus_panamon.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "uart/picofw_uart_pio_engine.h"

#include "pico/stdlib.h"
#include "pico/time.h"

#include <stdio.h>
#include <string.h>

void picofw_fast_emit_prog_push(
    const char *phase,
    uint16_t sa,
    uint32_t done,
    uint32_t total,
    uint32_t crc32
) {
    char line[128];
    const uint32_t vdd_mv = picofw_hw_read_vdd_mv();
    if (picofw_fast_format_prog_line(line, sizeof(line), phase, sa, done, total, crc32, vdd_mv) > 0u) {
        printf("%s\n", line);
        fflush(stdout);
    }
}

bool picofw_fast_pio_read_exact_ms(uint8_t *dst, size_t n, uint32_t timeout_ms) {
    const uint64_t t0 = time_us_64();
    const uint64_t deadline = t0 + ((uint64_t)timeout_ms * 1000ULL);
    size_t got = 0u;
    bool aborted = false;
    while ((got < n) && picofw_wdt_loop_step(t0, deadline, true, &aborted)) {
        if (picofw_hw_is_nsw5v_fault_latched()) {
            return false;
        }
        uint8_t b = 0u;
        for (uint32_t k = 0u; (k < 4096u) && (got < n) && picofw_pio_pop_rx_byte(&b); ++k) {
            if ((k & 63u) == 63u) {
                picofw_wdt_kick();
            }
            dst[got++] = b;
        }
    }
    return (!aborted) && (got == n);
}

void picofw_fast_pio_tx_paced_ascii(const char *str) {
    for (size_t i = 0u; str[i] != '\0'; ++i) {
        picofw_wdt_kick();
        picofw_pio_tx_bytes((const uint8_t *)&str[i], 1u, false);
        sleep_us(350u);
        uint8_t drop = 0u;
        for (uint32_t k = 0u; (k < 4096u) && picofw_pio_pop_rx_byte(&drop); ++k) {
            /* Drain synchronous echo from MN103S SC0 */
        }
    }
}

void picofw_fast_pio_tx_paced_bin(const uint8_t *buf, size_t len) {
    for (size_t i = 0u; i < len; ++i) {
        picofw_wdt_kick();
        picofw_pio_tx_bytes(&buf[i], 1u, false);
        sleep_us(150u);
    }
}

bool picofw_fast_stub_ping_open(void) {
    /* Send '!' first: PANAMON1> echoes '!', whereas our 0x9043A000 stub replies '?'. */
    picofw_pio_drain_rx();
    const uint8_t probe = (uint8_t)'!';
    picofw_pio_tx_bytes(&probe, 1u, false);
    uint8_t reply = 0u;
    if (!picofw_fast_pio_read_exact_ms(&reply, 1u, 120u) || (reply != (uint8_t)'?')) {
        return false;
    }
    const uint8_t p = (uint8_t)'P';
    picofw_pio_tx_bytes(&p, 1u, false);
    uint8_t pong = 0u;
    return picofw_fast_pio_read_exact_ms(&pong, 1u, 120u) && (pong == (uint8_t)'P');
}

bool picofw_fast_stub_exit_open(void) {
    picofw_pio_drain_rx();
    const uint8_t q = (uint8_t)'Q';
    picofw_pio_tx_bytes(&q, 1u, false);
    uint8_t rx[96];
    size_t len = 0u;
    const uint64_t t0 = time_us_64();
    const uint64_t deadline = t0 + 400000ULL;
    bool aborted = false;
    while (picofw_wdt_loop_step(t0, deadline, true, &aborted)) {
        if (picofw_hw_is_nsw5v_fault_latched()) {
            return false;
        }
        len = picofw_pio_pop_rx_bounded(rx, sizeof(rx), len, true);
        if (picofw_bus_panamon1_has_prompt(rx, len)) {
            picofw_bus_panamon1_drain_tail(2u);
            return true;
        }
    }
    return false;
}

bool picofw_fast_stub_jump_and_ping_open(void) {
    char cmd[24];
    snprintf(cmd, sizeof(cmd), "J %08lX\r", (unsigned long)PICOFW_FAST_STUB_LOAD_VA);
    picofw_fast_pio_tx_paced_ascii(cmd);
    sleep_ms(30u);
    picofw_pio_drain_rx();
    return picofw_fast_stub_ping_open();
}

bool picofw_fast_stub_upload_and_enter_open(bool *out_uploaded) {
    if (out_uploaded != NULL) {
        *out_uploaded = false;
    }
    bool pm_ok = false;
    if (!picofw_bus_panamon1_probe_open(true, &pm_ok)) {
        /* If PANAMON1> did not answer ?\r, check if our stub is already active. */
        return picofw_fast_stub_ping_open();
    }
    const bool already_loaded = picofw_bus_panamon1_buf_matches_open(
        PICOFW_FAST_STUB_LOAD_VA, k_picofw_fast_bgo_stub, PICOFW_FAST_STUB_LEN, true, &pm_ok
    );
    if (!already_loaded) {
        for (size_t off = 0u; off < PICOFW_FAST_STUB_LEN; off += 64u) {
            const size_t remain = PICOFW_FAST_STUB_LEN - off;
            const size_t chunk = (remain > 64u) ? 64u : remain;
            if (!picofw_bus_panamon1_write_buf_open(
                    PICOFW_FAST_STUB_LOAD_VA + (uint32_t)off,
                    &k_picofw_fast_bgo_stub[off],
                    chunk,
                    true,
                    &pm_ok
                )) {
                return false;
            }
            picofw_fast_emit_prog_push(
                "STUB", 0u, (uint32_t)(off + chunk), PICOFW_FAST_STUB_LEN, PICOFW_FAST_STUB_CRC32
            );
        }
        if (out_uploaded != NULL) {
            *out_uploaded = true;
        }
    }
    return picofw_fast_stub_jump_and_ping_open();
}

bool picofw_fast_stub_read_page_open(uint32_t page_va, uint8_t *out256, bool *out_blank) {
    uint8_t pkt[7];
    (void)picofw_fast_encode_read_pkt(page_va, 1u, pkt);
    picofw_fast_pio_tx_paced_bin(pkt, sizeof(pkt));

    uint8_t tag = 0xFFu;
    if (!picofw_fast_pio_read_exact_ms(&tag, 1u, 300u)) {
        return false;
    }
    if (tag == 0x00u) {
        memset(out256, 0xFF, PICOFW_FAST_STUB_PAGE_BYTES);
        if (out_blank != NULL) {
            *out_blank = true;
        }
        return true;
    }
    if (tag != 0x01u) {
        return false;
    }
    uint8_t rx_sum = 0u;
    if (!picofw_fast_pio_read_exact_ms(out256, PICOFW_FAST_STUB_PAGE_BYTES, 400u) ||
        !picofw_fast_pio_read_exact_ms(&rx_sum, 1u, 150u)) {
        return false;
    }
    if (picofw_fast_sum8(out256, PICOFW_FAST_STUB_PAGE_BYTES) != rx_sum) {
        return false;
    }
    if (out_blank != NULL) {
        *out_blank = false;
    }
    return true;
}
