/**
 * @file picofw_gps_cli.c
 * @brief Pico CLI handlers `GPS` and `ROUTE` + shared checked read & telemetry emitters.
 *
 * (PiE-OS `XRAY` handler lives separately in `diag/picofw_xray_cli.c`.)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_gps_cli.h"

#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "crypto/picofw_feistel.h"
#include "diag/picofw_i2c_codec.h"
#include "flash/picofw_flash.h"
#include "flash/picofw_payloads.h"
#include "generated/picofw_ssot.h"
#include "hal/picofw_hw.h"
#include "picofw_types.h"
#include "uart/picofw_pana_dumpnor.h"

#include "hardware/gpio.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static picofw_gps_env_t s_gps_env;
static bool s_gps_init = false;

picofw_gps_env_t *picofw_gps_global_env(void) {
    if (!s_gps_init) {
        picofw_gps_env_init(&s_gps_env);
        s_gps_init = true;
    }
    return &s_gps_env;
}

void picofw_gps_sync_from_classifier(const picofw_boot_classifier_t *cls) {
    picofw_gps_calibrate_from_classifier(picofw_gps_global_env(), cls);
}

void picofw_gps_emit_tag(const char *tag, const char *line) {
    printf("%s\n", line);
    const char *kv = strchr(line, ' ');
    kv = (kv != NULL) ? (kv + 1) : line;
    (void)picofw_push_emit_str(tag, kv);
    (void)picofw_push_flush_global();
}

static void emit_gps_env_ok(const picofw_gps_env_t *env) {
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)picofw_gps_format_env(env, msg, 256u);
    picofw_gps_emit_tag("GPS_ENV", msg);
    printf("GPS: OK\n");
}

static void refresh_gps_live_sensors(picofw_gps_env_t *env) {
    picofw_hw_enforce_nsw5v_hiz();
    env->vdd_mv = picofw_hw_read_vdd_mv();
    env->nsw5v_high = gpio_get(P7501_PIN_NSW5V);
    env->monitor_online = picofw_hw_is_target_online();
    env->calibrated_mask |= PICOFW_CAL_ELEC;

    static const uint8_t k_addrs[4] = {
        PICOFW_SSOT_IC3001_BR_NA_ADDR7_L,
        PICOFW_SSOT_IC3001_BR_NA_ADDR7_H,
        PICOFW_SSOT_IC3001_EC_ADDR7,
        0x50u,
    };
    env->ic3001_mask = picofw_i2c_probe_addrs_safe(k_addrs, 4u);
    if (env->vdd_mv >= PICOFW_SSOT_VDD_WAKE_MIN_MV) {
        env->calibrated_mask |= PICOFW_CAL_IC3001;
    }
    if (env->monitor_online) {
        uint8_t *const probe8 = g_picofw_cmd_arena.raw;
        size_t kidx = 0u;
        if (picofw_hw_read_block_checked(0x84080000UL, probe8, 8u, true)) {
            uint8_t plain8[8];
            if (picofw_feistel_autodetect_key(probe8, plain8, &kidx)) {
                env->feistel_key_idx = (int8_t)kidx;
            }
        }
    }
}

bool picofw_gps_read_window_checked(
    const char *cmd_tag,
    uint32_t va,
    uint32_t len,
    uint8_t *dst
) {
    if ((len == 0u) || (len > 4096u) || (dst == NULL)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("%s: ERROR (len must be 1..4096 bytes)\n", cmd_tag);
        return false;
    }
    picofw_trap_audit_result_t tr;
    (void)picofw_trap_check_full(
        picofw_session_default_ctx(),
        picofw_flash_get_active_size(),
        va,
        len,
        PICOFW_SSOT_TRAP_R,
        picofw_flash_is_upper_nor_unlocked(),
        &tr
    );
    if (tr.verdict == PICOFW_TRAP_DENY) {
        picofw_cli_set_result("ERR", "E05");
        printf("%s: DENIED (%s)\n", cmd_tag, tr.bus_deny);
        return false;
    }
    if (!picofw_hw_is_target_online() || !picofw_hw_read_block_checked(va, dst, len, true)) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("%s: ERROR (monitor offline or read failed @ 0x%08" PRIX32 ")\n", cmd_tag, va);
        return false;
    }
    return true;
}

static void handle_gps_window_analysis(
    picofw_gps_env_t *env,
    const char *subcmd,
    uint32_t va,
    uint32_t len
) {
    uint8_t *const buf = g_picofw_cmd_arena.raw;
    if (!picofw_gps_read_window_checked("GPS", va, len, buf)) {
        return;
    }
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    if (picofw_ascii_ieq(subcmd, "PATTERNS")) {
        uint32_t *const hist = (uint32_t *)(void *)&g_picofw_cmd_arena.raw[4096u];
        picofw_math_pattern_report_t *const rep =
            (picofw_math_pattern_report_t *)(void *)&g_picofw_cmd_arena.raw[8192u];
        (void)picofw_math_scan_patterns(buf, (size_t)len, hist, rep);
        (void)snprintf(
            msg, 224u,
            "va=0x%08" PRIX32 " kind=%s mbit=%" PRIu32 " chi2_q8=%" PRIu32
            " mirror=%" PRIu32 " ecb64=%u sbox=%u@0x%03X",
            va, picofw_block_kind_name(rep->stats.kind), rep->stats.entropy_mbit,
            rep->chi2_q8, rep->mirror_period, (unsigned)rep->ecb64_repeats,
            rep->sbox_256_found ? 1u : 0u, (unsigned)rep->sbox_offset
        );
        (void)picofw_push_emit_str("GPS_PATTERNS", msg);
        (void)picofw_push_flush_global();
        printf("[GPS PATTERNS] %s\nGPS: OK\n", msg);
        return;
    }
    if (picofw_ascii_ieq(subcmd, "ENTROPY")) {
        uint32_t *const hist = (uint32_t *)(void *)&g_picofw_cmd_arena.raw[4096u];
        picofw_math_block_stats_t *const st =
            (picofw_math_block_stats_t *)(void *)&g_picofw_cmd_arena.raw[8192u];
        (void)picofw_math_analyze_block(buf, (size_t)len, hist, st);
        if (st->feistel_key_idx >= 0) {
            env->feistel_key_idx = st->feistel_key_idx;
        }
        (void)snprintf(
            msg, 224u,
            "va=0x%08" PRIX32 " len=%" PRIu32 " class=%s mbit=%" PRIu32
            " crc32=0x%08" PRIX32 " ff=%u zero=%u ascii=%u mn103=%u swap16=%u zlib=%u kidx=%d",
            va, len, picofw_block_kind_name(st->kind), st->entropy_mbit, st->crc32,
            (unsigned)st->ff_pct, (unsigned)st->zero_pct, (unsigned)st->ascii_pct,
            (unsigned)st->mn103_hits, st->byte_swapped_16 ? 1u : 0u,
            st->zlib_header ? 1u : 0u, (int)st->feistel_key_idx
        );
        (void)picofw_push_emit_str("GPS_ENTROPY", msg);
        (void)picofw_push_flush_global();
        printf("[GPS ENTROPY] %s\nGPS: OK\n", msg);
        return;
    }
    picofw_mn103_edge_t *const edges = (picofw_mn103_edge_t *)(void *)&g_picofw_cmd_arena.raw[4096u];
    const size_t total = picofw_mn103_scan_edges(buf, (size_t)len, va, edges, 32u);
    const size_t shown = (total > 32u) ? 32u : total;
    for (size_t i = 0u; i < shown; ++i) {
        size_t aidx = 0u;
        const bool has_sym = picofw_gps_find_anchor_by_va(env, edges[i].target_va, &aidx);
        (void)snprintf(
            msg, 192u, "kind=%s site=0x%08" PRIX32 " target=0x%08" PRIX32 " sym=%s",
            edges[i].label, edges[i].site_va, edges[i].target_va,
            has_sym ? k_picofw_ssot_pieos_anchors[aidx].name : "NONE"
        );
        (void)picofw_push_emit_str("GPS_EDGE", msg);
        printf("[GPS EDGE] %s\n", msg);
    }
    (void)picofw_push_flush_global();
    printf("GPS: OK (edges=%u shown=%u)\n", (unsigned)total, (unsigned)shown);
}

void picofw_cmd_gps(const char *arg) {
    picofw_gps_env_t *const env = picofw_gps_global_env();
    char *const copy = (char *)&g_picofw_cmd_arena.raw[12288u];
    char *toks[4] = { NULL, NULL, NULL, NULL };
    const size_t n = picofw_copy_and_split_ws(arg, copy, 128u, toks, 4u);

    if ((n == 0u) || picofw_ascii_ieq(toks[0], "ENV") || picofw_ascii_ieq(toks[0], "CAL")) {
        if ((n > 0u) && picofw_ascii_ieq(toks[0], "CAL")) {
            refresh_gps_live_sensors(env);
        }
        emit_gps_env_ok(env);
        return;
    }
    if ((n >= 2u) && picofw_ascii_ieq(toks[0], "AUTOCAL")) {
        if (!picofw_gps_autocal_model_suffix(env, toks[1])) {
            picofw_cli_set_result("ERR", "ENOENT");
            printf("GPS: ERROR (unknown model/suffix '%s')\n", toks[1]);
            return;
        }
        emit_gps_env_ok(env);
        return;
    }
    if ((n >= 2u) && picofw_ascii_ieq(toks[0], "RESOLVE")) {
        size_t idx = 0u;
        uint32_t query_va = 0u;
        bool matched = picofw_gps_find_anchor_by_name(toks[1], &idx);
        if (!matched && picofw_dumpnor_parse_hex_tok(toks[1], &query_va)) {
            matched = picofw_gps_find_anchor_by_va(env, query_va, &idx);
        }
        if (!matched) {
            picofw_cli_set_result("ERR", "ENOENT");
            printf("GPS: ERROR (unknown PiE-OS anchor '%s')\n", toks[1]);
            return;
        }
        const picofw_ssot_pieos_anchor_t *const a = &k_picofw_ssot_pieos_anchors[idx];
        char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
        (void)snprintf(
            msg, 224u,
            "name=%s va=0x%08" PRIX32 " mmio=0x%08" PRIX32 " stage=S%u caller=%s build=%s delta=%" PRId32,
            a->name, picofw_gps_resolve_anchor(env, idx), a->mmio_ref, (unsigned)a->stage_id,
            a->caller, picofw_gps_build_name(env->main_build), env->affine_delta
        );
        (void)picofw_push_emit_str("GPS_RESOLVE", msg);
        (void)picofw_push_flush_global();
        printf("[GPS RESOLVE] %s\nGPS: OK\n", msg);
        return;
    }
    if ((n == 3u) && (picofw_ascii_ieq(toks[0], "ENTROPY") ||
                      picofw_ascii_ieq(toks[0], "PATTERNS") ||
                      picofw_ascii_ieq(toks[0], "CALLS"))) {
        uint32_t va = 0u, len = 0u;
        if (!picofw_dumpnor_parse_hex_tok(toks[1], &va) || !picofw_parse_u32_auto(toks[2], &len)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("GPS: ERROR (Usage: GPS <ENTROPY|PATTERNS|CALLS> <va_hex> <1..4096>)\n");
            return;
        }
        handle_gps_window_analysis(env, toks[0], va, len);
        return;
    }
    picofw_cli_set_result("ERR", "EINVAL");
    printf("GPS: ERROR (Usage: GPS <CAL|ENV|AUTOCAL <m+s>|PATTERNS|ENTROPY|CALLS|RESOLVE>)\n");
}

void picofw_cmd_route(const char *arg) {
    char copy[32];
    char *toks[2] = { NULL, NULL };
    const size_t n = picofw_copy_and_split_ws(arg, copy, sizeof(copy), toks, 2u);
    picofw_route_domain_t dom = PICOFW_ROUTE_BOOT;
    if (!picofw_gps_parse_route_domain((n > 0u) ? toks[0] : NULL, &dom)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("ROUTE: ERROR (Usage: ROUTE [BOOT|VIDEO|BGO|HDD|NVRAM|UNBRICK])\n");
        return;
    }
    char *const msg = (char *)&g_picofw_cmd_arena.raw[12544u];
    (void)picofw_gps_build_route(picofw_gps_global_env(), dom, msg, 256u);
    picofw_gps_emit_tag("ROUTE", msg);
    printf("ROUTE: OK\n");
}
