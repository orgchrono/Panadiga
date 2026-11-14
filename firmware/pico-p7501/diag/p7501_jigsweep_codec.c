/**
 * @file p7501_jigsweep_codec.c
 * @brief Pure ISO C11 argument parser and outcome classifier for `JIGSWEEP`.
 *
 * Compiled on both ARM/RISC-V Pico targets and x86_64 CI hosts (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_jigsweep_codec.h"

#include "common/p7501_hex.h"
#include "uart/p7501_uart_codec.h"

#include <string.h>

static bool sweep_contains(const uint8_t *hay, size_t hay_len, const char *needle) {
    const size_t nlen = strlen(needle);
    if (hay == NULL || nlen == 0u || hay_len < nlen) {
        return false;
    }
    for (size_t i = 0u; i + nlen <= hay_len; ++i) {
        if (memcmp(hay + i, needle, nlen) == 0) {
            return true;
        }
    }
    return false;
}

static bool sweep_parse_u32_strict(const char *tok, uint32_t *out_val) {
    if (tok == NULL || tok[0] == '\0' || out_val == NULL) {
        return false;
    }
    uint32_t acc = 0u;
    for (size_t i = 0u; tok[i] != '\0'; ++i) {
        const char ch = tok[i];
        if (ch < '0' || ch > '9') {
            return false;
        }
        const uint32_t digit = (uint32_t)(ch - '0');
        if (acc > 2000u) {
            acc = 2000u;
        } else {
            acc = (acc * 10u) + digit;
        }
    }
    *out_val = (acc > 2000u) ? 2000u : acc;
    return true;
}

static void sweep_load_defaults(p7501_jigsweep_spec_t *out) {
    static const uint32_t k_hold_defaults[] = { 0u, 5u, 10u, 12u, 15u, 20u, 25u, 30u };
    static const uint32_t k_delay_defaults[] = { 0u, 5u, 10u, 12u, 15u, 18u, 22u, 25u, 28u, 35u };
    if (out->mode == P7501_JIGSWEEP_DELAY) {
        out->point_count = sizeof(k_delay_defaults) / sizeof(k_delay_defaults[0]);
        memcpy(out->points_ms, k_delay_defaults, sizeof(k_delay_defaults));
    } else if (out->mode == P7501_JIGSWEEP_PREBOOT) {
        out->point_count = 6u;
        for (size_t i = 0u; i < 6u; ++i) {
            out->points_ms[i] = (uint32_t)i;
        }
    } else {
        out->point_count = sizeof(k_hold_defaults) / sizeof(k_hold_defaults[0]);
        memcpy(out->points_ms, k_hold_defaults, sizeof(k_hold_defaults));
    }
}

bool p7501_jigsweep_parse_args(const char *arg, p7501_jigsweep_spec_t *out) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->mode = P7501_JIGSWEEP_HOLD;

    const char *cur = arg;
    char tok[32];
    if (cur != NULL && (cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL && tok[0] != '\0') {
        uint32_t v = 0u;
        if (p7501_ascii_ieq(tok, "HOLD")) {
            out->mode = P7501_JIGSWEEP_HOLD;
        } else if (p7501_ascii_ieq(tok, "DELAY")) {
            out->mode = P7501_JIGSWEEP_DELAY;
        } else if (p7501_ascii_ieq(tok, "PREBOOT")) {
            out->mode = P7501_JIGSWEEP_PREBOOT;
        } else if (sweep_parse_u32_strict(tok, &v)) {
            out->points_ms[out->point_count++] = v;
        } else {
            return false;
        }
    }

    while (cur != NULL && (cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL && tok[0] != '\0') {
        uint32_t v = 0u;
        if (!sweep_parse_u32_strict(tok, &v)) {
            return false;
        }
        if (out->point_count < P7501_JIGSWEEP_MAX_POINTS) {
            out->points_ms[out->point_count++] = v;
        }
    }

    if (out->point_count == 0u) {
        sweep_load_defaults(out);
    }
    return true;
}

p7501_jigsweep_outcome_t p7501_jigsweep_classify_rx(const uint8_t *buf, size_t len) {
    if (buf == NULL || len == 0u) {
        return P7501_SWEEP_OUTCOME_SILENT;
    }
    if (sweep_contains(buf, len, "serial boot mode")) {
        return P7501_SWEEP_OUTCOME_SERIAL;
    }
    if (sweep_contains(buf, len, "normal mode")) {
        return P7501_SWEEP_OUTCOME_NORMAL;
    }
    return P7501_SWEEP_OUTCOME_EARLY_OTHER;
}

const char *p7501_jigsweep_outcome_name(p7501_jigsweep_outcome_t outcome) {
    switch (outcome) {
        case P7501_SWEEP_OUTCOME_SERIAL:
            return "SERIAL_BOOT";
        case P7501_SWEEP_OUTCOME_NORMAL:
            return "NORMAL_BOOT";
        case P7501_SWEEP_OUTCOME_EARLY_OTHER:
            return "EARLY_OTHER";
        case P7501_SWEEP_OUTCOME_SILENT:
        default:
            return "SILENT";
    }
}
