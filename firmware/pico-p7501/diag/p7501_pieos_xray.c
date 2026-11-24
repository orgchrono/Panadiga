/**
 * @file p7501_pieos_xray.c
 * @brief Pure functional 12-stage PiE-OS boot-chain & metabolism X-Ray logger.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_pieos_xray.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

typedef struct xray_rule {
    const char *needle;
    p7501_xray_stage_t stage;
    p7501_xray_level_t level;
    int8_t anchor_idx;
    const char *subsys;
} xray_rule_t;

/* Declarative rule table ordered from highest-specificity fault markers to pipeline milestones. */
static const xray_rule_t k_xray_rules[] = {
    { "BGO driver init failed", P7501_XRAY_S7_BGO_NFROM,    P7501_XRAY_LVL_CRIT, 6,  "NFROM" },
    { "U99",                    P7501_XRAY_S11_STEADY_FAULT, P7501_XRAY_LVL_CRIT, 6,  "SYS"   },
    { "HDD_MAX_LBA get failed", P7501_XRAY_S3_HDD_BRIDGE,   P7501_XRAY_LVL_ERR,  5,  "RTSD"  },
    { "SD init failed",         P7501_XRAY_S4_STAGE2_SD,    P7501_XRAY_LVL_ERR,  1,  "SD"    },
    { "FPC_EMERGENCY",          P7501_XRAY_S11_STEADY_FAULT, P7501_XRAY_LVL_ERR,  -1, "FPC"   },
    { "IIC send error",         P7501_XRAY_S8_AVIN_AVOUT,   P7501_XRAY_LVL_ERR,  4,  "AVIN"  },
    { "80040904",               P7501_XRAY_S9_DRIVE_HDD,    P7501_XRAY_LVL_ERR,  8,  "ODD"   },
    { "HDDERR",                 P7501_XRAY_S11_STEADY_FAULT, P7501_XRAY_LVL_ERR,  8,  "RTSD"  },
    { "U61",                    P7501_XRAY_S11_STEADY_FAULT, P7501_XRAY_LVL_ERR,  8,  "ODD"   },
    { "U88",                    P7501_XRAY_S11_STEADY_FAULT, P7501_XRAY_LVL_ERR,  5,  "RTSD"  },
    { "SD Card remove",         P7501_XRAY_S4_STAGE2_SD,    P7501_XRAY_LVL_WARN, 1,  "SD"    },
    { "FPC_ReciveTASK",         P7501_XRAY_S6_SYS_BOOT,     P7501_XRAY_LVL_WARN, -1, "FPC"   },
    { "DDR CHECK",              P7501_XRAY_S1_IPL_DDR,      P7501_XRAY_LVL_OK,   -1, "IPL"   },
    { "Loader ver",             P7501_XRAY_S2_LOADER,       P7501_XRAY_LVL_INFO, 0,  "LDR"   },
    { "startup reason",         P7501_XRAY_S2_LOADER,       P7501_XRAY_LVL_INFO, 0,  "LDR"   },
    { "hung-up time changed",   P7501_XRAY_S2_LOADER,       P7501_XRAY_LVL_INFO, 0,  "WDT"   },
    { "Nor-FROM",               P7501_XRAY_S2_LOADER,       P7501_XRAY_LVL_INFO, 1,  "LDR"   },
    { "Read fvu program",       P7501_XRAY_S4_STAGE2_SD,    P7501_XRAY_LVL_INFO, 1,  "SD2"   },
    { "SD Card inserted",       P7501_XRAY_S4_STAGE2_SD,    P7501_XRAY_LVL_OK,   1,  "SD"    },
    { "FAT Mount ID",           P7501_XRAY_S4_STAGE2_SD,    P7501_XRAY_LVL_OK,   1,  "FAT"   },
    { "PANAMON1>",              P7501_XRAY_S4_STAGE2_SD,    P7501_XRAY_LVL_OK,   1,  "MON"   },
    { "[STAGE2]",               P7501_XRAY_S4_STAGE2_SD,    P7501_XRAY_LVL_OK,   1,  "SD2"   },
    { "PiE-OS",                 P7501_XRAY_S5_PIE_CRT0,     P7501_XRAY_LVL_OK,   3,  "MAIN"  },
    { "System Version",         P7501_XRAY_S5_PIE_CRT0,     P7501_XRAY_LVL_OK,   3,  "MAIN"  },
    { "H/W System",             P7501_XRAY_S6_SYS_BOOT,     P7501_XRAY_LVL_INFO, 3,  "BOOT"  },
    { "RegionCode",             P7501_XRAY_S6_SYS_BOOT,     P7501_XRAY_LVL_INFO, 3,  "BOOT"  },
    { "Timer Version",          P7501_XRAY_S6_SYS_BOOT,     P7501_XRAY_LVL_INFO, 3,  "BOOT"  },
    { "nfrom back up ok",       P7501_XRAY_S7_BGO_NFROM,    P7501_XRAY_LVL_OK,   7,  "BGO"   },
    { "sys_rsv init success",   P7501_XRAY_S7_BGO_NFROM,    P7501_XRAY_LVL_OK,   7,  "BGO"   },
    { "AVOUT_init() success",   P7501_XRAY_S8_AVIN_AVOUT,   P7501_XRAY_LVL_OK,   4,  "AVOUT" },
    { "AVOUT_V_Mute",           P7501_XRAY_S8_AVIN_AVOUT,   P7501_XRAY_LVL_INFO, 10, "AVOUT" },
    { "AVOUT_AV_Select",        P7501_XRAY_S8_AVIN_AVOUT,   P7501_XRAY_LVL_INFO, 10, "AVOUT" },
    { "CAV:",                   P7501_XRAY_S8_AVIN_AVOUT,   P7501_XRAY_LVL_INFO, 4,  "AVOUT" },
    { "Drive Revision",         P7501_XRAY_S9_DRIVE_HDD,    P7501_XRAY_LVL_OK,   8,  "RTSD"  },
    { "HDD opened",             P7501_XRAY_S9_DRIVE_HDD,    P7501_XRAY_LVL_OK,   8,  "RTSD"  },
    { "HDD is_writable",        P7501_XRAY_S9_DRIVE_HDD,    P7501_XRAY_LVL_INFO, 8,  "RTSD"  },
    { "SetGuiMode",             P7501_XRAY_S10_VFD_VV,      P7501_XRAY_LVL_OK,   7,  "GUI"   },
    { "SetSysState[VV]",        P7501_XRAY_S10_VFD_VV,      P7501_XRAY_LVL_OK,   7,  "GUI"   },
};

