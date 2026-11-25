/**
 * @file picofw_tbus_codec.h
 * @brief Pure C11 Panasonic Timer-MCU T-BUS frame decoder & M0 keepalive codec (#14).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_TBUS_CODEC_H
#define PICOFW_TBUS_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_TBUS_SYNC_BYTE          0xA5u
#define PICOFW_TBUS_MAX_PAYLOAD        16u
#define PICOFW_M0_KEEPALIVE_DEFAULT_MS 5000u
#define PICOFW_M0_KEEPALIVE_MIN_MS     1000u
#define PICOFW_M0_KEEPALIVE_MAX_MS     30000u

typedef enum picofw_m0_keepalive_action {
    PICOFW_M0_KA_ON = 0,
    PICOFW_M0_KA_OFF = 1,
    PICOFW_M0_KA_STATUS = 2
} picofw_m0_keepalive_action_t;

typedef struct picofw_tbus_frame {
    uint8_t sync;
    uint8_t opcode;
    uint8_t len;
    uint8_t payload[PICOFW_TBUS_MAX_PAYLOAD];
    uint8_t checksum;
} picofw_tbus_frame_t;

/** Build deterministic M0/Timer-MCU keepalive ping frame with XOR checksum. */
size_t picofw_tbus_build_keepalive(uint8_t seq, uint8_t *out_buf, size_t cap);

/** Compute 8-bit XOR checksum over `[sync, opcode, len, payload...]`. */
uint8_t picofw_tbus_checksum(uint8_t opcode, const uint8_t *payload, uint8_t len);

/** Decode incoming T-BUS frame slice `[sync, opcode, len, payload..., csum]`. */
bool picofw_tbus_decode_frame(const uint8_t *raw, size_t raw_len, picofw_tbus_frame_t *out);

/** Parse `M0KEEPALIVE [ON [interval_ms]|OFF|STATUS|<interval_ms>]`. */
bool picofw_tbus_parse_keepalive_args(
    const char *arg,
    picofw_m0_keepalive_action_t *out_action,
    uint32_t *out_interval_ms
);

typedef struct picofw_tbus_audit {
    uint32_t frames_valid;
    uint32_t frames_rejected_br;
    uint32_t nak_7f_seen;
    uint8_t rejected_bitmap[32];
} picofw_tbus_audit_t;

/** Return true for the 13 European SCART/FPC opcodes rejected by Brazilian `IC7501` (`MN67788RA`). */
bool picofw_tbus_opcode_rejected_by_br_ic7501(uint8_t opcode);

/** Reset deterministic T-BUS audit counters and rejected opcode bitmap. */
void picofw_tbus_audit_init(picofw_tbus_audit_t *st);

/** Record a decoded T-BUS frame (direct EU opcode or `0x7F <OP>` NAK) into `st`. */
void picofw_tbus_audit_record_frame(picofw_tbus_audit_t *st, const picofw_tbus_frame_t *frame);

/** Format `@PICO_TBUS_AUDIT frames=... br_rejected=... nak7f=... ops=<CSV|NONE>`. */
size_t picofw_tbus_audit_format_summary(const picofw_tbus_audit_t *st, char *out, size_t cap);

void picofw_cmd_tbussniff(const char *arg);
void picofw_cmd_m0keepalive(const char *arg);
void picofw_tbus_keepalive_poll(void);
void picofw_tbus_keepalive_stop(void);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_TBUS_CODEC_H */
