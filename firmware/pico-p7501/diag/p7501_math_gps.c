/**
 * @file p7501_math_gps.c
 * @brief Declarative SSOT model+region auto-calibrator, affine VA solver & domain router.
 *
 * Zero hardcoded commercial model branches; all geometry & regional broadcast rules are
 * queried declaratively from SSOT `.rodata` tables (`k_p7501_ssot_target_profiles[]`,
 * `k_p7501_ssot_region_rules[]`, `k_p7501_ssot_pieos_anchors[]`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_math_gps.h"

#include "common/p7501_hex.h"
#include "flash/p7501_payloads.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static void apply_target_profile(p7501_gps_env_t *env, size_t idx) {
    const size_t safe_idx = (idx < P7501_TARGET_PROFILE_COUNT) ? idx : 0u;
    const p7501_ssot_target_profile_t *const prof = &k_p7501_ssot_target_profiles[safe_idx];
    env->profile_id = prof->profile_id;
    env->flash_bytes = prof->total_bytes;
    env->bgo_base = (prof->bgo_size > 0u) ? (P7501_SSOT_CS0_UNCACHED_BASE + prof->bgo_offset) : 0u;
    env->bgo_sectors = prof->bgo_sector_count;
    env->nvram_base = (prof->eemul_size > 0u) ? (P7501_SSOT_CS0_UNCACHED_BASE + prof->eemul_offset) : 0u;
    (void)snprintf(env->model_id, sizeof(env->model_id), "%s", prof->model_name);
}

static void apply_region_rule(p7501_gps_env_t *env, const p7501_ssot_region_rule_t *rule) {
    if ((env == NULL) || (rule == NULL)) {
        return;
    }
    env->aiof_vmodesel = rule->aiof_vmodesel;
    env->dvd_region = rule->dvd_region;
    env->ic3001_expected_addr7 = rule->ic3001_addr7_primary;
    (void)snprintf(env->region_suffix, sizeof(env->region_suffix), "%s", rule->suffix);
    (void)snprintf(env->tv_standard, sizeof(env->tv_standard), "%s", rule->tv_standard);
}

void p7501_gps_env_init(p7501_gps_env_t *env) {
    if (env == NULL) {
        return;
    }
    memset(env, 0, sizeof(*env));
    env->feistel_key_idx = -1;
    apply_target_profile(env, 0u);
    apply_region_rule(env, &k_p7501_ssot_region_rules[0]);
}

static const p7501_ssot_region_rule_t *find_longest_suffix(const char *s, size_t *out_stem_len) {
    const size_t slen = strlen(s);
    const p7501_ssot_region_rule_t *best = NULL;
    size_t best_len = 0u;
    for (size_t i = 0u; i < P7501_SSOT_REGION_RULE_COUNT; ++i) {
        const char *const suf = k_p7501_ssot_region_rules[i].suffix;
        const size_t rlen = strlen(suf);
        if ((rlen <= slen) && (rlen > best_len) && p7501_ascii_ieq(&s[slen - rlen], suf)) {
            best = &k_p7501_ssot_region_rules[i];
            best_len = rlen;
        }
    }
    if ((best != NULL) && (out_stem_len != NULL)) {
        *out_stem_len = slen - best_len;
    }
    return best;
}

bool p7501_gps_parse_model_suffix(
    const char *spec,
    char *out_model,
    size_t cap_m,
    char *out_suffix,
    size_t cap_s
) {
    if ((spec == NULL) || (*spec == '\0') || (out_model == NULL) || (cap_m == 0u) ||
        (out_suffix == NULL) || (cap_s == 0u)) {
        return false;
    }
    char work[48];
    (void)snprintf(work, sizeof(work), "%s", spec);
    size_t wlen = strlen(work);
    /* Strip cosmetic chassis finish suffixes (`-S` silver, `-K` black, `-W`, `-N`). */
    if ((wlen > 2u) && (work[wlen - 2u] == '-')) {
        work[wlen - 2u] = '\0';
        wlen -= 2u;
    }

    size_t stem_len = wlen;
    const p7501_ssot_region_rule_t *rule = find_longest_suffix(work, &stem_len);
    (void)snprintf(out_suffix, cap_s, "%s", (rule != NULL) ? rule->suffix : "");
    work[stem_len] = '\0';

    /* Normalize Panasonic board HW code prefix (`M6930*` -> first matching profile). */
    if ((stem_len >= 5u) && (work[0] == 'M' || work[0] == 'm') && (work[1] == '6')) {
        (void)snprintf(out_model, cap_m, "%s", k_p7501_ssot_target_profiles[0].model_name);
        return true;
    }
    (void)snprintf(out_model, cap_m, "%s", work);
    return (out_model[0] != '\0') || (out_suffix[0] != '\0');
}

