/**
 * @file p7501_mmio_snapshot.c
 * @brief Imperative `MMIOSNAP` CLI handler reading safe AM34 registers via `p7501_hw_read_mem32`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_mmio_snapshot_codec.h"
#include "cli/p7501_cli.h"
#include "common/p7501_hex.h"
#include "hal/p7501_hw.h"
#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static p7501_mmio_snapshot_t s_prev_snap;
static bool s_has_prev = false;

void p7501_cmd_mmio_snap(const char *arg) {
    p7501_hw_enforce_nsw5v_hiz();
    if (!p7501_hw_is_target_powered() || !p7501_hw_is_target_online()) {
        p7501_cli_set_result("ERR", "EOFFLINE");
        printf("MMIO_SNAP: ERROR (target offline or unpowered)\n");
        return;
    }

    const bool diff_only = (arg != NULL) && p7501_ascii_ieq(arg, "DIFF");
    size_t count = 0u;
    const p7501_mmio_reg_desc_t *tbl = p7501_mmio_safe_table(&count);
    p7501_mmio_snapshot_t cur;
    memset(&cur, 0, sizeof(cur));

    for (size_t i = 0u; i < count; ++i) {
        if (p7501_mmio_is_safe_read_addr(tbl[i].addr)) {
            uint32_t val = 0u;
            if (p7501_hw_read_mem32_checked(tbl[i].addr, &val)) {
                cur.values[i] = val;
                cur.valid[i] = true;
            }
        }
    }
    const uint32_t diff = s_has_prev ? p7501_mmio_diff_mask(&s_prev_snap, &cur) : 0u;
    printf("[MMIO_SNAP] count=%u diff_mask=0x%02lX (0xC000100C excluded)\n",
           (unsigned)count, (unsigned long)diff);
    for (size_t i = 0u; i < count; ++i) {
        const bool chg = (diff & (1u << i)) != 0u;
        if (!diff_only || chg || !s_has_prev) {
            if (cur.valid[i]) {
                printf("  %-9s 0x%08" PRIX32 " = 0x%08" PRIX32 "%s\n",
                       tbl[i].name, tbl[i].addr, cur.values[i], chg ? " *" : "");
            } else {
                printf("  %-9s 0x%08" PRIX32 " = <BUS_ERR>\n", tbl[i].name, tbl[i].addr);
            }
        }
    }
    s_prev_snap = cur;
    s_has_prev = true;
    printf("MMIO_SNAP: OK\n");
}
