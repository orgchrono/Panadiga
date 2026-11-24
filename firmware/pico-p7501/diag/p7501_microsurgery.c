/**
 * @file p7501_microsurgery.c
 * @brief Pure host-testable transactional time-travel journal, RAM gate & timeout/hook synthesizer.
 *
 * (Wildcard search & bitwise engine live in `diag/p7501_bitwise_search.c`.)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_microsurgery.h"

#include "common/p7501_hex.h"
#include "flash/p7501_payloads.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

void p7501_surg_journal_init(p7501_surg_journal_t *j) {
    if (j != NULL) {
        memset(j, 0, sizeof(*j));
    }
}

bool p7501_surg_journal_push(p7501_surg_journal_t *j, const p7501_surg_entry_t *e) {
    if ((j == NULL) || (e == NULL) || (e->len > P7501_SURG_MAX_PATCH_BYTES)) {
        return false;
    }
    if (j->cursor < j->count) {
        j->count = j->cursor;
    }
    if (j->count >= P7501_SURG_JOURNAL_CAP) {
        for (size_t i = 1u; i < P7501_SURG_JOURNAL_CAP; ++i) {
            j->entries[i - 1u] = j->entries[i];
        }
        j->count = P7501_SURG_JOURNAL_CAP - 1u;
        j->cursor = j->count;
    }
    j->entries[j->count++] = *e;
    j->cursor = j->count;
    return true;
}

const p7501_surg_entry_t *p7501_surg_journal_peek_undo(const p7501_surg_journal_t *j) {
    return ((j != NULL) && (j->cursor > 0u) && (j->cursor <= j->count))
        ? &j->entries[j->cursor - 1u]
        : NULL;
}

bool p7501_surg_journal_commit_undo(p7501_surg_journal_t *j) {
    if (p7501_surg_journal_peek_undo(j) == NULL) {
        return false;
    }
    j->cursor -= 1u;
    return true;
}

const p7501_surg_entry_t *p7501_surg_journal_peek_redo(const p7501_surg_journal_t *j) {
    return ((j != NULL) && (j->cursor < j->count)) ? &j->entries[j->cursor] : NULL;
}

bool p7501_surg_journal_commit_redo(p7501_surg_journal_t *j) {
    if (p7501_surg_journal_peek_redo(j) == NULL) {
        return false;
    }
    j->cursor += 1u;
    return true;
}

size_t p7501_surg_journal_format_summary(const p7501_surg_journal_t *j, char *out, size_t cap) {
    if ((j == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    const uint8_t redo_cnt = (j->count >= j->cursor) ? (uint8_t)(j->count - j->cursor) : 0u;
    const int n = snprintf(
        out, cap, "@P7501_SURG op=LOG entries=%u cursor=%u undo=%u redo=%u",
        (unsigned)j->count, (unsigned)j->cursor, (unsigned)j->cursor, (unsigned)redo_cnt
    );
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}

static bool u64_span_contains(uint64_t r0, uint64_t r1, uint64_t a0, uint64_t a1) {
    return (r0 <= a0) && (a1 <= r1);
}

bool p7501_surg_is_live_ram_span(uint32_t va, uint32_t len) {
    if (len == 0u) {
        return false;
    }
    const uint64_t a0 = (uint64_t)va;
    const uint64_t a1 = a0 + (uint64_t)len;
    const bool in_ram = u64_span_contains(0x00000000ULL, 0x00040000ULL, a0, a1) ||
                        u64_span_contains(0x40000000ULL, 0x60000000ULL, a0, a1) ||
                        u64_span_contains(0x90000000ULL, 0xA0000000ULL, a0, a1);
    if (!in_ram) {
        return false;
    }
    p7501_trap_audit_result_t tr;
    return p7501_trap_audit(va, len, P7501_SSOT_TRAP_R | P7501_SSOT_TRAP_W, &tr) &&
           (tr.verdict == P7501_TRAP_ALLOW);
}

bool p7501_surg_plan_timeout_patch(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t new_u32,
    p7501_surg_plan_t *out
) {
    p7501_mn103_insn_info_t insn;
    if ((orig == NULL) || (out == NULL) ||
        !p7501_mn103_decode_insn(orig, orig_len, site_va, &insn) ||
        !insn.valid || (insn.imm_kind == P7501_IMM_NONE) ||
        (insn.len > P7501_SURG_MAX_PATCH_BYTES) ||
        !p7501_surg_is_live_ram_span(site_va, (uint32_t)insn.len)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->site_va = site_va;
    out->site_len = insn.len;
    memcpy(out->orig, orig, insn.len);
    memcpy(out->patch, orig, insn.len);
    if (insn.imm_kind == P7501_IMM_U8) {
        if (new_u32 > 0xFFu) {
            return false;
        }
        out->patch[insn.imm_off] = (uint8_t)new_u32;
    } else if (insn.imm_kind == P7501_IMM_U16) {
        if (new_u32 > 0xFFFFu) {
            return false;
        }
        out->patch[insn.imm_off] = (uint8_t)(new_u32 & 0xFFu);
        out->patch[insn.imm_off + 1u] = (uint8_t)((new_u32 >> 8u) & 0xFFu);
    } else {
        p7501_write_u32_le(&out->patch[insn.imm_off], new_u32);
    }
    (void)snprintf(out->summary, sizeof(out->summary), "%s -> 0x%" PRIX32, insn.mnemonic, new_u32);
    return true;
}

bool p7501_surg_plan_hook(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t tramp_va,
    p7501_hook_mode_t mode,
    uint32_t arg_u32,
    p7501_surg_plan_t *out
) {
    if ((orig == NULL) || (out == NULL)) {
        return false;
    }
    size_t cover = 0u;
    bool reloc_ok = false;
    if (!p7501_mn103_cover_span(orig, orig_len, site_va, 5u, &cover, &reloc_ok) ||
        (cover < 5u) || (cover > 12u)) {
        return false;
    }
    if (((mode != P7501_HOOK_D0_RET) && !reloc_ok) ||
        ((mode == P7501_HOOK_CALL) && (arg_u32 == 0u))) {
        return false;
    }
    if (!p7501_surg_is_live_ram_span(site_va, (uint32_t)cover) ||
        !p7501_surg_is_live_ram_span(tramp_va, P7501_SURG_MAX_TRAMP_BYTES)) {
        return false;
    }
    const uint64_t s0 = site_va, s1 = s0 + cover, t0 = tramp_va, t1 = t0 + P7501_SURG_MAX_TRAMP_BYTES;
    if ((s0 < t1) && (t0 < s1)) {
        return false;
    }

    memset(out, 0, sizeof(*out));
    out->site_va = site_va;
    out->tramp_va = tramp_va;
    out->site_len = (uint8_t)cover;
    memcpy(out->orig, orig, cover);
    out->patch[0] = 0xDCu;
    p7501_write_u32_le(&out->patch[1], (uint32_t)((int32_t)tramp_va - (int32_t)site_va));
    for (size_t i = 5u; i < cover; ++i) {
        out->patch[i] = 0xCBu;
    }

    size_t tp = 0u;
    if (mode == P7501_HOOK_CALL) {
        out->tramp[tp++] = 0xDDu;
        p7501_write_u32_le(&out->tramp[tp], (uint32_t)((int32_t)arg_u32 - (int32_t)tramp_va));
        tp += 4u;
        out->tramp[tp++] = 0x00u;
        out->tramp[tp++] = 0x00u;
    } else {
        out->tramp[tp++] = 0xFCu;
        out->tramp[tp++] = 0xCCu;
        p7501_write_u32_le(&out->tramp[tp], arg_u32);
        tp += 4u;
    }
    if (mode == P7501_HOOK_D0_RET) {
        out->tramp[tp++] = 0xF0u;
        out->tramp[tp++] = 0xFCu;
    } else {
        memcpy(&out->tramp[tp], orig, cover);
        tp += cover;
        const uint32_t jmp_back_va = tramp_va + (uint32_t)tp;
        out->tramp[tp++] = 0xDCu;
        p7501_write_u32_le(&out->tramp[tp], (uint32_t)((int32_t)(site_va + (uint32_t)cover) - (int32_t)jmp_back_va));
        tp += 4u;
    }
    out->tramp_len = (uint8_t)tp;
    (void)snprintf(out->summary, sizeof(out->summary), "cover=%u tramp_len=%u", (unsigned)cover, (unsigned)tp);
    return tp <= P7501_SURG_MAX_TRAMP_BYTES;
}
