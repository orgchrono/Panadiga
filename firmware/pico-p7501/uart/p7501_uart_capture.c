/**
 * @file p7501_uart_capture.c
 * @brief Shared 16 KB P7501 RX buffer and PIO1 `BOOTCAP`/`UARTMON`/`JIGBOOT` implementation.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_capture.h"
#include "hal/p7501_led.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_uart_bitbang.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>
#include "pico/stdlib.h"
#include "pico/time.h"

/** Single shared RX capture buffer (16 KB) reused across all UART/FMU probes. */
static uint8_t s_p7501_rx_buf[16384];

uint8_t *p7501_uart_rx_buffer(size_t *out_capacity) {
    if (out_capacity != NULL) {
        *out_capacity = sizeof(s_p7501_rx_buf);
    }
    return s_p7501_rx_buf;
}

bool p7501_uart_poll_capture_byte(bool *prev_high, uint32_t *rx_len, uint32_t max_cap) {
    uint8_t rx_byte = 0U;
    if (!p7501_uart_try_sample_byte(prev_high, &rx_byte, NULL)) {
        return false;
    }
    if (*rx_len < max_cap) {
        s_p7501_rx_buf[(*rx_len)++] = rx_byte;
    }
    return true;
}

uint32_t p7501_uart_capture_window(uint32_t dur_us, uint32_t max_cap) {
    uint32_t rx_len = 0U;
    const absolute_time_t end_t = make_timeout_time_us(dur_us);
    uint8_t rx_byte = 0U;
    while (!time_reached(end_t)) {
        p7501_wdt_kick();
        (void)p7501_pio_poll_and_clear_framing_err();
        while (p7501_pio_pop_rx_byte(&rx_byte)) {
            if (rx_len < max_cap) {
                s_p7501_rx_buf[rx_len++] = rx_byte;
            }
        }
        tight_loop_contents();
    }
    return rx_len;
}

void p7501_cmd_boot_capture(const char *arg, bool do_reset, bool is_jig, const char *cmd_name) {
    uint32_t duration_ms = 7500U;
    uint32_t jig_hold_ms = 0U;
    char tok1[24] = {0};
    char tok2[24] = {0};
    const char *cur = p7501_next_token(arg, tok1, sizeof(tok1));
    if (tok1[0] != '\0') {
        duration_ms = p7501_parse_uint32_clamped(tok1, 7500U, 100U, 30000U);
    }
    if (cur != NULL && p7501_next_token(cur, tok2, sizeof(tok2)) != NULL) {
        jig_hold_ms = p7501_parse_uint32_clamped(tok2, 0U, 0U, 30000U);
    }
    if (is_jig && jig_hold_ms == 0U) {
        jig_hold_ms = (duration_ms > 2500U) ? 2500U : duration_ms;
    }

    const p7501_boot_action_t action = is_jig
        ? P7501_BOOT_ACTION_JIG
        : (do_reset ? P7501_BOOT_ACTION_NORMAL : P7501_BOOT_ACTION_PASSIVE);

    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session_ex(action, jig_hold_ms);

    const absolute_time_t end_t = make_timeout_time_ms(duration_ms);
    uint8_t ring[24];
    size_t ring_len = 0U;
    bool sw_rel_seen = false;
    uint32_t rx_len = 0U;
    uint32_t ferr = 0U;
    uint8_t byte = 0U;

    while (!time_reached(end_t)) {
        p7501_wdt_kick();
        if (p7501_pio_poll_sw_uart_release()) {
            sw_rel_seen = true;
        }
        if (p7501_pio_poll_and_clear_framing_err()) {
            ++ferr;
        }
        while (p7501_pio_pop_rx_byte(&byte)) {
            if (is_jig && !sw_rel_seen &&
                p7501_pio_poll_jig_serial_release(ring, &ring_len, sizeof(ring), byte)) {
                sw_rel_seen = true;
            }
            if (rx_len < (uint32_t)(sizeof(s_p7501_rx_buf) - 1U)) {
                s_p7501_rx_buf[rx_len++] = byte;
            }
        }
    }

    const uint32_t perr = p7501_pio_get_and_clear_parity_errors();
    const bool ovf = p7501_pio_had_rx_overflow();
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf(
        "\n=== %s: PIO1 %" PRIu32 " %s UART CAPTURE ON GP2 (%" PRIu32 " bytes, %" PRIu32 " ms, ferr=%" PRIu32 ", perr=%" PRIu32 ", ovf=%s%s) ===\n",
        cmd_name,
        p7501_pio_get_baud(),
        p7501_framing_name(p7501_pio_get_framing()),
        rx_len,
        duration_ms,
        ferr,
        perr,
        ovf ? "YES" : "NO",
        is_jig ? (sw_rel_seen ? ", sw_rel=YES" : ", sw_rel=END") : ""
    );
    p7501_uart_print_ascii_stream(s_p7501_rx_buf, rx_len);
    printf("\n%s: OK (Bus kept in Hi-Z with pull-up so target continues running)\n", cmd_name);
}