bool p7501_gps_autocal_model_suffix(p7501_gps_env_t *env, const char *spec) {
    if ((env == NULL) || (spec == NULL) || (*spec == '\0')) {
        return false;
    }
    char model[24];
    char suffix[12];
    if (!p7501_gps_parse_model_suffix(spec, model, sizeof(model), suffix, sizeof(suffix))) {
        return false;
    }
    bool matched = false;
    for (size_t i = 0u; i < P7501_TARGET_PROFILE_COUNT; ++i) {
        if (p7501_ascii_ieq(model, k_p7501_ssot_target_profiles[i].model_name)) {
            apply_target_profile(env, i);
            matched = true;
            break;
        }
    }
    for (size_t i = 0u; i < P7501_SSOT_REGION_RULE_COUNT; ++i) {
        if (p7501_ascii_ieq(suffix, k_p7501_ssot_region_rules[i].suffix)) {
            apply_region_rule(env, &k_p7501_ssot_region_rules[i]);
            matched = true;
            break;
        }
    }
    if (matched) {
        env->calibrated_mask |= P7501_CAL_PIEOS;
    }
    return matched;
}

void p7501_gps_calibrate_from_classifier(p7501_gps_env_t *env, const p7501_boot_classifier_t *cls) {
    if ((env == NULL) || (cls == NULL)) {
        return;
    }
    if (cls->main_build != (uint8_t)P7501_MAIN_BUILD_UNKNOWN) {
        env->main_build = cls->main_build;
    }
    if (cls->hw_sys[0] != '\0') {
        (void)snprintf(env->hw_sys, sizeof(env->hw_sys), "%s", cls->hw_sys);
        (void)p7501_gps_autocal_model_suffix(env, cls->hw_sys);
    }
    if (cls->market[0] != '\0') {
        (void)snprintf(env->market, sizeof(env->market), "%s", cls->market);
    }
    if (cls->timer_ver[0] != '\0') {
        (void)snprintf(env->timer_ver, sizeof(env->timer_ver), "%s", cls->timer_ver);
    }
    if (cls->has_region) {
        env->region_code = cls->region_code;
        env->has_region = true;
        if (env->region_suffix[0] == '\0') {
            for (size_t i = 0u; i < P7501_SSOT_REGION_RULE_COUNT; ++i) {
                if (k_p7501_ssot_region_rules[i].dvd_region == cls->region_code) {
                    apply_region_rule(env, &k_p7501_ssot_region_rules[i]);
                    break;
                }
            }
        }
    }
    env->avin_iic_err = cls->iic_err_cnt;
    env->bgo_err = (uint16_t)(cls->nfrom_bgo_err_cnt + cls->u99_cnt);
    env->fpc_err = (uint16_t)(cls->fpc_err_cnt + cls->fpc_to_cnt);
    env->hdd_fail = cls->hdd_lba_fail_cnt;
    env->calibrated_mask |= P7501_CAL_PIEOS;
}

static uint32_t anchor_ref_va_for_build(const p7501_ssot_pieos_anchor_t *a, uint8_t main_build) {
    if ((main_build == (uint8_t)P7501_MAIN_BUILD_3410PQ) && (a->va_3410pq != 0u)) {
        return a->va_3410pq;
    }
    return (a->va_3060np != 0u) ? a->va_3060np : a->va_3410pq;
}

