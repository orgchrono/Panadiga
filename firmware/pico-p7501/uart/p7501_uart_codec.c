/**
 * @file p7501_uart_codec.c
 * @brief Pure ISO C11 UART baudrate math, PIO CLKDIV computation, and rolling trigger matching.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_uart_codec.h"
#include "p7501_types.h"

#include <limits.h>
#include <stddef.h>
#include <string.h>

bool p7501_baud_is_valid(uint32_t baud) {
    return baud >= P7501_UART_MIN_BAUD && baud <= P7501_UART_MAX_BAUD;
}

uint32_t p7501_baud_to_bit_ns(uint32_t baud) {
    if (!p7501_baud_is_valid(baud)) {
        baud = P7501_UART_DEFAULT_BAUD;
    }
    return 1000000000u / baud;
}

void p7501_calc_pio_clkdiv_fp16(
    uint32_t sys_clk_hz,
    uint32_t baud,
    uint16_t *out_div_int,
    uint8_t *out_div_frac
) {
    if (!p7501_baud_is_valid(baud)) {
        baud = P7501_UART_DEFAULT_BAUD;
    }
    const uint64_t denom = 8ULL * (uint64_t)baud;
    uint64_t scaled = (((uint64_t)sys_clk_hz) * 256ULL + (denom / 2ULL)) / denom;
    if (scaled < 256ULL) {
        scaled = 256ULL;
    }
    if (scaled > 0xFFFF00ULL) {
        scaled = 0xFFFF00ULL;
    }
    if (out_div_int) {
        *out_div_int = (uint16_t)(scaled >> 8);
    }
    if (out_div_frac) {
        *out_div_frac = (uint8_t)(scaled & 0xFFu);
    }
}

uint32_t p7501_snap_standard_baud(uint32_t min_low_us, uint32_t *out_raw_bps) {
    static const uint32_t k_rates[] = {
        460800u, 230400u, 115200u, 57600u, 38400u, 31250u, 19200u, 9600u, 4800u, 2400u, 1200u
    };
    uint32_t raw_bps = 0u;
    uint32_t best_rate = 0u;
    uint32_t best_diff = UINT32_MAX;

    if (min_low_us == 0u || min_low_us > 2000u) {
        if (out_raw_bps) {
            *out_raw_bps = 0u;
        }
        return 0u;
    }

    raw_bps = 1000000u / min_low_us;
    if (out_raw_bps) {
        *out_raw_bps = raw_bps;
    }

    for (size_t i = 0u; i < (sizeof(k_rates) / sizeof(k_rates[0])); ++i) {
        const uint32_t cand = k_rates[i];
        const uint32_t diff = (raw_bps > cand) ? (raw_bps - cand) : (cand - raw_bps);
        if ((diff * 100u) <= (cand * 16u) && diff < best_diff) {
            best_diff = diff;
            best_rate = cand;
        }
    }
    return best_rate;
}

bool p7501_rolling_match_push(
    uint8_t *ring,
    size_t *ring_len,
    size_t cap,
    uint8_t byte,
    const char *needle
) {
    const size_t nlen = needle ? strlen(needle) : 0u;

    if (!ring || !ring_len || cap == 0u || nlen == 0u || nlen > cap) {
        return false;
    }
    if (*ring_len >= cap) {
        memmove(ring, ring + 1u, cap - 1u);
        *ring_len = cap - 1u;
    }
    ring[(*ring_len)++] = byte;

    if (*ring_len < nlen) {
        return false;
    }
    return memcmp(ring + (*ring_len - nlen), needle, nlen) == 0;
}

bool p7501_parse_framing(const char *token, p7501_uart_framing_t *out) {
    if (token == NULL || out == NULL) {
        return false;
    }
    if (p7501_ascii_ieq(token, "8N1")) {
        *out = P7501_FRAMING_8N1;
        return true;
    }
    if (p7501_ascii_ieq(token, "8E1")) {
        *out = P7501_FRAMING_8E1;
        return true;
    }
    if (p7501_ascii_ieq(token, "8O1")) {
        *out = P7501_FRAMING_8O1;
        return true;
    }
    return false;
}

const char *p7501_framing_name(p7501_uart_framing_t framing) {
    switch (framing) {
        case P7501_FRAMING_8E1:
            return "8E1";
        case P7501_FRAMING_8O1:
            return "8O1";
        case P7501_FRAMING_8N1:
        default:
            return "8N1";
    }
}

uint8_t p7501_uart_even_parity_bit(uint8_t byte) {
    uint8_t x = byte;
    x ^= (uint8_t)(x >> 4u);
    x ^= (uint8_t)(x >> 2u);
    x ^= (uint8_t)(x >> 1u);
    return (uint8_t)(x & 1u);
}

uint32_t p7501_uart_encode_tx_word(uint8_t byte, p7501_uart_framing_t framing) {
    if (framing == P7501_FRAMING_8N1) {
        return (uint32_t)byte;
    }
    const uint8_t even_p = p7501_uart_even_parity_bit(byte);
    const uint8_t pbit = (framing == P7501_FRAMING_8O1) ? (uint8_t)(even_p ^ 1u) : even_p;
    return (uint32_t)byte | ((uint32_t)pbit << 8u);
}

void p7501_uart_decode_rx_word(
    uint32_t raw_isr_msb,
    p7501_uart_framing_t framing,
    uint8_t *out_byte,
    bool *out_parity_ok
) {
    if (framing == P7501_FRAMING_8N1) {
        if (out_byte != NULL) {
            *out_byte = (uint8_t)(raw_isr_msb >> 24u);
        }
        if (out_parity_ok != NULL) {
            *out_parity_ok = true;
        }
        return;
    }
    const uint8_t byte = (uint8_t)((raw_isr_msb >> 23u) & 0xFFu);
    const uint8_t rx_p = (uint8_t)((raw_isr_msb >> 31u) & 1u);
    const uint8_t exp_even = p7501_uart_even_parity_bit(byte);
    const uint8_t exp_p = (framing == P7501_FRAMING_8O1) ? (uint8_t)(exp_even ^ 1u) : exp_even;
    if (out_byte != NULL) {
        *out_byte = byte;
    }
    if (out_parity_ok != NULL) {
        *out_parity_ok = (rx_p == exp_p);
    }
}

const char *p7501_classify_standby_state_mv(
    uint32_t vdd_mv,
    uint32_t sw_pu_mv,
    bool nsw5v_high,
    bool txd_high,
    bool jig_latched
) {
    if (jig_latched) {
        return nsw5v_high
            ? "AWAKE / JIGPIN_LOW ACTIVE (TW7501=0V, NSW5V=1)"
            : "STANDBY / JIGPIN_LOW LATCHED (TW7501=0V, NSW5V=0)";
    }
    if (vdd_mv >= P7501_SSOT_VDD_FLOATING_CEIL_MV) {
        return "OVER_RANGE / FLOATING ADC";
    }
    if (vdd_mv >= P7501_SSOT_VDD_MIN_MV) {
        return "POWERED / AWAKE (SW_UART ~3.3V)";
    }
    if (nsw5v_high) {
        return "AWAKE / NSW5V HIGH (TW7501 GROUNDED OR LOW)";
    }
    if (sw_pu_mv < P7501_SSOT_VDD_MIN_MV) {
        return "STANDBY (P-OFF) OR TW7501 CLAMPED LOW (NSW5V=0)";
    }
    if (txd_high) {
        return "STANDBY WAKING / PARTIAL PULL-UP (NSW5V=0)";
    }
    return "STANDBY (P-OFF) OR P7501 CABLE UNPLUGGED (ALL 0V / HI-Z)";
}
