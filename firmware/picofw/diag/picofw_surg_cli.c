/**
 * @file picofw_surg_cli.c
 * @brief Offline/read-only `SURG` CLI dispatcher (`BIT`, `SEARCH`, `DISASM`, `ASM`, `MEIHFS`, `HDD_PLAN`).
 *
 * Live target mutations, hooks, time-travel undo/redo & `HDD_UNLOCK` live in `diag/picofw_surg_live.c`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_surg_cli.h"

#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "diag/picofw_gps_cli.h"
#include "diag/picofw_hdd_unlock.h"
#include "diag/picofw_math_gps.h"
#include "diag/picofw_meihfs_probe.h"
#include "diag/picofw_microsurgery.h"
#include "diag/picofw_surg_internal.h"
#include "flash/picofw_fast_stream_codec.h"
#include "uart/picofw_pana_dumpnor.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static void handle_surg_bit(char *toks[], size_t n) {
    uint32_t v0 = 0u, a1 = 0u, a2 = 0u, a3 = 0u, res = 0u;
    picofw_bit_op_t bop = PICOFW_BIT_INFO;
    const bool live = (n >= 4u) && picofw_ascii_ieq(toks[2], "LIVE");
    const size_t op_i = live ? 3u : 2u;
    if ((n < (op_i + 1u)) || !picofw_dumpnor_parse_hex_tok(toks[1], &v0) ||
        !picofw_bit_parse_op(toks[op_i], &bop)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SURG: ERROR (Usage: SURG BIT <hex> [LIVE] <op> [a1] [a2] [a3])\n");
        return;
    }
    if ((n > (op_i + 1u) && !picofw_parse_u32_auto(toks[op_i + 1u], &a1)) ||
        (n > (op_i + 2u) && !picofw_parse_u32_auto(toks[op_i + 2u], &a2)) ||
        (n > (op_i + 3u) && !picofw_parse_u32_auto(toks[op_i + 3u], &a3))) {
        picofw_cli_set_result("ERR", "EINVAL");
        return;
    }
    if (live) {
        picofw_surg_handle_bit_live(v0, bop, a1, a2, a3);
        return;
    }
    if (!picofw_bit_eval(bop, v0, a1, a2, a3, &res)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SURG: ERROR (invalid bitwise width/shift)\n");
        return;
    }
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)snprintf(msg, 192u, "@PICO_SURG op=BIT live=0 in=0x%08" PRIX32 " out=0x%08" PRIX32 " xor=0x%08" PRIX32,
                   v0, res, v0 ^ res);
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

static void handle_surg_search_or_disasm(char *toks[], size_t n, bool is_search) {
    uint32_t va = 0u, len = is_search ? 0u : 64u;
    if ((n < (is_search ? 4u : 2u)) || !picofw_dumpnor_parse_hex_tok(toks[1], &va) ||
        ((n >= 3u) && !picofw_parse_u32_auto(toks[2], &len))) {
        picofw_cli_set_result("ERR", "EINVAL");
        return;
    }
    uint8_t *const buf = g_picofw_cmd_arena.raw;
    if (!picofw_gps_read_window_checked("SURG", va, len, buf)) {
        return;
    }
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    if (is_search) {
        uint8_t pat[PICOFW_SURG_MAX_PATTERN], mask[PICOFW_SURG_MAX_PATTERN];
        size_t plen = 0u;
        uint32_t *const hits = (uint32_t *)(void *)&g_picofw_cmd_arena.raw[4096u];
        if (!picofw_surg_parse_wildcard_hex(toks[3], pat, mask, &plen)) {
            picofw_cli_set_result("ERR", "EINVAL");
            return;
        }
        const size_t cnt = picofw_surg_search_buf(buf, len, va, pat, mask, plen, hits, 16u);
        for (size_t i = 0u; (i < cnt) && (i < 16u); ++i) {
            printf("[SURG HIT #%u] va=0x%08" PRIX32 "\n", (unsigned)i, hits[i]);
        }
        (void)snprintf(msg, 192u, "@PICO_SURG op=SEARCH va=0x%08" PRIX32 " hits=%u", va, (unsigned)cnt);
    } else {
        size_t pos = 0u, lines = 0u;
        while ((pos < len) && (lines < 16u)) {
            picofw_mn103_insn_info_t insn;
            if (!picofw_mn103_decode_insn(&buf[pos], len - pos, va + (uint32_t)pos, &insn)) {
                break;
            }
            printf("[0x%08" PRIX32 "] len=%u %s\n", va + (uint32_t)pos, (unsigned)insn.len, insn.mnemonic);
            pos += insn.len;
            lines += 1u;
        }
        (void)snprintf(msg, 192u, "@PICO_SURG op=DISASM va=0x%08" PRIX32 " insns=%u", va, (unsigned)lines);
    }
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

static void handle_surg_asm(char *toks[], size_t n) {
    uint32_t pc_va = 0u;
    if ((n < 3u) || !picofw_surg_resolve_va_or_anchor(toks[1], &pc_va)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SURG: ERROR (Usage: SURG ASM <pc_va> <mnemonic...>)\n");
        return;
    }
    char line[64] = {0};
    size_t pos = 0u;
    for (size_t i = 2u; i < n; ++i) {
        const int w = snprintf(&line[pos], sizeof(line) - pos, "%s%s", (i > 2u) ? " " : "", toks[i]);
        if ((w < 0) || ((pos + (size_t)w) >= sizeof(line))) {
            break;
        }
        pos += (size_t)w;
    }
    uint8_t bytes[8] = {0};
    uint8_t blen = 0u;
    picofw_mn103_insn_info_t dec;
    if (!picofw_mn103_assemble_insn(line, pc_va, bytes, &blen) ||
        !picofw_mn103_decode_insn(bytes, blen, pc_va, &dec)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SURG: ERROR (invalid MN103S syntax '%s')\n", line);
        return;
    }
    char hex[20];
    (void)picofw_fast_format_page_hex(bytes, blen, hex, sizeof(hex));
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)snprintf(msg, 192u, "@PICO_SURG op=ASM va=0x%08" PRIX32 " len=%u hex=%s dec=\"%s\"",
                   pc_va, (unsigned)blen, hex, dec.mnemonic);
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

static void handle_surg_hdd_plan(char *toks[], size_t n) {
    uint32_t lba_u32 = 0u, fw_bytes = 0u;
    if ((n < 3u) || !picofw_parse_u32_auto(toks[1], &lba_u32) ||
        !picofw_parse_u32_auto(toks[2], &fw_bytes)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SURG: ERROR (Usage: SURG HDD_PLAN <total_lba> <fw_bytes>)\n");
        return;
    }
    picofw_hdd_full_plan_t *const plan =
        (picofw_hdd_full_plan_t *)(void *)&g_picofw_cmd_arena.raw[8192u];
    if (!picofw_hdd_compute_full_plan((uint64_t)lba_u32, fw_bytes, plan)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SURG: ERROR (insufficient LBA span for full MEIHFS)\n");
        return;
    }
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)snprintf(
        msg, 224u,
        "@PICO_SURG op=HDD_PLAN total_lba=%" PRIu32 " primary=0x%08" PRIX32
        " mirror=0x%08" PRIX32 " data_lba=0x%08" PRIX32 " usable_2kb=%" PRIu32
        " rtsd_2kb=0x%08" PRIX32,
        (uint32_t)plan->total_lba, plan->primary_fw_lba, (uint32_t)plan->mirror_fw_lba,
        (uint32_t)plan->data_start_lba, plan->usable_blocks_2kb, plan->target_rtsd_blocks_2kb
    );
    picofw_gps_emit_tag("SURG", msg);
    printf("SURG: OK\n");
}

void picofw_cmd_surg(const char *arg) {
    char *const copy = (char *)&g_picofw_cmd_arena.raw[12288u];
    char *toks[8] = { NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL };
    const size_t n = picofw_copy_and_split_ws(arg, copy, 192u, toks, 8u);

    if ((n >= 1u) && picofw_ascii_ieq(toks[0], "BIT")) {
        handle_surg_bit(toks, n);
    } else if ((n >= 1u) && (picofw_ascii_ieq(toks[0], "SEARCH") || picofw_ascii_ieq(toks[0], "DISASM"))) {
        handle_surg_search_or_disasm(toks, n, picofw_ascii_ieq(toks[0], "SEARCH"));
    } else if ((n >= 1u) && picofw_ascii_ieq(toks[0], "ASM")) {
        handle_surg_asm(toks, n);
    } else if ((n >= 1u) && picofw_ascii_ieq(toks[0], "HDD_PLAN")) {
        handle_surg_hdd_plan(toks, n);
    } else if ((n >= 1u) && picofw_ascii_ieq(toks[0], "HDD_UNLOCK")) {
        picofw_surg_handle_hdd_unlock(toks, n);
    } else if ((n >= 1u) && (picofw_ascii_ieq(toks[0], "PATCH") ||
                             picofw_ascii_ieq(toks[0], "TIMEOUT") ||
                             picofw_ascii_ieq(toks[0], "HOOK"))) {
        picofw_surg_handle_patch_timeout_hook(toks, n);
    } else if ((n >= 2u) && picofw_ascii_ieq(toks[0], "MEIHFS")) {
        uint32_t va = 0u;
        uint8_t *const sec = g_picofw_cmd_arena.raw;
        picofw_meihfs_mbr_info_t info;
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &va) ||
            !picofw_gps_read_window_checked("SURG", va, 512u, sec)) {
            return;
        }
        (void)picofw_meihfs_inspect_sector0(sec, &info);
        char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
        (void)picofw_meihfs_format_report(va, &info, msg, 192u);
        picofw_gps_emit_tag("SURG", msg);
        printf("SURG: OK\n");
    } else {
        picofw_surg_handle_time_and_cpu(toks, n);
    }
}
