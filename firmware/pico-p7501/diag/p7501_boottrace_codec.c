/**
 * @file p7501_boottrace_codec.c
 * @brief Pure C11 real-time on-chip Panasonic DMR-EH55 boot log classifier.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_boottrace_codec.h"
#include "common/p7501_hex.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static void copy_trimmed_until(
    const char *src,
    char stop1,
    char stop2,
    char *dst,
    size_t cap
) {
    if ((src == NULL) || (dst == NULL) || (cap == 0u)) {
        return;
    }
    while (*src == ' ') {
        ++src;
    }
    size_t i = 0u;
    while ((i + 1u) < cap) {
        const char ch = src[i];
        if ((ch == '\0') || (ch == stop1) || (ch == stop2) || (ch == '\r') || (ch == '\n')) {
            break;
        }
        dst[i] = ch;
        ++i;
    }
    while ((i > 0u) && (dst[i - 1u] == ' ')) {
        --i;
    }
    dst[i] = '\0';
}

static bool extract_marked_token(
    const char *line,
    const char *marker,
    char stop1,
    char stop2,
    char *dst,
    size_t cap
) {
    const char *hit = strstr(line, marker);
    if (hit == NULL) {
        return false;
    }
    copy_trimmed_until(hit + strlen(marker), stop1, stop2, dst, cap);
    return dst[0] != '\0';
}

static void extract_deep_fields(p7501_boot_classifier_t *st, const char *line) {
    const char *hit = NULL;
    uint32_t v1 = 0u;
    uint32_t v2 = 0u;

    (void)extract_marked_token(line, "H/W System  = ", ' ', '\0', st->hw_sys, sizeof(st->hw_sys));
    (void)extract_marked_token(line, "market_code=", ' ', '\0', st->market, sizeof(st->market));
    (void)extract_marked_token(line, "Timer Version [", ']', '\0', st->timer_ver, sizeof(st->timer_ver));
    (void)extract_marked_token(line, "CAV:", ' ', '\0', st->cav, sizeof(st->cav));
    (void)extract_marked_token(line, "SYS_CNTL_SetSysState[", ']', '\0', st->sys_state, sizeof(st->sys_state));
    (void)extract_marked_token(line, "Drive Revision : ", '\r', '\n', st->drive_rev, sizeof(st->drive_rev));

    hit = strstr(line, "RegionCode(EEPROM) [");
    if (hit != NULL) {
        const char *p = hit + strlen("RegionCode(EEPROM) [");
        char num[16];
        copy_trimmed_until(p, ']', '\0', num, sizeof(num));
        if (p7501_parse_u32_auto(num, &v1)) {
            st->region_code = (uint8_t)v1;
            st->has_region = true;
        }
    }

    hit = strstr(line, "HDD is_writable=");
    if (hit != NULL) {
        const char *p = hit + strlen("HDD is_writable=");
        char num[16];
        copy_trimmed_until(p, ' ', '\0', num, sizeof(num));
        if (p7501_parse_u32_auto(num, &v1)) {
            st->hdd_writable = (v1 != 0u) ? 1 : 0;
        }
    }

    hit = strstr(line, "format=");
    if (hit != NULL) {
        const char *p = hit + strlen("format=");
        char num[20];
        copy_trimmed_until(p, ' ', '\0', num, sizeof(num));
        if (p7501_parse_u32_auto(num, &v1)) {
            st->hdd_format = v1;
            st->has_hdd_format = true;
        }
    }

    if ((strstr(line, "SetGuiMode") != NULL) || (strstr(line, "setgui") != NULL)) {
        hit = strstr(line, "mode=");
        if (hit != NULL) {
            char num[12];
            copy_trimmed_until(hit + 5u, ' ', '\0', num, sizeof(num));
            if (p7501_parse_u32_auto(num, &v1)) {
                st->gui_mode = (int16_t)v1;
            }
        }
    }

    hit = strstr(line, "AVOUT_AV_Select: video=");
    if (hit != NULL) {
        const char *vp = hit + strlen("AVOUT_AV_Select: video=");
        const char *ap = strstr(vp, "audio=");
        char num_v[12];
        char num_a[12];
        copy_trimmed_until(vp, ',', ' ', num_v, sizeof(num_v));
        if (ap != NULL) {
            copy_trimmed_until(ap + 6u, ' ', ',', num_a, sizeof(num_a));
            if (p7501_parse_u32_auto(num_v, &v1) && p7501_parse_u32_auto(num_a, &v2)) {
                st->avsel_video = (uint8_t)v1;
                st->avsel_audio = (uint8_t)v2;
                st->has_avsel = true;
            }
        }
    }

    hit = strstr(line, "[AVIN] target:");
    if ((hit != NULL) && (strstr(line, "IIC send error") != NULL)) {
        const char *tp = hit + strlen("[AVIN] target:");
        const char *ip = strstr(tp, "input:");
        char num_t[12];
        char num_i[12];
        copy_trimmed_until(tp, ' ', '\0', num_t, sizeof(num_t));
        if (ip != NULL) {
            copy_trimmed_until(ip + 6u, ' ', '\0', num_i, sizeof(num_i));
            if (p7501_parse_u32_auto(num_t, &v1) && p7501_parse_u32_auto(num_i, &v2)) {
                st->avin_iic_target = (uint8_t)v1;
                st->avin_iic_input = (uint8_t)v2;
                st->has_avin_iic = true;
                st->avin_iic_pending = true;
            }
        }
    } else if (st->avin_iic_pending) {
        st->avin_iic_pending = false;
        hit = strstr(line, "Line:");
        if (hit != NULL) {
            char num[16];
            copy_trimmed_until(hit + 5u, ' ', '\0', num, sizeof(num));
            if (p7501_parse_u32_auto(num, &v1)) {
                st->avin_iic_line = v1;
            }
        }
    }
}

static void inc_sat_u16(uint16_t *counter) {
    if (*counter < 0xFFFFu) {
        *counter = (uint16_t)(*counter + 1u);
    }
}

static bool is_ascii_alnum(char c) {
    return ((c >= '0') && (c <= '9')) ||
           ((c >= 'A') && (c <= 'Z')) ||
           ((c >= 'a') && (c <= 'z'));
}

static bool contains_token_word(const char *haystack, const char *word) {
    const size_t wlen = strlen(word);
    const char *cur = haystack;
    while ((cur = strstr(cur, word)) != NULL) {
        const bool left_ok = (cur == haystack) || !is_ascii_alnum(cur[-1]);
        const bool right_ok = !is_ascii_alnum(cur[wlen]);
        if (left_ok && right_ok) {
            return true;
        }
        cur += wlen;
    }
    return false;
}

static bool any_op7f_recorded(const p7501_boot_classifier_t *st) {
    for (size_t i = 0u; i < sizeof(st->op7f_bitmap); ++i) {
        if (st->op7f_bitmap[i] != 0u) {
            return true;
        }
    }
    return false;
}

static void record_fpc_op7f(p7501_boot_classifier_t *st, const char *line) {
    const char *fpc = strstr(line, "FPC_ReciveTASK()");
    if (fpc == NULL) {
        return;
    }
    const char *tag = strstr(fpc, ": 7F ");
    if (tag == NULL) {
        tag = strstr(fpc, " 7F ");
        if (tag == NULL) {
            return;
        }
        tag += 4u;
    } else {
        tag += 5u;
    }
    const int hi = p7501_hex_char_to_nibble(tag[0]);
    const int lo = (hi >= 0) ? p7501_hex_char_to_nibble(tag[1]) : -1;
    if ((hi < 0) || (lo < 0)) {
        return;
    }
    const uint8_t op = (uint8_t)(((unsigned)hi << 4u) | (unsigned)lo);
    st->op7f_bitmap[op >> 3u] |= (uint8_t)(1u << (op & 7u));
}

void p7501_boot_classifier_init(p7501_boot_classifier_t *st) {
    if (st == NULL) {
        return;
    }
    memset(st, 0, sizeof(*st));
    st->hdd_writable = -1;
    st->gui_mode = -1;
}

void p7501_boot_classifier_feed_line(p7501_boot_classifier_t *st, const char *line) {
    if ((st == NULL) || (line == NULL) || (*line == '\0')) {
        return;
    }
    st->lines_seen += 1u;
    extract_deep_fields(st, line);

    if (strstr(line, "[STAGE2]") != NULL) {
        st->stage2_seen = true;
        if (strstr(line, "Boot MAIN") != NULL) {
            st->main_entered = true;
        }
    }
    if (strstr(line, "PiE-OS") != NULL) {
        st->main_entered = true;
    }
    if (strstr(line, "System Version :") != NULL) {
        st->main_entered = true;
        if (strstr(line, "3060NP") != NULL) {
            st->main_build = (uint8_t)P7501_MAIN_BUILD_3060NP;
        } else if (strstr(line, "3410PQ") != NULL) {
            st->main_build = (uint8_t)P7501_MAIN_BUILD_3410PQ;
        }
    }

    if (strstr(line, "AVOUT_init() success.") != NULL) {
        st->avout_init_ok = true;
    }
    if (strstr(line, "AVOUT_V_Mute: onoff=0") != NULL) {
        st->avout_vmute_off = true;
    }
    if ((strstr(line, "TV_NTSC(0)") != NULL) || (strstr(line, "avout_tv_system = 0") != NULL)) {
        st->tv_ntsc_seen = true;
    }
    if ((strstr(line, "nfrom back up ok.") != NULL) || (strstr(line, "sys_rsv init success.") != NULL)) {
        st->sys_rsv_ok = true;
    }
    if ((strstr(line, "startup: HDD opened") != NULL) || (strstr(line, "MEDIA = HDD") != NULL)) {
        st->hdd_opened = true;
    }

    if (strstr(line, "HDD_MAX_LBA get failed") != NULL) {
        inc_sat_u16(&st->hdd_lba_fail_cnt);
    }
    if ((strstr(line, "BGO driver init failed") != NULL) ||
        (strstr(line, "Status flag change error") != NULL) ||
        (strstr(line, "File omap update error") != NULL) ||
        (strstr(line, "File rewrite end error") != NULL) ||
        (strstr(line, "NFROMError Power off") != NULL) ||
        ((strstr(line, "bgo_writeD") != NULL) && (strstr(line, "error(") != NULL))) {
        inc_sat_u16(&st->nfrom_bgo_err_cnt);
    }
    if (contains_token_word(line, "U99")) {
        inc_sat_u16(&st->u99_cnt);
    }
    if (strstr(line, "80040904") != NULL) {
        inc_sat_u16(&st->odd_80040904_cnt);
    }
    if (strstr(line, "FPC_EMERGENCY_ERROR") != NULL) {
        inc_sat_u16(&st->fpc_err_cnt);
    }
    if (strstr(line, "FPC_EMERGENCY_TIMEOUT") != NULL) {
        inc_sat_u16(&st->fpc_to_cnt);
    }
    if (strstr(line, "IIC send error") != NULL) {
        inc_sat_u16(&st->iic_err_cnt);
    }
    record_fpc_op7f(st, line);
}

p7501_boot_verdict_t p7501_boot_classifier_verdict(const p7501_boot_classifier_t *st) {
    if ((st == NULL) || (st->lines_seen == 0u)) {
        return P7501_BOOT_VERDICT_NO_UART;
    }
    if ((st->nfrom_bgo_err_cnt > 0u) || (st->u99_cnt > 0u)) {
        return P7501_BOOT_VERDICT_BGO_NFROM_FAULT;
    }
    if ((st->fpc_err_cnt > 0u) || (st->fpc_to_cnt > 0u) || any_op7f_recorded(st)) {
        return P7501_BOOT_VERDICT_FPC_IC7501_MISMATCH;
    }
    if ((st->hdd_lba_fail_cnt > 0u) && !st->stage2_seen && !st->main_entered) {
        return P7501_BOOT_VERDICT_HDD_BRIDGE_WAIT_FAIL;
    }
    if (!st->main_entered) {
        return P7501_BOOT_VERDICT_LOADER_ONLY;
    }
    return P7501_BOOT_VERDICT_CLEAN_MAIN_OK;
}

uint32_t p7501_boot_classifier_format_op7f(
    const p7501_boot_classifier_t *st,
    char *out,
    size_t cap
) {
    if (st == NULL) {
        if ((out != NULL) && (cap > 0u)) {
            out[0] = '\0';
        }
        return 0u;
    }
    return p7501_format_hex_bitmap_csv(st->op7f_bitmap, out, cap);
}

static const char *verdict_name(p7501_boot_verdict_t v) {
    switch (v) {
        case P7501_BOOT_VERDICT_NO_UART:
            return "NO_UART";
        case P7501_BOOT_VERDICT_LOADER_ONLY:
            return "LOADER_ONLY";
        case P7501_BOOT_VERDICT_HDD_BRIDGE_WAIT_FAIL:
            return "HDD_BRIDGE_WAIT_FAIL";
        case P7501_BOOT_VERDICT_BGO_NFROM_FAULT:
            return "BGO_NFROM_FAULT";
        case P7501_BOOT_VERDICT_FPC_IC7501_MISMATCH:
            return "FPC_IC7501_MISMATCH";
        case P7501_BOOT_VERDICT_CLEAN_MAIN_OK:
            return "CLEAN_MAIN_OK";
        default:
            return "UNKNOWN";
    }
}

static const char *build_name(uint8_t build) {
    switch ((p7501_main_build_t)build) {
        case P7501_MAIN_BUILD_3060NP:
            return "3060NP";
        case P7501_MAIN_BUILD_3410PQ:
            return "3410PQ";
        default:
            return "UNKNOWN";
    }
}

size_t p7501_boot_classifier_format_summary(
    const p7501_boot_classifier_t *st,
    char *out,
    size_t cap
) {
    if ((st == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    char op7f_csv[96];
    const uint32_t op7f_cnt = p7501_boot_classifier_format_op7f(st, op7f_csv, sizeof(op7f_csv));
    const p7501_boot_verdict_t verdict = p7501_boot_classifier_verdict(st);
    const int n = snprintf(
        out,
        cap,
        "@P7501_BOOT_CLASS verdict=%s build=%s stage2=%u avout_ok=%u vmute0=%u "
        "ntsc=%u rsv_ok=%u hdd_fail=%u bgo_err=%u fpc_err=%u fpc_to=%u "
        "iic_err=%u odd_err=%u op7f_cnt=%u op7f=%s",
        verdict_name(verdict),
        build_name(st->main_build),
        st->stage2_seen ? 1u : 0u,
        st->avout_init_ok ? 1u : 0u,
        st->avout_vmute_off ? 1u : 0u,
        st->tv_ntsc_seen ? 1u : 0u,
        st->sys_rsv_ok ? 1u : 0u,
        (unsigned)st->hdd_lba_fail_cnt,
        (unsigned)(st->nfrom_bgo_err_cnt + st->u99_cnt),
        (unsigned)st->fpc_err_cnt,
        (unsigned)st->fpc_to_cnt,
        (unsigned)st->iic_err_cnt,
        (unsigned)st->odd_80040904_cnt,
        (unsigned)op7f_cnt,
        op7f_csv
    );
    if (n < 0) {
        out[0] = '\0';
        return 0u;
    }
    return ((size_t)n < cap) ? (size_t)n : (cap - 1u);
}

size_t p7501_boot_classifier_format_deep(
    const p7501_boot_classifier_t *st,
    char *out,
    size_t cap
) {
    if ((st == NULL) || (out == NULL) || (cap == 0u)) {
        return 0u;
    }
    char reg_buf[8];
    char av_buf[12];
    char gui_buf[8];
    char wr_buf[6];
    char fmt_buf[14];
    char iic_buf[24];

    if (st->has_region) {
        (void)snprintf(reg_buf, sizeof(reg_buf), "0x%02X", (unsigned)st->region_code);
    } else {
        (void)snprintf(reg_buf, sizeof(reg_buf), "NONE");
    }
    if (st->has_avsel) {
        (void)snprintf(av_buf, sizeof(av_buf), "%u/%u",
                       (unsigned)st->avsel_video, (unsigned)st->avsel_audio);
    } else {
        (void)snprintf(av_buf, sizeof(av_buf), "NONE");
    }
    if (st->gui_mode >= 0) {
        (void)snprintf(gui_buf, sizeof(gui_buf), "%d", (int)st->gui_mode);
    } else {
        (void)snprintf(gui_buf, sizeof(gui_buf), "NONE");
    }
    if (st->hdd_writable >= 0) {
        (void)snprintf(wr_buf, sizeof(wr_buf), "%d", (int)st->hdd_writable);
    } else {
        (void)snprintf(wr_buf, sizeof(wr_buf), "NONE");
    }
    if (st->has_hdd_format) {
        (void)snprintf(fmt_buf, sizeof(fmt_buf), "0x%" PRIX32, st->hdd_format);
    } else {
        (void)snprintf(fmt_buf, sizeof(fmt_buf), "NONE");
    }
    if (st->has_avin_iic) {
        (void)snprintf(iic_buf, sizeof(iic_buf), "%u:0x%X@%" PRIu32,
                       (unsigned)st->avin_iic_target,
                       (unsigned)st->avin_iic_input,
                       st->avin_iic_line);
    } else {
        (void)snprintf(iic_buf, sizeof(iic_buf), "NONE");
    }

    const int n = snprintf(
        out,
        cap,
        "@P7501_DIAG_DEEP hw=%s market=%s timer=%s region=%s cav=%s "
        "avsel=%s gui=%s sys=%s hdd_wr=%s hdd_fmt=%s drv=%s avin_iic=%s",
        (st->hw_sys[0] != '\0') ? st->hw_sys : "UNKNOWN",
        (st->market[0] != '\0') ? st->market : "UNKNOWN",
        (st->timer_ver[0] != '\0') ? st->timer_ver : "NONE",
        reg_buf,
        (st->cav[0] != '\0') ? st->cav : "NONE",
        av_buf,
        gui_buf,
        (st->sys_state[0] != '\0') ? st->sys_state : "NONE",
        wr_buf,
        fmt_buf,
        (st->drive_rev[0] != '\0') ? st->drive_rev : "NONE",
        iic_buf
    );
    if (n < 0) {
        out[0] = '\0';
        return 0u;
    }
    return ((size_t)n < cap) ? (size_t)n : (cap - 1u);
}
