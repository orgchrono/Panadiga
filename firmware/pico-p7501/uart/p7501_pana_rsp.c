/**
 * @file p7501_pana_rsp.c
 * @brief Active `PANARSP` probe against EH55 MAIN `_dev_SERIAL` RSP stub over PIO1.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_uart_pio_engine.h"
#include "common/p7501_hex.h"
#include "hal/p7501_led.h"
#include "p7501_types.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#define PANARSP_RX_RAM_CAP 4096u
#define PANARSP_PRETRIG_KEEP 512u

typedef struct panarsp_rx_cap {
    uint8_t buf[PANARSP_RX_RAM_CAP];
    size_t len;
    size_t pretrig_dropped;
    bool overflow;
} panarsp_rx_cap_t;

static void panarsp_compact_pretrig(panarsp_rx_cap_t *cap, size_t keep) {
    if (cap->len <= keep) {
        return;
    }
    const size_t drop = cap->len - keep;
    memmove(cap->buf, &cap->buf[drop], keep);
    cap->len = keep;
    cap->pretrig_dropped += drop;
}

static void panarsp_push_rx(panarsp_rx_cap_t *cap, uint8_t b, bool compact_pretrig) {
    if (cap->len >= PANARSP_RX_RAM_CAP) {
        if (compact_pretrig) {
            panarsp_compact_pretrig(cap, PANARSP_PRETRIG_KEEP);
        } else {
            cap->overflow = true;
            return;
        }
    }
    cap->buf[cap->len++] = b;
}

static bool panarsp_drain_for_ms(
    panarsp_rx_cap_t *cap,
    uint32_t wait_ms,
    bool stop_on_rsp_frame,
    size_t inspect_from
) {
    const uint64_t deadline_us = time_us_64() + ((uint64_t)wait_ms * 1000ULL);
    uint64_t frame_grace_us = 0ULL;

    while (time_us_64() < deadline_us) {
        p7501_pio_poll_sw_uart_release();
        uint8_t b = 0u;
        bool pushed = false;
        while (p7501_pio_pop_rx_byte(&b)) {
            panarsp_push_rx(cap, b, false);
            pushed = true;
        }
        if (stop_on_rsp_frame && pushed && frame_grace_us == 0ULL) {
            p7501_rsp_frame_info_t info;
            if (p7501_rsp_inspect_frame_from(cap->buf, cap->len, inspect_from, &info) && info.checksum_ok) {
                frame_grace_us = time_us_64() + 40000ULL;
            }
        }
        if (frame_grace_us != 0ULL && time_us_64() >= frame_grace_us) {
            return true;
        }
        tight_loop_contents();
    }
    return false;
}

static uint32_t panarsp_tx_frame(const char *payload) {
    char frame[P7501_RSP_MAX_FRAME];
    const size_t flen = p7501_rsp_build_frame(payload, frame, sizeof(frame));
    if (flen > 0u) {
        p7501_pio_tx_bytes((const uint8_t *)frame, flen, false);
    }
    return (uint32_t)flen;
}

static void panarsp_print_report(
    const p7501_pana_rsp_spec_t *spec,
    const panarsp_rx_cap_t *cap,
    size_t post_tx_off,
    size_t primary_off,
    bool triggered,
    bool fifo_ovf
) {
    p7501_rsp_frame_info_t info;
    bool has_frame = p7501_rsp_inspect_frame_from(cap->buf, cap->len, primary_off, &info);
    if (!has_frame && post_tx_off < primary_off) {
        has_frame = p7501_rsp_inspect_frame_from(cap->buf, cap->len, post_tx_off, &info);
    }

    printf("[PANARSP] RX (%u bytes, pretrig_dropped=%u, inspect_from=%u%s):\n",
           (unsigned)cap->len,
           (unsigned)cap->pretrig_dropped,
           (unsigned)post_tx_off,
           cap->overflow ? ", TRUNCATED" : "");
    for (size_t off = 0u; off < cap->len; off += 16u) {
        const size_t chunk = (cap->len - off > 16u) ? 16u : (cap->len - off);
        printf("  %04X: ", (unsigned)off);
        for (size_t i = 0u; i < 16u; ++i) {
            if (i < chunk) {
                printf("%02X ", cap->buf[off + i]);
            } else {
                printf("   ");
            }
        }
        printf(" |");
        for (size_t i = 0u; i < chunk; ++i) {
            const uint8_t ch = cap->buf[off + i];
            putchar((ch >= 0x20u && ch <= 0x7Eu) ? (int)ch : '.');
        }
        printf("|\n");
    }

    if (has_frame) {
        bool has_non_printable = false;
        for (size_t i = 0u; i < info.payload_len; ++i) {
            const uint8_t ch = cap->buf[info.payload_offset + i];
            if (ch < 0x20u || ch > 0x7Eu) {
                has_non_printable = true;
                break;
            }
        }
        char ptext[96];
        const size_t copy_len = (info.payload_len < (sizeof(ptext) - 1u))
                                    ? info.payload_len
                                    : (sizeof(ptext) - 1u);
        for (size_t i = 0u; i < copy_len; ++i) {
            const uint8_t ch = cap->buf[info.payload_offset + i];
            ptext[i] = (ch >= 0x20u && ch <= 0x7Eu) ? (char)ch : '.';
        }
        ptext[copy_len] = '\0';
        printf("[PANARSP] Frame: ack=%s nak=%s nak0x15=%s | payload=\"%s\"%s | csum=0x%02X/0x%02X (%s)\n",
               info.ack_plus ? "+" : "no",
               info.nak_minus ? "-" : "no",
               info.raw_nak_0x15 ? "yes" : "no",
               ptext,
               (info.payload_len > copy_len) ? "..." : "",
               info.actual_csum,
               info.expected_csum,
               info.checksum_ok ? "VALID" : "MISMATCH");
        const bool print_hex_bytes = has_non_printable || (spec->action == PANA_RSP_ACT_XMEM);
        printf("[PANARSP] PayloadHex: ");
        for (size_t i = 0u; i < info.payload_len; ++i) {
            const uint8_t b = cap->buf[info.payload_offset + i];
            if (print_hex_bytes) {
                printf("%02X", b);
            } else {
                putchar((int)b);
            }
        }
        putchar('\n');
    } else {
        printf("[PANARSP] Frame: none (ack=%s nak=%s nak0x15=%s)\n",
               info.ack_plus ? "+" : "no",
               info.nak_minus ? "-" : "no",
               info.raw_nak_0x15 ? "yes" : "no");
    }

    if (spec->wait_trigger_enabled && !triggered) {
        printf("PANARSP: OK (triggered=NO, rx=%u bytes, rsp=%s, fifo_ovf=%s)\n",
               (unsigned)cap->len,
               has_frame ? (info.checksum_ok ? "VALID" : "BAD_CSUM") : "NO_REPLY",
               fifo_ovf ? "YES" : "NO");
        return;
    }
    printf("PANARSP: OK (rx=%u bytes, rsp=%s, fifo_ovf=%s)\n",
           (unsigned)cap->len,
           has_frame ? (info.checksum_ok ? "VALID" : "BAD_CSUM") : "NO_REPLY",
           fifo_ovf ? "YES" : "NO");
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

void p7501_cmd_pana_rsp(const char *arg) {
    static panarsp_rx_cap_t s_cap;
    p7501_pana_rsp_spec_t spec;

    if (!p7501_panarsp_parse_args(arg, &spec)) {
        printf("PANARSP: ERROR (Usage: PANARSP [ms] [NORMAL|JIG[:ms]|NOPULL] [MAGIC] [NONAK] [WAIT:<trig>] [PROBESEQ|G|Z0|Z1|MEM:<addr>,<len>|XMEM:<addr>,<len>|WMEM:<addr>,<hex>|WRITE32:<addr>,<u32>|PASSTHROUGH|CALL:<addr>|NAK|RAW:<txt>|HEX:<hex>])\n");
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
        size_t ring_len = 0u;
        const uint64_t wait_deadline = time_us_64() + ((uint64_t)spec.timeout_ms * 1000ULL);
        while (!triggered && time_us_64() < wait_deadline) {
            p7501_pio_poll_sw_uart_release();
            uint8_t b = 0u;
            while (p7501_pio_pop_rx_byte(&b)) {
                panarsp_push_rx(&s_cap, b, true);
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
    if (triggered) {
        const bool need_handshake = spec.send_magic_prefix ||
                                    (spec.action == PANA_RSP_ACT_MAGIC) ||
                                    (spec.action == PANA_RSP_ACT_PASSTHROUGH) ||
                                    (spec.action == PANA_RSP_ACT_CALL);
        if (need_handshake) {
            const size_t step_off = s_cap.len;
            p7501_pio_tx_bytes(k_p7501_rsp_handshake, P7501_RSP_HANDSHAKE_LEN, false);
            panarsp_drain_for_ms(&s_cap, 30u, false, step_off);
        }
        if (spec.send_nak_prefix && spec.action != PANA_RSP_ACT_MAGIC) {
            static const uint8_t k_nak = 0x15u;
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
                static const char *const k_pass_seq[] = {
                    "M5771ABFB,6:8001f0fccbcb",
                    "M574F7B8C,4:ccd000cb",
                    "M574F7C5C,7:8000cbcbcbcbcb",
                };
                printf("[PANARSP] Applying 3 MAIN AV TBC E-E passthrough sites via $M...\n");
                primary_off = panarsp_tx_sequence(&s_cap, k_pass_seq, 3u, 250u);
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
            case PANA_RSP_ACT_G:
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
    panarsp_print_report(&spec, &s_cap, post_tx_off, primary_off, triggered, fifo_ovf);
}
