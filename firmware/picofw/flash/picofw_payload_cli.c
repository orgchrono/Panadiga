/**
 * @file picofw_payload_cli.c
 * @brief Pico hardware CLI handlers `TRAP` and `PAYLOAD` (imperative shell).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_payloads.h"

#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "diag/picofw_mmio_snapshot_codec.h"
#include "flash/picofw_fast_stub_io.h"
#include "flash/picofw_flash.h"
#include "hal/picofw_hw.h"
#include "uart/picofw_pana_dumpnor.h"
#include "uart/picofw_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static bool resolve_payload_arg(
    const char *name,
    picofw_payload_id_t *out_pid,
    picofw_payload_info_t *out_info
) {
    if (!picofw_payload_find(name, out_pid) ||
        !picofw_payload_build_and_verify(*out_pid, g_picofw_cmd_arena.raw, 2048u, out_info)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("PAYLOAD: ERROR (unknown payload '%s')\n", name);
        return false;
    }
    return true;
}

static const char *verdict_name(picofw_trap_verdict_t v) {
    if (v == PICOFW_TRAP_DENY) {
        return "DENY";
    }
    if (v == PICOFW_TRAP_WARN) {
        return "WARN";
    }
    return "ALLOW";
}

void picofw_cmd_trap(const char *arg) {
    if ((arg == NULL) || (*arg == '\0') || picofw_ascii_ieq(arg, "LIST")) {
        char buf[192];
        for (size_t i = 0u; i < PICOFW_SSOT_TRAP_COUNT; ++i) {
            const picofw_ssot_trap_t *const tr = &k_picofw_ssot_traps[i];
            (void)snprintf(
                buf, sizeof(buf),
                "id=%s start=0x%08" PRIX32 " len=0x%08" PRIX32 " mask=0x%02X sev=%s reason=%s",
                tr->id, tr->start, tr->len, (unsigned)tr->mask_rwe,
                (tr->severity >= PICOFW_SSOT_TRAP_SEV_DENY) ? "DENY" : "WARN", tr->reason
            );
            (void)picofw_push_emit_str("TRAP", buf);
            printf("[TRAP #%u] %s\n", (unsigned)(i + 1u), buf);
        }
        (void)picofw_push_flush_global();
        return;
    }

    char copy[128];
    char *toks[4] = { NULL, NULL, NULL, NULL };
    const size_t n = picofw_copy_and_split_ws(arg, copy, sizeof(copy), toks, 4u);

    if ((n != 4u) || !picofw_ascii_ieq(toks[0], "CHECK")) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("TRAP: ERROR (Usage: TRAP LIST | TRAP CHECK <addr_hex> <len_hex> <R|W|E>)\n");
        return;
    }

    uint32_t addr = 0u;
    uint32_t len = 0u;
    const uint8_t op_mask = picofw_trap_parse_op(toks[3]);
    if (!picofw_dumpnor_parse_hex_tok(toks[1], &addr) ||
        !picofw_dumpnor_parse_hex_tok(toks[2], &len) ||
        (op_mask == 0u)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("TRAP: ERROR (invalid addr/len/op)\n");
        return;
    }

    picofw_trap_audit_result_t res;
    (void)picofw_trap_check_full(
        picofw_session_default_ctx(),
        picofw_flash_get_active_size(),
        addr,
        len,
        op_mask,
        picofw_flash_is_upper_nor_unlocked(),
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
    (void)picofw_push_emit_str("TRAP", msg);
    (void)picofw_push_flush_global();
    printf("[TRAP] CHECK %s\n", msg);

    if (res.verdict == PICOFW_TRAP_DENY) {
        picofw_cli_set_result("ERR", "E05");
    }
}

static void handle_payload_list(void) {
    for (size_t i = 0u; i < PICOFW_PAY_COUNT; ++i) {
        picofw_payload_info_t info;
        picofw_payload_desc_t desc;
        if (picofw_payload_describe(i, &desc) &&
            picofw_payload_build_and_verify(desc.id, g_picofw_cmd_arena.raw, 2048u, &info)) {
            char msg[192];
            (void)snprintf(
                msg, sizeof(msg),
                "name=%s load_va=0x%08" PRIX32 " len=%u crc32=0x%08" PRIX32 " safety=%s",
                info.desc.name, info.desc.load_va, (unsigned)info.len, info.crc32,
                info.desc.safety
            );
            (void)picofw_push_emit_str("PAYLOAD", msg);
            printf("[PAYLOAD #%u] %s | %s\n", (unsigned)(i + 1u), msg, info.desc.summary);
        }
    }
    (void)picofw_push_flush_global();
}

static void handle_payload_verify(const char *filter) {
    const bool check_all = (filter == NULL) || picofw_ascii_ieq(filter, "ALL");
    bool all_ok = true;
    uint32_t checked = 0u;
    for (size_t i = 0u; i < PICOFW_PAY_COUNT; ++i) {
        picofw_payload_desc_t desc;
        if (!picofw_payload_describe(i, &desc)) {
            continue;
        }
        if (!check_all && !picofw_ascii_ieq(filter, desc.name)) {
            continue;
        }
        picofw_payload_info_t info;
        const bool ok = picofw_payload_build_and_verify(
            desc.id, g_picofw_cmd_arena.raw, 2048u, &info
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
        (void)picofw_push_emit_str("PAYLOAD", msg);
        printf("[PAYLOAD VERIFY] %s\n", msg);
    }
    (void)picofw_push_flush_global();
    if (!all_ok || (checked == 0u)) {
        picofw_cli_set_result("ERR", (checked == 0u) ? "EINVAL" : "ECRC");
    }
}

static void handle_payload_run(picofw_payload_id_t pid, const picofw_payload_info_t *info, bool confirmed) {
    if (((pid == PICOFW_PAY_TBC_PASSTHROUGH) || (pid == PICOFW_PAY_JEDEC_NVRAM_ERASE)) && !confirmed) {
        picofw_cli_set_result("ERR", "ECONFIRM");
        printf("PAYLOAD: ERROR (state-modifying payload '%s' requires explicit CONFIRM token)\n",
               info->desc.name);
        return;
    }
    if (pid == PICOFW_PAY_VIDEO_MMIO_DUMP) {
        static picofw_mmio_snapshot_t s_vid_snap;
        uint32_t valid_cnt = 0u;
        if (!picofw_mmio_snap_domain_hw(PICOFW_SSOT_MMIO_DOM_VIDEO, &s_vid_snap, &valid_cnt) ||
            (valid_cnt == 0u)) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (target offline for live VIDEO MMIO dump)\n");
            return;
        }
        printf("[PAYLOAD RUN] video_mmio_dump complete (valid_regs=%" PRIu32 ")\n", valid_cnt);
        return;
    }
    if (pid == PICOFW_PAY_FAST_BGO_STREAM) {
        if (!picofw_hw_is_rail_healthy()) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (fast_bgo_stream requires active PANAMON1>)\n");
            return;
        }
        bool uploaded = false;
        picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
        const bool entered = picofw_fast_stub_upload_and_enter_open(&uploaded);
        if (!entered) {
            picofw_pio_end_session();
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (fast_bgo_stream requires active PANAMON1>)\n");
            return;
        }
        const bool exited = picofw_fast_stub_exit_open();
        picofw_pio_end_session();
        if (!exited) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (fast_bgo_stream failed returning to PANAMON1>)\n");
            return;
        }
        printf("[PAYLOAD RUN] fast_bgo_stream verified live (uploaded=%u exited=1 crc32=0x%08" PRIX32 ")\n",
               uploaded ? 1u : 0u, info->crc32);
        return;
    }
    if (pid == PICOFW_PAY_JEDEC_PROBE_ID) {
        uint16_t mfg = 0u;
        uint16_t dev[3] = { 0u, 0u, 0u };
        bool jedec_ok = false;
        if (!picofw_hw_query_jedec_id(&mfg, dev, &jedec_ok)) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (jedec_probe_id failed or monitor offline)\n");
            return;
        }
        printf("[PAYLOAD RUN] jedec_probe_id mfg=0x%04X dev=0x%04X:0x%04X:0x%04X verified=%s\n",
               (unsigned)mfg, (unsigned)dev[0], (unsigned)dev[1], (unsigned)dev[2],
               jedec_ok ? "YES" : "NO");
        return;
    }
    if (pid == PICOFW_PAY_TBC_PASSTHROUGH) {
        if (!picofw_hw_is_rail_healthy() || !picofw_hw_is_target_online()) {
            picofw_cli_set_result("ERR", "EOFFLINE");
            printf("PAYLOAD: ERROR (tbc_passthrough requires active PANAMON1>)\n");
            return;
        }
        picofw_hw_write_mem32(PICOFW_SSOT_AIOF_BASE + 0x64UL, 0x80010020UL);
        picofw_hw_write_mem32(PICOFW_SSOT_AIOF_BASE + 0xA4UL, 0x00004000UL);
        printf("[PAYLOAD RUN] tbc_passthrough applied (AIOF_YC3V=0x80010020 AIOF_CONT=0x00004000)\n");
        return;
    }
    picofw_cli_set_result("ERR", "EPERM");
    printf("PAYLOAD: ERROR (use PANARSP NVRAM_ERASE for audited multi-sector erase)\n");
}

void picofw_cmd_payload(const char *arg) {
    if ((arg == NULL) || (*arg == '\0') || picofw_ascii_ieq(arg, "LIST")) {
        handle_payload_list();
        return;
    }
    char copy[128];
    char *toks[3] = { NULL, NULL, NULL };
    const size_t n = picofw_copy_and_split_ws(arg, copy, sizeof(copy), toks, 3u);
    if ((n >= 1u) && picofw_ascii_ieq(toks[0], "VERIFY")) {
        handle_payload_verify((n >= 2u) ? toks[1] : NULL);
        return;
    }
    if ((n >= 2u) && picofw_ascii_ieq(toks[0], "INFO")) {
        picofw_payload_id_t pid = PICOFW_PAY_FAST_BGO_STREAM;
        picofw_payload_info_t info;
        if (resolve_payload_arg(toks[1], &pid, &info)) {
            printf("[PAYLOAD INFO] name=%s load_va=0x%08" PRIX32 " len=%u crc32=0x%08" PRIX32 " safety=%s desc=%s\n",
                   info.desc.name, info.desc.load_va, (unsigned)info.len, info.crc32,
                   info.desc.safety, info.desc.summary);
        }
        return;
    }
    if ((n >= 2u) && picofw_ascii_ieq(toks[0], "RUN")) {
        picofw_payload_id_t pid = PICOFW_PAY_FAST_BGO_STREAM;
        picofw_payload_info_t info;
        if (resolve_payload_arg(toks[1], &pid, &info)) {
            handle_payload_run(pid, &info, (n == 3u) && picofw_ascii_ieq(toks[2], "CONFIRM"));
        }
        return;
    }
    picofw_cli_set_result("ERR", "EINVAL");
    printf("PAYLOAD: ERROR (Usage: PAYLOAD [LIST|INFO <name>|VERIFY [name|ALL]|RUN <name> [CONFIRM]])\n");
}