void p7501_xray_init(p7501_xray_trace_t *tr, bool color_enabled) {
    if (tr == NULL) {
        return;
    }
    memset(tr, 0, sizeof(*tr));
    tr->fault_stage = -1;
    tr->color_enabled = color_enabled;
}

const char *p7501_xray_stage_name(p7501_xray_stage_t st) {
    static const char *const k_names[P7501_XRAY_STAGE_COUNT] = {
        "S0_POWER",
        "S1_IPL_DDR",
        "S2_LOADER",
        "S3_HDD_BRIDGE",
        "S4_STAGE2_SD",
        "S5_PIE_CRT0",
        "S6_SYS_BOOT",
        "S7_BGO_NFROM",
        "S8_AVIN_AVOUT",
        "S9_DRIVE_HDD",
        "S10_VFD_VV",
        "S11_STEADY_FAULT",
    };
    return ((unsigned)st < P7501_XRAY_STAGE_COUNT) ? k_names[(unsigned)st] : "UNKNOWN";
}

const char *p7501_xray_level_name(p7501_xray_level_t lvl) {
    static const char *const k_lvl[5] = { "INF", "OK ", "WRN", "ERR", "CRT" };
    return ((unsigned)lvl < 5u) ? k_lvl[(unsigned)lvl] : "INF";
}

