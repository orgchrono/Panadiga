/**
 * @file p7501_trap_guard.c
 * @brief Pure host-testable SSOT hardware trap auditor & bus access policy gate (`uint64_t` `[lo, hi)`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_payloads.h"
#include "flash/p7501_bus_policy.h"

#include <string.h>

uint8_t p7501_trap_parse_op(const char *tok) {
    if ((tok == NULL) || (*tok == '\0')) {
        return 0u;
    }
    uint8_t mask = 0u;
    for (size_t i = 0u; tok[i] != '\0'; ++i) {
        char c = tok[i];
        if ((c >= 'a') && (c <= 'z')) {
            c = (char)(c - ('a' - 'A'));
        }
        if (c == 'R') {
            mask |= P7501_SSOT_TRAP_R;
        } else if (c == 'W') {
            mask |= P7501_SSOT_TRAP_W;
        } else if (c == 'E') {
            mask |= P7501_SSOT_TRAP_E;
        } else {
            return 0u;
        }
    }
    return mask;
}

bool p7501_trap_audit(
    uint32_t addr,
    uint32_t len,
    uint8_t op_mask,
    p7501_trap_audit_result_t *out
) {
    if ((out == NULL) || (op_mask == 0u)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->verdict = P7501_TRAP_ALLOW;
    out->bus_deny = "ALLOW";

    /* Half-open interval [a0, a1) in u64 prevents 32-bit wraparound bypass. */
    const uint64_t a0 = (uint64_t)addr;
    const uint64_t a1 = a0 + ((len > 0u) ? (uint64_t)len : 1ULL);

    for (size_t i = 0u; i < P7501_SSOT_TRAP_COUNT; ++i) {
        const p7501_ssot_trap_t *const tr = &k_p7501_ssot_traps[i];
        const uint64_t t0 = (uint64_t)tr->start;
        const uint64_t t1 = t0 + (uint64_t)tr->len;
        if ((a0 < t1) && (t0 < a1) && ((tr->mask_rwe & op_mask) != 0u)) {
            out->match_count += 1u;
            const p7501_trap_verdict_t v = (tr->severity >= P7501_SSOT_TRAP_SEV_DENY)
                ? P7501_TRAP_DENY
                : P7501_TRAP_WARN;
            if ((out->trap == NULL) || (v > out->verdict)) {
                out->verdict = v;
                out->trap = tr;
            }
        }
    }
    return true;
}

bool p7501_trap_check_full(
    const p7501_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    uint32_t len,
    uint8_t op_mask,
    bool upper_unlocked,
    p7501_trap_audit_result_t *out
) {
    if (!p7501_trap_audit(addr, len, op_mask, out)) {
        return false;
    }
    const bool is_write = (op_mask & (P7501_SSOT_TRAP_W | P7501_SSOT_TRAP_E)) != 0u;
    const bool is_erase = (op_mask & P7501_SSOT_TRAP_E) != 0u;
    const p7501_bus_deny_t deny = p7501_bus_check_access(
        ctx,
        flash_size_bytes,
        addr,
        (len > 0u) ? (size_t)len : 1u,
        is_write,
        is_erase,
        upper_unlocked
    );
    out->bus_deny = p7501_bus_deny_name(deny);
    if (deny != P7501_BUS_ALLOW) {
        out->verdict = P7501_TRAP_DENY;
    }
    return true;
}
