/**
 * @file p7501_diag_watch_codec.h
 * @brief Portable ISO C11 `DIAGWATCH` boot watchdog parser and multi-signature event FSM.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_DIAG_WATCH_CODEC_H
#define P7501_DIAG_WATCH_CODEC_H

#include "uart/p7501_uart_codec.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum p7501_diag_action {
    P7501_DIAG_ACT_AUTO = 0,
    P7501_DIAG_ACT_BYPASS,
    P7501_DIAG_ACT_READ,
    P7501_DIAG_ACT_WRITE,
    P7501_DIAG_ACT_FORMAT,
    P7501_DIAG_ACT_DUMP,
    P7501_DIAG_ACT_JUMP,
    P7501_DIAG_ACT_ERASE,
    P7501_DIAG_ACT_QUIT,
    P7501_DIAG_ACT_NONE
} p7501_diag_action_t;

enum {
    P7501_DIAG_EVT_DDR_OK      = (1u << 0),
    P7501_DIAG_EVT_SERIAL_BOOT = (1u << 1),
    P7501_DIAG_EVT_SD_ERR      = (1u << 2),
    P7501_DIAG_EVT_ODD_OPEN    = (1u << 3),
    P7501_DIAG_EVT_CD_APC_ERR  = (1u << 4),
    P7501_DIAG_EVT_BGO_FAIL    = (1u << 5),
    P7501_DIAG_EVT_HUNG_U99    = (1u << 6),
    P7501_DIAG_EVT_PANAMON     = (1u << 7)
};

typedef struct p7501_diag_watch_spec {
    uint32_t timeout_ms;
    p7501_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    p7501_diag_action_t action;
    bool release_on_ddr;
    char tx_seq[32];
} p7501_diag_watch_spec_t;

typedef struct p7501_diag_watch_state {
    uint8_t ring[64];
    size_t ring_len;
    uint32_t seen_events;
    bool sw_uart_released;
    bool tx_dispatched;
} p7501_diag_watch_state_t;

/** Parse `DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] [AUTO|BYPASS|READ[:a[,n]]|WRITE:<a,v>|FORMAT|DUMP|JUMP:<a>|ERASE:<a_nvram>|QUIT|NONE] [RELEASE|HOLD]`. */
bool p7501_diag_watch_parse_args(const char *arg, p7501_diag_watch_spec_t *out);

/** Initialize a clean rolling `DIAGWATCH` state machine. */
void p7501_diag_watch_init(p7501_diag_watch_state_t *st);

/** Return single-byte `PANAMON1` command char (`'B'`/`'R'`/`'W'`/`'F'`/`'D'`/`'J'`/`'E'`/`'Q'`) or `'\0'`. */
char p7501_diag_watch_action_char(p7501_diag_action_t action);

/** Format full `PANAMON1` TX payload (single char or parameterized `<op><hex>\r`), returning byte length. */
size_t p7501_diag_watch_format_tx(const p7501_diag_watch_spec_t *spec, char *out, size_t cap);

/**
 * @brief Feed one RX byte into the `DIAGWATCH` event classifier.
 *
 * Returns newly triggered event bitmask on this byte (0 if none), sets `*out_release_sw_uart`
 * cleanly when `spec->release_on_ddr` is enabled (in `JIG` mode on `serial boot mode` or
 * normal-mode fallback so `P50` stays LOW during sampling; in `NORMAL`/`NOPULL` on `DDR CHECK: OK`),
 * and sets `*out_tx_char` once when `PANAMON1>` appears and an active TX action is configured.
 */
uint32_t p7501_diag_watch_feed_byte(
    p7501_diag_watch_state_t *st,
    const p7501_diag_watch_spec_t *spec,
    uint8_t byte,
    char *out_tx_char,
    bool *out_release_sw_uart
);

/** Human-readable diagnosis summary for detected `P7501_DIAG_EVT_*` bitmask. */
const char *p7501_diag_watch_diagnosis(uint32_t events);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DIAG_WATCH_CODEC_H */
