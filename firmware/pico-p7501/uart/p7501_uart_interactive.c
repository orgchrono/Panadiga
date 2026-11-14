/**
 * @file p7501_uart_interactive.c
 * @brief Interactive P7501 PIO1 TX/RX and BREAK probes (`UARTSEND`, `UARTPROBE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_interactive.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "uart/p7501_uart_bitbang.h"
#include "uart/p7501_uart_capture.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pico/stdlib.h"
#include "pico/time.h"
#include "hardware/gpio.h"

void p7501_cmd_uart_send(const char *arg) {
    /* Default to SW_UART=Hi-Z so Q7510 releases XFWE and Q7514 conducts TX toward AM34.
     * Prefix payload with "LOW:" to intentionally probe Bootrom while holding SW_UART=LOW. */
    bool sw_uart_out_low = false;
    const char *payload = (arg != NULL) ? arg : "";
    if ((payload[0] == 'L') && (payload[1] == 'O') && (payload[2] == 'W') && (payload[3] == ':')) {
        sw_uart_out_low = true;
        payload += 4;
    }

    char tx_str[128];
    if (*payload != '\0') {
        snprintf(tx_str, sizeof(tx_str), "%s\r\n", payload);
    } else {
        snprintf(tx_str, sizeof(tx_str), "\r\n");
    }

    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
    p7501_pio_drain_rx();
    p7501_pio_tx_bytes((const uint8_t *)tx_str, strlen(tx_str), sw_uart_out_low);

    uint8_t *rx_buf = p7501_uart_rx_buffer(NULL);
    const uint32_t rx_len = p7501_uart_capture_window(2500000UL, 4096U);
    const uint32_t perr = p7501_pio_get_and_clear_parity_errors();
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf(
        "\n=== UARTSEND PIO1 REPLY (%" PRIu32 " bytes, %s @ %" PRIu32 ", SW_UART=%s, perr=%" PRIu32 ") ===\n",
        rx_len,
        p7501_framing_name(p7501_pio_get_framing()),
        p7501_pio_get_baud(),
        sw_uart_out_low ? "LOW" : "Hi-Z",
        perr
    );
    p7501_uart_print_ascii_stream(rx_buf, rx_len);
    printf("\nUARTSEND: OK\n");
}

void p7501_cmd_uart_probe(const char *arg) {
    const uint32_t pmode = (arg != NULL && *arg != '\0') ? (uint32_t)atoi(arg) : 5U;
    const bool sw_uart_low = ((pmode == 5U) || (pmode == 6U) || (pmode == 7U));
    const bool start_sdata_low = ((pmode == 7U) || (pmode == 8U) || (pmode == 2U));

    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    /* Start PIO1 RX passively first so XRESET release edges and earliest bytes never drop. */
    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
    p7501_pio_set_sw_uart(sw_uart_low);
    if (start_sdata_low) {
        gpio_init(P7501_PIN_TXD);
        gpio_put(P7501_PIN_TXD, false);
        gpio_set_dir(P7501_PIN_TXD, GPIO_OUT);
    }
    sleep_ms(5U);
    p7501_pio_pulse_xreset_ms(15U);

    size_t rx_cap = 0U;
    uint8_t *rx_buf = p7501_uart_rx_buffer(&rx_cap);
    uint32_t rx_len = 0U;
    uint32_t ferr = 0U;
    uint8_t ring[24];
    size_t ring_len = 0U;
    bool sw_released = !sw_uart_low;
    uint32_t sw_rel_ms = 0U;
    bool break_released = !start_sdata_low;
    uint8_t probe_idx = 0U;
    static const uint8_t probe_chars[8] = { '\r', '\n', ' ', '?', 0x1BU, 0x03U, 'd', '\r' };

    const uint64_t t0_us = time_us_64();
    const absolute_time_t end_t = make_timeout_time_ms(4500U);
    absolute_time_t next_probe_t = make_timeout_time_ms(150U);

    while (!time_reached(end_t)) {
        const uint32_t elapsed_ms = (uint32_t)((time_us_64() - t0_us) / 1000ULL);
        if (!sw_released && elapsed_ms >= 350U) {
            p7501_pio_release_sw_uart_now();
            sw_released = true;
            sw_rel_ms = elapsed_ms;
        }
        if (!break_released && elapsed_ms >= 550U) {
            gpio_set_dir(P7501_PIN_TXD, GPIO_IN);
            gpio_pull_up(P7501_PIN_TXD);
            break_released = true;
        }
        if ((pmode == 6U) && sw_released && time_reached(next_probe_t)) {
            const uint8_t ch = probe_chars[probe_idx % 8U];
            p7501_pio_tx_bytes(&ch, 1U, false);
            probe_idx++;
            next_probe_t = make_timeout_time_ms(80U);
        }
        if (p7501_pio_poll_and_clear_framing_err()) {
            ++ferr;
        }
        uint8_t byte = 0U;
        while (p7501_pio_pop_rx_byte(&byte)) {
            if (!sw_released &&
                p7501_rolling_match_push(ring, &ring_len, sizeof(ring), byte, "serial boot mode")) {
                p7501_pio_release_sw_uart_now();
                sw_released = true;
                sw_rel_ms = (uint32_t)((time_us_64() - t0_us) / 1000ULL);
            }
            if (rx_len < (rx_cap - 1U) && rx_len < 8192U) {
                rx_buf[rx_len++] = byte;
            }
        }
        p7501_led_tick();
    }

    const uint32_t perr = p7501_pio_get_and_clear_parity_errors();
    const bool ovf = p7501_pio_had_rx_overflow();
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);

    printf("\n=== UARTPROBE PIO1 MODE %" PRIu32 " (bytes=%" PRIu32 ", sw_rel=%" PRIu32 "ms, ferr=%" PRIu32 ", perr=%" PRIu32 ", ovf=%s) ===\n",
           pmode, rx_len, sw_rel_ms, ferr, perr, ovf ? "YES" : "NO");
    p7501_uart_print_ascii_stream(rx_buf, rx_len);
    printf("\nUARTPROBE: OK\n");
}