bool p7501_gps_calibrate_affine(p7501_gps_env_t *env, size_t anchor_idx, uint32_t observed_va) {
    if ((env == NULL) || (anchor_idx >= P7501_SSOT_PIEOS_ANCHOR_COUNT) || (observed_va == 0u)) {
        return false;
    }
    const uint32_t ref_va = anchor_ref_va_for_build(&k_p7501_ssot_pieos_anchors[anchor_idx], env->main_build);
    if (ref_va == 0u) {
        return false;
    }
    env->override_va[anchor_idx] = observed_va;
    env->affine_anchor_idx = (uint8_t)anchor_idx;
    env->affine_delta = (int32_t)(observed_va - ref_va);
    env->affine_valid = true;
    env->calibrated_mask |= P7501_CAL_AFFINE;
    return true;
}

uint32_t p7501_gps_resolve_anchor(const p7501_gps_env_t *env, size_t idx) {
    if (idx >= P7501_SSOT_PIEOS_ANCHOR_COUNT) {
        return 0u;
    }
    if ((env != NULL) && (env->override_va[idx] != 0u)) {
        return env->override_va[idx];
    }
    const uint32_t ref = anchor_ref_va_for_build(&k_p7501_ssot_pieos_anchors[idx], (env != NULL) ? env->main_build : 0u);
    if ((env != NULL) && env->affine_valid && (ref >= 0x57000000UL)) {
        return (uint32_t)((int32_t)ref + env->affine_delta);
    }
    return ref;
}

bool p7501_gps_find_anchor_by_name(const char *name, size_t *out_idx) {
    if ((name == NULL) || (*name == '\0') || (out_idx == NULL)) {
        return false;
    }
    for (size_t i = 0u; i < P7501_SSOT_PIEOS_ANCHOR_COUNT; ++i) {
        if (p7501_ascii_ieq(name, k_p7501_ssot_pieos_anchors[i].name)) {
            *out_idx = i;
            return true;
        }
    }
    return false;
}

bool p7501_gps_find_anchor_by_va(const p7501_gps_env_t *env, uint32_t va, size_t *out_idx) {
    if ((va == 0u) || (out_idx == NULL)) {
        return false;
    }
    for (size_t i = 0u; i < P7501_SSOT_PIEOS_ANCHOR_COUNT; ++i) {
        const uint32_t resolved = p7501_gps_resolve_anchor(env, i);
        if (((resolved != 0u) && (va >= resolved) && ((va - resolved) <= 0x300u)) ||
            (va == k_p7501_ssot_pieos_anchors[i].mmio_ref)) {
            *out_idx = i;
            return true;
        }
    }
    return false;
}

bool p7501_gps_parse_route_domain(const char *tok, p7501_route_domain_t *out_dom) {
    static const char *const k_kws[P7501_ROUTE_COUNT] = {
        "BOOT", "VIDEO", "BGO", "HDD", "NVRAM", "UNBRICK"
    };
    if (out_dom == NULL) {
        return false;
    }
    if ((tok == NULL) || (*tok == '\0')) {
        *out_dom = P7501_ROUTE_BOOT;
        return true;
    }
    for (size_t i = 0u; i < P7501_ROUTE_COUNT; ++i) {
        if (p7501_ascii_ieq(tok, k_kws[i])) {
            *out_dom = (p7501_route_domain_t)i;
            return true;
        }
    }
    return false;
}

const char *p7501_gps_build_name(uint8_t main_build) {
    if (main_build == (uint8_t)P7501_MAIN_BUILD_3060NP) {
        return "3060NP";
    }
    return (main_build == (uint8_t)P7501_MAIN_BUILD_3410PQ) ? "3410PQ" : "UNKNOWN";
}