bool p7501_xray_feed_line(
    p7501_xray_trace_t *tr,
    const p7501_gps_env_t *env,
    uint64_t ts_us,
    const char *line,
    p7501_xray_event_t *out_ev
) {
    if ((tr == NULL) || (line == NULL) || (*line == '\0')) {
        return false;
    }
    const size_t n_rules = sizeof(k_xray_rules) / sizeof(k_xray_rules[0]);
    const xray_rule_t *hit = NULL;
    for (size_t i = 0u; i < n_rules; ++i) {
        if (strstr(line, k_xray_rules[i].needle) != NULL) {
            hit = &k_xray_rules[i];
            break;
        }
    }
    if (hit == NULL) {
        return false;
    }

    if (!tr->has_started) {
        tr->has_started = true;
        tr->first_ts_us = ts_us;
        tr->last_ts_us = ts_us;
        tr->stage_seen_mask |= (uint16_t)(1u << (unsigned)P7501_XRAY_S0_POWER);
        tr->stage_first_us[P7501_XRAY_S0_POWER] = ts_us;
    }
    const uint64_t delta_us = (ts_us >= tr->last_ts_us) ? (ts_us - tr->last_ts_us) : 0ULL;
    tr->last_ts_us = ts_us;
    tr->events_seen += 1u;

    const unsigned sidx = (unsigned)hit->stage;
    if ((tr->stage_seen_mask & (uint16_t)(1u << sidx)) == 0u) {
        tr->stage_seen_mask |= (uint16_t)(1u << sidx);
        tr->stage_first_us[sidx] = ts_us;
    }
    if (sidx > (unsigned)tr->max_stage) {
        tr->max_stage = (uint8_t)sidx;
    }

    if (hit->level >= P7501_XRAY_LVL_ERR) {
        tr->errors_seen += 1u;
        if (tr->fault_stage < 0) {
            tr->fault_stage = (int8_t)sidx;
        }
    }

    if (out_ev != NULL) {
        out_ev->ts_us = ts_us;
        out_ev->delta_us = delta_us;
        out_ev->anchor_idx = hit->anchor_idx;
        out_ev->resolved_va = (hit->anchor_idx >= 0)
            ? p7501_gps_resolve_anchor(env, (size_t)hit->anchor_idx)
            : 0u;
        out_ev->stage = hit->stage;
        out_ev->level = hit->level;
        out_ev->subsys = hit->subsys;
        (void)snprintf(out_ev->excerpt, sizeof(out_ev->excerpt), "%s", line);
    }
    return true;
}

size_t p7501_xray_format_event(
    const p7501_xray_event_t *ev,
    bool ansi_color,
    char *out,
    size_t cap
) {
    static const char *const k_ansi_on[5] = {
        "\x1b[36m", "\x1b[32m", "\x1b[33m", "\x1b[31m", "\x1b[1;35m"
    };
    if ((ev == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    const uint64_t ms = ev->ts_us / 1000ULL;
    const uint32_t tenth = (uint32_t)((ev->ts_us % 1000ULL) / 100ULL);
    const uint64_t dms = ev->delta_us / 1000ULL;
    const uint32_t dtenth = (uint32_t)((ev->delta_us % 1000ULL) / 100ULL);

    char sym[64];
    if (ev->anchor_idx >= 0) {
        const p7501_ssot_pieos_anchor_t *const a =
            &k_p7501_ssot_pieos_anchors[(size_t)ev->anchor_idx];
        (void)snprintf(sym, sizeof(sym), "0x%08" PRIX32 "(%s<-%s)",
                       ev->resolved_va, a->name, a->caller);
    } else {
        (void)snprintf(sym, sizeof(sym), "unmapped(%s)", ev->subsys);
    }

    const unsigned lidx = ((unsigned)ev->level < 5u) ? (unsigned)ev->level : 0u;
    const char *c_pre = ansi_color ? k_ansi_on[lidx] : "";
    const char *c_post = ansi_color ? "\x1b[0m" : "";
    const int n = snprintf(
        out,
        cap,
        "[+%llu.%lums d%llu.%lums] [%s] %s[%s]%s %s | %s",
        (unsigned long long)ms,
        (unsigned long)tenth,
        (unsigned long long)dms,
        (unsigned long)dtenth,
        p7501_xray_stage_name(ev->stage),
        c_pre,
        p7501_xray_level_name(ev->level),
        c_post,
        sym,
        ev->excerpt
    );
    if (n < 0) {
        out[0] = '\0';
        return 0u;
    }
    return ((size_t)n < cap) ? (size_t)n : (cap - 1u);
}

size_t p7501_xray_format_summary(
    const p7501_xray_trace_t *tr,
    const p7501_gps_env_t *env,
    char *out,
    size_t cap
) {
    if ((tr == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    const char *fault_str = (tr->fault_stage >= 0)
        ? p7501_xray_stage_name((p7501_xray_stage_t)tr->fault_stage)
        : "NONE";
    const uint8_t b = (env != NULL) ? env->main_build : 0u;
    const char *bname = p7501_gps_build_name(b);
    const int32_t delta = (env != NULL) ? env->affine_delta : 0;

    const int n = snprintf(
        out,
        cap,
        "@P7501_XRAY max_stage=%s fault_stage=%s stages_mask=0x%04X "
        "events=%" PRIu32 " errors=%" PRIu32 " build=%s delta=%" PRId32 " color=%s",
        p7501_xray_stage_name((p7501_xray_stage_t)tr->max_stage),
        fault_str,
        (unsigned)tr->stage_seen_mask,
        tr->events_seen,
        tr->errors_seen,
        bname,
        delta,
        tr->color_enabled ? "ON" : "OFF"
    );
    if ((n < 0) || ((size_t)n >= cap)) {
        out[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}
