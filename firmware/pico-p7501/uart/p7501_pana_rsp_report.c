/**
 * @file p7501_pana_rsp_report.c
 * @brief RX buffer compaction, polling drain, hexdump, and MN103S register report for `PANARSP`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_rsp_report.h"
#include "uart/p7501_uart_pio_engine.h"
#include "cli/p7501_cli.h"
#include "common/p7501_hex.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

void panarsp_compact_pretrig(panarsp_rx_cap_t *cap, size_t keep) {
    if (cap->len <= keep) {
        return;
    }
    const size_t drop = cap->len - keep;
    memmove(cap->buf, &cap->buf[drop], keep);
    cap->len = keep;
    cap->pretrig_dropped += drop;
}

void panarsp_push_rx(panarsp_rx_cap_t *cap, uint8_t b, bool compact_pretrig) {
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

bool panarsp_drain_for_ms(
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
        p7501_wdt_kick();
        tight_loop_contents();
    }
    return false;
}

static void print_rx_hexdump(const panarsp_rx_cap_t *cap) {
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
}

void panarsp_print_report(
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
    print_rx_hexdump(cap);

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

        uint32_t regs[12];
        if (info.checksum_ok &&
            ((spec->action == PANA_RSP_ACT_REGS) || (spec->action == PANA_RSP_ACT_G) ||
             (spec->action == PANA_RSP_ACT_SEQ) || (spec->action == PANA_RSP_ACT_CALL)) &&
            p7501_rsp_decode_mn103_regs(&cap->buf[info.payload_offset], info.payload_len, regs)) {
            printf("[PANARSP] MN103S Regs: D0=0x%08lX D1=0x%08lX D2=0x%08lX D3=0x%08lX"
                   " | A0=0x%08lX A1=0x%08lX A2=0x%08lX A3=0x%08lX"
                   " | MDR=0x%08lX PSW=0x%08lX PC=0x%08lX SP=0x%08lX\n",
                   (unsigned long)regs[0], (unsigned long)regs[1],
                   (unsigned long)regs[2], (unsigned long)regs[3],
                   (unsigned long)regs[4], (unsigned long)regs[5],
                   (unsigned long)regs[6], (unsigned long)regs[7],
                   (unsigned long)regs[8], (unsigned long)regs[9],
                   (unsigned long)regs[10], (unsigned long)regs[11]);
        }
    } else {
        printf("[PANARSP] Frame: none (ack=%s nak=%s nak0x15=%s)\n",
               info.ack_plus ? "+" : "no",
               info.nak_minus ? "-" : "no",
               info.raw_nak_0x15 ? "yes" : "no");
    }

    if (spec->wait_trigger_enabled && !triggered) {
        p7501_cli_set_result("ERR", "ETIMEOUT");
        printf("PANARSP: ERROR (triggered=NO, rx=%u bytes, rsp=%s, fifo_ovf=%s)\n",
               (unsigned)cap->len,
               has_frame ? (info.checksum_ok ? "VALID" : "BAD_CSUM") : "NO_REPLY",
               fifo_ovf ? "YES" : "NO");
        return;
    }
    bool rsp_err = false;
    if (has_frame && !info.checksum_ok) {
        p7501_cli_set_result("ERR", "ECSUM");
        rsp_err = true;
    } else if (!has_frame && (cap->len == 0u)) {
        p7501_cli_set_result("ERR", "ENOREPLY");
        rsp_err = true;
    }
    printf("PANARSP: %s (rx=%u bytes, rsp=%s, fifo_ovf=%s)\n",
           rsp_err ? "ERROR" : "OK",
           (unsigned)cap->len,
           has_frame ? (info.checksum_ok ? "VALID" : "BAD_CSUM") : "NO_REPLY",
           fifo_ovf ? "YES" : "NO");
}

static char s_rsp_tx_frame[P7501_RSP_MAX_FRAME];

uint32_t panarsp_tx_frame(const char *payload) {
    const size_t flen = p7501_rsp_build_frame(payload, s_rsp_tx_frame, sizeof(s_rsp_tx_frame));
    if (flen > 0u) {
        p7501_pio_tx_bytes((const uint8_t *)s_rsp_tx_frame, flen, false);
    }
    return (uint32_t)flen;
}

static bool panarsp_upload_stub_and_go(
    panarsp_rx_cap_t *cap,
    uint32_t stub_addr,
    const uint8_t *stub,
    size_t stub_len
) {
    for (size_t off = 0u; off < stub_len; off += 16u) {
        const size_t chunk = (stub_len - off > 16u) ? 16u : (stub_len - off);
        const size_t flen = p7501_rsp_build_write_mem(
            stub_addr + (uint32_t)off, &stub[off], chunk, s_rsp_tx_frame, sizeof(s_rsp_tx_frame)
        );
        if (flen == 0u) {
            return false;
        }
        const size_t step_off = cap->len;
        p7501_pio_tx_bytes((const uint8_t *)s_rsp_tx_frame, flen, false);
        panarsp_drain_for_ms(cap, 45u, true, step_off);
    }
    const size_t cb_len = p7501_rsp_build_write_u32(
        P7501_AM34_SERIAL_CB_PTR_ADDR, stub_addr, s_rsp_tx_frame, sizeof(s_rsp_tx_frame)
    );
    if (cb_len == 0u) {
        return false;
    }
    const size_t cb_off = cap->len;
    p7501_pio_tx_bytes((const uint8_t *)s_rsp_tx_frame, cb_len, false);
    panarsp_drain_for_ms(cap, 80u, true, cb_off);
    stdio_flush();
    (void)panarsp_tx_frame("g");
    return true;
}

size_t panarsp_stream_until_tokens_ex(
    panarsp_rx_cap_t *cap,
    uint32_t stub_addr,
    const uint8_t *stub,
    size_t stub_len,
    uint32_t timeout_ms,
    const char *tok_hdr,
    const char *tok_done,
    const char *tok_fail,
    uint32_t min_post_hdr_bytes,
    bool sliding_timeout,
    bool *out_hdr_seen,
    bool *out_done_seen,
    bool *out_fail_seen,
    uint32_t *out_stream_bytes,
    uint32_t *out_post_hdr_bytes
) {
    if (!panarsp_upload_stub_and_go(cap, stub_addr, stub, stub_len)) {
        if (out_hdr_seen) *out_hdr_seen = false;
        if (out_done_seen) *out_done_seen = false;
        if (out_fail_seen) *out_fail_seen = false;
        if (out_stream_bytes) *out_stream_bytes = 0u;
        if (out_post_hdr_bytes) *out_post_hdr_bytes = 0u;
        return cap->len;
    }
    const size_t stream_off = cap->len;
    uint8_t r1[16], r2[16], r3[16];
    size_t l1 = 0u, l2 = 0u, l3 = 0u;
    bool s1 = (tok_hdr == NULL), s2 = false, s3 = false;
    uint32_t rx = 0u;
    uint32_t post_hdr = 0u;
    uint64_t activity_t0 = time_us_64();
    const uint32_t idle_ms = (timeout_ms < 5000u) ? timeout_ms : 5000u;
    uint64_t deadline = activity_t0 + ((uint64_t)timeout_ms * 1000ULL);
    while (!s2 && !s3 && p7501_wdt_loop_step(activity_t0, deadline, true, NULL)) {
        p7501_pio_poll_sw_uart_release();
        uint8_t b = 0u;
        bool got_any = false;
        while (p7501_pio_pop_rx_byte(&b)) {
            got_any = true;
            /* Emit raw binary byte without stdio CR/LF translation (critical for PANADMP1). */
            putchar_raw((int)b);
            if ((tok_hdr != NULL) && (cap->len < PANARSP_RX_RAM_CAP)) {
                cap->buf[cap->len++] = b;
            }
            rx += 1u;
            if (s1) {
                post_hdr += 1u;
            } else if (p7501_rolling_match_push(r1, &l1, sizeof(r1), b, tok_hdr)) {
                s1 = true;
            }
            if (tok_done && p7501_rolling_match_push(r2, &l2, sizeof(r2), b, tok_done)) {
                if (s1 && (post_hdr >= min_post_hdr_bytes)) {
                    s2 = true;
                    break;
                }
            }
            if (tok_fail && p7501_rolling_match_push(r3, &l3, sizeof(r3), b, tok_fail)) {
                s3 = true;
                break;
            }
            if ((rx & 0xFFu) == 0u) stdio_flush();
        }
        if (got_any && sliding_timeout) {
            activity_t0 = time_us_64();
            deadline = activity_t0 + ((uint64_t)(s1 ? idle_ms : timeout_ms) * 1000ULL);
        }
        tight_loop_contents();
    }
    stdio_flush();
    if (out_hdr_seen) *out_hdr_seen = s1;
    if (out_done_seen) *out_done_seen = s2;
    if (out_fail_seen) *out_fail_seen = s3;
    if (out_stream_bytes) *out_stream_bytes = rx;
    if (out_post_hdr_bytes) *out_post_hdr_bytes = post_hdr;
    return stream_off;
}

size_t panarsp_stream_until_tokens(
    panarsp_rx_cap_t *cap,
    uint32_t stub_addr,
    const uint8_t *stub,
    size_t stub_len,
    uint32_t timeout_ms,
    const char *tok_hdr,
    const char *tok_done,
    const char *tok_fail,
    bool *out_hdr_seen,
    bool *out_done_seen,
    bool *out_fail_seen,
    uint32_t *out_stream_bytes
) {
    return panarsp_stream_until_tokens_ex(
        cap, stub_addr, stub, stub_len, timeout_ms,
        tok_hdr, tok_done, tok_fail, 0u, false,
        out_hdr_seen, out_done_seen, out_fail_seen, out_stream_bytes, NULL
    );
}
