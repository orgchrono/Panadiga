/**
 * @file p7501_pana_rsp.h
 * @brief Panasonic AM34 `_dev_SERIAL` RSP client/probe (`FUN_574f0590` / `FUN_574f0775`).
 *
 * Implements pure C11 8-bit RSP frame construction/verification and PIO1 UART probe
 * sequences (`0x15` NAK, `0x0E 0x04 0x02` 3-byte state handshake from Rust SSOT `p7501_pana_rsp_ssot.h`,
 * `$g#67`, `$Z0#8A`, `$Z1#8B`, `$m<addr>,<len>#XX`, `$x<addr>,<len>#XX`, `$M<addr>,4:<val>#XX`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PANA_RSP_H
#define P7501_PANA_RSP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "uart/p7501_pana_rsp_ssot.h"
#include "uart/p7501_uart_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_RSP_MAX_FRAME    256u
#define P7501_RSP_MAX_TRIG     P7501_WAITTX_MAX_TRIGGER
#define P7501_RSP_MAX_RAW_TX   64u

/** Verified 3-byte UART state-machine entry sequence (`0 -> 2 -> 4`, paired with `0x15` NAK). */
extern const uint8_t k_p7501_rsp_handshake[P7501_RSP_HANDSHAKE_LEN];

typedef struct p7501_rsp_frame_info {
    bool found;
    bool ack_plus;
    bool nak_minus;
    bool raw_nak_0x15;
    bool checksum_ok;
    uint8_t expected_csum;
    uint8_t actual_csum;
    size_t payload_offset;
    size_t payload_len;
} p7501_rsp_frame_info_t;

typedef enum p7501_pana_rsp_action {
    PANA_RSP_ACT_SEQ = 0,
    PANA_RSP_ACT_G,
    PANA_RSP_ACT_Z0,
    PANA_RSP_ACT_Z1,
    PANA_RSP_ACT_MEM,
    PANA_RSP_ACT_XMEM,
    PANA_RSP_ACT_NAK,
    PANA_RSP_ACT_MAGIC,
    PANA_RSP_ACT_RAW,
    PANA_RSP_ACT_HEX,
    PANA_RSP_ACT_WMEM,
    PANA_RSP_ACT_WRITE32,
    PANA_RSP_ACT_PASSTHROUGH,
    PANA_RSP_ACT_CALL,
    PANA_RSP_ACT_DUMPNOR,
    PANA_RSP_ACT_REGS,
    PANA_RSP_ACT_NOR_POLICY,
    PANA_RSP_ACT_JEDEC_ERASE,
    PANA_RSP_ACT_JEDEC_PROG
} p7501_pana_rsp_action_t;

typedef struct p7501_pana_rsp_spec {
    uint32_t timeout_ms;
    p7501_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    bool send_nak_prefix;
    bool send_magic_prefix;
    bool wait_trigger_enabled;
    bool nor_unlocked_effective;
    char trigger[P7501_RSP_MAX_TRIG + 1u];
    p7501_pana_rsp_action_t action;
    char payload[128];
    uint8_t raw_bytes[P7501_RSP_MAX_RAW_TX];
    size_t raw_len;
    uint32_t addr;
    uint32_t value_u32;
    uint8_t data_bytes[16];
    size_t data_len;
} p7501_pana_rsp_spec_t;

/** Format `<payload>` as `$<payload>#<xx>` (lowercase hex checksum); returns byte count or 0 on overflow. */
size_t p7501_rsp_build_frame(const char *payload, char *out, size_t out_cap);

/** Format a raw byte slice write as `$M<addr>,<len>:<hex>#<xx>`. */
size_t p7501_rsp_build_write_mem(
    uint32_t addr,
    const uint8_t *data,
    size_t len,
    char *out,
    size_t out_cap
);

/** Format a 32-bit little-endian word write as `$M<addr>,4:<hex>#<xx>`. */
size_t p7501_rsp_build_write_u32(uint32_t addr, uint32_t value_le, char *out, size_t out_cap);

/** Inspect RX buffer from `start_offset` for `$<payload>#<xx>` RSP reply and validate 8-bit sum. */
bool p7501_rsp_inspect_frame_from(
    const uint8_t *buf,
    size_t len,
    size_t start_offset,
    p7501_rsp_frame_info_t *out
);

/** Inspect RX buffer from index 0 for `$<payload>#<xx>` RSP reply (plus `+`/`-` or raw `0x15`). */
bool p7501_rsp_inspect_frame(const uint8_t *buf, size_t len, p7501_rsp_frame_info_t *out);

/** Decode 48-byte MN103S GDB `$g` register reply (96 ASCII hex chars -> 12 LE u32s: D0..D3, A0..A3, MDR, PSW, PC, SP). */
bool p7501_rsp_decode_mn103_regs(const uint8_t *payload, size_t len, uint32_t out_regs[12]);

/** Parse `PANARSP [ms] [NORMAL|JIG[:ms]|NOPULL] [MAGIC] [NONAK|NAK] [WAIT:<trig>] <action>` args. */
bool p7501_panarsp_parse_args(const char *arg, p7501_pana_rsp_spec_t *out);

/** CLI command `PANARSP`: actively probes MAIN `FUN_574f0590`/`FUN_574f0775` RSP stub over PIO1. */
void p7501_cmd_pana_rsp(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_RSP_H */
