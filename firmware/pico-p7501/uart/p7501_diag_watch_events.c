/**
 * @file p7501_diag_watch_events.c
 * @brief Signature classifier and human-readable diagnosis formatter for `DIAGWATCH`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_diag_watch_codec.h"
#include "p7501_types.h"

#include <stddef.h>
#include <string.h>

typedef struct diag_sig_rule {
    const char *needle;
    size_t len;
    uint32_t flag;
} diag_sig_rule_t;

static const diag_sig_rule_t k_diag_rules[] = P7501_SSOT_DIAG_RULES_INIT;

_Static_assert(
    (sizeof(k_diag_rules) / sizeof(k_diag_rules[0])) == P7501_SSOT_DIAG_RULE_COUNT,
    "SSOT diag signature rule table count mismatch"
);

uint32_t p7501_diag_watch_feed_byte_ex(
    p7501_diag_watch_state_t *st,
    const p7501_diag_watch_spec_t *spec,
    uint8_t byte,
    char *out_tx_char,
    bool *out_release_sw_uart,
    bool *out_relock_now
) {
    if (out_tx_char != NULL) {
        *out_tx_char = '\0';
    }
    if (out_release_sw_uart != NULL) {
        *out_release_sw_uart = false;
    }
    if (out_relock_now != NULL) {
        *out_relock_now = false;
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

    bool panamon_prompt_now = false;
    uint32_t newly_hit = 0u;
    for (size_t i = 0u; i < (sizeof(k_diag_rules) / sizeof(k_diag_rules[0])); ++i) {
        const size_t nlen = k_diag_rules[i].len;
        if (st->ring_len >= nlen &&
            memcmp(&st->ring[st->ring_len - nlen], k_diag_rules[i].needle, nlen) == 0) {
            const uint32_t flag = k_diag_rules[i].flag;
            if (flag == P7501_DIAG_EVT_PANAMON) {
                panamon_prompt_now = true;
            }
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

    const bool is_write_or_erase =
        (spec->action == P7501_DIAG_ACT_WRITE) || (spec->action == P7501_DIAG_ACT_ERASE) ||
        (spec->action == P7501_DIAG_ACT_FORMAT) ||
        ((spec->action == P7501_DIAG_ACT_TXN) && spec->nor_unlock);
    if (panamon_prompt_now) {
        if (!st->tx_dispatched) {
            if (spec->nor_unlock && !st->unlock_dispatched) {
                st->unlock_dispatched = true;
                if (out_tx_char != NULL) {
                    *out_tx_char = 'U';
                }
            } else if ((spec->action == P7501_DIAG_ACT_TXN) && (st->txn_cursor < spec->txn_count)) {
                st->txn_cursor += 1u;
                if (st->txn_cursor >= spec->txn_count) {
                    st->tx_dispatched = true;
                }
                if (out_tx_char != NULL) {
                    *out_tx_char = 'T';
                }
            } else {
                const char ch = p7501_diag_watch_action_char(spec->action);
                if (ch != '\0') {
                    st->tx_dispatched = true;
                    if (out_tx_char != NULL) {
                        *out_tx_char = ch;
                    }
                }
            }
        } else if (spec->nor_unlock && is_write_or_erase && !st->relock_dispatched) {
            st->relock_dispatched = true;
            if (out_relock_now != NULL) {
                *out_relock_now = true;
            }
            if (out_tx_char != NULL) {
                *out_tx_char = 'L';
            }
        }
    }
    return newly_hit;
}

uint32_t p7501_diag_watch_feed_byte(
    p7501_diag_watch_state_t *st,
    const p7501_diag_watch_spec_t *spec,
    uint8_t byte,
    char *out_tx_char,
    bool *out_release_sw_uart
) {
    bool unused_relock = false;
    return p7501_diag_watch_feed_byte_ex(
        st, spec, byte, out_tx_char, out_release_sw_uart, &unused_relock
    );
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
