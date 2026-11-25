/**
 * @file picofw_diag_watch_codec.h
 * @brief Portable ISO C11 `DIAGWATCH` boot watchdog parser and multi-signature event FSM.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_DIAG_WATCH_CODEC_H
#define PICOFW_DIAG_WATCH_CODEC_H

#include "flash/picofw_flash.h"
#include "uart/picofw_uart_codec.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum picofw_diag_action {
    PICOFW_DIAG_ACT_AUTO = 0,
    PICOFW_DIAG_ACT_BYPASS,
    PICOFW_DIAG_ACT_READ,
    PICOFW_DIAG_ACT_WRITE,
    PICOFW_DIAG_ACT_FORMAT,
    PICOFW_DIAG_ACT_DUMP,
    PICOFW_DIAG_ACT_JUMP,
    PICOFW_DIAG_ACT_ERASE,
    PICOFW_DIAG_ACT_QUIT,
    PICOFW_DIAG_ACT_TXN,
    PICOFW_DIAG_ACT_NONE
} picofw_diag_action_t;

#define PICOFW_DIAG_TXN_MAX_STEPS 8u

enum {
    PICOFW_DIAG_EVT_DDR_OK             = (1u << 0),
    PICOFW_DIAG_EVT_SERIAL_BOOT        = (1u << 1),
    PICOFW_DIAG_EVT_SD_ERR             = (1u << 2),
    PICOFW_DIAG_EVT_ODD_OPEN           = (1u << 3),
    PICOFW_DIAG_EVT_CD_APC_ERR         = (1u << 4),
    PICOFW_DIAG_EVT_BGO_FAIL           = (1u << 5),
    PICOFW_DIAG_EVT_HUNG_U99           = (1u << 6),
    PICOFW_DIAG_EVT_PANAMON            = (1u << 7),
    PICOFW_DIAG_EVT_SD_HW_OK           = (1u << 8),
    PICOFW_DIAG_EVT_SD_DIAG_LOAD_FAIL  = (1u << 9),
    PICOFW_DIAG_EVT_SD_UNCOMPRESS_FAIL = (1u << 10),
    PICOFW_DIAG_EVT_DIAG_EXEC          = (1u << 11),
    PICOFW_DIAG_EVT_ODD_TRAY_FAIL      = (1u << 12),
    PICOFW_DIAG_EVT_FAT_FAIL           = (1u << 13)
};

#define PICOFW_DIAG_WATCH_UNLOCK_TX "U CONFIRM_UPPER_NOR\r"
#define PICOFW_DIAG_WATCH_LOCK_TX   "U LOCK\r"

typedef struct picofw_diag_watch_spec {
    uint32_t timeout_ms;
    picofw_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    picofw_diag_action_t action;
    bool release_on_ddr;
    bool no_xreset;
    bool wait_for_wake;
    bool nor_unlock;
    picofw_policy_req_t policy_req;
    uint32_t wake_timeout_ms;
    uint32_t target_addr;
    char tx_seq[32];
    uint8_t txn_count;
    char txn_steps[PICOFW_DIAG_TXN_MAX_STEPS][32];
} picofw_diag_watch_spec_t;

typedef struct picofw_diag_watch_state {
    uint8_t ring[64];
    size_t ring_len;
    uint32_t seen_events;
    bool sw_uart_released;
    bool unlock_dispatched;
    bool tx_dispatched;
    bool relock_dispatched;
    uint8_t txn_cursor;
} picofw_diag_watch_state_t;

/** Purely parse `DIAGWATCH ...` arguments (zero session state mutation). */
bool picofw_diag_watch_parse_args(const char *arg, picofw_diag_watch_spec_t *out);

/** Apply parsed NOR unlock/lock request from `DIAGWATCH` spec to active session. */
void picofw_diag_watch_apply_policy(const picofw_diag_watch_spec_t *spec);

/** Initialize a clean rolling `DIAGWATCH` state machine. */
void picofw_diag_watch_init(picofw_diag_watch_state_t *st);

/** Return single-byte `PANAMON1` command char (`'B'`/`'R'`/`'W'`/`'F'`/`'D'`/`'J'`/`'E'`/`'Q'`) or `'\0'`. */
char picofw_diag_watch_action_char(picofw_diag_action_t action);

/** Format full `PANAMON1` TX payload (single char or parameterized `<op><hex>\r`), returning byte length. */
size_t picofw_diag_watch_format_tx(const picofw_diag_watch_spec_t *spec, char *out, size_t cap);

/**
 * @brief Feed one RX byte into the `DIAGWATCH` event classifier.
 *
 * Returns newly triggered event bitmask on this byte (0 if none), sets `*out_release_sw_uart`
 * cleanly when `spec->release_on_ddr` is enabled (in `JIG` mode on `serial boot mode` or
 * normal-mode fallback so `P50` stays LOW during sampling; in `NORMAL`/`NOPULL` on `DDR CHECK: OK`),
 * and sets `*out_tx_char` once when `PANAMON1>` appears and an active TX action is configured.
 */
/** Pure RX byte classifier with explicit relock event output (`*out_relock_now`). */
uint32_t picofw_diag_watch_feed_byte_ex(
    picofw_diag_watch_state_t *st,
    const picofw_diag_watch_spec_t *spec,
    uint8_t byte,
    char *out_tx_char,
    bool *out_release_sw_uart,
    bool *out_relock_now
);

uint32_t picofw_diag_watch_feed_byte(
    picofw_diag_watch_state_t *st,
    const picofw_diag_watch_spec_t *spec,
    uint8_t byte,
    char *out_tx_char,
    bool *out_release_sw_uart
);

/** Human-readable diagnosis summary for detected `PICOFW_DIAG_EVT_*` bitmask. */
const char *picofw_diag_watch_diagnosis(uint32_t events);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_DIAG_WATCH_CODEC_H */
