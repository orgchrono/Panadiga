/**
 * @file p7501_fast_stub_io.c
 * @brief PIO1 paced I/O, `@P7501_PROG` push emitter, and `0x9043A000` stub lifecycle transport.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_fast_stub_io.h"
#include "common/p7501_hex.h"
#include "flash/p7501_fast_stream_codec.h"
#include "flash/p7501_flash.h"
#include "flash/p7501_target_bus_panamon.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_uart_pio_engine.h"

#include "pico/stdlib.h"
#include "pico/time.h"

#include <stdio.h>
#include <string.h>

void p7501_fast_emit_prog_push(
    const char *phase,
    uint16_t sa,
    uint32_t done,
    uint32_t total,
    uint32_t crc32
) {
    char line[128];
    const uint32_t vdd_mv = p7501_hw_read_vdd_mv();
    if (p7501_fast_format_prog_line(line, sizeof(line), phase, sa, done, total, crc32, vdd_mv) > 0u) {
        printf("%s\n", line);
        fflush(stdout);
    }
}

bool p7501_fast_pio_read_exact_ms(uint8_t *dst, size_t n, uint32_t timeout_ms) {
    const uint64_t t0 = time_us_64();
    const uint64_t deadline = t0 + ((uint64_t)timeout_ms * 1000ULL);
    size_t got = 0u;
    bool aborted = false;
    while ((got < n) && p7501_wdt_loop_step(t0, deadline, true, &aborted)) {
        if (p7501_hw_is_nsw5v_fault_latched()) {
            return false;
        }
        uint8_t b = 0u;
        while ((got < n) && p7501_pio_pop_rx_byte(&b)) {
            dst[got++] = b;
        }
    }
    return (!aborted) && (got == n);
}

void p7501_fast_pio_tx_paced_ascii(const char *str) {
    for (size_t i = 0u; str[i] != '\0'; ++i) {
        p7501_wdt_kick();
        p7501_pio_tx_bytes((const uint8_t *)&str[i], 1u, false);
        sleep_us(350u);
        uint8_t drop = 0u;
        while (p7501_pio_pop_rx_byte(&drop)) {
            /* Drain synchronous echo from MN103S SC0 */
        }
    }
}

void p7501_fast_pio_tx_paced_bin(const uint8_t *buf, size_t len) {
    for (size_t i = 0u; i < len; ++i) {
        p7501_wdt_kick();
        p7501_pio_tx_bytes(&buf[i], 1u, false);
        sleep_us(150u);
    }
}

bool p7501_fast_stub_ping_open(void) {
    /* Send '!' first: PANAMON1> echoes '!', whereas our 0x9043A000 stub replies '?'. */
    p7501_pio_drain_rx();
    const uint8_t probe = (uint8_t)'!';
    p7501_pio_tx_bytes(&probe, 1u, false);
    uint8_t reply = 0u;
    if (!p7501_fast_pio_read_exact_ms(&reply, 1u, 120u) || (reply != (uint8_t)'?')) {
        return false;
    }
    const uint8_t p = (uint8_t)'P';
    p7501_pio_tx_bytes(&p, 1u, false);
    uint8_t pong = 0u;
    return p7501_fast_pio_read_exact_ms(&pong, 1u, 120u) && (pong == (uint8_t)'P');
}

bool p7501_fast_stub_exit_open(void) {
    const uint8_t q = (uint8_t)'Q';
    p7501_pio_tx_bytes(&q, 1u, false);
    uint8_t rx[96];
    size_t len = 0u;
    const uint64_t t0 = time_us_64();
    const uint64_t deadline = t0 + 400000ULL;
    bool aborted = false;
    while (p7501_wdt_loop_step(t0, deadline, true, &aborted)) {
        uint8_t b = 0u;
        while (p7501_pio_pop_rx_byte(&b)) {
            if (len < sizeof(rx)) {
                rx[len++] = b;
            }
        }
        if (p7501_bus_panamon1_has_prompt(rx, len)) {
            p7501_bus_panamon1_drain_tail(2u);
            return true;
        }
    }
    return false;
}

