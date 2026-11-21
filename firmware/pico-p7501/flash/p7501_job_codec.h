/**
 * @file p7501_job_codec.h
 * @brief Pure ISO C11 Atomic Probe Job parser and safety policy validator (`JOB`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 * Strictly enforces:
 * - Explicit `CONFIRM` on `NVRAM_CLEAN` and `BGO_APPLY`
 * - `F14` fail-closed wireless NOR guard (`p7501_wireless_nor_write_allowed`)
 * - Target profile read-only & `DMR-ES35V` `MAIN_XIP` / `SA0..SA10` IPL protections (`E05`)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_JOB_CODEC_H
#define P7501_JOB_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "flash/p7501_flash.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum p7501_job_kind {
    P7501_JOB_STATUS = 0,
    P7501_JOB_NVRAM_CLEAN = 1,
    P7501_JOB_BGO_APPLY = 2,
    P7501_JOB_VAULT_BACKUP = 3
} p7501_job_kind_t;

typedef struct p7501_job_spec {
    p7501_job_kind_t kind;
    bool confirmed;
    uint8_t vault_slot;
} p7501_job_spec_t;

/** Parse `JOB [STATUS|NVRAM_CLEAN CONFIRM|BGO_APPLY CONFIRM|VAULT_BACKUP [1..3]]`. */
bool p7501_job_parse_args(const char *arg, p7501_job_spec_t *out_spec);

/**
 * Validate whether `spec` is permitted under active session & wireless F14 policy.
 * When blocked, sets `*out_err_code = "E05"` and populates `*out_reason`.
 */
bool p7501_job_check_policy(
    const p7501_job_spec_t *spec,
    const p7501_session_ctx_t *sess,
    const p7501_wireless_auth_state_t *wireless,
    const char **out_err_code,
    const char **out_reason
);

/** Return canonical job kind name. */
const char *p7501_job_kind_name(p7501_job_kind_t kind);

#ifdef __cplusplus
}
#endif

#endif /* P7501_JOB_CODEC_H */
