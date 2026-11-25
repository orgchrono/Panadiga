/**
 * @file picofw_sector_ops.c
 * @brief NOR sector inspector, signature/Feistel locator & safe `1->0` bit-clear patcher (`SECTOR`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_sector_ops.h"

#include "common/picofw_hex.h"
#include "crypto/picofw_feistel.h"
#include "flash/picofw_fast_stream_codec.h"
#include "flash/picofw_flash.h"
#include "uart/picofw_pana_dumpnor.h"

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_push_stream.h"
#include "flash/picofw_fast_stream.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#endif

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static const char *classify_8mb_zone(uint16_t sa, uint32_t uncached_addr, const picofw_flash_geometry_t *geo) {
    if ((geo != NULL) && (geo->mb == 8u)) {
        if (sa <= 10u) {
            return "IPL_L1_RO";
        }
        if (sa <= 14u) {
            return "NORFROM_META";
        }
        if (sa <= 25u) {
            return "FE_DRIVE_AREA";
        }
        if (sa <= 70u) {
            return "BGO_FTL_DATA";
        }
        if (sa <= 133u) {
            return "BGO_EEMUL_LOG";
        }
        return "NVRAM_EEMUL";
    }
    if ((geo != NULL) && (geo->nvram_size > 0u) &&
        (uncached_addr >= geo->hw_nvram_base) && (uncached_addr < geo->hw_nvram_end)) {
        return "NVRAM_EEMUL";
    }
    if (uncached_addr < (PICOFW_SSOT_CS0_UNCACHED_BASE + PICOFW_SSOT_IPL_LAYER1_SIZE)) {
        return "IPL_L1_RO";
    }
    return "UPPER_NOR_APP";
}

bool picofw_sector_describe(
    const picofw_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    bool upper_unlocked,
    picofw_sector_info_t *out
) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->sa_index = -1;

    const uint32_t eff_size = picofw_flash_normalize_size(flash_size_bytes);
    if (!picofw_flash_is_readonly_nor_range(eff_size, addr, 1u)) {
        return false;
    }
    const uint32_t uaddr = picofw_flash_normalize_uncached_addr(addr);
    uint32_t sec_base = 0u;
    uint32_t sec_size = 0u;
    if (!picofw_flash_get_sector_span_at(eff_size, uaddr, &sec_base, &sec_size)) {
        return false;
    }

    const picofw_flash_geometry_t geo = picofw_flash_get_geometry(eff_size);
    if (geo.mb == 8u) {
        const uint16_t sa = picofw_fast_va_to_sa(uaddr);
        if (sa < PICOFW_NORMAP_SECTOR_COUNT) {
            out->sa_index = (int32_t)sa;
        }
    } else if (sec_size > 0u) {
        out->sa_index = (int32_t)((sec_base - PICOFW_SSOT_CS0_UNCACHED_BASE) / sec_size);
    }

    out->query_addr = addr;
    out->uncached_addr = uaddr;
    out->sector_base = picofw_flash_normalize_uncached_addr(sec_base);
    out->sector_size = sec_size;
    out->offset_in_sector = uaddr - out->sector_base;
    out->zone = classify_8mb_zone((out->sa_index >= 0) ? (uint16_t)out->sa_index : 0u, uaddr, &geo);
    (void)picofw_trap_check_full(
        ctx,
        eff_size,
        uaddr,
        4u,
        PICOFW_SSOT_TRAP_W,
        upper_unlocked,
        &out->write_audit
    );
    return true;
}

size_t picofw_sig_find_in_buf(
    const uint8_t *buf,
    size_t buf_len,
    uint32_t base_va,
    const uint8_t *needle,
    size_t needle_len,
    uint32_t *out_hits,
    size_t max_hits
) {
    if ((buf == NULL) || (needle == NULL) || (needle_len == 0u) || (buf_len < needle_len)) {
        return 0u;
    }
    size_t hits = 0u;
    for (size_t i = 0u; (i + needle_len) <= buf_len; ++i) {
        if (memcmp(&buf[i], needle, needle_len) == 0) {
            if ((out_hits != NULL) && (hits < max_hits)) {
                out_hits[hits] = base_va + (uint32_t)i;
            }
            hits += 1u;
        }
    }
    return hits;
}

bool picofw_sector_plan_patch(
    const picofw_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    bool upper_unlocked,
    uint32_t target_va,
    const uint8_t *old_bytes,
    const uint8_t *new_bytes,
    size_t len,
    picofw_sector_patch_plan_t *out_plan
) {
    if ((old_bytes == NULL) || (new_bytes == NULL) || (out_plan == NULL) ||
        (len == 0u) || (len > PICOFW_SSOT_MAX_FAST_WRITE_BYTES)) {
        return false;
    }
    memset(out_plan, 0, sizeof(*out_plan));

    (void)picofw_trap_check_full(
        ctx,
        flash_size_bytes,
        target_va,
        (uint32_t)len,
        PICOFW_SSOT_TRAP_W,
        upper_unlocked,
        &out_plan->audit
    );
    const bool gate_ok = (out_plan->audit.verdict != PICOFW_TRAP_DENY);
    const bool mono_ok = picofw_fast_check_monotonic_1_to_0(
        old_bytes, new_bytes, len, &out_plan->first_bad_offset
    );
    out_plan->bitclear_monotonic = mono_ok;
    out_plan->needs_erase = !mono_ok;
    out_plan->allowed = gate_ok && mono_ok;
    return true;
}

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)

static bool resolve_sig_pattern(const char *tok, uint8_t *out, size_t cap, size_t *out_len) {
    for (size_t i = 0u; i < PICOFW_SSOT_SIG_COUNT; ++i) {
        if (picofw_ascii_ieq(tok, k_picofw_ssot_sigs[i].name)) {
            const size_t n = (size_t)k_picofw_ssot_sigs[i].len;
            if (n > cap) {
                return false;
            }
            memcpy(out, k_picofw_ssot_sigs[i].bytes, n);
            *out_len = n;
            return true;
        }
    }
    return picofw_decode_hex_bytes(tok, out, cap, out_len) && (*out_len > 0u);
}

void picofw_cmd_sector(const char *arg) {
    if ((arg == NULL) || (*arg == '\0')) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SECTOR: ERROR (Usage: SECTOR INFO <addr> | READ <addr> [len] | FIND <SIG|hex> [start] [len] | PATCH <addr> <hex> CONFIRM)\n");
        return;
    }

    char *const copy = (char *)&g_picofw_cmd_arena.raw[4096];
    (void)snprintf(copy, 320u, "%s", arg);
    char *toks[4] = { NULL, NULL, NULL, NULL };
    const size_t n = picofw_split_ws_tokens(copy, toks, 4u);

    const picofw_session_ctx_t *const ctx = picofw_session_default_ctx();
    const uint32_t flash_sz = picofw_flash_get_active_size();
    const bool upper_ok = picofw_flash_is_upper_nor_unlocked();

    if ((n == 2u) && picofw_ascii_ieq(toks[0], "INFO")) {
        uint32_t addr = 0u;
        picofw_sector_info_t *const info = (picofw_sector_info_t *)(void *)&g_picofw_cmd_arena.raw[4608];
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &addr) ||
            !picofw_sector_describe(ctx, flash_sz, addr, upper_ok, info)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("SECTOR: ERROR (address outside active %u MiB CS0 NOR window)\n",
                   (unsigned)(flash_sz / (1024UL * 1024UL)));
            return;
        }
        char *const msg = (char *)&g_picofw_cmd_arena.raw[4864];
        (void)snprintf(
            msg, 256u,
            "va=0x%08" PRIX32 " sa=%" PRId32 " base=0x%08" PRIX32 " size=0x%" PRIX32
            " off=0x%" PRIX32 " zone=%s write_policy=%s trap=%s",
            info->uncached_addr,
            info->sa_index,
            info->sector_base,
            info->sector_size,
            info->offset_in_sector,
            info->zone,
            info->write_audit.bus_deny,
            (info->write_audit.trap != NULL) ? info->write_audit.trap->id : "NONE"
        );
        (void)picofw_push_emit_str("SECTOR", msg);
        (void)picofw_push_flush_global();
        printf("[SECTOR] %s\n", msg);
        return;
    }

    if ((n >= 2u) && (n <= 3u) && picofw_ascii_ieq(toks[0], "READ")) {
        uint32_t addr = 0u;
        uint32_t rlen = 64u;
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &addr) ||
            ((n == 3u) && !picofw_parse_u32_auto(toks[2], &rlen)) ||
            (rlen == 0u) || (rlen > 128u)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("SECTOR: ERROR (Usage: SECTOR READ <addr_hex> [1..128])\n");
            return;
        }
        picofw_trap_audit_result_t *const audit =
            (picofw_trap_audit_result_t *)(void *)&g_picofw_cmd_arena.raw[4608];
        (void)picofw_trap_check_full(ctx, flash_sz, addr, rlen, PICOFW_SSOT_TRAP_R, upper_ok, audit);
        if (audit->verdict == PICOFW_TRAP_DENY) {
            picofw_cli_set_result("ERR", "E05");
            printf("SECTOR: DENIED (%s)\n", audit->bus_deny);
            return;
        }
        uint8_t *const buf = g_picofw_cmd_arena.raw;
        if (!picofw_hw_is_target_online() || !picofw_hw_read_block_checked(addr, buf, (size_t)rlen, true)) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("SECTOR: ERROR (target offline or PANAMON1> read failed)\n");
            return;
        }
        char *const hex_out = (char *)&g_picofw_cmd_arena.raw[256];
        char *const hdr = (char *)&g_picofw_cmd_arena.raw[1024];
        (void)picofw_fast_format_page_hex(buf, (size_t)rlen, hex_out, 512u);
        (void)snprintf(hdr, 512u, "op=READ va=0x%08" PRIX32 " len=%" PRIu32 " data=%s", addr, rlen, hex_out);
        (void)picofw_push_emit_str("SECTOR", hdr);
        (void)picofw_push_flush_global();
        printf("[SECTOR READ] 0x%08" PRIX32 " (%u B): %s\n", addr, (unsigned)rlen, hex_out);
        return;
    }

    if ((n >= 2u) && (n <= 4u) && picofw_ascii_ieq(toks[0], "FIND")) {
        uint8_t *const needle = &g_picofw_cmd_arena.raw[4608];
        size_t needle_len = 0u;
        uint32_t start_va = PICOFW_SSOT_CS0_UNCACHED_BASE;
        uint32_t span = 0x100000UL;
        if (!resolve_sig_pattern(toks[1], needle, 32u, &needle_len) ||
            ((n >= 3u) && !picofw_dumpnor_parse_hex_tok(toks[2], &start_va)) ||
            ((n >= 4u) && !picofw_dumpnor_parse_hex_tok(toks[3], &span)) ||
            (span == 0u)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("SECTOR: ERROR (Usage: SECTOR FIND <SIG|hex> [start_va] [len_hex])\n");
            return;
        }
        if (!picofw_hw_is_target_online()) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("SECTOR: ERROR (target monitor offline)\n");
            return;
        }

        uint8_t *const chunk = g_picofw_cmd_arena.raw;
        uint32_t *const hits = (uint32_t *)(void *)&g_picofw_cmd_arena.raw[4672];
        picofw_trap_audit_result_t *const audit =
            (picofw_trap_audit_result_t *)(void *)&g_picofw_cmd_arena.raw[4736];
        char *const msg = (char *)&g_picofw_cmd_arena.raw[4800];
        uint32_t total_hits = 0u;
        for (uint32_t off = 0u; off < span; ) {
            picofw_wdt_kick();
            if (picofw_wdt_poll_abort()) {
                picofw_cli_set_result("ERR", "EABORT");
                printf("SECTOR FIND: aborted\n");
                return;
            }
            const uint32_t cur_va = start_va + off;
            const uint32_t step = ((span - off) > 4096u) ? 4096u : (span - off);
            (void)picofw_trap_check_full(ctx, flash_sz, cur_va, step, PICOFW_SSOT_TRAP_R, upper_ok, audit);
            if (audit->verdict == PICOFW_TRAP_DENY) {
                break;
            }
            if (!picofw_hw_read_block_checked(cur_va, chunk, (size_t)step, true)) {
                picofw_cli_set_result("ERR", "EIO");
                printf("SECTOR FIND: read failed @ 0x%08" PRIX32 "\n", cur_va);
                return;
            }
            const size_t h = picofw_sig_find_in_buf(chunk, (size_t)step, cur_va, needle, needle_len, hits, 8u);
            for (size_t i = 0u; (i < h) && (i < 8u); ++i) {
                (void)snprintf(msg, 128u, "sig=%s va=0x%08" PRIX32, toks[1], hits[i]);
                (void)picofw_push_emit_str("SIG_HIT", msg);
                printf("[SECTOR FIND] hit %s\n", msg);
            }
            if ((h == 0u) && (step >= 8u) && picofw_ascii_ieq(toks[1], "EXTRHEAD")) {
                uint8_t plain8[8];
                size_t kidx = 0u;
                if (picofw_feistel_autodetect_key(chunk, plain8, &kidx)) {
                    (void)snprintf(
                        msg, 128u,
                        "sig=FEISTEL_EXTRHEAD va=0x%08" PRIX32 " key=%s",
                        cur_va, k_picofw_ssot_feistel_keys[kidx].name
                    );
                    (void)picofw_push_emit_str("SIG_HIT", msg);
                    printf("[SECTOR FIND] hit %s\n", msg);
                    total_hits += 1u;
                }
            }
            total_hits += (uint32_t)h;
            off += step;
        }
        (void)picofw_push_flush_global();
        printf("SECTOR FIND: OK (hits=%" PRIu32 ")\n", total_hits);
        return;
    }

    if ((n >= 3u) && (n <= 4u) && picofw_ascii_ieq(toks[0], "PATCH")) {
        uint32_t addr = 0u;
        uint8_t *const tgt = &g_picofw_cmd_arena.raw[0];
        uint8_t *const cur = &g_picofw_cmd_arena.raw[PICOFW_SSOT_MAX_FAST_WRITE_BYTES];
        size_t pay_len = 0u;
        const bool confirmed = (n == 4u) && picofw_ascii_ieq(toks[3], "CONFIRM");
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &addr) ||
            !picofw_decode_hex_bytes(toks[2], tgt, PICOFW_SSOT_MAX_FAST_WRITE_BYTES, &pay_len) ||
            (pay_len == 0u)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("SECTOR: ERROR (Usage: SECTOR PATCH <addr_hex> <hex> CONFIRM)\n");
            return;
        }
        if (!confirmed) {
            picofw_cli_set_result("ERR", "ECONFIRM");
            printf("SECTOR PATCH: ERROR (missing CONFIRM token)\n");
            return;
        }
        if (!picofw_hw_is_target_online() || !picofw_hw_read_block_checked(addr, cur, pay_len, true)) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("SECTOR PATCH: ERROR (target offline or pre-read failed)\n");
            return;
        }
        picofw_sector_patch_plan_t *const plan =
            (picofw_sector_patch_plan_t *)(void *)&g_picofw_cmd_arena.raw[4608];
        if (!picofw_sector_plan_patch(ctx, flash_sz, upper_ok, addr, cur, tgt, pay_len, plan) || !plan->allowed) {
            picofw_cli_set_result("ERR", plan->needs_erase ? "ENEED_ERASE" : "E05");
            printf("SECTOR PATCH: DENIED (policy=%s needs_erase=%u bad_off=%u)\n",
                   plan->audit.bus_deny, plan->needs_erase ? 1u : 0u, (unsigned)plan->first_bad_offset);
            return;
        }
        if (!picofw_fast_write_bytes_hw("SECTOR_PATCH", addr, tgt, pay_len, true)) {
            picofw_cli_set_result("ERR", "EIO");
            return;
        }
        printf("SECTOR PATCH: OK (0x%08" PRIX32 ", %u B)\n", addr, (unsigned)pay_len);
        return;
    }

    picofw_cli_set_result("ERR", "EINVAL");
    printf("SECTOR: ERROR (unknown subcommand '%s')\n", toks[0]);
}

#endif /* !PICO_HOST_TEST */
