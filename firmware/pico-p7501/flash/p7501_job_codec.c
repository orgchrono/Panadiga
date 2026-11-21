/**
 * @file p7501_job_codec.c
 * @brief Pure ISO C11 implementation of `JOB` parsing and E05/F14 policy validation.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_job_codec.h"
#include "common/p7501_hex.h"
#include "flash/p7501_vault_codec.h"
#include "uart/p7501_uart_codec.h"

#include <string.h>

bool p7501_job_parse_args(const char *arg, p7501_job_spec_t *out_spec) {
    if (out_spec == NULL) {
        return false;
    }
    memset(out_spec, 0, sizeof(*out_spec));
    out_spec->kind = P7501_JOB_STATUS;
    out_spec->vault_slot = 1u;

    char tok1[24] = {0};
    char tok2[24] = {0};
    const char *cur = (arg != NULL) ? p7501_next_token(arg, tok1, sizeof(tok1)) : NULL;
    if ((cur == NULL) || (tok1[0] == '\0') || p7501_ascii_ieq(tok1, "STATUS")) {
        out_spec->kind = P7501_JOB_STATUS;
        return true;
    }
    (void)p7501_next_token(cur, tok2, sizeof(tok2));

    if (p7501_ascii_ieq(tok1, "NVRAM_CLEAN")) {
        out_spec->kind = P7501_JOB_NVRAM_CLEAN;
        out_spec->confirmed = p7501_ascii_ieq(tok2, "CONFIRM");
        return true;
    }
    if (p7501_ascii_ieq(tok1, "BGO_APPLY")) {
        out_spec->kind = P7501_JOB_BGO_APPLY;
        out_spec->confirmed = p7501_ascii_ieq(tok2, "CONFIRM");
        return true;
    }
    if (p7501_ascii_ieq(tok1, "VAULT_BACKUP")) {
        out_spec->kind = P7501_JOB_VAULT_BACKUP;
        out_spec->confirmed = true;
        if (tok2[0] != '\0') {
            uint32_t slot = 0u;
            if (!p7501_parse_u32_strict(tok2, &slot) || (slot == 0u) ||
                (slot > P7501_VAULT_PAYLOAD_SLOTS)) {
                return false;
            }
            out_spec->vault_slot = (uint8_t)slot;
        }
        return true;
    }
    return false;
}

bool p7501_job_check_policy(
    const p7501_job_spec_t *spec,
    const p7501_session_ctx_t *sess,
    const p7501_wireless_auth_state_t *wireless,
    const char **out_err_code,
    const char **out_reason
) {
    const char *code = "0";
    const char *reason = "OK";
    if (spec == NULL) {
        code = "E22";
        reason = "NULL_SPEC";
        if (out_err_code != NULL) *out_err_code = code;
        if (out_reason != NULL) *out_reason = reason;
        return false;
    }
    if ((spec->kind == P7501_JOB_STATUS) || (spec->kind == P7501_JOB_VAULT_BACKUP)) {
        if (out_err_code != NULL) *out_err_code = code;
        if (out_reason != NULL) *out_reason = reason;
        return true;
    }

    if (!spec->confirmed) {
        code = "E05";
        reason = "CONFIRM_REQUIRED";
        if (out_err_code != NULL) *out_err_code = code;
        if (out_reason != NULL) *out_reason = reason;
        return false;
    }

    const p7501_wireless_auth_state_t *w =
        (wireless != NULL) ? wireless : p7501_wireless_global_state();
    if (!p7501_wireless_nor_write_allowed(w)) {
        code = "E05";
        reason = "WIRELESS_NOR_LOCKED_F14";
        if (out_err_code != NULL) *out_err_code = code;
        if (out_reason != NULL) *out_reason = reason;
        return false;
    }

    const p7501_session_ctx_t *s = (sess != NULL) ? sess : p7501_session_default_ctx();
    const p7501_ssot_target_profile_t *prof =
        ((size_t)s->active_profile < P7501_TARGET_PROFILE_COUNT)
            ? &k_p7501_ssot_target_profiles[s->active_profile]
            : &k_p7501_ssot_target_profiles[P7501_TARGET_PROFILE_DMR_EH55];

    if (prof->full_nor_readonly || !prof->writable_nvram) {
        code = "E05";
        reason = "PROFILE_NOR_READONLY";
        if (out_err_code != NULL) *out_err_code = code;
        if (out_reason != NULL) *out_reason = reason;
        return false;
    }

    if (spec->kind == P7501_JOB_NVRAM_CLEAN) {
        const p7501_flash_geometry_t geo = p7501_flash_get_geometry(s->active_flash_size);
        if (p7501_flash_is_write_blocked_in_session(
                s, s->active_flash_size, geo.hw_nvram_base, geo.nvram_size, false)) {
            code = "E05";
            reason = "NVRAM_RANGE_BLOCKED";
            if (out_err_code != NULL) *out_err_code = code;
            if (out_reason != NULL) *out_reason = reason;
            return false;
        }
    } else if (spec->kind == P7501_JOB_BGO_APPLY) {
        if (!prof->upper_nor_unlock_allowed || (prof->bgo_size == 0u)) {
            code = "E05";
            reason = "UPPER_NOR_BGO_NOT_SUPPORTED";
            if (out_err_code != NULL) *out_err_code = code;
            if (out_reason != NULL) *out_reason = reason;
            return false;
        }
        const uint32_t bgo_va = P7501_FLASH_HW_UNCACHED_BASE + prof->bgo_offset;
        if (p7501_flash_is_write_blocked_in_session(
                s, s->active_flash_size, bgo_va, prof->bgo_size, true)) {
            code = "E05";
            reason = "BGO_RANGE_OVERLAPS_PROTECTED_RO";
            if (out_err_code != NULL) *out_err_code = code;
            if (out_reason != NULL) *out_reason = reason;
            return false;
        }
    }

    if (out_err_code != NULL) *out_err_code = code;
    if (out_reason != NULL) *out_reason = reason;
    return true;
}

const char *p7501_job_kind_name(p7501_job_kind_t kind) {
    switch (kind) {
        case P7501_JOB_NVRAM_CLEAN:
            return "NVRAM_CLEAN";
        case P7501_JOB_BGO_APPLY:
            return "BGO_APPLY";
        case P7501_JOB_VAULT_BACKUP:
            return "VAULT_BACKUP";
        case P7501_JOB_STATUS:
        default:
            return "STATUS";
    }
}
