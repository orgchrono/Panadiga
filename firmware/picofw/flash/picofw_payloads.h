/**
 * @file picofw_payloads.h
 * @brief SSOT hardware trap auditor (`TRAP`) & self-verifying MN103S SDRAM payload registry (`PAYLOAD`).
 *
 * Host-testable C11 core + Pico hardware CLI dispatchers (`@PICO_FRAME` staged).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PAYLOADS_H
#define PICOFW_PAYLOADS_H

#include "generated/picofw_ssot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct picofw_session_ctx picofw_session_ctx_t;

typedef enum picofw_trap_verdict {
    PICOFW_TRAP_ALLOW = 0,
    PICOFW_TRAP_WARN = 1,
    PICOFW_TRAP_DENY = 2
} picofw_trap_verdict_t;

typedef struct picofw_trap_audit_result {
    picofw_trap_verdict_t verdict;
    const picofw_ssot_trap_t *trap;
    uint32_t match_count;
    const char *bus_deny;
} picofw_trap_audit_result_t;

typedef enum picofw_payload_id {
    PICOFW_PAY_FAST_BGO_STREAM = 0,
    PICOFW_PAY_VIDEO_MMIO_DUMP = 1,
    PICOFW_PAY_TBC_PASSTHROUGH = 2,
    PICOFW_PAY_JEDEC_NVRAM_ERASE = 3,
    PICOFW_PAY_JEDEC_PROBE_ID = 4,
    PICOFW_PAY_COUNT = 5
} picofw_payload_id_t;

typedef struct picofw_payload_desc {
    picofw_payload_id_t id;
    const char *name;
    uint32_t load_va;
    const char *safety;
    const char *summary;
} picofw_payload_desc_t;

typedef struct picofw_payload_info {
    picofw_payload_desc_t desc;
    size_t len;
    uint32_t crc32;
    bool verified;
} picofw_payload_info_t;

/** Parse `"R"`, `"W"`, `"E"`, or combinations into `PICOFW_SSOT_TRAP_{R,W,E}` mask (`0` invalid). */
uint8_t picofw_trap_parse_op(const char *tok);

/** Audit `[addr, addr + len)` against all `k_picofw_ssot_traps` entries purely deterministically. */
bool picofw_trap_audit(
    uint32_t addr,
    uint32_t len,
    uint8_t op_mask,
    picofw_trap_audit_result_t *out
);

/** Audit `[addr, addr + len)` against both `k_picofw_ssot_traps` and `picofw_bus_check_access`. */
bool picofw_trap_check_full(
    const picofw_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    uint32_t len,
    uint8_t op_mask,
    bool upper_unlocked,
    picofw_trap_audit_result_t *out
);

/** Return number of registered embedded MN103S probe payloads (`PICOFW_PAY_COUNT`). */
size_t picofw_payload_count(void);

/** Look up descriptor by zero-based index (`0..PICOFW_PAY_COUNT-1`). */
bool picofw_payload_describe(size_t idx, picofw_payload_desc_t *out);

/** Case-insensitive payload name lookup. */
bool picofw_payload_find(const char *name, picofw_payload_id_t *out_id);

/**
 * @brief Build/load payload `id` into `out_buf` and verify size, CRC32, and trap safety.
 */
bool picofw_payload_build_and_verify(
    picofw_payload_id_t id,
    uint8_t *out_buf,
    size_t cap,
    picofw_payload_info_t *out_info
);

/** Verify all built-in payloads using `scratch[0..cap)` and populate verified bitmask. */
bool picofw_payload_verify_all(uint8_t *scratch, size_t cap, uint32_t *out_verified_mask);

/** CLI `TRAP [LIST|CHECK <addr_hex> <len_hex> <R|W|E>]`. */
void picofw_cmd_trap(const char *arg);

/** CLI `PAYLOAD [LIST|INFO <name>|VERIFY [name|ALL]|RUN <name> [CONFIRM]]`. */
void picofw_cmd_payload(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PAYLOADS_H */
