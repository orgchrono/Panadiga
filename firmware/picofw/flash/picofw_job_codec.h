/**
 * @file picofw_job_codec.h
 * @brief Pure ISO C11 Atomic Probe Job parser and safety policy validator (`JOB`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 * Strictly enforces:
 * - Explicit `CONFIRM` on `NVRAM_CLEAN` and `BGO_APPLY`
 * - `F14` fail-closed wireless NOR guard (`picofw_wireless_nor_write_allowed`)
 * - Target profile read-only & `DMR-ES35V` `MAIN_XIP` / `SA0..SA10` IPL protections (`E05`)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_JOB_CODEC_H
#define PICOFW_JOB_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "flash/picofw_flash.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum picofw_job_kind {
    PICOFW_JOB_STATUS = 0,
    PICOFW_JOB_NVRAM_CLEAN = 1,
    PICOFW_JOB_BGO_APPLY = 2,
    PICOFW_JOB_VAULT_BACKUP = 3
} picofw_job_kind_t;

typedef struct picofw_job_spec {
    picofw_job_kind_t kind;
    bool confirmed;
    uint8_t vault_slot;
} picofw_job_spec_t;

/** Parse `JOB [STATUS|NVRAM_CLEAN CONFIRM|BGO_APPLY CONFIRM|VAULT_BACKUP [1..3]]`. */
bool picofw_job_parse_args(const char *arg, picofw_job_spec_t *out_spec);

/**
 * Validate whether `spec` is permitted under active session & wireless F14 policy.
 * When blocked, sets `*out_err_code = "E05"` and populates `*out_reason`.
 */
bool picofw_job_check_policy(
    const picofw_job_spec_t *spec,
    const picofw_session_ctx_t *sess,
    const picofw_wireless_auth_state_t *wireless,
    const char **out_err_code,
    const char **out_reason
);

/** Return canonical job kind name. */
const char *picofw_job_kind_name(picofw_job_kind_t kind);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_JOB_CODEC_H */
