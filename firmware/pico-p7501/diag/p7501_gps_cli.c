/**
 * @file p7501_gps_cli.c
 * @brief Pico CLI handlers `GPS` and `ROUTE` + shared checked read & telemetry emitters.
 *
 * (PiE-OS `XRAY` handler lives separately in `diag/p7501_xray_cli.c`.)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_gps_cli.h"

#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "common/p7501_push_stream.h"
#include "crypto/p7501_feistel.h"
#include "diag/p7501_i2c_codec.h"
#include "flash/p7501_flash.h"
#include "flash/p7501_payloads.h"
#include "generated/p7501_ssot.h"
#include "hal/p7501_hw.h"
#include "p7501_types.h"
#include "uart/p7501_pana_dumpnor.h"

#include "hardware/gpio.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static p7501_gps_env_t s_gps_env;
static bool s_gps_init = false;

p7501_gps_env_t *p7501_gps_global_env(void) {
    if (!s_gps_init) {
        p7501_gps_env_init(&s_gps_env);
        s_gps_init = true;
    }
    return &s_gps_env;
}

void p7501_gps_sync_from_classifier(const p7501_boot_classifier_t *cls) {
    p7501_gps_calibrate_from_classifier(p7501_gps_global_env(), cls);
}

void p7501_gps_emit_tag(const char *tag, const char *line) {
    printf("%s\n", line);
    const char *kv = strchr(line, ' ');
    kv = (kv != NULL) ? (kv + 1) : line;
    (void)p7501_push_emit_str(tag, kv);
    (void)p7501_push_flush_global();
}

static void emit_gps_env_ok(const p7501_gps_env_t *env) {
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    (void)p7501_gps_format_env(env, msg, 256u);
    p7501_gps_emit_tag("GPS_ENV", msg);
    printf("GPS: OK\n");
}

static void refresh_gps_live_sensors(p7501_gps_env_t *env) {
    p7501_hw_enforce_nsw5v_hiz();
    env->vdd_mv = p7501_hw_read_vdd_mv();
    env->nsw5v_high = gpio_get(P7501_PIN_NSW5V);
    env->monitor_online = p7501_hw_is_target_online();
    env->calibrated_mask |= P7501_CAL_ELEC;

    static const uint8_t k_addrs[4] = {
        P7501_SSOT_IC3001_BR_NA_ADDR7_L,
        P7501_SSOT_IC3001_BR_NA_ADDR7_H,
        P7501_SSOT_IC3001_EC_ADDR7,
        0x50u,
    };
    env->ic3001_mask = p7501_i2c_probe_addrs_safe(k_addrs, 4u);
    if (env->vdd_mv >= P7501_SSOT_VDD_WAKE_MIN_MV) {
        env->calibrated_mask |= P7501_CAL_IC3001;
    }
    if (env->monitor_online) {
        uint8_t *const probe8 = g_p7501_cmd_arena.raw;
        size_t kidx = 0u;
        if (p7501_hw_read_block_checked(0x84080000UL, probe8, 8u, true)) {
            uint8_t plain8[8];
            if (p7501_feistel_autodetect_key(probe8, plain8, &kidx)) {
                env->feistel_key_idx = (int8_t)kidx;
            }
        }
    }
}

bool p7501_gps_read_window_checked(
    const char *cmd_tag,
    uint32_t va,
    uint32_t len,
    uint8_t *dst
) {
    if ((len == 0u) || (len > 4096u) || (dst == NULL)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("%s: ERROR (len must be 1..4096 bytes)\n", cmd_tag);
        return false;
    }
    p7501_trap_audit_result_t tr;
    (void)p7501_trap_check_full(
        p7501_session_default_ctx(),
        p7501_flash_get_active_size(),
        va,
        len,
        P7501_SSOT_TRAP_R,
        p7501_flash_is_upper_nor_unlocked(),
        &tr
    );
    if (tr.verdict == P7501_TRAP_DENY) {
        p7501_cli_set_result("ERR", "E05");
        printf("%s: DENIED (%s)\n", cmd_tag, tr.bus_deny);
        return false;
    }
    if (!p7501_hw_is_target_online() || !p7501_hw_read_block_checked(va, dst, len, true)) {
        p7501_cli_set_result("ERR", "EOFFLINE");
        printf("%s: ERROR (monitor offline or read failed @ 0x%08" PRIX32 ")\n", cmd_tag, va);
        return false;
    }
    return true;
}

static void handle_gps_window_analysis(
    p7501_gps_env_t *env,
    const char *subcmd,
    uint32_t va,
    uint32_t len
) {
    uint8_t *const buf = g_p7501_cmd_arena.raw;
    if (!p7501_gps_read_window_checked("GPS", va, len, buf)) {
        return;
    }
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    if (p7501_ascii_ieq(subcmd, "PATTERNS")) {
        uint32_t *const hist = (uint32_t *)(void *)&g_p7501_cmd_arena.raw[4096u];
        p7501_math_pattern_report_t *const rep =
            (p7501_math_pattern_report_t *)(void *)&g_p7501_cmd_arena.raw[8192u];
        (void)p7501_math_scan_patterns(buf, (size_t)len, hist, rep);
        (void)snprintf(
            msg, 224u,
            "va=0x%08" PRIX32 " kind=%s mbit=%" PRIu32 " chi2_q8=%" PRIu32
            " mirror=%" PRIu32 " ecb64=%u sbox=%u@0x%03X",
            va, p7501_block_kind_name(rep->stats.kind), rep->stats.entropy_mbit,
            rep->chi2_q8, rep->mirror_period, (unsigned)rep->ecb64_repeats,
            rep->sbox_256_found ? 1u : 0u, (unsigned)rep->sbox_offset
        );
        (void)p7501_push_emit_str("GPS_PATTERNS", msg);
        (void)p7501_push_flush_global();
        printf("[GPS PATTERNS] %s\nGPS: OK\n", msg);
        return;
    }
    if (p7501_ascii_ieq(subcmd, "ENTROPY")) {
        uint32_t *const hist = (uint32_t *)(void *)&g_p7501_cmd_arena.raw[4096u];
        p7501_math_block_stats_t *const st =
            (p7501_math_block_stats_t *)(void *)&g_p7501_cmd_arena.raw[8192u];
        (void)p7501_math_analyze_block(buf, (size_t)len, hist, st);
        if (st->feistel_key_idx >= 0) {
            env->feistel_key_idx = st->feistel_key_idx;
        }
        (void)snprintf(
            msg, 224u,
            "va=0x%08" PRIX32 " len=%" PRIu32 " class=%s mbit=%" PRIu32
            " crc32=0x%08" PRIX32 " ff=%u zero=%u ascii=%u mn103=%u swap16=%u zlib=%u kidx=%d",
            va, len, p7501_block_kind_name(st->kind), st->entropy_mbit, st->crc32,
            (unsigned)st->ff_pct, (unsigned)st->zero_pct, (unsigned)st->ascii_pct,
            (unsigned)st->mn103_hits, st->byte_swapped_16 ? 1u : 0u,
            st->zlib_header ? 1u : 0u, (int)st->feistel_key_idx
        );
        (void)p7501_push_emit_str("GPS_ENTROPY", msg);
        (void)p7501_push_flush_global();
        printf("[GPS ENTROPY] %s\nGPS: OK\n", msg);
        return;
    }
    p7501_mn103_edge_t *const edges = (p7501_mn103_edge_t *)(void *)&g_p7501_cmd_arena.raw[4096u];
    const size_t total = p7501_mn103_scan_edges(buf, (size_t)len, va, edges, 32u);
    const size_t shown = (total > 32u) ? 32u : total;
    for (size_t i = 0u; i < shown; ++i) {
        size_t aidx = 0u;
        const bool has_sym = p7501_gps_find_anchor_by_va(env, edges[i].target_va, &aidx);
        (void)snprintf(
            msg, 192u, "kind=%s site=0x%08" PRIX32 " target=0x%08" PRIX32 " sym=%s",
            edges[i].label, edges[i].site_va, edges[i].target_va,
            has_sym ? k_p7501_ssot_pieos_anchors[aidx].name : "NONE"
        );
        (void)p7501_push_emit_str("GPS_EDGE", msg);
        printf("[GPS EDGE] %s\n", msg);
    }
    (void)p7501_push_flush_global();
    printf("GPS: OK (edges=%u shown=%u)\n", (unsigned)total, (unsigned)shown);
}

void p7501_cmd_gps(const char *arg) {
    p7501_gps_env_t *const env = p7501_gps_global_env();
    char *const copy = (char *)&g_p7501_cmd_arena.raw[12288u];
    char *toks[4] = { NULL, NULL, NULL, NULL };
    const size_t n = p7501_copy_and_split_ws(arg, copy, 128u, toks, 4u);

    if ((n == 0u) || p7501_ascii_ieq(toks[0], "ENV") || p7501_ascii_ieq(toks[0], "CAL")) {
        if ((n > 0u) && p7501_ascii_ieq(toks[0], "CAL")) {
            refresh_gps_live_sensors(env);
        }
        emit_gps_env_ok(env);
        return;
    }
    if ((n >= 2u) && p7501_ascii_ieq(toks[0], "AUTOCAL")) {
        if (!p7501_gps_autocal_model_suffix(env, toks[1])) {
            p7501_cli_set_result("ERR", "ENOENT");
            printf("GPS: ERROR (unknown model/suffix '%s')\n", toks[1]);
            return;
        }
        emit_gps_env_ok(env);
        return;
    }
    if ((n >= 2u) && p7501_ascii_ieq(toks[0], "RESOLVE")) {
        size_t idx = 0u;
        uint32_t query_va = 0u;
        bool matched = p7501_gps_find_anchor_by_name(toks[1], &idx);
        if (!matched && p7501_dumpnor_parse_hex_tok(toks[1], &query_va)) {
            matched = p7501_gps_find_anchor_by_va(env, query_va, &idx);
        }
        if (!matched) {
            p7501_cli_set_result("ERR", "ENOENT");
            printf("GPS: ERROR (unknown PiE-OS anchor '%s')\n", toks[1]);
            return;
        }
        const p7501_ssot_pieos_anchor_t *const a = &k_p7501_ssot_pieos_anchors[idx];
        char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
        (void)snprintf(
            msg, 224u,
            "name=%s va=0x%08" PRIX32 " mmio=0x%08" PRIX32 " stage=S%u caller=%s build=%s delta=%" PRId32,
            a->name, p7501_gps_resolve_anchor(env, idx), a->mmio_ref, (unsigned)a->stage_id,
            a->caller, p7501_gps_build_name(env->main_build), env->affine_delta
        );
        (void)p7501_push_emit_str("GPS_RESOLVE", msg);
        (void)p7501_push_flush_global();
        printf("[GPS RESOLVE] %s\nGPS: OK\n", msg);
        return;
    }
    if ((n == 3u) && (p7501_ascii_ieq(toks[0], "ENTROPY") ||
                      p7501_ascii_ieq(toks[0], "PATTERNS") ||
                      p7501_ascii_ieq(toks[0], "CALLS"))) {
        uint32_t va = 0u, len = 0u;
        if (!p7501_dumpnor_parse_hex_tok(toks[1], &va) || !p7501_parse_u32_auto(toks[2], &len)) {
            p7501_cli_set_result("ERR", "EINVAL");
            printf("GPS: ERROR (Usage: GPS <ENTROPY|PATTERNS|CALLS> <va_hex> <1..4096>)\n");
            return;
        }
        handle_gps_window_analysis(env, toks[0], va, len);
        return;
    }
    p7501_cli_set_result("ERR", "EINVAL");
    printf("GPS: ERROR (Usage: GPS <CAL|ENV|AUTOCAL <m+s>|PATTERNS|ENTROPY|CALLS|RESOLVE>)\n");
}

void p7501_cmd_route(const char *arg) {
    char copy[32];
    char *toks[2] = { NULL, NULL };
    const size_t n = p7501_copy_and_split_ws(arg, copy, sizeof(copy), toks, 2u);
    p7501_route_domain_t dom = P7501_ROUTE_BOOT;
    if (!p7501_gps_parse_route_domain((n > 0u) ? toks[0] : NULL, &dom)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("ROUTE: ERROR (Usage: ROUTE [BOOT|VIDEO|BGO|HDD|NVRAM|UNBRICK])\n");
        return;
    }
    char *const msg = (char *)&g_p7501_cmd_arena.raw[12544u];
    (void)p7501_gps_build_route(p7501_gps_global_env(), dom, msg, 256u);
    p7501_gps_emit_tag("ROUTE", msg);
    printf("ROUTE: OK\n");
}
