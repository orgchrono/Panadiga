/**
 * @file picofw_mmio_snapshot.c
 * @brief Imperative `MMIO_SNAP` CLI handler reading domain-scoped SSOT AM34 registers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_mmio_snapshot_codec.h"
#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "hal/picofw_hw.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static picofw_mmio_snapshot_t s_prev_snap;
static bool s_has_prev = false;

bool picofw_mmio_snap_domain_hw(
    uint8_t domain_mask,
    picofw_mmio_snapshot_t *out_snap,
    uint32_t *out_valid
) {
    if (out_valid != NULL) {
        *out_valid = 0u;
    }
    if ((out_snap == NULL) || (domain_mask == 0u)) {
        return false;
    }
    picofw_hw_enforce_nsw5v_hiz();
    if (!picofw_hw_is_target_powered() || !picofw_hw_is_target_online()) {
        return false;
    }

    size_t count = 0u;
    const picofw_mmio_reg_desc_t *const tbl = picofw_mmio_safe_table(&count);
    memset(out_snap, 0, sizeof(*out_snap));
    uint32_t valid_cnt = 0u;

    for (size_t i = 0u; i < count; ++i) {
        if ((tbl[i].domain & domain_mask) == 0u) {
            continue;
        }
        if (picofw_mmio_is_safe_read_addr(tbl[i].addr)) {
            uint32_t val = 0u;
            if (picofw_hw_read_mem32_checked(tbl[i].addr, &val)) {
                out_snap->values[i] = val;
                out_snap->valid[i] = true;
                valid_cnt += 1u;

                char dec[64];
                const size_t dlen = picofw_mmio_decode_video_reg(tbl[i].addr, val, dec, sizeof(dec));
                char msg[160];
                (void)snprintf(
                    msg, sizeof(msg),
                    "reg=%s addr=0x%08" PRIX32 " val=0x%08" PRIX32 " dom=0x%02X%s%s",
                    tbl[i].name, tbl[i].addr, val, (unsigned)tbl[i].domain,
                    (dlen > 0u) ? " " : "", (dlen > 0u) ? dec : ""
                );
                (void)picofw_push_emit_str("MMIO", msg);
            }
        }
    }
    (void)picofw_push_flush_global();
    if (out_valid != NULL) {
        *out_valid = valid_cnt;
    }
    return true;
}

void picofw_cmd_mmio_snap(const char *arg) {
    bool diff_only = false;
    const uint8_t domain_mask = picofw_mmio_parse_domain(arg, &diff_only);
    if (domain_mask == 0u) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("MMIO_SNAP: ERROR (Usage: MMIO_SNAP [CORE|VIDEO|STORAGE|ALL|DIFF])\n");
        return;
    }

    picofw_mmio_snapshot_t cur;
    uint32_t valid_cnt = 0u;
    if (!picofw_mmio_snap_domain_hw(domain_mask, &cur, &valid_cnt)) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("MMIO_SNAP: ERROR (target offline or unpowered)\n");
        return;
    }

    size_t count = 0u;
    const picofw_mmio_reg_desc_t *const tbl = picofw_mmio_safe_table(&count);
    const uint32_t diff = s_has_prev ? picofw_mmio_diff_mask(&s_prev_snap, &cur) : 0u;
    printf("[MMIO_SNAP] dom=0x%02X valid=%" PRIu32 " diff_mask=0x%07lX (0xC000100C+0xDC0F0000 excluded)\n",
           (unsigned)domain_mask, valid_cnt, (unsigned long)diff);

    for (size_t i = 0u; i < count; ++i) {
        if ((tbl[i].domain & domain_mask) == 0u) {
            continue;
        }
        const bool chg = (diff & (1u << i)) != 0u;
        if (!diff_only || chg || !s_has_prev) {
            if (cur.valid[i]) {
                char dec[64];
                const size_t dlen = picofw_mmio_decode_video_reg(tbl[i].addr, cur.values[i], dec, sizeof(dec));
                printf("  %-9s 0x%08" PRIX32 " = 0x%08" PRIX32 "%s%s%s\n",
                       tbl[i].name, tbl[i].addr, cur.values[i],
                       (dlen > 0u) ? " [" : "", (dlen > 0u) ? dec : "", (dlen > 0u) ? "]" : "");
                if (chg) {
                    printf("    *changed\n");
                }
            } else {
                printf("  %-9s 0x%08" PRIX32 " = <BUS_ERR>\n", tbl[i].name, tbl[i].addr);
            }
        }
    }
    s_prev_snap = cur;
    s_has_prev = true;
    printf("MMIO_SNAP: OK\n");
}