size_t p7501_gps_format_env(const p7501_gps_env_t *env, char *out, size_t cap) {
    if ((env == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    const int n = snprintf(
        out, cap,
        "@P7501_GPS_ENV build=%s hw=%s flash_mb=%u bgo=0x%08" PRIX32 " nvram=0x%08" PRIX32
        " vdd_mv=%" PRIu32 " nsw5v=%u ic3001=0x%02X cal=0x%02X affine=%u delta=%" PRId32
        " kidx=%d model=%s suf=%s tv=%s vmode=0x%04X dvd=%u ic3k_exp=0x%02X",
        p7501_gps_build_name(env->main_build),
        (env->hw_sys[0] != '\0') ? env->hw_sys : "UNKNOWN",
        (unsigned)(env->flash_bytes / (1024UL * 1024UL)),
        env->bgo_base, env->nvram_base, env->vdd_mv, env->nsw5v_high ? 1u : 0u,
        (unsigned)env->ic3001_mask, (unsigned)env->calibrated_mask,
        env->affine_valid ? 1u : 0u, env->affine_delta, (int)env->feistel_key_idx,
        env->model_id, env->region_suffix, env->tv_standard,
        (unsigned)env->aiof_vmodesel, (unsigned)env->dvd_region, (unsigned)env->ic3001_expected_addr7
    );
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}

size_t p7501_gps_build_route(
    const p7501_gps_env_t *env,
    p7501_route_domain_t dom,
    char *out,
    size_t cap
) {
    if ((env == NULL) || (out == NULL) || (cap == 0u) || (dom >= P7501_ROUTE_COUNT)) {
        return 0u;
    }
    const char *dom_str = "BOOT", *payload = "live_xray", *next = "XRAY_3000";
    uint32_t target = p7501_gps_resolve_anchor(env, 1u);
    uint8_t op_mask = P7501_SSOT_TRAP_R;

    if (dom == P7501_ROUTE_BOOT) {
        payload = (env->fpc_err > 0u) ? "main_3060np_br" : "live_xray";
        next = env->monitor_online ? "GPS_CALLS" : "XRAY_BOOT_15000";
    } else if (dom == P7501_ROUTE_VIDEO) {
        dom_str = "VIDEO";
        target = P7501_SSOT_AIOF_BASE;
        payload = (env->avin_iic_err > 0u) ? "video_mmio_dump" : "tbc_passthrough";
        next = env->monitor_online ? "MMIO_SNAP_VIDEO" : "LIVE_MAP_VIDEO_2000";
    } else if (dom == P7501_ROUTE_BGO) {
        dom_str = "BGO";
        target = env->bgo_base;
        payload = "fast_bgo_stream";
        next = (env->bgo_err > 0u) ? "BGO_INSPECT_DETAIL" : "NOR_MAP";
    } else if (dom == P7501_ROUTE_HDD) {
        dom_str = "HDD";
        target = p7501_gps_resolve_anchor(env, 5u);
        payload = "none";
        next = (env->hdd_fail > 0u) ? "DIAGWATCH_5000_NORMAL_CH1" : "MMIO_SNAP_STORAGE";
    } else if (dom == P7501_ROUTE_NVRAM) {
        dom_str = "NVRAM";
        target = env->nvram_base;
        payload = "jedec_nvram_erase";
        op_mask = P7501_SSOT_TRAP_W;
        next = (env->bgo_err > 0u) ? "BGO_INSPECT NORFROM" : "EEMUL_INSPECT";
    } else {
        dom_str = "UNBRICK";
        target = (env->bgo_err > 0u) ? env->nvram_base : P7501_SSOT_AIOF_BASE;
        payload = (env->bgo_err > 0u) ? "jedec_nvram_erase" : "video_mmio_dump";
        op_mask = (env->bgo_err > 0u) ? P7501_SSOT_TRAP_W : P7501_SSOT_TRAP_R;
        next = env->monitor_online ? "NOR_MAP" : "XRAY_BOOT_15000";
    }

    p7501_trap_audit_result_t tr;
    (void)p7501_trap_audit(target, 4u, op_mask, &tr);
    const char *const guard = (tr.verdict == P7501_TRAP_DENY) ? "DENY"
        : ((tr.verdict == P7501_TRAP_WARN) ? "WARN" : "ALLOW");

    const int n = snprintf(
        out, cap,
        "@P7501_ROUTE dom=%s build=%s target=0x%08" PRIX32 " payload=%s guard=%s trap=%s next=%s",
        dom_str, p7501_gps_build_name(env->main_build), target,
        payload, guard, (tr.trap != NULL) ? tr.trap->id : "NONE", next
    );
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}
