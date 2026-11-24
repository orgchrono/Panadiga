/**
 * @file p7501_tbus_codec.h
 * @brief Pure C11 Panasonic Timer-MCU T-BUS frame decoder & M0 keepalive codec (#14).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_TBUS_CODEC_H
#define P7501_TBUS_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_TBUS_SYNC_BYTE          0xA5u
#define P7501_TBUS_MAX_PAYLOAD        16u
#define P7501_M0_KEEPALIVE_DEFAULT_MS 5000u
#define P7501_M0_KEEPALIVE_MIN_MS     1000u
#define P7501_M0_KEEPALIVE_MAX_MS     30000u

typedef enum p7501_m0_keepalive_action {
    P7501_M0_KA_ON = 0,
    P7501_M0_KA_OFF = 1,
    P7501_M0_KA_STATUS = 2
} p7501_m0_keepalive_action_t;

typedef struct p7501_tbus_frame {
    uint8_t sync;
    uint8_t opcode;
    uint8_t len;
    uint8_t payload[P7501_TBUS_MAX_PAYLOAD];
    uint8_t checksum;
} p7501_tbus_frame_t;

/** Build deterministic M0/Timer-MCU keepalive ping frame with XOR checksum. */
size_t p7501_tbus_build_keepalive(uint8_t seq, uint8_t *out_buf, size_t cap);

/** Compute 8-bit XOR checksum over `[sync, opcode, len, payload...]`. */
uint8_t p7501_tbus_checksum(uint8_t opcode, const uint8_t *payload, uint8_t len);

/** Decode incoming T-BUS frame slice `[sync, opcode, len, payload..., csum]`. */
bool p7501_tbus_decode_frame(const uint8_t *raw, size_t raw_len, p7501_tbus_frame_t *out);

/** Parse `M0KEEPALIVE [ON [interval_ms]|OFF|STATUS|<interval_ms>]`. */
bool p7501_tbus_parse_keepalive_args(
    const char *arg,
    p7501_m0_keepalive_action_t *out_action,
    uint32_t *out_interval_ms
);

typedef struct p7501_tbus_audit {
    uint32_t frames_valid;
    uint32_t frames_rejected_br;
    uint32_t nak_7f_seen;
    uint8_t rejected_bitmap[32];
} p7501_tbus_audit_t;

/** Return true for the 13 European SCART/FPC opcodes rejected by Brazilian `IC7501` (`MN67788RA`). */
bool p7501_tbus_opcode_rejected_by_br_ic7501(uint8_t opcode);

/** Reset deterministic T-BUS audit counters and rejected opcode bitmap. */
void p7501_tbus_audit_init(p7501_tbus_audit_t *st);

/** Record a decoded T-BUS frame (direct EU opcode or `0x7F <OP>` NAK) into `st`. */
void p7501_tbus_audit_record_frame(p7501_tbus_audit_t *st, const p7501_tbus_frame_t *frame);

/** Format `@P7501_TBUS_AUDIT frames=... br_rejected=... nak7f=... ops=<CSV|NONE>`. */
size_t p7501_tbus_audit_format_summary(const p7501_tbus_audit_t *st, char *out, size_t cap);

void p7501_cmd_tbussniff(const char *arg);
void p7501_cmd_m0keepalive(const char *arg);
void p7501_tbus_keepalive_poll(void);

#ifdef __cplusplus
}
#endif

#endif /* P7501_TBUS_CODEC_H */
