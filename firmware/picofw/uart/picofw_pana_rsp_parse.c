/**
 * @file picofw_pana_rsp_parse.c
 * @brief Pure ISO C11 `PANARSP` CLI argument tokenizer and action specification parser.
 *
 * Compiled identically on Pico ARM/RISC-V and host C11 test targets (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_pana_rsp.h"
#include "uart/picofw_pana_rsp_internal.h"
#include "uart/picofw_pana_dumpnor.h"
#include "uart/picofw_pana_jedec.h"
#include "flash/picofw_flash.h"
#include "common/picofw_hex.h"
#include "common/picofw_wdt_logic.h"
#include "hal/picofw_wireless_codec.h"

#include <stdio.h>
#include <string.h>

static bool is_mutating_rsp_action(picofw_pana_rsp_action_t action) {
    switch (action) {
        case PANA_RSP_ACT_RAW:
        case PANA_RSP_ACT_HEX:
        case PANA_RSP_ACT_WMEM:
        case PANA_RSP_ACT_WRITE32:
        case PANA_RSP_ACT_PASSTHROUGH:
        case PANA_RSP_ACT_CALL:
        case PANA_RSP_ACT_JEDEC_ERASE:
        case PANA_RSP_ACT_JEDEC_PROG:
        case PANA_RSP_ACT_BGO_BITCLEAR:
            return true;
        default:
            return false;
    }
}

bool picofw_panarsp_parse_args_ctx(
    const char *arg,
    bool session_unlocked,
    picofw_pana_rsp_spec_t *out
) {
    if (out == NULL) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->timeout_ms = 4000u;
    out->boot_action = PICOFW_BOOT_ACTION_PASSIVE;
    out->send_nak_prefix = true;
    out->action = PANA_RSP_ACT_SEQ;
    out->policy_req = PICOFW_POLICY_REQ_NONE;

    if (arg == NULL || arg[0] == '\0') {
        out->nor_unlocked_effective = picofw_flash_effective_policy(session_unlocked ? 1 : 0);
        return true;
    }

    const char *cur = arg;
    char tok[180];
    bool saw_boot = false;
    bool saw_action = false;
    bool explicit_nak_only = false;
    bool explicit_magic_only = false;
    int policy_override = -1;
    uint8_t dumpnor_argc = 0u;

    while ((cur = picofw_next_token(cur, tok, sizeof(tok))) != NULL) {
        if (tok[0] == '\0') {
            continue;
        }
        uint32_t dump_hex = 0u;
        bool bad_pol = false;
        if (saw_action && (out->action == PANA_RSP_ACT_DUMPNOR) && (dumpnor_argc < 2u) &&
            picofw_dumpnor_parse_hex_tok(tok, &dump_hex)) {
            if (dumpnor_argc == 0u) {
                out->addr = dump_hex;
            } else {
                out->value_u32 = dump_hex;
            }
            dumpnor_argc += 1u;
        } else if (picofw_flash_consume_policy_tok(tok, &policy_override, &bad_pol)) {
            if (bad_pol) return false;
        } else if (picofw_ascii_is_digits(tok)) {
            out->timeout_ms = picofw_parse_uint32_clamped(
                tok,
                4000u,
                150u,
                PICOFW_WDT_MAX_CMD_WINDOW_MS
            );
        } else if (picofw_ascii_ieq(tok, "MAGIC")) {
            out->send_magic_prefix = true;
            explicit_magic_only = true;
        } else if (picofw_ascii_ieq(tok, "NONAK")) {
            out->send_nak_prefix = false;
        } else if (picofw_ascii_ieq(tok, "NAK")) {
            out->send_nak_prefix = true;
            explicit_nak_only = true;
        } else if (picofw_ascii_iprefix(tok, "WAIT:") && tok[5] != '\0') {
            out->wait_trigger_enabled = true;
            snprintf(out->trigger, sizeof(out->trigger), "%s", tok + 5);
        } else if (!saw_boot && picofw_is_boot_spec_token(tok)) {
            out->boot_action = picofw_parse_boot_spec(tok, &out->jig_hold_ms);
            saw_boot = true;
        } else if (!picofw_rsp_apply_action_token(tok, out, &saw_action)) {
            return false;
        }
    }

    const bool eff_unlocked = (policy_override != -1)
        ? picofw_flash_effective_policy(policy_override)
        : picofw_flash_effective_policy(session_unlocked ? 1 : 0);
    out->nor_unlocked_effective = eff_unlocked;
    out->policy_req = (policy_override == 1)
        ? PICOFW_POLICY_REQ_UNLOCK
        : ((policy_override == 0) ? PICOFW_POLICY_REQ_LOCK : PICOFW_POLICY_REQ_NONE);

    if (!saw_action) {
        if (policy_override != -1) {
            out->action = PANA_RSP_ACT_NOR_POLICY;
            out->send_nak_prefix = false;
        } else if (explicit_nak_only) {
            out->action = PANA_RSP_ACT_NAK;
        } else if (explicit_magic_only && !out->send_nak_prefix) {
            out->action = PANA_RSP_ACT_MAGIC;
        }
    }
    if ((out->action == PANA_RSP_ACT_MEM || out->action == PANA_RSP_ACT_XMEM) &&
        picofw_bus_is_lethal_mmio_range(out->addr, (size_t)out->value_u32)) {
        out->rejected_e05 = true;
        return false;
    }
    if ((out->action == PANA_RSP_ACT_PASSTHROUGH || out->action == PANA_RSP_ACT_CALL) &&
        !picofw_wireless_nor_write_allowed(picofw_wireless_global_state())) {
        out->rejected_e05 = true;
        return false;
    }
    if ((out->action == PANA_RSP_ACT_CALL) &&
        picofw_bus_is_lethal_mmio_range(out->addr, 4u)) {
        out->rejected_e05 = true;
        return false;
    }
    if ((out->action == PANA_RSP_ACT_WMEM || out->action == PANA_RSP_ACT_WRITE32) &&
        picofw_flash_is_write_blocked_with_policy(out->addr, out->data_len, eff_unlocked)) {
        out->rejected_e05 = true;
        return false;
    }
    if ((out->action == PANA_RSP_ACT_JEDEC_ERASE) &&
        !picofw_jedec_validate_erase_addr(out->addr, eff_unlocked, NULL)) {
        out->rejected_e05 = true;
        return false;
    }
    if ((out->action == PANA_RSP_ACT_JEDEC_PROG || out->action == PANA_RSP_ACT_BGO_BITCLEAR) &&
        !picofw_jedec_validate_prog_range(out->addr, out->data_len, eff_unlocked, NULL)) {
        out->rejected_e05 = true;
        return false;
    }
    if ((out->action == PANA_RSP_ACT_RAW) &&
        !picofw_rsp_validate_raw_or_hex_guard(
            (const uint8_t *)out->payload, strlen(out->payload), eff_unlocked, true, &out->rejected_e05)) {
        return false;
    }
    if ((out->action == PANA_RSP_ACT_HEX) &&
        !picofw_rsp_validate_raw_or_hex_guard(
            out->raw_bytes, out->raw_len, eff_unlocked, false, &out->rejected_e05)) {
        return false;
    }
    if ((out->action == PANA_RSP_ACT_DUMPNOR) &&
        (picofw_bus_is_lethal_mmio_range(out->addr, (size_t)out->value_u32) ||
         !picofw_dumpnor_validate_range(out->addr, out->value_u32))) {
        return false;
    }
    out->auto_relock_nor = eff_unlocked && is_mutating_rsp_action(out->action);
    return true;
}

bool picofw_panarsp_parse_args(const char *arg, picofw_pana_rsp_spec_t *out) {
    return picofw_panarsp_parse_args_ctx(arg, picofw_flash_is_upper_nor_unlocked(), out);
}

void picofw_panarsp_apply_policy(const picofw_pana_rsp_spec_t *spec) {
    if (spec != NULL) {
        picofw_flash_apply_policy_req(spec->policy_req);
    }
}

bool picofw_panarsp_finish_action(const picofw_pana_rsp_spec_t *spec) {
    bool relocked = false;
    if (spec != NULL) {
        if (is_mutating_rsp_action(spec->action)) {
            picofw_wireless_consume_nor_op(picofw_wireless_global_state());
        }
        if (spec->auto_relock_nor && picofw_flash_is_upper_nor_unlocked()) {
            picofw_flash_set_upper_nor_unlocked(false);
            relocked = true;
        }
    }
    return relocked;
}