bool p7501_fast_stub_jump_and_ping_open(void) {
    char cmd[24];
    snprintf(cmd, sizeof(cmd), "J %08lX\r", (unsigned long)P7501_FAST_STUB_LOAD_VA);
    p7501_fast_pio_tx_paced_ascii(cmd);
    sleep_ms(30u);
    p7501_pio_drain_rx();
    return p7501_fast_stub_ping_open();
}

bool p7501_fast_stub_upload_and_enter_open(bool *out_uploaded) {
    if (out_uploaded != NULL) {
        *out_uploaded = false;
    }
    bool pm_ok = false;
    if (!p7501_bus_panamon1_probe_open(true, &pm_ok)) {
        /* If PANAMON1> did not answer ?\r, check if our stub is already active. */
        return p7501_fast_stub_ping_open();
    }
    uint8_t pfx[4] = { 0u, 0u, 0u, 0u };
    uint8_t sfx[4] = { 0u, 0u, 0u, 0u };
    const uint32_t tail_va = P7501_FAST_STUB_LOAD_VA + (uint32_t)(P7501_FAST_STUB_LEN - 4u);
    const bool already_loaded =
        p7501_bus_panamon1_read_chunk_open(P7501_FAST_STUB_LOAD_VA, pfx, 4u, true, &pm_ok) &&
        (memcmp(pfx, k_p7501_fast_bgo_stub, 4u) == 0) &&
        p7501_bus_panamon1_read_chunk_open(tail_va, sfx, 4u, true, &pm_ok) &&
        (memcmp(sfx, &k_p7501_fast_bgo_stub[P7501_FAST_STUB_LEN - 4u], 4u) == 0);
    if (!already_loaded) {
        char req[24];
        char rep[96];
        for (size_t i = 0u; i < P7501_FAST_STUB_LEN; ++i) {
            snprintf(
                req, sizeof(req), "W %08lX,%02X\r",
                (unsigned long)(P7501_FAST_STUB_LOAD_VA + (uint32_t)i),
                (unsigned)k_p7501_fast_bgo_stub[i]
            );
            if (!p7501_bus_panamon1_cmd_open(req, 250u, true, rep, sizeof(rep), &pm_ok) ||
                (strstr(rep, "OK:WRITE") == NULL)) {
                return false;
            }
            if ((((i + 1u) & 63u) == 0u) || ((i + 1u) == P7501_FAST_STUB_LEN)) {
                p7501_fast_emit_prog_push("STUB", 0u, (uint32_t)(i + 1u), P7501_FAST_STUB_LEN, P7501_FAST_STUB_CRC32);
            }
        }
        if (out_uploaded != NULL) {
            *out_uploaded = true;
        }
    }
    return p7501_fast_stub_jump_and_ping_open();
}

bool p7501_fast_stub_read_page_open(uint32_t page_va, uint8_t *out256, bool *out_blank) {
    uint8_t pkt[7];
    (void)p7501_fast_encode_read_pkt(page_va, 1u, pkt);
    p7501_fast_pio_tx_paced_bin(pkt, sizeof(pkt));

    uint8_t tag = 0xFFu;
    if (!p7501_fast_pio_read_exact_ms(&tag, 1u, 300u)) {
        return false;
    }
    if (tag == 0x00u) {
        memset(out256, 0xFF, P7501_FAST_STUB_PAGE_BYTES);
        if (out_blank != NULL) {
            *out_blank = true;
        }
        return true;
    }
    if (tag != 0x01u) {
        return false;
    }
    uint8_t rx_sum = 0u;
    if (!p7501_fast_pio_read_exact_ms(out256, P7501_FAST_STUB_PAGE_BYTES, 400u) ||
        !p7501_fast_pio_read_exact_ms(&rx_sum, 1u, 150u)) {
        return false;
    }
    if (p7501_fast_sum8(out256, P7501_FAST_STUB_PAGE_BYTES) != rx_sum) {
        return false;
    }
    if (out_blank != NULL) {
        *out_blank = false;
    }
    return true;
}
