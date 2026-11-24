/**
 * @file p7501_payload_cli.c
 * @brief Pico hardware CLI handlers `TRAP` and `PAYLOAD` (imperative shell).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_payloads.h"

#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "common/p7501_push_stream.h"
#include "diag/p7501_mmio_snapshot_codec.h"
#include "flash/p7501_fast_stub_io.h"
#include "flash/p7501_flash.h"
#include "hal/p7501_hw.h"
#include "uart/p7501_pana_dumpnor.h"
#include "uart/p7501_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static bool resolve_payload_arg(
    const char *name,
    p7501_payload_id_t *out_pid,
    p7501_payload_info_t *out_info
) {
    if (!p7501_payload_find(name, out_pid) ||
        !p7501_payload_build_and_verify(*out_pid, g_p7501_cmd_arena.raw, 2048u, out_info)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("PAYLOAD: ERROR (unknown payload '%s')\n", name);
        return false;
    }
    return true;
}

static const char *verdict_name(p7501_trap_verdict_t v) {
    if (v == P7501_TRAP_DENY) {
        return "DENY";
    }
    if (v == P7501_TRAP_WARN) {
        return "WARN";
    }
    return "ALLOW";
}

void p7501_cmd_trap(const char *arg) {
    if ((arg == NULL) || (*arg == '\0') || p7501_ascii_ieq(arg, "LIST")) {
        char buf[192];
        for (size_t i = 0u; i < P7501_SSOT_TRAP_COUNT; ++i) {
            const p7501_ssot_trap_t *const tr = &k_p7501_ssot_traps[i];
            (void)snprintf(
                buf, sizeof(buf),
                "id=%s start=0x%08" PRIX32 " len=0x%08" PRIX32 " mask=0x%02X sev=%s reason=%s",
                tr->id, tr->start, tr->len, (unsigned)tr->mask_rwe,
                (tr->severity >= P7501_SSOT_TRAP_SEV_DENY) ? "DENY" : "WARN", tr->reason
            );
            (void)p7501_push_emit_str("TRAP", buf);
            printf("[TRAP #%u] %s\n", (unsigned)(i + 1u), buf);
        }
        (void)p7501_push_flush_global();
        return;
    }

    char copy[128];
    char *toks[4] = { NULL, NULL, NULL, NULL };
    const size_t n = p7501_copy_and_split_ws(arg, copy, sizeof(copy), toks, 4u);

    if ((n != 4u) || !p7501_ascii_ieq(toks[0], "CHECK")) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("TRAP: ERROR (Usage: TRAP LIST | TRAP CHECK <addr_hex> <len_hex> <R|W|E>)\n");
        return;
    }

    uint32_t addr = 0u;
    uint32_t len = 0u;
    const uint8_t op_mask = p7501_trap_parse_op(toks[3]);
    if (!p7501_dumpnor_parse_hex_tok(toks[1], &addr) ||
        !p7501_dumpnor_parse_hex_tok(toks[2], &len) ||
        (op_mask == 0u)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("TRAP: ERROR (invalid addr/len/op)\n");
        return;
    }

    p7501_trap_audit_result_t res;
    (void)p7501_trap_check_full(
        p7501_session_default_ctx(),
        p7501_flash_get_active_size(),
        addr,
        len,
        op_mask,
        p7501_flash_is_upper_nor_unlocked(),
        &res
    );

    char msg[224];
    (void)snprintf(
        msg, sizeof(msg),
        "addr=0x%08" PRIX32 " len=0x%" PRIX32 " op=%s verdict=%s trap=%s bus_policy=%s reason=%s",
        addr,
        len,
        toks[3],
        verdict_name(res.verdict),
        (res.trap != NULL) ? res.trap->id : "NONE",
        res.bus_deny,
        (res.trap != NULL) ? res.trap->reason : "none"
    );
    (void)p7501_push_emit_str("TRAP", msg);
    (void)p7501_push_flush_global();
    printf("[TRAP] CHECK %s\n", msg);

    if (res.verdict == P7501_TRAP_DENY) {
        p7501_cli_set_result("ERR", "E05");
    }
}

static void handle_payload_list(void) {
    for (size_t i = 0u; i < P7501_PAY_COUNT; ++i) {
        p7501_payload_info_t info;
        p7501_payload_desc_t desc;
        if (p7501_payload_describe(i, &desc) &&
            p7501_payload_build_and_verify(desc.id, g_p7501_cmd_arena.raw, 2048u, &info)) {
            char msg[192];
            (void)snprintf(
                msg, sizeof(msg),
                "name=%s load_va=0x%08" PRIX32 " len=%u crc32=0x%08" PRIX32 " safety=%s",
                info.desc.name, info.desc.load_va, (unsigned)info.len, info.crc32,
                info.desc.safety
            );
            (void)p7501_push_emit_str("PAYLOAD", msg);
            printf("[PAYLOAD #%u] %s | %s\n", (unsigned)(i + 1u), msg, info.desc.summary);
        }
    }
    (void)p7501_push_flush_global();
}

static void handle_payload_verify(const char *filter) {
    const bool check_all = (filter == NULL) || p7501_ascii_ieq(filter, "ALL");
    bool all_ok = true;
    uint32_t checked = 0u;
    for (size_t i = 0u; i < P7501_PAY_COUNT; ++i) {
        p7501_payload_desc_t desc;
        if (!p7501_payload_describe(i, &desc)) {
            continue;
        }
        if (!check_all && !p7501_ascii_ieq(filter, desc.name)) {
            continue;
        }
        p7501_payload_info_t info;
        const bool ok = p7501_payload_build_and_verify(
            desc.id, g_p7501_cmd_arena.raw, 2048u, &info
        );
        all_ok = all_ok && ok;
        checked += 1u;
        char msg[160];
        (void)snprintf(
            msg, sizeof(msg),
            "name=%s ok=%u load_va=0x%08" PRIX32 " len=%u crc32=0x%08" PRIX32,
            desc.name, ok ? 1u : 0u,
            info.desc.load_va, (unsigned)info.len, info.crc32
        );
        (void)p7501_push_emit_str("PAYLOAD", msg);
        printf("[PAYLOAD VERIFY] %s\n", msg);
    }
    (void)p7501_push_flush_global();
    if (!all_ok || (checked == 0u)) {
        p7501_cli_set_result("ERR", (checked == 0u) ? "EINVAL" : "ECRC");
    }
}

static void handle_payload_run(p7501_payload_id_t pid, const p7501_payload_info_t *info, bool confirmed) {
    if (((pid == P7501_PAY_TBC_PASSTHROUGH) || (pid == P7501_PAY_JEDEC_NVRAM_ERASE)) && !confirmed) {
        p7501_cli_set_result("ERR", "ECONFIRM");
        printf("PAYLOAD: ERROR (state-modifying payload '%s' requires explicit CONFIRM token)\n",
               info->desc.name);
        return;
    }
    if (pid == P7501_PAY_VIDEO_MMIO_DUMP) {
        static p7501_mmio_snapshot_t s_vid_snap;
        uint32_t valid_cnt = 0u;
        if (!p7501_mmio_snap_domain_hw(P7501_SSOT_MMIO_DOM_VIDEO, &s_vid_snap, &valid_cnt) ||
            (valid_cnt == 0u)) {
            p7501_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (target offline for live VIDEO MMIO dump)\n");
            return;
        }
        printf("[PAYLOAD RUN] video_mmio_dump complete (valid_regs=%" PRIu32 ")\n", valid_cnt);
        return;
    }
    if (pid == P7501_PAY_FAST_BGO_STREAM) {
        if (!p7501_hw_is_target_powered()) {
            p7501_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (fast_bgo_stream requires active PANAMON1>)\n");
            return;
        }
        bool uploaded = false;
        p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
        const bool entered = p7501_fast_stub_upload_and_enter_open(&uploaded);
        if (!entered) {
            p7501_pio_end_session();
            p7501_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (fast_bgo_stream requires active PANAMON1>)\n");
            return;
        }
        const bool exited = p7501_fast_stub_exit_open();
        p7501_pio_end_session();
        if (!exited) {
            p7501_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (fast_bgo_stream failed returning to PANAMON1>)\n");
            return;
        }
        printf("[PAYLOAD RUN] fast_bgo_stream verified live (uploaded=%u exited=1 crc32=0x%08" PRIX32 ")\n",
               uploaded ? 1u : 0u, info->crc32);
        return;
    }
    if (pid == P7501_PAY_JEDEC_PROBE_ID) {
        uint32_t cfi_sz = 0u;
        if (!p7501_hw_query_cfi(&cfi_sz)) {
            p7501_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (jedec_probe_id failed or monitor offline)\n");
            return;
        }
        printf("[PAYLOAD RUN] jedec_probe_id CFI size=%" PRIu32 " bytes\n", cfi_sz);
        return;
    }
    if (pid == P7501_PAY_TBC_PASSTHROUGH) {
        if (!p7501_hw_is_target_online()) {
            p7501_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (tbc_passthrough requires active PANAMON1>)\n");
            return;
        }
        p7501_hw_write_mem32(P7501_SSOT_AIOF_BASE + 0x64UL, 0x80010020UL);
        p7501_hw_write_mem32(P7501_SSOT_AIOF_BASE + 0xA4UL, 0x00004000UL);
        printf("[PAYLOAD RUN] tbc_passthrough applied (AIOF_YC3V=0x80010020 AIOF_CONT=0x00004000)\n");
        return;
    }
    p7501_cli_set_result("ERR", "EPERM");
    printf("PAYLOAD: ERROR (use PANARSP NVRAM_ERASE for audited multi-sector erase)\n");
}

void p7501_cmd_payload(const char *arg) {
    if ((arg == NULL) || (*arg == '\0') || p7501_ascii_ieq(arg, "LIST")) {
        handle_payload_list();
        return;
    }
    char copy[128];
    char *toks[3] = { NULL, NULL, NULL };
    const size_t n = p7501_copy_and_split_ws(arg, copy, sizeof(copy), toks, 3u);
    if ((n >= 1u) && p7501_ascii_ieq(toks[0], "VERIFY")) {
        handle_payload_verify((n >= 2u) ? toks[1] : NULL);
        return;
    }
    if ((n >= 2u) && p7501_ascii_ieq(toks[0], "INFO")) {
        p7501_payload_id_t pid = P7501_PAY_FAST_BGO_STREAM;
        p7501_payload_info_t info;
        if (resolve_payload_arg(toks[1], &pid, &info)) {
            printf("[PAYLOAD INFO] name=%s load_va=0x%08" PRIX32 " len=%u crc32=0x%08" PRIX32 " safety=%s desc=%s\n",
                   info.desc.name, info.desc.load_va, (unsigned)info.len, info.crc32,
                   info.desc.safety, info.desc.summary);
        }
        return;
    }
    if ((n >= 2u) && p7501_ascii_ieq(toks[0], "RUN")) {
        p7501_payload_id_t pid = P7501_PAY_FAST_BGO_STREAM;
        p7501_payload_info_t info;
        if (resolve_payload_arg(toks[1], &pid, &info)) {
            handle_payload_run(pid, &info, (n == 3u) && p7501_ascii_ieq(toks[2], "CONFIRM"));
        }
        return;
    }
    p7501_cli_set_result("ERR", "EINVAL");
    printf("PAYLOAD: ERROR (Usage: PAYLOAD [LIST|INFO <name>|VERIFY [name|ALL]|RUN <name> [CONFIRM]])\n");
}
