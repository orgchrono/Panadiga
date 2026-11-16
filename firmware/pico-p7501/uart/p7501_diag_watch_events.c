/**
 * @file p7501_diag_watch_events.c
 * @brief Signature classifier and human-readable diagnosis formatter for `DIAGWATCH`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_diag_watch_codec.h"

#include <stddef.h>
#include <string.h>

typedef struct diag_sig_rule {
    const char *needle;
    size_t len;
    uint32_t flag;
} diag_sig_rule_t;

#define DIAG_RULE(lit, flag_val) { (lit), sizeof(lit) - 1u, (flag_val) }

static const diag_sig_rule_t k_diag_rules[] = {
    DIAG_RULE("DDR CHECK: OK",                  P7501_DIAG_EVT_DDR_OK),
    DIAG_RULE("serial boot mode",               P7501_DIAG_EVT_SERIAL_BOOT),
    DIAG_RULE("... SD init end",                P7501_DIAG_EVT_SD_HW_OK),
    DIAG_RULE("-91006",                         P7501_DIAG_EVT_SD_ERR),
    DIAG_RULE("-90002",                         P7501_DIAG_EVT_SD_ERR),
    DIAG_RULE("-90001",                         P7501_DIAG_EVT_SD_ERR),
    DIAG_RULE("FAT Mount failed",               P7501_DIAG_EVT_FAT_FAIL),
    DIAG_RULE("FAT Open(",                      P7501_DIAG_EVT_FAT_FAIL),
    DIAG_RULE("SD diag load failed",            P7501_DIAG_EVT_SD_DIAG_LOAD_FAIL),
    DIAG_RULE("diag loader uncompress failed",  P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL),
    DIAG_RULE("fvu uncompress failed",          P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL),
    DIAG_RULE("no header!",                     P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL),
    DIAG_RULE("no footer!",                     P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL),
    DIAG_RULE("check sum mismatch",             P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL),
    DIAG_RULE("*** start diag ***",             P7501_DIAG_EVT_DIAG_EXEC),
    DIAG_RULE("diag loader is preserved",       P7501_DIAG_EVT_DIAG_EXEC),
    DIAG_RULE("** open tray **",                P7501_DIAG_EVT_ODD_OPEN),
    DIAG_RULE("can not close tray",             P7501_DIAG_EVT_ODD_TRAY_FAIL),
    DIAG_RULE("80045300",                       P7501_DIAG_EVT_ODD_TRAY_FAIL),
    DIAG_RULE("80040904",                       P7501_DIAG_EVT_CD_APC_ERR),
    DIAG_RULE("83A7",                           P7501_DIAG_EVT_CD_APC_ERR),
    DIAG_RULE("BGO driver init failed",         P7501_DIAG_EVT_BGO_FAIL),
    DIAG_RULE("hung-up time changed",           P7501_DIAG_EVT_HUNG_U99),
    DIAG_RULE("PANAMON1>",                      P7501_DIAG_EVT_PANAMON),
};

#undef DIAG_RULE

uint32_t p7501_diag_watch_feed_byte(
    p7501_diag_watch_state_t *st,
    const p7501_diag_watch_spec_t *spec,
    uint8_t byte,
    char *out_tx_char,
    bool *out_release_sw_uart
) {
    if (out_tx_char != NULL) {
        *out_tx_char = '\0';
    }
    if (out_release_sw_uart != NULL) {
        *out_release_sw_uart = false;
    }
    if ((st == NULL) || (spec == NULL)) {
        return 0u;
    }

    const size_t cap = sizeof(st->ring);
    if (st->ring_len < cap) {
        st->ring[st->ring_len++] = byte;
    } else {
        memmove(&st->ring[0], &st->ring[1], cap - 1u);
        st->ring[cap - 1u] = byte;
    }

    uint32_t newly_hit = 0u;
    for (size_t i = 0u; i < (sizeof(k_diag_rules) / sizeof(k_diag_rules[0])); ++i) {
        const size_t nlen = k_diag_rules[i].len;
        if (st->ring_len >= nlen &&
            memcmp(&st->ring[st->ring_len - nlen], k_diag_rules[i].needle, nlen) == 0) {
            const uint32_t flag = k_diag_rules[i].flag;
            if ((st->seen_events & flag) == 0u) {
                st->seen_events |= flag;
                newly_hit |= flag;
            }
        }
    }

    const uint32_t release_mask = (spec->boot_action == P7501_BOOT_ACTION_JIG)
        ? (P7501_DIAG_EVT_SERIAL_BOOT | P7501_DIAG_EVT_BGO_FAIL | P7501_DIAG_EVT_HUNG_U99)
        : P7501_DIAG_EVT_DDR_OK;

    if ((newly_hit & release_mask) != 0u &&
        spec->release_on_ddr && !st->sw_uart_released) {
        st->sw_uart_released = true;
        if (out_release_sw_uart != NULL) {
            *out_release_sw_uart = true;
        }
    }

    if ((newly_hit & P7501_DIAG_EVT_PANAMON) != 0u && !st->tx_dispatched) {
        const char ch = p7501_diag_watch_action_char(spec->action);
        if (ch != '\0') {
            st->tx_dispatched = true;
            if (out_tx_char != NULL) {
                *out_tx_char = ch;
            }
        }
    }
    return newly_hit;
}

const char *p7501_diag_watch_diagnosis(uint32_t events) {
    if ((events & P7501_DIAG_EVT_PANAMON) != 0u) {
        return "PANAMON1 active on SC0 (ready for [B/R/W/D/F/J/E/Q])";
    }
    if ((events & P7501_DIAG_EVT_DIAG_EXEC) != 0u) {
        return "Stage-1 DIGADIAG.FRM decompressed & jumped (waiting PANAMON1> / factory UI)";
    }
    if ((events & P7501_DIAG_EVT_SD_UNCOMPRESS_FAIL) != 0u) {
        return ((events & P7501_DIAG_EVT_BGO_FAIL) != 0u)
            ? "Stage-1 DIGADIAG.FRM opened, EXTRHEAD/zlib unpack failed (masked by post-reset BGO)"
            : "Stage-1 DIGADIAG.FRM opened, EXTRHEAD/zlib unpack failed (check container header/footer)";
    }
    if ((events & P7501_DIAG_EVT_SD_DIAG_LOAD_FAIL) != 0u) {
        if ((events & P7501_DIAG_EVT_FAT_FAIL) != 0u) {
            return "Stage-1 SD HW OK (4-bit), FAT mount/open failed (-10010=missing,-10020=8.3,-10025=BPB/FAT32) before BGO reboot";
        }
        if ((events & P7501_DIAG_EVT_SD_HW_OK) != 0u) {
            return ((events & P7501_DIAG_EVT_BGO_FAIL) != 0u)
                ? "Stage-1 SD HW OK (4-bit), FAILED at SD diag load failed[0] (Mount+Open OK, fat_read 0x20000 failed) before BGO reboot"
                : "Stage-1 SD HW OK (4-bit), FAILED at SD diag load failed[0] (Mount+Open OK, DIGADIAG.FRM < 128KiB or cluster read fail)";
        }
        return "Stage-1 FAILED at SD diag load failed[0] (Mount+Open OK, fat_read 0x20000 != 128KiB)";
    }
    if ((events & P7501_DIAG_EVT_BGO_FAIL) != 0u && (events & P7501_DIAG_EVT_SERIAL_BOOT) == 0u) {
        if ((events & P7501_DIAG_EVT_CD_APC_ERR) != 0u) {
            return "BGO driver init failed[1] -> U99 hang (normal mode; ODD 80040904 also observed)";
        }
        if ((events & P7501_DIAG_EVT_ODD_TRAY_FAIL) != 0u) {
            return "BGO driver init failed[1] -> U99 hang (normal mode; ODD tray 80045300 also observed)";
        }
        return ((events & P7501_DIAG_EVT_HUNG_U99) != 0u)
            ? "BGO driver init failed[1] -> U99 hang (load patched DIGADIAG.FRM)"
            : "BGO driver init failed[1] (corrupt/erased BGO partition)";
    }
    if (((events & P7501_DIAG_EVT_SD_ERR) != 0u) && ((events & P7501_DIAG_EVT_CD_APC_ERR) != 0u)) {
        return "SDDIAG -91006 + CDDIAG 80040904 (SDSC 4-bit failed; ODD tray/laser APC failed)";
    }
    if ((events & P7501_DIAG_EVT_SD_ERR) != 0u) {
        return ((events & P7501_DIAG_EVT_ODD_TRAY_FAIL) != 0u)
            ? "SDDIAG SD init error + CDDIAG ODD tray 80045300 (check SDSC card/DAT lines)"
            : "SDDIAG -91006 (4-bit SD_STATUS mismatch)";
    }
    if ((events & P7501_DIAG_EVT_CD_APC_ERR) != 0u) {
        return "CDDIAG 80040904 / 0x83A7 (Laser APC < 0.5mW)";
    }
    if ((events & P7501_DIAG_EVT_ODD_TRAY_FAIL) != 0u) {
        return "ODD ** can not close tray ** (80045300 / no disc)";
    }
    if ((events & P7501_DIAG_EVT_BGO_FAIL) != 0u) {
        return ((events & P7501_DIAG_EVT_HUNG_U99) != 0u)
            ? "BGO driver init failed[1] -> U99 hang (load patched DIGADIAG.FRM)"
            : "BGO driver init failed[1] (corrupt/erased BGO partition)";
    }
    if ((events & P7501_DIAG_EVT_ODD_OPEN) != 0u) {
        return "Serial boot: ODD ** open tray ** state reported";
    }
    if ((events & P7501_DIAG_EVT_SERIAL_BOOT) != 0u) {
        return ((events & P7501_DIAG_EVT_SD_HW_OK) != 0u)
            ? "Serial boot: SD 4-bit init OK (waiting DIGADIAG load)"
            : "Serial boot mode entered (waiting SD/CD media)";
    }
    if ((events & P7501_DIAG_EVT_DDR_OK) != 0u) {
        return "DDR CHECK: OK seen; normal boot progress";
    }
    return "No known Panasonic boot signatures observed";
}
