/**
 * @file picofw_pana_rsp.c
 * @brief Active `PANARSP` probe against EH55 MAIN `_dev_SERIAL` RSP stub over PIO1.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_pana_rsp.h"
#include "cli/picofw_cli.h"
#include "uart/picofw_pana_rsp_exec.h"
#include "uart/picofw_pana_rsp_report.h"
#include "uart/picofw_uart_pio_engine.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static picofw_pana_rsp_spec_t s_rsp_spec;

void picofw_cmd_pana_rsp(const char *arg) {
    panarsp_rx_cap_t *const cap = &g_picofw_cmd_arena.panarsp;
    picofw_pana_rsp_spec_t *const sp = &s_rsp_spec;

    if (!picofw_panarsp_parse_args(arg, sp)) {
        if (sp->rejected_e05) {
            picofw_cli_set_result("ERR", "E05");
            printf("PANARSP: ERROR E05 (Write/erase blocked: SA0..SA10 is permanent read-only; Upper NOR requires NORUNLOCK:CONFIRM_UPPER_NOR)\n");
        } else {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("PANARSP: ERROR (Usage: PANARSP [ms] [NORMAL|JIG[:ms]|NOPULL] [MAGIC] [NONAK] [NORUNLOCK:CONFIRM_UPPER_NOR|NORLOCK] [WAIT:<trig>] [PROBESEQ|G|REGS|Z0|Z1|MEM:<addr>,<len>|XMEM:<addr>,<len>|WMEM:<addr>,<hex>|WRITE32:<addr>,<u32>|JEDEC_ERASE:<addr>|JEDEC_PROG:<addr>,<hex>|PASSTHROUGH|CALL:<addr>|DUMPNOR [start] [len]|NAK|RAW:<txt>|HEX:<hex>])\n");
        }
        return;
    }
    picofw_panarsp_apply_policy(sp);
    if (sp->action == PANA_RSP_ACT_NOR_POLICY) {
        printf("PANARSP: OK (upper_nor=%s, sa0_sa10=PERMANENT_E05)\n",
               sp->nor_unlocked_effective ? "UNLOCKED" : "LOCKED");
        return;
    }

    memset(cap, 0, sizeof(*cap));
    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RESET_SYNC);
    picofw_hw_enforce_nsw5v_hiz();
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    printf("=== PANARSP PIO1 (baud=%" PRIu32 " ms=%" PRIu32 " boot=%d hold=%" PRIu32 " action=%d) ===\n",
           picofw_pio_get_baud(),
           sp->timeout_ms,
           (int)sp->boot_action,
           sp->jig_hold_ms,
           (int)sp->action);

    picofw_pio_begin_rx_session_ex(sp->boot_action, sp->jig_hold_ms);

    bool triggered = !sp->wait_trigger_enabled;
    if (sp->wait_trigger_enabled) {
        uint8_t ring[PICOFW_RSP_MAX_TRIG + 1u];
        uint8_t jig_ring[24];
        size_t ring_len = 0u;
        size_t jig_ring_len = 0u;
        const uint64_t wait_t0 = time_us_64();
        const uint64_t wait_deadline = wait_t0 + ((uint64_t)sp->timeout_ms * 1000ULL);
        while (!triggered && picofw_wdt_loop_step(wait_t0, wait_deadline, true, NULL)) {
            picofw_pio_poll_sw_uart_release();
            uint8_t b = 0u;
            while (picofw_pio_pop_rx_byte(&b)) {
                panarsp_push_rx(cap, b, true);
                if (sp->boot_action == PICOFW_BOOT_ACTION_JIG) {
                    (void)picofw_pio_poll_jig_serial_release(
                        jig_ring, &jig_ring_len, sizeof(jig_ring), b
                    );
                }
                if (picofw_rolling_match_push(ring, &ring_len, sizeof(ring), b, sp->trigger)) {
                    triggered = true;
                    break;
                }
            }
            tight_loop_contents();
        }
        if (triggered) {
            panarsp_compact_pretrig(cap, PANARSP_PRETRIG_KEEP);
        }
    }

    const size_t post_tx_off = cap->len;
    size_t primary_off = post_tx_off;
    bool dump_hdr = false;
    bool dump_ftr = false;
    uint32_t dump_bytes = 0u;
    uint32_t dump_ff_blocks = 0u;
    const char *jedec_err_code = NULL;
    if (triggered) {
        picofw_wdt_set_stage(PICOFW_WDT_STAGE_TX_WAIT);
        const bool need_handshake = sp->send_magic_prefix ||
                                    (sp->action == PANA_RSP_ACT_MAGIC) ||
                                    (sp->action == PANA_RSP_ACT_PASSTHROUGH) ||
                                    (sp->action == PANA_RSP_ACT_CALL) ||
                                    (sp->action == PANA_RSP_ACT_DUMPNOR) ||
                                    (sp->action == PANA_RSP_ACT_FAST_DUMPNOR) ||
                                    (sp->action == PANA_RSP_ACT_DUMPNOR_RESUME) ||
                                    (sp->action == PANA_RSP_ACT_JEDEC_ERASE) ||
                                    (sp->action == PANA_RSP_ACT_JEDEC_PROG) ||
                                    (sp->action == PANA_RSP_ACT_BGO_BITCLEAR) ||
                                    (sp->action == PANA_RSP_ACT_JEDEC_ID) ||
                                    (sp->action == PANA_RSP_ACT_CFI_QUERY) ||
                                    (sp->action == PANA_RSP_ACT_CRC32_NOR) ||
                                    (sp->action == PANA_RSP_ACT_BLANK_CHECK);
        if (need_handshake) {
            const size_t step_off = cap->len;
            picofw_pio_tx_bytes(k_picofw_rsp_handshake, PICOFW_RSP_HANDSHAKE_LEN, false);
            panarsp_drain_for_ms(cap, 30u, false, step_off);
        }
        if (sp->send_nak_prefix && sp->action != PANA_RSP_ACT_MAGIC) {
            static const uint8_t k_nak = PICOFW_RSP_NAK_BYTE;
            const size_t step_off = cap->len;
            picofw_pio_tx_bytes(&k_nak, 1u, false);
            panarsp_drain_for_ms(cap, (sp->action == PANA_RSP_ACT_NAK) ? 350u : 80u, true, step_off);
        }
        primary_off = cap->len;
        switch (sp->action) {
            case PANA_RSP_ACT_SEQ: {
                static const char *const k_probe_seq[] = { "g", "Z0" };
                primary_off = panarsp_tx_sequence(cap, k_probe_seq, 2u, 250u);
                break;
            }
            case PANA_RSP_ACT_PASSTHROUGH: {
                static const char *const k_pass_seq[PICOFW_TBC_PASSTHROUGH_SEQ_LEN] =
                    PICOFW_TBC_PASSTHROUGH_SEQ_INIT;
                printf("[PANARSP] Applying %u MAIN AV TBC E-E passthrough sites via $M...\n",
                       (unsigned)PICOFW_TBC_PASSTHROUGH_SEQ_LEN);
                primary_off = panarsp_tx_sequence(
                    cap,
                    k_pass_seq,
                    PICOFW_TBC_PASSTHROUGH_SEQ_LEN,
                    250u
                );
                break;
            }
            case PANA_RSP_ACT_CALL: {
                const char *call_seq[2] = { sp->payload, "g" };
                printf("[PANARSP] Updating serial callback 0x%08X -> 0x%08X and triggering $g...\n",
                       (unsigned)PICOFW_AM34_SERIAL_CB_PTR_ADDR,
                       (unsigned)sp->addr);
                primary_off = panarsp_tx_sequence(cap, call_seq, 2u, 250u);
                break;
            }
            case PANA_RSP_ACT_DUMPNOR:
            case PANA_RSP_ACT_FAST_DUMPNOR:
            case PANA_RSP_ACT_DUMPNOR_RESUME:
                primary_off = panarsp_execute_dumpnor(
                    cap, sp, &dump_hdr, &dump_ftr, &dump_bytes, &dump_ff_blocks
                );
                break;
            case PANA_RSP_ACT_JEDEC_ERASE:
            case PANA_RSP_ACT_JEDEC_PROG:
            case PANA_RSP_ACT_BGO_BITCLEAR:
                primary_off = panarsp_execute_jedec(
                    cap, sp, &dump_ftr, &dump_hdr, &dump_bytes, &jedec_err_code
                );
                break;
            case PANA_RSP_ACT_JEDEC_ID:
            case PANA_RSP_ACT_CFI_QUERY:
            case PANA_RSP_ACT_CRC32_NOR:
            case PANA_RSP_ACT_BLANK_CHECK:
                primary_off = panarsp_execute_jedec_id_or_sum(
                    cap, sp, &dump_ftr, &dump_bytes
                );
                break;
            case PANA_RSP_ACT_G:
            case PANA_RSP_ACT_REGS:
            case PANA_RSP_ACT_Z0:
            case PANA_RSP_ACT_Z1:
            case PANA_RSP_ACT_MEM:
            case PANA_RSP_ACT_XMEM:
            case PANA_RSP_ACT_WMEM:
            case PANA_RSP_ACT_WRITE32:
            case PANA_RSP_ACT_RAW:
                (void)panarsp_tx_frame(sp->payload);
                panarsp_drain_for_ms(cap, (sp->timeout_ms < 700u) ? sp->timeout_ms : 700u, true, primary_off);
                break;
            case PANA_RSP_ACT_HEX:
                picofw_pio_tx_bytes(sp->raw_bytes, sp->raw_len, false);
                panarsp_drain_for_ms(cap, 500u, true, primary_off);
                break;
            case PANA_RSP_ACT_NAK:
            case PANA_RSP_ACT_MAGIC:
            default:
                primary_off = post_tx_off;
                break;
        }
    }

    const bool fifo_ovf = picofw_pio_had_rx_overflow();
    picofw_pio_end_session();
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    const bool relocked_nor = picofw_panarsp_finish_action(sp);
    if (!triggered) {
        picofw_cli_set_result("ERR", "ETIMEOUT");
    }
    if (sp->action == PANA_RSP_ACT_DUMPNOR ||
        sp->action == PANA_RSP_ACT_FAST_DUMPNOR ||
        sp->action == PANA_RSP_ACT_DUMPNOR_RESUME) {
        const bool ebaud = (sp->action == PANA_RSP_ACT_FAST_DUMPNOR) &&
                           (sp->baud != 0u) && (sp->baud != PICOFW_SSOT_DEFAULT_BAUD);
        if (triggered && !dump_ftr && !ebaud) {
            picofw_cli_set_result("ERR", "EIO");
        }
        char dump_extra[64];
        (void)snprintf(
            dump_extra,
            sizeof(dump_extra),
            "sub=%s ff_blocks=%lu",
            (sp->action == PANA_RSP_ACT_DUMPNOR_RESUME)
                ? "DUMPNOR_RESUME"
                : ((sp->action == PANA_RSP_ACT_FAST_DUMPNOR) ? "FAST_DUMPNOR" : "DUMPNOR"),
            (unsigned long)dump_ff_blocks
        );
        picofw_cli_set_result_extra(dump_extra);
        printf("\nPANARSP: %s (dump_rx=%" PRIu32 " bytes, ff_blocks=%" PRIu32 ", hdr=%s, ftr=%s, fifo_ovf=%s)\n",
               (triggered && dump_ftr) ? "OK" : "PARTIAL",
               dump_bytes,
               dump_ff_blocks,
               dump_hdr ? "YES" : "NO",
               dump_ftr ? "YES" : "NO",
               fifo_ovf ? "YES" : "NO");
        return;
    }
    if (sp->action == PANA_RSP_ACT_JEDEC_ID || sp->action == PANA_RSP_ACT_CFI_QUERY ||
        sp->action == PANA_RSP_ACT_CRC32_NOR || sp->action == PANA_RSP_ACT_BLANK_CHECK) {
        if (triggered && !dump_ftr) {
            picofw_cli_set_result("ERR", "EIO");
        }
        printf("\nPANARSP: %s (rx=%" PRIu32 " bytes, done=%s, fifo_ovf=%s)\n",
               (triggered && dump_ftr) ? "OK" : "PARTIAL",
               dump_bytes,
               dump_ftr ? "YES" : "NO",
               fifo_ovf ? "YES" : "NO");
        return;
    }
    if (sp->action == PANA_RSP_ACT_JEDEC_ERASE || sp->action == PANA_RSP_ACT_JEDEC_PROG ||
        sp->action == PANA_RSP_ACT_BGO_BITCLEAR) {
        if (triggered && !dump_ftr) {
            picofw_cli_set_result("ERR", (jedec_err_code != NULL) ? jedec_err_code : "EJEDEC");
        }
        printf("\nPANARSP: %s (jedec_rx=%" PRIu32 " bytes, ok=%s, timeout=%s, relock=%s, fifo_ovf=%s)\n",
               (triggered && dump_ftr) ? "OK" : "PARTIAL",
               dump_bytes,
               dump_ftr ? "YES" : "NO",
               dump_hdr ? "YES" : "NO",
               relocked_nor ? "YES" : "NO",
               fifo_ovf ? "YES" : "NO");
        return;
    }
    panarsp_print_report(sp, cap, post_tx_off, primary_off, triggered, fifo_ovf);
    if (relocked_nor) {
        printf("[PANARSP] One-shot Upper NOR auto-relocked (NORLOCK).\n");
    }
}
