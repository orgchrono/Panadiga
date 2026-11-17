/**
 * @file p7501_uart_codec.h
 * @brief Portable ISO C11 UART argument, hex, baudrate, and pattern-trigger helpers.
 *
 * Compiled identically on ARM/RISC-V Pico targets and x86_64 CI hosts (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_UART_CODEC_H
#define P7501_UART_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_UART_MIN_BAUD 1200u
#define P7501_UART_DEFAULT_BAUD 115200u
#define P7501_UART_MAX_BAUD 460800u
#define P7501_UART_HEX_MAX_BYTES 128u
#define P7501_WAITTX_MAX_TRIGGER 48u

typedef enum p7501_boot_action {
    P7501_BOOT_ACTION_PASSIVE = 0,
    P7501_BOOT_ACTION_NORMAL = 1,
    P7501_BOOT_ACTION_JIG = 2
} p7501_boot_action_t;

typedef enum p7501_uart_framing {
    P7501_FRAMING_8N1 = 0,
    P7501_FRAMING_8E1 = 1,
    P7501_FRAMING_8O1 = 2
} p7501_uart_framing_t;

typedef struct p7501_waittx_spec {
    uint32_t timeout_ms;
    p7501_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    char trigger[P7501_WAITTX_MAX_TRIGGER + 1u];
    uint8_t tx_bytes[P7501_UART_HEX_MAX_BYTES];
    size_t tx_len;
    uint32_t post_rx_ms;
    bool tx_break;
} p7501_waittx_spec_t;

uint32_t p7501_parse_uint32_clamped(
    const char *arg,
    uint32_t default_val,
    uint32_t min_val,
    uint32_t max_val
);

const char *p7501_next_token(const char *cursor, char *out, size_t out_cap);

bool p7501_decode_hex_bytes(
    const char *hex,
    uint8_t *out,
    size_t max_out,
    size_t *out_len
);

bool p7501_baud_is_valid(uint32_t baud);
uint32_t p7501_baud_to_bit_ns(uint32_t baud);

void p7501_calc_pio_clkdiv_fp16(
    uint32_t sys_clk_hz,
    uint32_t baud,
    uint16_t *out_div_int,
    uint8_t *out_div_frac
);

uint32_t p7501_snap_standard_baud(uint32_t min_low_us, uint32_t *out_raw_bps);

p7501_boot_action_t p7501_parse_boot_action(const char *token);

bool p7501_rolling_match_push(
    uint8_t *ring,
    size_t *ring_len,
    size_t cap,
    uint8_t byte,
    const char *needle
);

bool p7501_parse_waittx_args(const char *arg, p7501_waittx_spec_t *out);

#include "common/p7501_hex.h"

/** Parsed capture window + boot stimulation specification (`UARTSTREAM`/`HEXCAP`/`PWRTRACE`/`BOOTTRACE`/`BAUDSCAN`). */
typedef struct p7501_window_boot_spec {
    uint32_t window_ms;
    p7501_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    bool sticky_jig;
    bool no_xreset;
    bool wait_for_wake;
    uint32_t wake_timeout_ms;
} p7501_window_boot_spec_t;

/** Parsed `BRIDGE [baud] [PASSIVE|NORMAL|JIG[:ms]|NOPULL] [CTRL]` specification. */
typedef struct p7501_bridge_spec {
    uint32_t baud;
    p7501_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    bool log_ctrl_edges;
} p7501_bridge_spec_t;

/** Parse boot action (`NORMAL`, `JIG[:ms]`, `PREJIG[:ms]`, `NOPULL`) with XRESET-skip support. */
p7501_boot_action_t p7501_parse_boot_spec_ex(
    const char *token,
    uint32_t *out_jig_hold_ms,
    bool *out_no_xreset
);

/** Parse boot action (`NORMAL`, `JIG`, `JIG:<hold_ms>`, `NOPULL`) with optional timed SW_UART release. */
p7501_boot_action_t p7501_parse_boot_spec(const char *token, uint32_t *out_jig_hold_ms);

/** Parse optional `WAKE` or `WAKE:<ms>` standby trigger token cleanly (`[1000..WDT_MAX]` ms). */
bool p7501_parse_wake_arg_token(const char *token, bool *out_wait, uint32_t *out_timeout_ms);

/** Parse standard `<ms> [NORMAL|JIG[:ms]|PREJIG[:ms]|NOPULL] [STICKY] [WAKE[:ms]]` arguments. */
void p7501_parse_window_boot_args(
    const char *arg,
    uint32_t default_ms,
    uint32_t min_ms,
    uint32_t max_ms,
    p7501_window_boot_spec_t *out_spec
);

/** Parse `BRIDGE` arguments supporting optional baud, boot mode, and `CTRL` edge telemetry flag. */
void p7501_parse_bridge_args(
    const char *arg,
    uint32_t default_baud,
    p7501_bridge_spec_t *out_spec
);

bool p7501_is_boot_spec_token(const char *token);

bool p7501_parse_framing(const char *token, p7501_uart_framing_t *out);
const char *p7501_framing_name(p7501_uart_framing_t framing);
uint8_t p7501_uart_even_parity_bit(uint8_t byte);
uint32_t p7501_uart_encode_tx_word(uint8_t byte, p7501_uart_framing_t framing);
void p7501_uart_decode_rx_word(
    uint32_t raw_isr_msb,
    p7501_uart_framing_t framing,
    uint8_t *out_byte,
    bool *out_parity_ok
);

/** Software `TW7501` (`SW_UART` GP26) open-drain strap mode without pulsing `XRESET`. */
typedef enum p7501_jigpin_mode {
    P7501_JIGPIN_STATUS = 0,
    P7501_JIGPIN_LOW    = 1,
    P7501_JIGPIN_HIZ    = 2
} p7501_jigpin_mode_t;

/** Parse `JIGPIN [LOW|HIZ|STATUS]` argument cleanly (rejects unrecognized tokens). */
bool p7501_parse_jigpin_arg(const char *arg, p7501_jigpin_mode_t *out_mode);

/** Classify P7501 Standby vs Awake vs Clamped `TW7501` vs Unplugged target state. */
const char *p7501_classify_standby_state(
    float vdd_v,
    float sw_pu_v,
    bool nsw5v_high,
    bool txd_high,
    bool jig_latched
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_UART_CODEC_H */
