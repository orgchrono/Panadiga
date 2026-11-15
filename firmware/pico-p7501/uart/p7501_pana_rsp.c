/**
 * @file p7501_pana_rsp.c
 * @brief Active `PANARSP` probe against EH55 MAIN `_dev_SERIAL` RSP stub over PIO1.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_rsp_report.h"
#include "uart/p7501_pana_dumpnor.h"
#include "uart/p7501_uart_pio_engine.h"
#include "common/p7501_hex.h"
#include "hal/p7501_led.h"
#include "p7501_types.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static uint32_t panarsp_tx_frame(const char *payload) {
    char frame[P7501_RSP_MAX_FRAME];
    const size_t flen = p7501_rsp_build_frame(payload, frame, sizeof(frame));
    if (flen > 0u) {
        p7501_pio_tx_bytes((const uint8_t *)frame, flen, false);
    }
    return (uint32_t)flen;
}

static size_t panarsp_tx_sequence(
    panarsp_rx_cap_t *cap,
    const char *const *payloads,
    size_t count,
    uint32_t per_step_ms
) {
    size_t first_off = cap->len;
    for (size_t i = 0u; i < count; ++i) {
        const size_t step_off = cap->len;
        if (i == 0u) {
            first_off = step_off;
        }
        (void)panarsp_tx_frame(payloads[i]);
        panarsp_drain_for_ms(cap, per_step_ms, true, step_off);
    }
    return first_off;
}

static size_t panarsp_execute_dumpnor(
    panarsp_rx_cap_t *cap,
    const p7501_pana_rsp_spec_t *spec,
    bool *out_hdr_seen,
    bool *out_ftr_seen,
    uint32_t *out_stream_bytes
) {
    uint8_t stub[P7501_DUMPNOR_STUB_SIZE];
    const size_t stub_len = p7501_dumpnor_build_stub(
        spec->addr, spec->value_u32, P7501_DUMPNOR_STUB_ADDR, stub, sizeof(stub)
    );
    if (stub_len == 0u) {
        return cap->len;
    }

    char frame[P7501_RSP_MAX_FRAME];
    printf("[PANARSP] Uploading %uB PANADMP1 stub -> 0x%08lX (range 0x%08lX+0x%08lX)...\n",
           (unsigned)stub_len,
           (unsigned long)P7501_DUMPNOR_STUB_ADDR,
           (unsigned long)spec->addr,
           (unsigned long)spec->value_u32);

    for (size_t off = 0u; off < stub_len; off += 16u) {
        const size_t chunk = (stub_len - off > 16u) ? 16u : (stub_len - off);
        const size_t flen = p7501_rsp_build_write_mem(
            P7501_DUMPNOR_STUB_ADDR + (uint32_t)off, &stub[off], chunk, frame, sizeof(frame)
        );
        if (flen == 0u) {
            return cap->len;
        }
        const size_t step_off = cap->len;
        p7501_pio_tx_bytes((const uint8_t *)frame, flen, false);
        panarsp_drain_for_ms(cap, 45u, true, step_off);
    }

    const size_t cb_len = p7501_rsp_build_write_u32(
        P7501_AM34_SERIAL_CB_PTR_ADDR, P7501_DUMPNOR_STUB_ADDR, frame, sizeof(frame)
    );
    const size_t cb_off = cap->len;
    if (cb_len > 0u) {
        p7501_pio_tx_bytes((const uint8_t *)frame, cb_len, false);
        panarsp_drain_for_ms(cap, 80u, true, cb_off);
    }

    const size_t stream_off = cap->len;
    printf("[PANARSP] Triggering $g @ 0x%08lX; streaming raw PANADMP1..PANADONE:\n",
           (unsigned long)P7501_DUMPNOR_STUB_ADDR);
    stdio_flush();
    (void)panarsp_tx_frame("g");

    uint8_t hdr_ring[12];
    uint8_t ftr_ring[12];
    size_t hdr_len = 0u;
    size_t ftr_len = 0u;
    bool hdr_seen = false;
    bool ftr_seen = false;
    uint32_t streamed = 0u;
    const uint64_t deadline = time_us_64() + ((uint64_t)spec->timeout_ms * 1000ULL);
    while (!ftr_seen && time_us_64() < deadline) {
        p7501_pio_poll_sw_uart_release();
        uint8_t b = 0u;
        while (p7501_pio_pop_rx_byte(&b)) {
            putchar((int)b);
            streamed += 1u;
            if (!hdr_seen && p7501_rolling_match_push(hdr_ring, &hdr_len, sizeof(hdr_ring), b, "PANADMP1")) {
                hdr_seen = true;
            }
            if (p7501_rolling_match_push(ftr_ring, &ftr_len, sizeof(ftr_ring), b, "PANADONE")) {
                ftr_seen = true;
                break;
            }
            if ((streamed & 0xFFu) == 0u) {
                stdio_flush();
            }
        }
        tight_loop_contents();
    }
    stdio_flush();
    *out_hdr_seen = hdr_seen;
    *out_ftr_seen = ftr_seen;
    *out_stream_bytes = streamed;
    return stream_off;
}

void p7501_cmd_pana_rsp(const char *arg) {
    static panarsp_rx_cap_t s_cap;
    p7501_pana_rsp_spec_t spec;

    if (!p7501_panarsp_parse_args(arg, &spec)) {
        printf("PANARSP: ERROR (Usage: PANARSP [ms] [NORMAL|JIG[:ms]|NOPULL] [MAGIC] [NONAK] [WAIT:<trig>] [PROBESEQ|G|REGS|Z0|Z1|MEM:<addr>,<len>|XMEM:<addr>,<len>|WMEM:<addr>,<hex>|WRITE32:<addr>,<u32>|PASSTHROUGH|CALL:<addr>|DUMPNOR [start] [len]|NAK|RAW:<txt>|HEX:<hex>])\n");
        return;
    }

    memset(&s_cap, 0, sizeof(s_cap));
    p7501_pio_enforce_nsw5v_hiz();
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    printf("=== PANARSP PIO1 (baud=%" PRIu32 " ms=%" PRIu32 " boot=%d hold=%" PRIu32 " action=%d) ===\n",
           p7501_pio_get_baud(),
           spec.timeout_ms,
           (int)spec.boot_action,
           spec.jig_hold_ms,
           (int)spec.action);

    p7501_pio_begin_rx_session_ex(spec.boot_action, spec.jig_hold_ms);

    bool triggered = !spec.wait_trigger_enabled;
    if (spec.wait_trigger_enabled) {
        uint8_t ring[P7501_RSP_MAX_TRIG + 1u];
        uint8_t jig_ring[24];
        size_t ring_len = 0u;
        size_t jig_ring_len = 0u;
        const uint64_t wait_deadline = time_us_64() + ((uint64_t)spec.timeout_ms * 1000ULL);
        while (!triggered && time_us_64() < wait_deadline) {
            p7501_pio_poll_sw_uart_release();
            uint8_t b = 0u;
            while (p7501_pio_pop_rx_byte(&b)) {
                panarsp_push_rx(&s_cap, b, true);
                if (spec.boot_action == P7501_BOOT_ACTION_JIG) {
                    (void)p7501_pio_poll_jig_serial_release(
                        jig_ring, &jig_ring_len, sizeof(jig_ring), b
                    );
                }
                if (p7501_rolling_match_push(ring, &ring_len, sizeof(ring), b, spec.trigger)) {
                    triggered = true;
                    break;
                }
            }
            tight_loop_contents();
        }
        if (triggered) {
            panarsp_compact_pretrig(&s_cap, PANARSP_PRETRIG_KEEP);
        }
    }

    const size_t post_tx_off = s_cap.len;
    size_t primary_off = post_tx_off;
    bool dump_hdr = false;
    bool dump_ftr = false;
    uint32_t dump_bytes = 0u;
    if (triggered) {
        const bool need_handshake = spec.send_magic_prefix ||
                                    (spec.action == PANA_RSP_ACT_MAGIC) ||
                                    (spec.action == PANA_RSP_ACT_PASSTHROUGH) ||
                                    (spec.action == PANA_RSP_ACT_CALL) ||
                                    (spec.action == PANA_RSP_ACT_DUMPNOR);
        if (need_handshake) {
            const size_t step_off = s_cap.len;
            p7501_pio_tx_bytes(k_p7501_rsp_handshake, P7501_RSP_HANDSHAKE_LEN, false);
            panarsp_drain_for_ms(&s_cap, 30u, false, step_off);
        }
        if (spec.send_nak_prefix && spec.action != PANA_RSP_ACT_MAGIC) {
            static const uint8_t k_nak = P7501_RSP_NAK_BYTE;
            const size_t step_off = s_cap.len;
            p7501_pio_tx_bytes(&k_nak, 1u, false);
            panarsp_drain_for_ms(&s_cap, (spec.action == PANA_RSP_ACT_NAK) ? 350u : 80u, true, step_off);
        }
        primary_off = s_cap.len;
        switch (spec.action) {
            case PANA_RSP_ACT_SEQ: {
                static const char *const k_probe_seq[] = { "g", "Z0" };
                primary_off = panarsp_tx_sequence(&s_cap, k_probe_seq, 2u, 250u);
                break;
            }
            case PANA_RSP_ACT_PASSTHROUGH: {
                static const char *const k_pass_seq[P7501_TBC_PASSTHROUGH_SEQ_LEN] =
                    P7501_TBC_PASSTHROUGH_SEQ_INIT;
                printf("[PANARSP] Applying %u MAIN AV TBC E-E passthrough sites via $M...\n",
                       (unsigned)P7501_TBC_PASSTHROUGH_SEQ_LEN);
                primary_off = panarsp_tx_sequence(
                    &s_cap,
                    k_pass_seq,
                    P7501_TBC_PASSTHROUGH_SEQ_LEN,
                    250u
                );
                break;
            }
            case PANA_RSP_ACT_CALL: {
                const char *call_seq[2] = { spec.payload, "g" };
                printf("[PANARSP] Updating serial callback 0x%08X -> 0x%08X and triggering $g...\n",
                       (unsigned)P7501_AM34_SERIAL_CB_PTR_ADDR,
                       (unsigned)spec.addr);
                primary_off = panarsp_tx_sequence(&s_cap, call_seq, 2u, 250u);
                break;
            }
            case PANA_RSP_ACT_DUMPNOR:
                primary_off = panarsp_execute_dumpnor(
                    &s_cap, &spec, &dump_hdr, &dump_ftr, &dump_bytes
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
                (void)panarsp_tx_frame(spec.payload);
                panarsp_drain_for_ms(&s_cap, (spec.timeout_ms < 700u) ? spec.timeout_ms : 700u, true, primary_off);
                break;
            case PANA_RSP_ACT_HEX:
                p7501_pio_tx_bytes(spec.raw_bytes, spec.raw_len, false);
                panarsp_drain_for_ms(&s_cap, 500u, true, primary_off);
                break;
            case PANA_RSP_ACT_NAK:
            case PANA_RSP_ACT_MAGIC:
            default:
                primary_off = post_tx_off;
                break;
        }
    }

    const bool fifo_ovf = p7501_pio_had_rx_overflow();
    p7501_pio_end_session();
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
    if (spec.action == PANA_RSP_ACT_DUMPNOR) {
        printf("\nPANARSP: %s (dump_rx=%" PRIu32 " bytes, hdr=%s, ftr=%s, fifo_ovf=%s)\n",
               (triggered && dump_ftr) ? "OK" : "PARTIAL",
               dump_bytes,
               dump_hdr ? "YES" : "NO",
               dump_ftr ? "YES" : "NO",
               fifo_ovf ? "YES" : "NO");
        return;
    }
    panarsp_print_report(&spec, &s_cap, post_tx_off, primary_off, triggered, fifo_ovf);
}
