/**
 * @file p7501_microsurgery.h
 * @brief Pure host-testable runtime microsurgery, wildcard search, bitwise algebra & time-travel journal.
 *
 * Strictly restricts all mutable live patching & hook trampolines to AM34 SRAM/SDRAM
 * via `uint64_t` `[lo, hi)` interval math and `p7501_trap_audit`. Zero floats, zero malloc.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_MICROSURGERY_H
#define P7501_MICROSURGERY_H

#include "diag/p7501_math_gps.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_SURG_MAX_PATTERN     16u
#define P7501_SURG_JOURNAL_CAP     24u
#define P7501_SURG_MAX_PATCH_BYTES 16u
#define P7501_SURG_MAX_TRAMP_BYTES 32u

typedef enum p7501_bit_op {
    P7501_BIT_INFO = 0,
    P7501_BIT_SET = 1,
    P7501_BIT_CLR = 2,
    P7501_BIT_TOG = 3,
    P7501_BIT_FIELD = 4,
    P7501_BIT_EXT = 5,
    P7501_BIT_POPCNT = 6,
    P7501_BIT_CLZ = 7,
    P7501_BIT_CTZ = 8,
    P7501_BIT_BSWAP16 = 9,
    P7501_BIT_BSWAP32 = 10,
    P7501_BIT_ROTL = 11,
    P7501_BIT_ROTR = 12
} p7501_bit_op_t;

typedef enum p7501_hook_mode {
    P7501_HOOK_D0_RET = 0,
    P7501_HOOK_D0_CONT = 1,
    P7501_HOOK_CALL = 2
} p7501_hook_mode_t;

typedef enum p7501_surg_entry_kind {
    P7501_SURG_MEM_PATCH = 0,
    P7501_SURG_PC_JUMP = 1,
    P7501_SURG_HOOK = 2
} p7501_surg_entry_kind_t;

typedef struct p7501_surg_entry {
    uint8_t kind;
    uint8_t len;
    uint32_t site_va;
    uint32_t aux_va;
    uint32_t pc_before;
    uint32_t pc_after;
    uint64_t ts_us;
    uint8_t before[P7501_SURG_MAX_PATCH_BYTES];
    uint8_t after[P7501_SURG_MAX_PATCH_BYTES];
    char tag[20];
} p7501_surg_entry_t;

typedef struct p7501_surg_journal {
    p7501_surg_entry_t entries[P7501_SURG_JOURNAL_CAP];
    uint8_t count;
    uint8_t cursor;
} p7501_surg_journal_t;

typedef struct p7501_surg_plan {
    uint32_t site_va;
    uint32_t tramp_va;
    uint8_t site_len;
    uint8_t tramp_len;
    uint8_t orig[P7501_SURG_MAX_PATCH_BYTES];
    uint8_t patch[P7501_SURG_MAX_PATCH_BYTES];
    uint8_t tramp[P7501_SURG_MAX_TRAMP_BYTES];
    char summary[64];
} p7501_surg_plan_t;

/* --- Wildcard byte pattern search --- */
bool p7501_surg_parse_wildcard_hex(
    const char *str,
    uint8_t pat[P7501_SURG_MAX_PATTERN],
    uint8_t mask[P7501_SURG_MAX_PATTERN],
    size_t *out_len
);
size_t p7501_surg_search_buf(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    const uint8_t *pat,
    const uint8_t *mask,
    size_t plen,
    uint32_t *out_vas,
    size_t max_hits
);

/* --- UB-free bitwise algebra --- */
bool p7501_bit_parse_op(const char *tok, p7501_bit_op_t *out_op);
bool p7501_bit_eval(
    p7501_bit_op_t op,
    uint32_t val,
    uint32_t a1,
    uint32_t a2,
    uint32_t a3,
    uint32_t *out_val
);

/* --- Time-travel journal --- */
void p7501_surg_journal_init(p7501_surg_journal_t *j);
bool p7501_surg_journal_push(p7501_surg_journal_t *j, const p7501_surg_entry_t *e);
const p7501_surg_entry_t *p7501_surg_journal_peek_undo(const p7501_surg_journal_t *j);
bool p7501_surg_journal_commit_undo(p7501_surg_journal_t *j);
const p7501_surg_entry_t *p7501_surg_journal_peek_redo(const p7501_surg_journal_t *j);
bool p7501_surg_journal_commit_redo(p7501_surg_journal_t *j);
size_t p7501_surg_journal_format_summary(const p7501_surg_journal_t *j, char *out, size_t cap);

/* --- RAM safety gate & deterministic microsurgery planners --- */
bool p7501_surg_is_live_ram_span(uint32_t va, uint32_t len);
bool p7501_surg_plan_timeout_patch(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t new_u32,
    p7501_surg_plan_t *out
);
bool p7501_surg_plan_hook(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t tramp_va,
    p7501_hook_mode_t mode,
    uint32_t arg_u32,
    p7501_surg_plan_t *out
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_MICROSURGERY_H */
