/**
 * @file p7501_surg_live.c
 * @brief Live AM34 SDRAM/CPU microsurgery executor, time-travel controller & RTSD HDD unlocker.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_surg_internal.h"

#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "diag/p7501_gps_cli.h"
#include "diag/p7501_hdd_unlock.h"
#include "hal/p7501_hw.h"
#include "uart/p7501_pana_dumpnor.h"
#include "uart/p7501_uart_pio_engine.h"

#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static p7501_surg_journal_t s_journal;
static bool s_journal_init = false;
static uint32_t s_shadow_pc = 0u;

p7501_surg_journal_t *p7501_surg_global_journal(void) {
    if (!s_journal_init) {
        p7501_surg_journal_init(&s_journal);
        s_journal_init = true;
    }
    return &s_journal;
}

static bool ram_write_verified(uint32_t va, const uint8_t *data, size_t len) {
    if (!p7501_surg_is_live_ram_span(va, (uint32_t)len) || !p7501_hw_is_target_online()) {
        return false;
    }
    const uint32_t start_w = va & ~3UL;
    const uint32_t end_w = (va + (uint32_t)len + 3UL) & ~3UL;
    for (uint32_t waddr = start_w; waddr < end_w; waddr += 4UL) {
        uint32_t word = 0u;
        if (!p7501_hw_read_mem32_checked(waddr, &word)) {
            return false;
        }
        for (uint32_t b = 0u; b < 4u; ++b) {
            const uint32_t baddr = waddr + b;
            if ((baddr >= va) && (baddr < (va + (uint32_t)len))) {
                const uint32_t sh = b * 8u;
                word = (word & ~(0xFFUL << sh)) | ((uint32_t)data[baddr - va] << sh);
            }
        }
        p7501_hw_write_mem32(waddr, word);
        uint32_t rb = 0u;
        if (!p7501_hw_read_mem32_checked(waddr, &rb) || (rb != word)) {
            return false;
        }
    }
    return true;
}

static bool require_live_ram(uint32_t va, uint32_t len) {
    if (!p7501_surg_is_live_ram_span(va, len)) {
        p7501_cli_set_result("ERR", "E05");
        printf("SURG: DENIED (0x%08" PRIX32 "+0x%" PRIX32 " outside mutable AM34 RAM/SDRAM)\n", va, len);
        return false;
    }
    if (!p7501_hw_is_target_online()) {
        p7501_cli_set_result("ERR", "EOFFLINE");
        printf("SURG: ERROR (target monitor offline)\n");
        return false;
    }
    return true;
}

static void record_mem_entry(
    uint8_t kind,
    const char *tag,
    uint32_t site_va,
    uint32_t aux_va,
    const uint8_t *before,
    const uint8_t *after,
    size_t len
) {
    p7501_surg_entry_t e;
    memset(&e, 0, sizeof(e));
    e.kind = kind;
    e.len = (uint8_t)len;
    e.site_va = site_va;
    e.aux_va = aux_va;
    e.ts_us = time_us_64();
    memcpy(e.before, before, len);
    memcpy(e.after, after, len);
    (void)snprintf(e.tag, sizeof(e.tag), "%s", tag);
    (void)p7501_surg_journal_push(p7501_surg_global_journal(), &e);
}

static bool apply_journal_step(p7501_surg_journal_t *j, bool undo) {
    const p7501_surg_entry_t *const e = undo
        ? p7501_surg_journal_peek_undo(j)
        : p7501_surg_journal_peek_redo(j);
    if (e == NULL) {
        return false;
    }
    if ((e->kind == P7501_SURG_MEM_PATCH) || (e->kind == P7501_SURG_HOOK)) {
        const uint8_t *const data = undo ? e->before : e->after;
        if (!ram_write_verified(e->site_va, data, e->len)) {
            return false;
        }
    } else if (e->kind == P7501_SURG_PC_JUMP) {
        const uint32_t npc = undo ? e->pc_before : e->pc_after;
        if (!p7501_hw_is_target_online()) {
            return false;
        }
        p7501_hw_set_pc(npc);
        s_shadow_pc = npc;
    }
    return undo ? p7501_surg_journal_commit_undo(j) : p7501_surg_journal_commit_redo(j);
}

bool p7501_surg_resolve_va_or_anchor(const char *tok, uint32_t *out_va) {
    if (p7501_dumpnor_parse_hex_tok(tok, out_va)) {
        return true;
    }
    size_t idx = 0u;
    if (p7501_gps_find_anchor_by_name(tok, &idx)) {
        *out_va = p7501_gps_resolve_anchor(p7501_gps_global_env(), idx);
        return *out_va != 0u;
    }
    return false;
}

void p7501_surg_handle_bit_live(uint32_t target_va, p7501_bit_op_t bop, uint32_t a1, uint32_t a2, uint32_t a3) {
    const uint32_t waddr = target_va & ~3UL;
    uint32_t v0 = 0u, res = 0u;
    if (!require_live_ram(waddr, 4u) || !p7501_hw_read_mem32_checked(waddr, &v0)) {
        return;
    }
    if (!p7501_bit_eval(bop, v0, a1, a2, a3, &res)) {
        p7501_cli_set_result("ERR", "EINVAL");
        return;
    }
    const uint8_t b4[4] = { (uint8_t)v0, (uint8_t)(v0 >> 8u), (uint8_t)(v0 >> 16u), (uint8_t)(v0 >> 24u) };
    const uint8_t a4[4] = { (uint8_t)res, (uint8_t)(res >> 8u), (uint8_t)(res >> 16u), (uint8_t)(res >> 24u) };
    if (!ram_write_verified(waddr, a4, 4u)) {
        p7501_cli_set_result("ERR", "EVERIFY");
        return;
    }
    record_mem_entry(P7501_SURG_MEM_PATCH, "BIT_LIVE", waddr, 0u, b4, a4, 4u);
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    (void)snprintf(msg, 192u, "@P7501_SURG op=BIT live=1 in=0x%08" PRIX32 " out=0x%08" PRIX32 " xor=0x%08" PRIX32,
                   v0, res, v0 ^ res);
    p7501_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

void p7501_surg_handle_patch_timeout_hook(char *toks[], size_t n) {
    p7501_surg_plan_t *const plan = (p7501_surg_plan_t *)(void *)&g_p7501_cmd_arena.raw[4096u];
    uint8_t *const orig = g_p7501_cmd_arena.raw;
    uint32_t site_va = 0u, val = 0u;
    if (p7501_ascii_ieq(toks[0], "PATCH") && (n == 3u)) {
        uint8_t after[P7501_SURG_MAX_PATCH_BYTES], before[P7501_SURG_MAX_PATCH_BYTES];
        size_t plen = 0u;
        if (!p7501_dumpnor_parse_hex_tok(toks[1], &site_va) ||
            !p7501_decode_hex_bytes(toks[2], after, sizeof(after), &plen) || (plen == 0u) ||
            !require_live_ram(site_va, (uint32_t)plen) ||
            !p7501_hw_read_block_checked(site_va, before, (uint32_t)plen, true) ||
            !ram_write_verified(site_va, after, plen)) {
            p7501_cli_set_result("ERR", "EINVAL");
            return;
        }
        record_mem_entry(P7501_SURG_MEM_PATCH, "PATCH", site_va, 0u, before, after, plen);
    } else if (p7501_ascii_ieq(toks[0], "TIMEOUT") && (n == 3u)) {
        if (!p7501_dumpnor_parse_hex_tok(toks[1], &site_va) || !p7501_parse_u32_auto(toks[2], &val) ||
            !require_live_ram(site_va, 8u) || !p7501_hw_read_block_checked(site_va, orig, 8u, true) ||
            !p7501_surg_plan_timeout_patch(site_va, orig, 8u, val, plan) ||
            !ram_write_verified(site_va, plan->patch, plan->site_len)) {
            p7501_cli_set_result("ERR", "EINVAL");
            return;
        }
        record_mem_entry(P7501_SURG_MEM_PATCH, "TIMEOUT", site_va, 0u, plan->orig, plan->patch, plan->site_len);
    } else if (p7501_ascii_ieq(toks[0], "HOOK") && (n == 5u)) {
        uint32_t tramp_va = 0u;
        p7501_hook_mode_t mode = P7501_HOOK_D0_RET;
        if (p7501_ascii_ieq(toks[3], "D0_CONT")) { mode = P7501_HOOK_D0_CONT; }
        else if (p7501_ascii_ieq(toks[3], "CALL")) { mode = P7501_HOOK_CALL; }
        else if (!p7501_ascii_ieq(toks[3], "D0_RET")) { p7501_cli_set_result("ERR", "EINVAL"); return; }
        if (!p7501_dumpnor_parse_hex_tok(toks[1], &site_va) ||
            !p7501_dumpnor_parse_hex_tok(toks[2], &tramp_va) ||
            !p7501_surg_resolve_va_or_anchor(toks[4], &val) ||
            !require_live_ram(site_va, 16u) || !require_live_ram(tramp_va, 32u) ||
            !p7501_hw_read_block_checked(site_va, orig, 16u, true) ||
            !p7501_surg_plan_hook(site_va, orig, 16u, tramp_va, mode, val, plan) ||
            !ram_write_verified(tramp_va, plan->tramp, plan->tramp_len) ||
            !ram_write_verified(site_va, plan->patch, plan->site_len)) {
            p7501_cli_set_result("ERR", "EINVAL");
            return;
        }
        record_mem_entry(P7501_SURG_HOOK, "HOOK", site_va, tramp_va, plan->orig, plan->patch, plan->site_len);
    } else {
        p7501_cli_set_result("ERR", "EINVAL");
        return;
    }
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    (void)snprintf(msg, 192u, "@P7501_SURG op=%s site=0x%08" PRIX32 " cursor=%u",
                   toks[0], site_va, (unsigned)p7501_surg_global_journal()->cursor);
    p7501_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

void p7501_surg_handle_hdd_unlock(char *toks[], size_t n) {
    uint32_t site_va = 0u, total_lba = 0u;
    const bool apply = (n >= 5u) && p7501_ascii_ieq(toks[3], "APPLY") && p7501_ascii_ieq(toks[4], "CONFIRM");
    if ((n < 3u) || !p7501_dumpnor_parse_hex_tok(toks[1], &site_va) || !p7501_parse_u32_auto(toks[2], &total_lba) ||
        !require_live_ram(site_va, P7501_SSOT_HDD_CLAMP_WIN_LEN)) {
        return;
    }
    uint8_t *const win = g_p7501_cmd_arena.raw;
    p7501_hdd_full_plan_t *const fp = (p7501_hdd_full_plan_t *)(void *)&g_p7501_cmd_arena.raw[4096u];
    p7501_hdd_unlock_plan_t *const up = (p7501_hdd_unlock_plan_t *)(void *)&g_p7501_cmd_arena.raw[4608u];
    if (!p7501_hw_read_block_checked(site_va, win, P7501_SSOT_HDD_CLAMP_WIN_LEN, true) ||
        !p7501_hdd_compute_full_plan((uint64_t)total_lba, 0x400000UL, fp) ||
        !p7501_hdd_plan_rtsd_unlock(site_va, win, fp->target_rtsd_blocks_2kb, up)) {
        p7501_cli_set_result("ERR", "ENOENT");
        printf("SURG: ERROR (src/hdd_cmd.c clamp block not found @ 0x%08" PRIX32 ")\n", site_va);
        return;
    }
    if (apply) {
        static const char *const k_tags[4] = { "HDD_S400", "HDD_S200", "HDD_SFB", "HDD_BRA" };
        for (size_t i = 0u; i < 4u; ++i) {
            const uint32_t off = up->ops[i].va - site_va;
            if (!ram_write_verified(up->ops[i].va, up->ops[i].data, up->ops[i].len)) {
                p7501_cli_set_result("ERR", "EVERIFY");
                return;
            }
            record_mem_entry(P7501_SURG_MEM_PATCH, k_tags[i], up->ops[i].va, 0u,
                             &win[off], up->ops[i].data, up->ops[i].len);
        }
    }
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    (void)snprintf(msg, 224u, "@P7501_SURG op=HDD_UNLOCK site=0x%08" PRIX32 " applied=%u target_2kb=0x%08" PRIX32 " prev_op=0x%02X",
                   site_va, apply ? 1u : 0u, up->target_blocks_2kb, (unsigned)up->prev_branch_op);
    p7501_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

void p7501_surg_handle_time_and_cpu(char *toks[], size_t n) {
    p7501_surg_journal_t *const j = p7501_surg_global_journal();
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    if ((n == 0u) || p7501_ascii_ieq(toks[0], "LOG")) {
        (void)p7501_surg_journal_format_summary(j, msg, 192u);
        p7501_gps_emit_tag("SURG", msg);
        for (uint8_t i = 0u; i < j->count; ++i) {
            printf("  %c[%u] %s va=0x%08" PRIX32 " len=%u\n", (i + 1u == j->cursor) ? '*' : ' ',
                   (unsigned)i, j->entries[i].tag, j->entries[i].site_va, (unsigned)j->entries[i].len);
        }
        printf("SURG: OK\n");
        return;
    }
    if (p7501_ascii_ieq(toks[0], "UNDO") || p7501_ascii_ieq(toks[0], "UNHOOK") ||
        p7501_ascii_ieq(toks[0], "REDO")) {
        if (!apply_journal_step(j, !p7501_ascii_ieq(toks[0], "REDO"))) {
            p7501_cli_set_result("ERR", "ENOENT");
            return;
        }
    } else if (p7501_ascii_ieq(toks[0], "REWIND") && (n >= 2u)) {
        uint32_t target = 0u;
        if (!p7501_parse_u32_auto(toks[1], &target) || (target > j->count)) { p7501_cli_set_result("ERR", "EINVAL"); return; }
        while (j->cursor != target) {
            if (!apply_journal_step(j, j->cursor > target)) { p7501_cli_set_result("ERR", "EIO"); return; }
        }
    } else if (p7501_ascii_ieq(toks[0], "PAUSE")) {
        if (!p7501_hw_is_nsw5v_high()) { p7501_cli_set_result("ERR", "EOFFLINE"); return; }
        if (!p7501_hw_is_cpu_halted()) { const uint8_t brk = 0x03u; p7501_pio_tx_bytes(&brk, 1u, false); }
        (void)snprintf(msg, 192u, "@P7501_SURG op=PAUSE online=%u halted=%u dcsr=0x%08" PRIX32,
                       p7501_hw_is_target_online() ? 1u : 0u, p7501_hw_is_cpu_halted() ? 1u : 0u, p7501_hw_read_dcsr());
        p7501_gps_emit_tag("SURG", msg);
        printf("SURG: OK\n");
        return;
    } else if (p7501_ascii_ieq(toks[0], "CONT") || p7501_ascii_ieq(toks[0], "JUMP") || p7501_ascii_ieq(toks[0], "STEP")) {
        if (!p7501_hw_is_target_online()) { p7501_cli_set_result("ERR", "EOFFLINE"); return; }
        uint32_t npc = s_shadow_pc;
        if (p7501_ascii_ieq(toks[0], "STEP")) { npc = s_shadow_pc + 2u; }
        else if ((n >= 2u) && !p7501_surg_resolve_va_or_anchor(toks[1], &npc)) { p7501_cli_set_result("ERR", "EINVAL"); return; }
        if (npc != 0u) {
            p7501_surg_entry_t e;
            memset(&e, 0, sizeof(e));
            e.kind = P7501_SURG_PC_JUMP; e.pc_before = s_shadow_pc; e.pc_after = npc; e.ts_us = time_us_64();
            (void)snprintf(e.tag, sizeof(e.tag), "%s", toks[0]);
            (void)p7501_surg_journal_push(j, &e);
            p7501_hw_set_pc(npc);
            s_shadow_pc = npc;
        }
        if (p7501_ascii_ieq(toks[0], "CONT")) { p7501_hw_resume_cpu(); }
    } else {
        p7501_cli_set_result("ERR", "EINVAL");
        return;
    }
    (void)p7501_surg_journal_format_summary(j, msg, 192u);
    p7501_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}
