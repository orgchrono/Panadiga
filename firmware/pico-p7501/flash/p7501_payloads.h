/**
 * @file p7501_payloads.h
 * @brief SSOT hardware trap auditor (`TRAP`) & self-verifying MN103S SDRAM payload registry (`PAYLOAD`).
 *
 * Host-testable C11 core + Pico hardware CLI dispatchers (`@P7501_FRAME` staged).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PAYLOADS_H
#define P7501_PAYLOADS_H

#include "generated/p7501_ssot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct p7501_session_ctx p7501_session_ctx_t;

typedef enum p7501_trap_verdict {
    P7501_TRAP_ALLOW = 0,
    P7501_TRAP_WARN = 1,
    P7501_TRAP_DENY = 2
} p7501_trap_verdict_t;

typedef struct p7501_trap_audit_result {
    p7501_trap_verdict_t verdict;
    const p7501_ssot_trap_t *trap;
    uint32_t match_count;
    const char *bus_deny;
} p7501_trap_audit_result_t;

typedef enum p7501_payload_id {
    P7501_PAY_FAST_BGO_STREAM = 0,
    P7501_PAY_VIDEO_MMIO_DUMP = 1,
    P7501_PAY_TBC_PASSTHROUGH = 2,
    P7501_PAY_JEDEC_NVRAM_ERASE = 3,
    P7501_PAY_JEDEC_PROBE_ID = 4,
    P7501_PAY_COUNT = 5
} p7501_payload_id_t;

typedef struct p7501_payload_desc {
    p7501_payload_id_t id;
    const char *name;
    uint32_t load_va;
    const char *safety;
    const char *summary;
} p7501_payload_desc_t;

typedef struct p7501_payload_info {
    p7501_payload_desc_t desc;
    size_t len;
    uint32_t crc32;
    bool verified;
} p7501_payload_info_t;

/** Parse `"R"`, `"W"`, `"E"`, or combinations into `P7501_SSOT_TRAP_{R,W,E}` mask (`0` invalid). */
uint8_t p7501_trap_parse_op(const char *tok);

/** Audit `[addr, addr + len)` against all `k_p7501_ssot_traps` entries purely deterministically. */
bool p7501_trap_audit(
    uint32_t addr,
    uint32_t len,
    uint8_t op_mask,
    p7501_trap_audit_result_t *out
);

/** Audit `[addr, addr + len)` against both `k_p7501_ssot_traps` and `p7501_bus_check_access`. */
bool p7501_trap_check_full(
    const p7501_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    uint32_t len,
    uint8_t op_mask,
    bool upper_unlocked,
    p7501_trap_audit_result_t *out
);

/** Return number of registered embedded MN103S probe payloads (`P7501_PAY_COUNT`). */
size_t p7501_payload_count(void);

/** Look up descriptor by zero-based index (`0..P7501_PAY_COUNT-1`). */
bool p7501_payload_describe(size_t idx, p7501_payload_desc_t *out);

/** Case-insensitive payload name lookup. */
bool p7501_payload_find(const char *name, p7501_payload_id_t *out_id);

/**
 * @brief Build/load payload `id` into `out_buf` and verify size, CRC32, and trap safety.
 */
bool p7501_payload_build_and_verify(
    p7501_payload_id_t id,
    uint8_t *out_buf,
    size_t cap,
    p7501_payload_info_t *out_info
);

/** Verify all built-in payloads using `scratch[0..cap)` and populate verified bitmask. */
bool p7501_payload_verify_all(uint8_t *scratch, size_t cap, uint32_t *out_verified_mask);

/** CLI `TRAP [LIST|CHECK <addr_hex> <len_hex> <R|W|E>]`. */
void p7501_cmd_trap(const char *arg);

/** CLI `PAYLOAD [LIST|INFO <name>|VERIFY [name|ALL]|RUN <name> [CONFIRM]]`. */
void p7501_cmd_payload(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PAYLOADS_H */
