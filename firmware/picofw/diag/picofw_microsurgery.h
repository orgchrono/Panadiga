/**
 * @file picofw_microsurgery.h
 * @brief Pure host-testable runtime microsurgery, wildcard search, bitwise algebra & time-travel journal.
 *
 * Strictly restricts all mutable live patching & hook trampolines to AM34 SRAM/SDRAM
 * via `uint64_t` `[lo, hi)` interval math and `picofw_trap_audit`. Zero floats, zero malloc.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_MICROSURGERY_H
#define PICOFW_MICROSURGERY_H

#include "diag/picofw_math_gps.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_SURG_MAX_PATTERN     16u
#define PICOFW_SURG_JOURNAL_CAP     24u
#define PICOFW_SURG_MAX_PATCH_BYTES 16u
#define PICOFW_SURG_MAX_TRAMP_BYTES 32u

typedef enum picofw_bit_op {
    PICOFW_BIT_INFO = 0,
    PICOFW_BIT_SET = 1,
    PICOFW_BIT_CLR = 2,
    PICOFW_BIT_TOG = 3,
    PICOFW_BIT_FIELD = 4,
    PICOFW_BIT_EXT = 5,
    PICOFW_BIT_POPCNT = 6,
    PICOFW_BIT_CLZ = 7,
    PICOFW_BIT_CTZ = 8,
    PICOFW_BIT_BSWAP16 = 9,
    PICOFW_BIT_BSWAP32 = 10,
    PICOFW_BIT_ROTL = 11,
    PICOFW_BIT_ROTR = 12
} picofw_bit_op_t;

typedef enum picofw_hook_mode {
    PICOFW_HOOK_D0_RET = 0,
    PICOFW_HOOK_D0_CONT = 1,
    PICOFW_HOOK_CALL = 2
} picofw_hook_mode_t;

typedef enum picofw_surg_entry_kind {
    PICOFW_SURG_MEM_PATCH = 0,
    PICOFW_SURG_PC_JUMP = 1,
    PICOFW_SURG_HOOK = 2
} picofw_surg_entry_kind_t;

typedef struct picofw_surg_entry {
    uint8_t kind;
    uint8_t len;
    uint32_t site_va;
    uint32_t aux_va;
    uint32_t pc_before;
    uint32_t pc_after;
    uint64_t ts_us;
    uint8_t before[PICOFW_SURG_MAX_PATCH_BYTES];
    uint8_t after[PICOFW_SURG_MAX_PATCH_BYTES];
    char tag[20];
} picofw_surg_entry_t;

typedef struct picofw_surg_journal {
    picofw_surg_entry_t entries[PICOFW_SURG_JOURNAL_CAP];
    uint8_t count;
    uint8_t cursor;
} picofw_surg_journal_t;

typedef struct picofw_surg_plan {
    uint32_t site_va;
    uint32_t tramp_va;
    uint8_t site_len;
    uint8_t tramp_len;
    uint8_t orig[PICOFW_SURG_MAX_PATCH_BYTES];
    uint8_t patch[PICOFW_SURG_MAX_PATCH_BYTES];
    uint8_t tramp[PICOFW_SURG_MAX_TRAMP_BYTES];
    char summary[64];
} picofw_surg_plan_t;

/* --- Wildcard byte pattern search --- */
bool picofw_surg_parse_wildcard_hex(
    const char *str,
    uint8_t pat[PICOFW_SURG_MAX_PATTERN],
    uint8_t mask[PICOFW_SURG_MAX_PATTERN],
    size_t *out_len
);
size_t picofw_surg_search_buf(
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
bool picofw_bit_parse_op(const char *tok, picofw_bit_op_t *out_op);
bool picofw_bit_eval(
    picofw_bit_op_t op,
    uint32_t val,
    uint32_t a1,
    uint32_t a2,
    uint32_t a3,
    uint32_t *out_val
);

/* --- Time-travel journal --- */
void picofw_surg_journal_init(picofw_surg_journal_t *j);
bool picofw_surg_journal_push(picofw_surg_journal_t *j, const picofw_surg_entry_t *e);
const picofw_surg_entry_t *picofw_surg_journal_peek_undo(const picofw_surg_journal_t *j);
bool picofw_surg_journal_commit_undo(picofw_surg_journal_t *j);
const picofw_surg_entry_t *picofw_surg_journal_peek_redo(const picofw_surg_journal_t *j);
bool picofw_surg_journal_commit_redo(picofw_surg_journal_t *j);
size_t picofw_surg_journal_format_summary(const picofw_surg_journal_t *j, char *out, size_t cap);

/* --- RAM safety gate & deterministic microsurgery planners --- */
bool picofw_surg_is_live_ram_span(uint32_t va, uint32_t len);
bool picofw_surg_plan_timeout_patch(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t new_u32,
    picofw_surg_plan_t *out
);
bool picofw_surg_plan_hook(
    uint32_t site_va,
    const uint8_t *orig,
    size_t orig_len,
    uint32_t tramp_va,
    picofw_hook_mode_t mode,
    uint32_t arg_u32,
    picofw_surg_plan_t *out
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_MICROSURGERY_H */
