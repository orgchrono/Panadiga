/**
 * @file picofw_microsurgery.c
 * @brief Pure host-testable transactional time-travel journal, RAM gate & timeout/hook synthesizer.
 *
 * (Wildcard search & bitwise engine live in `diag/picofw_bitwise_search.c`.)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_microsurgery.h"

#include "common/picofw_hex.h"
#include "flash/picofw_payloads.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

void picofw_surg_journal_init(picofw_surg_journal_t *j) {
    if (j != NULL) {
        memset(j, 0, sizeof(*j));
    }
}

bool picofw_surg_journal_push(picofw_surg_journal_t *j, const picofw_surg_entry_t *e) {
    if ((j == NULL) || (e == NULL) || (e->len > PICOFW_SURG_MAX_PATCH_BYTES)) {
        return false;
    }
    if (j->cursor < j->count) {
        j->count = j->cursor;
    }
    if (j->count >= PICOFW_SURG_JOURNAL_CAP) {
        for (size_t i = 1u; i < PICOFW_SURG_JOURNAL_CAP; ++i) {
            j->entries[i - 1u] = j->entries[i];
        }
        j->count = PICOFW_SURG_JOURNAL_CAP - 1u;
        j->cursor = j->count;
    }
    j->entries[j->count++] = *e;
    j->cursor = j->count;
    return true;
}

const picofw_surg_entry_t *picofw_surg_journal_peek_undo(const picofw_surg_journal_t *j) {
    return ((j != NULL) && (j->cursor > 0u) && (j->cursor <= j->count))
        ? &j->entries[j->cursor - 1u]
        : NULL;
}

bool picofw_surg_journal_commit_undo(picofw_surg_journal_t *j) {
    if (picofw_surg_journal_peek_undo(j) == NULL) {
        return false;
    }
    j->cursor -= 1u;
    return true;
}

const picofw_surg_entry_t *picofw_surg_journal_peek_redo(const picofw_surg_journal_t *j) {
    return ((j != NULL) && (j->cursor < j->count)) ? &j->entries[j->cursor] : NULL;
}

bool picofw_surg_journal_commit_redo(picofw_surg_journal_t *j) {
    if (picofw_surg_journal_peek_redo(j) == NULL) {
        return false;
    }
    j->cursor += 1u;
    return true;
}

size_t picofw_surg_journal_format_summary(const picofw_surg_journal_t *j, char *out, size_t cap) {
    if ((j == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    const uint8_t redo_cnt = (j->count >= j->cursor) ? (uint8_t)(j->count - j->cursor) : 0u;
    const int n = snprintf(
        out, cap, "@PICO_SURG op=LOG entries=%u cursor=%u undo=%u redo=%u",
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

bool picofw_surg_is_live_ram_span(uint32_t va, uint32_t len) {
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
    picofw_trap_audit_result_t tr;
    return picofw_trap_audit(va, len, PICOFW_SSOT_TRAP_R | PICOFW_SSOT_TRAP_W, &tr) &&
           (tr.verdict == PICOFW_TRAP_ALLOW);
}

bool picofw_surg_plan_timeout_patch(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t new_u32,
    picofw_surg_plan_t *out
) {
    picofw_mn103_insn_info_t insn;
    if ((orig == NULL) || (out == NULL) ||
        !picofw_mn103_decode_insn(orig, orig_len, site_va, &insn) ||
        !insn.valid || (insn.imm_kind == PICOFW_IMM_NONE) ||
        (insn.len > PICOFW_SURG_MAX_PATCH_BYTES) ||
        !picofw_surg_is_live_ram_span(site_va, (uint32_t)insn.len)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->site_va = site_va;
    out->site_len = insn.len;
    memcpy(out->orig, orig, insn.len);
    memcpy(out->patch, orig, insn.len);
    if (insn.imm_kind == PICOFW_IMM_U8) {
        if (new_u32 > 0xFFu) {
            return false;
        }
        out->patch[insn.imm_off] = (uint8_t)new_u32;
    } else if (insn.imm_kind == PICOFW_IMM_U16) {
        if (new_u32 > 0xFFFFu) {
            return false;
        }
        out->patch[insn.imm_off] = (uint8_t)(new_u32 & 0xFFu);
        out->patch[insn.imm_off + 1u] = (uint8_t)((new_u32 >> 8u) & 0xFFu);
    } else {
        picofw_write_u32_le(&out->patch[insn.imm_off], new_u32);
    }
    (void)snprintf(out->summary, sizeof(out->summary), "%s -> 0x%" PRIX32, insn.mnemonic, new_u32);
    return true;
}

bool picofw_surg_plan_hook(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t tramp_va,
    picofw_hook_mode_t mode,
    uint32_t arg_u32,
    picofw_surg_plan_t *out
) {
    if ((orig == NULL) || (out == NULL)) {
        return false;
    }
    size_t cover = 0u;
    bool reloc_ok = false;
    if (!picofw_mn103_cover_span(orig, orig_len, site_va, 5u, &cover, &reloc_ok) ||
        (cover < 5u) || (cover > 12u)) {
        return false;
    }
    if (((mode != PICOFW_HOOK_D0_RET) && !reloc_ok) ||
        ((mode == PICOFW_HOOK_CALL) && (arg_u32 == 0u))) {
        return false;
    }
    if (!picofw_surg_is_live_ram_span(site_va, (uint32_t)cover) ||
        !picofw_surg_is_live_ram_span(tramp_va, PICOFW_SURG_MAX_TRAMP_BYTES)) {
        return false;
    }
    const uint64_t s0 = site_va, s1 = s0 + cover, t0 = tramp_va, t1 = t0 + PICOFW_SURG_MAX_TRAMP_BYTES;
    if ((s0 < t1) && (t0 < s1)) {
        return false;
    }

    memset(out, 0, sizeof(*out));
    out->site_va = site_va;
    out->tramp_va = tramp_va;
    out->site_len = (uint8_t)cover;
    memcpy(out->orig, orig, cover);
    out->patch[0] = 0xDCu;
    picofw_write_u32_le(&out->patch[1], (uint32_t)((int32_t)tramp_va - (int32_t)site_va));
    for (size_t i = 5u; i < cover; ++i) {
        out->patch[i] = 0xCBu;
    }

    size_t tp = 0u;
    if (mode == PICOFW_HOOK_CALL) {
        out->tramp[tp++] = 0xDDu;
        picofw_write_u32_le(&out->tramp[tp], (uint32_t)((int32_t)arg_u32 - (int32_t)tramp_va));
        tp += 4u;
        out->tramp[tp++] = 0x00u;
        out->tramp[tp++] = 0x00u;
    } else {
        out->tramp[tp++] = 0xFCu;
        out->tramp[tp++] = 0xCCu;
        picofw_write_u32_le(&out->tramp[tp], arg_u32);
        tp += 4u;
    }
    if (mode == PICOFW_HOOK_D0_RET) {
        out->tramp[tp++] = 0xF0u;
        out->tramp[tp++] = 0xFCu;
    } else {
        memcpy(&out->tramp[tp], orig, cover);
        tp += cover;
        const uint32_t jmp_back_va = tramp_va + (uint32_t)tp;
        out->tramp[tp++] = 0xDCu;
        picofw_write_u32_le(&out->tramp[tp], (uint32_t)((int32_t)(site_va + (uint32_t)cover) - (int32_t)jmp_back_va));
        tp += 4u;
    }
    out->tramp_len = (uint8_t)tp;
    (void)snprintf(out->summary, sizeof(out->summary), "cover=%u tramp_len=%u", (unsigned)cover, (unsigned)tp);
    return tp <= PICOFW_SURG_MAX_TRAMP_BYTES;
}
