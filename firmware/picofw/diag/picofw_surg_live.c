/**
 * @file picofw_surg_live.c
 * @brief Live AM34 SDRAM/CPU microsurgery executor, time-travel controller & RTSD HDD unlocker.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_surg_internal.h"

#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "diag/picofw_gps_cli.h"
#include "diag/picofw_hdd_unlock.h"
#include "hal/picofw_hw.h"
#include "uart/picofw_pana_dumpnor.h"
#include "uart/picofw_uart_pio_engine.h"

#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static picofw_surg_journal_t s_journal;
static bool s_journal_init = false;
static uint32_t s_shadow_pc = 0u;

picofw_surg_journal_t *picofw_surg_global_journal(void) {
    if (!s_journal_init) {
        picofw_surg_journal_init(&s_journal);
        s_journal_init = true;
    }
    return &s_journal;
}

static bool ram_write_verified(uint32_t va, const uint8_t *data, size_t len) {
    if (!picofw_surg_is_live_ram_span(va, (uint32_t)len) || !picofw_hw_is_target_online()) {
        return false;
    }
    const uint32_t start_w = va & ~3UL;
    const uint32_t end_w = (va + (uint32_t)len + 3UL) & ~3UL;
    for (uint32_t waddr = start_w; waddr < end_w; waddr += 4UL) {
        uint32_t word = 0u;
        if (!picofw_hw_read_mem32_checked(waddr, &word)) {
            return false;
        }
        for (uint32_t b = 0u; b < 4u; ++b) {
            const uint32_t baddr = waddr + b;
            if ((baddr >= va) && (baddr < (va + (uint32_t)len))) {
                const uint32_t sh = b * 8u;
                word = (word & ~(0xFFUL << sh)) | ((uint32_t)data[baddr - va] << sh);
            }
        }
        picofw_hw_write_mem32(waddr, word);
        uint32_t rb = 0u;
        if (!picofw_hw_read_mem32_checked(waddr, &rb) || (rb != word)) {
            return false;
        }
    }
    return true;
}

static bool require_live_ram(uint32_t va, uint32_t len) {
    if (!picofw_surg_is_live_ram_span(va, len)) {
        picofw_cli_set_result("ERR", "E05");
        printf("SURG: DENIED (0x%08" PRIX32 "+0x%" PRIX32 " outside mutable AM34 RAM/SDRAM)\n", va, len);
        return false;
    }
    if (!picofw_hw_is_target_online()) {
        picofw_cli_set_result("ERR", "EOFFLINE");
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
    picofw_surg_entry_t e;
    memset(&e, 0, sizeof(e));
    e.kind = kind;
    e.len = (uint8_t)len;
    e.site_va = site_va;
    e.aux_va = aux_va;
    e.ts_us = time_us_64();
    memcpy(e.before, before, len);
    memcpy(e.after, after, len);
    (void)snprintf(e.tag, sizeof(e.tag), "%s", tag);
    (void)picofw_surg_journal_push(picofw_surg_global_journal(), &e);
}

static bool apply_journal_step(picofw_surg_journal_t *j, bool undo) {
    const picofw_surg_entry_t *const e = undo
        ? picofw_surg_journal_peek_undo(j)
        : picofw_surg_journal_peek_redo(j);
    if (e == NULL) {
        return false;
    }
    if ((e->kind == PICOFW_SURG_MEM_PATCH) || (e->kind == PICOFW_SURG_HOOK)) {
        const uint8_t *const data = undo ? e->before : e->after;
        if (!ram_write_verified(e->site_va, data, e->len)) {
            return false;
        }
    } else if (e->kind == PICOFW_SURG_PC_JUMP) {
        const uint32_t npc = undo ? e->pc_before : e->pc_after;
        if (!picofw_hw_is_target_online()) {
            return false;
        }
        picofw_hw_set_pc(npc);
        s_shadow_pc = npc;
    }
    return undo ? picofw_surg_journal_commit_undo(j) : picofw_surg_journal_commit_redo(j);
}

bool picofw_surg_resolve_va_or_anchor(const char *tok, uint32_t *out_va) {
    if (picofw_dumpnor_parse_hex_tok(tok, out_va)) {
        return true;
    }
    size_t idx = 0u;
    if (picofw_gps_find_anchor_by_name(tok, &idx)) {
        *out_va = picofw_gps_resolve_anchor(picofw_gps_global_env(), idx);
        return *out_va != 0u;
    }
    return false;
}

void picofw_surg_handle_bit_live(uint32_t target_va, picofw_bit_op_t bop, uint32_t a1, uint32_t a2, uint32_t a3) {
    const uint32_t waddr = target_va & ~3UL;
    uint32_t v0 = 0u, res = 0u;
    if (!require_live_ram(waddr, 4u) || !picofw_hw_read_mem32_checked(waddr, &v0)) {
        return;
    }
    if (!picofw_bit_eval(bop, v0, a1, a2, a3, &res)) {
        picofw_cli_set_result("ERR", "EINVAL");
        return;
    }
    const uint8_t b4[4] = { (uint8_t)v0, (uint8_t)(v0 >> 8u), (uint8_t)(v0 >> 16u), (uint8_t)(v0 >> 24u) };
    const uint8_t a4[4] = { (uint8_t)res, (uint8_t)(res >> 8u), (uint8_t)(res >> 16u), (uint8_t)(res >> 24u) };
    if (!ram_write_verified(waddr, a4, 4u)) {
        picofw_cli_set_result("ERR", "EVERIFY");
        return;
    }
    record_mem_entry(PICOFW_SURG_MEM_PATCH, "BIT_LIVE", waddr, 0u, b4, a4, 4u);
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)snprintf(msg, 192u, "@PICO_SURG op=BIT live=1 in=0x%08" PRIX32 " out=0x%08" PRIX32 " xor=0x%08" PRIX32,
                   v0, res, v0 ^ res);
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

void picofw_surg_handle_patch_timeout_hook(char *toks[], size_t n) {
    picofw_surg_plan_t *const plan = (picofw_surg_plan_t *)(void *)&g_picofw_cmd_arena.raw[4096u];
    uint8_t *const orig = g_picofw_cmd_arena.raw;
    uint32_t site_va = 0u, val = 0u;
    if (picofw_ascii_ieq(toks[0], "PATCH") && (n == 3u)) {
        uint8_t after[PICOFW_SURG_MAX_PATCH_BYTES], before[PICOFW_SURG_MAX_PATCH_BYTES];
        size_t plen = 0u;
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &site_va) ||
            !picofw_decode_hex_bytes(toks[2], after, sizeof(after), &plen) || (plen == 0u) ||
            !require_live_ram(site_va, (uint32_t)plen) ||
            !picofw_hw_read_block_checked(site_va, before, (uint32_t)plen, true) ||
            !ram_write_verified(site_va, after, plen)) {
            picofw_cli_set_result("ERR", "EINVAL");
            return;
        }
        record_mem_entry(PICOFW_SURG_MEM_PATCH, "PATCH", site_va, 0u, before, after, plen);
    } else if (picofw_ascii_ieq(toks[0], "TIMEOUT") && (n == 3u)) {
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &site_va) || !picofw_parse_u32_auto(toks[2], &val) ||
            !require_live_ram(site_va, 8u) || !picofw_hw_read_block_checked(site_va, orig, 8u, true) ||
            !picofw_surg_plan_timeout_patch(site_va, orig, 8u, val, plan) ||
            !ram_write_verified(site_va, plan->patch, plan->site_len)) {
            picofw_cli_set_result("ERR", "EINVAL");
            return;
        }
        record_mem_entry(PICOFW_SURG_MEM_PATCH, "TIMEOUT", site_va, 0u, plan->orig, plan->patch, plan->site_len);
    } else if (picofw_ascii_ieq(toks[0], "HOOK") && (n == 5u)) {
        uint32_t tramp_va = 0u;
        picofw_hook_mode_t mode = PICOFW_HOOK_D0_RET;
        if (picofw_ascii_ieq(toks[3], "D0_CONT")) { mode = PICOFW_HOOK_D0_CONT; }
        else if (picofw_ascii_ieq(toks[3], "CALL")) { mode = PICOFW_HOOK_CALL; }
        else if (!picofw_ascii_ieq(toks[3], "D0_RET")) { picofw_cli_set_result("ERR", "EINVAL"); return; }
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &site_va) ||
            !picofw_dumpnor_parse_hex_tok(toks[2], &tramp_va) ||
            !picofw_surg_resolve_va_or_anchor(toks[4], &val) ||
            !require_live_ram(site_va, 16u) || !require_live_ram(tramp_va, 32u) ||
            !picofw_hw_read_block_checked(site_va, orig, 16u, true) ||
            !picofw_surg_plan_hook(site_va, orig, 16u, tramp_va, mode, val, plan) ||
            !ram_write_verified(tramp_va, plan->tramp, plan->tramp_len) ||
            !ram_write_verified(site_va, plan->patch, plan->site_len)) {
            picofw_cli_set_result("ERR", "EINVAL");
            return;
        }
        record_mem_entry(PICOFW_SURG_HOOK, "HOOK", site_va, tramp_va, plan->orig, plan->patch, plan->site_len);
    } else {
        picofw_cli_set_result("ERR", "EINVAL");
        return;
    }
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)snprintf(msg, 192u, "@PICO_SURG op=%s site=0x%08" PRIX32 " cursor=%u",
                   toks[0], site_va, (unsigned)picofw_surg_global_journal()->cursor);
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

void picofw_surg_handle_hdd_unlock(char *toks[], size_t n) {
    uint32_t site_va = 0u, total_lba = 0u;
    const bool apply = (n >= 5u) && picofw_ascii_ieq(toks[3], "APPLY") && picofw_ascii_ieq(toks[4], "CONFIRM");
    if ((n < 3u) || !picofw_dumpnor_parse_hex_tok(toks[1], &site_va) || !picofw_parse_u32_auto(toks[2], &total_lba) ||
        !require_live_ram(site_va, PICOFW_SSOT_HDD_CLAMP_WIN_LEN)) {
        return;
    }
    uint8_t *const win = g_picofw_cmd_arena.raw;
    picofw_hdd_full_plan_t *const fp = (picofw_hdd_full_plan_t *)(void *)&g_picofw_cmd_arena.raw[4096u];
    picofw_hdd_unlock_plan_t *const up = (picofw_hdd_unlock_plan_t *)(void *)&g_picofw_cmd_arena.raw[4608u];
    if (!picofw_hw_read_block_checked(site_va, win, PICOFW_SSOT_HDD_CLAMP_WIN_LEN, true) ||
        !picofw_hdd_compute_full_plan((uint64_t)total_lba, 0x400000UL, fp) ||
        !picofw_hdd_plan_rtsd_unlock(site_va, win, fp->target_rtsd_blocks_2kb, up)) {
        picofw_cli_set_result("ERR", "ENOENT");
        printf("SURG: ERROR (src/hdd_cmd.c clamp block not found @ 0x%08" PRIX32 ")\n", site_va);
        return;
    }
    if (apply) {
        static const char *const k_tags[4] = { "HDD_S400", "HDD_S200", "HDD_SFB", "HDD_BRA" };
        for (size_t i = 0u; i < 4u; ++i) {
            const uint32_t off = up->ops[i].va - site_va;
            if (!ram_write_verified(up->ops[i].va, up->ops[i].data, up->ops[i].len)) {
                picofw_cli_set_result("ERR", "EVERIFY");
                return;
            }
            record_mem_entry(PICOFW_SURG_MEM_PATCH, k_tags[i], up->ops[i].va, 0u,
                             &win[off], up->ops[i].data, up->ops[i].len);
        }
    }
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)snprintf(msg, 224u, "@PICO_SURG op=HDD_UNLOCK site=0x%08" PRIX32 " applied=%u target_2kb=0x%08" PRIX32 " prev_op=0x%02X",
                   site_va, apply ? 1u : 0u, up->target_blocks_2kb, (unsigned)up->prev_branch_op);
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

void picofw_surg_handle_time_and_cpu(char *toks[], size_t n) {
    picofw_surg_journal_t *const j = picofw_surg_global_journal();
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    if ((n == 0u) || picofw_ascii_ieq(toks[0], "LOG")) {
        (void)picofw_surg_journal_format_summary(j, msg, 192u);
        picofw_gps_emit_tag("SURG", msg);
        for (uint8_t i = 0u; i < j->count; ++i) {
            printf("  %c[%u] %s va=0x%08" PRIX32 " len=%u\n", (i + 1u == j->cursor) ? '*' : ' ',
                   (unsigned)i, j->entries[i].tag, j->entries[i].site_va, (unsigned)j->entries[i].len);
        }
        printf("SURG: OK\n");
        return;
    }
    if (picofw_ascii_ieq(toks[0], "UNDO") || picofw_ascii_ieq(toks[0], "UNHOOK") ||
        picofw_ascii_ieq(toks[0], "REDO")) {
        if (!apply_journal_step(j, !picofw_ascii_ieq(toks[0], "REDO"))) {
            picofw_cli_set_result("ERR", "ENOENT");
            return;
        }
    } else if (picofw_ascii_ieq(toks[0], "REWIND") && (n >= 2u)) {
        uint32_t target = 0u;
        if (!picofw_parse_u32_auto(toks[1], &target) || (target > j->count)) { picofw_cli_set_result("ERR", "EINVAL"); return; }
        while (j->cursor != target) {
            if (!apply_journal_step(j, j->cursor > target)) { picofw_cli_set_result("ERR", "EIO"); return; }
        }
    } else if (picofw_ascii_ieq(toks[0], "PAUSE")) {
        if (!picofw_hw_is_nsw5v_high()) { picofw_cli_set_result("ERR", "EOFFLINE"); return; }
        if (!picofw_hw_is_cpu_halted()) { const uint8_t brk = 0x03u; picofw_pio_tx_bytes(&brk, 1u, false); }
        (void)snprintf(msg, 192u, "@PICO_SURG op=PAUSE online=%u halted=%u dcsr=0x%08" PRIX32,
                       picofw_hw_is_target_online() ? 1u : 0u, picofw_hw_is_cpu_halted() ? 1u : 0u, picofw_hw_read_dcsr());
        picofw_gps_emit_tag("SURG", msg);
        printf("SURG: OK\n");
        return;
    } else if (picofw_ascii_ieq(toks[0], "CONT") || picofw_ascii_ieq(toks[0], "JUMP") || picofw_ascii_ieq(toks[0], "STEP")) {
        if (!picofw_hw_is_target_online()) { picofw_cli_set_result("ERR", "EOFFLINE"); return; }
        uint32_t npc = s_shadow_pc;
        if (picofw_ascii_ieq(toks[0], "STEP")) { npc = s_shadow_pc + 2u; }
        else if ((n >= 2u) && !picofw_surg_resolve_va_or_anchor(toks[1], &npc)) { picofw_cli_set_result("ERR", "EINVAL"); return; }
        if (npc != 0u) {
            picofw_surg_entry_t e;
            memset(&e, 0, sizeof(e));
            e.kind = PICOFW_SURG_PC_JUMP; e.pc_before = s_shadow_pc; e.pc_after = npc; e.ts_us = time_us_64();
            (void)snprintf(e.tag, sizeof(e.tag), "%s", toks[0]);
            (void)picofw_surg_journal_push(j, &e);
            picofw_hw_set_pc(npc);
            s_shadow_pc = npc;
        }
        if (picofw_ascii_ieq(toks[0], "CONT")) { picofw_hw_resume_cpu(); }
    } else {
        picofw_cli_set_result("ERR", "EINVAL");
        return;
    }
    (void)picofw_surg_journal_format_summary(j, msg, 192u);
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}
