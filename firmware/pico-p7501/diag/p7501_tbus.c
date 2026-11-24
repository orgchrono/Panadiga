/**
 * @file p7501_tbus.c
 * @brief Passive Timer-MCU T-BUS PIO/GPIO sniffer (`TBUSSNIFF`) & keepalive injector (`M0KEEPALIVE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_tbus_codec.h"
#include "cli/p7501_cli.h"
#include "cli/p7501_cli_sys.h"
#include "common/p7501_hex.h"
#include "common/p7501_push_stream.h"
#include "hal/p7501_hw.h"
#include "pico/time.h"
#include "uart/p7501_uart_capture.h"
#include "uart/p7501_uart_pio_engine.h"
#include <inttypes.h>
#include <stdio.h>

static bool s_m0_keepalive_enabled = false;
static uint8_t s_m0_seq = 0u;
static uint32_t s_m0_interval_ms = P7501_M0_KEEPALIVE_DEFAULT_MS;
static uint64_t s_m0_next_due_us = 0ULL;

static size_t send_m0_keepalive_frame(void) {
    uint8_t pkt[8];
    const size_t n = p7501_tbus_build_keepalive(++s_m0_seq, pkt, sizeof(pkt));
    if (n > 0u) {
        /* Transmit directly via TX PIO without touching GP26 SW_UART or isolating active sessions. */
        p7501_pio_tx_bytes_duplex(pkt, n, false, true, NULL, NULL);
    }
    return n;
}

void p7501_cmd_tbussniff(const char *arg) {
    const uint32_t dur_ms = p7501_parse_uint32_clamped(arg, 1000u, 100u, 15000u);
    p7501_hw_enforce_nsw5v_hiz();
    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
    uint8_t *buf = p7501_uart_rx_buffer(NULL);
    const uint32_t got = p7501_uart_capture_window(dur_ms * 1000UL, 2048u);
    p7501_pio_end_session();

    p7501_tbus_audit_t audit;
    p7501_tbus_audit_init(&audit);
    for (uint32_t i = 0u; i + 4u <= got; ++i) {
        p7501_tbus_frame_t frm;
        if (p7501_tbus_decode_frame(&buf[i], got - i, &frm)) {
            p7501_tbus_audit_record_frame(&audit, &frm);
            printf("[TBUSSNIFF] frame off=%lu op=0x%02X len=%u csum=0x%02X\n",
                   (unsigned long)i, frm.opcode, frm.len, frm.checksum);
        }
    }
    char audit_line[160];
    (void)p7501_tbus_audit_format_summary(&audit, audit_line, sizeof(audit_line));
    printf("%s\n", audit_line);
    printf("TBUSSNIFF: OK (transport=P7501_SC0_PASSIVE bytes=%" PRIu32 ", frames=%" PRIu32 ")\n",
           got, audit.frames_valid);
}

void p7501_tbus_keepalive_stop(void) {
    s_m0_keepalive_enabled = false;
}

void p7501_cmd_m0keepalive(const char *arg) {
    p7501_m0_keepalive_action_t action = P7501_M0_KA_ON;
    uint32_t req_ms = P7501_M0_KEEPALIVE_DEFAULT_MS;
    if (!p7501_tbus_parse_keepalive_args(arg, &action, &req_ms)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("M0KEEPALIVE: ERROR (Usage: M0KEEPALIVE [ON [1000..30000ms]|OFF|STATUS])\n");
        return;
    }

    if (action == P7501_M0_KA_OFF) {
        s_m0_keepalive_enabled = false;
        printf("M0KEEPALIVE: OK (state=OFF)\n");
        return;
    }
    if (action == P7501_M0_KA_STATUS) {
        printf("M0KEEPALIVE: OK (state=%s interval_ms=%" PRIu32 " seq=%u)\n",
               s_m0_keepalive_enabled ? "PERIODIC_SC0" : "OFF",
               s_m0_interval_ms,
               (unsigned)s_m0_seq);
        return;
    }
    if (!p7501_probe_dispatch_event(P7501_EV_ENGAGE, true)) {
        s_m0_keepalive_enabled = false;
        p7501_cli_set_result("ERR", "EOFFLINE");
        printf("M0KEEPALIVE: ERROR code=EOFFLINE reason=TARGET_UNPOWERED\n");
        return;
    }
    s_m0_interval_ms = req_ms;
    s_m0_keepalive_enabled = true;
    const size_t n = send_m0_keepalive_frame();
    s_m0_next_due_us = time_us_64() + ((uint64_t)s_m0_interval_ms * 1000ULL);
    printf("M0KEEPALIVE: OK (mode=PERIODIC_SC0 interval_ms=%" PRIu32 " seq=%u len=%u)\n",
           s_m0_interval_ms, (unsigned)s_m0_seq, (unsigned)n);
}

void p7501_tbus_keepalive_poll(void) {
    if (!s_m0_keepalive_enabled) {
        return;
    }
    const p7501_ps_t ps = p7501_probe_get_state();
    if ((ps != P7501_PS_ENGAGED) && (ps != P7501_PS_ACTIVE_OP)) {
        s_m0_keepalive_enabled = false;
        return;
    }
    const uint64_t now_us = time_us_64();
    if (now_us < s_m0_next_due_us) {
        return;
    }
    /* Passive NSW5V sense avoids pulling GP26 (SW_UART) LOW during active sessions. */
    if (!p7501_hw_is_nsw5v_high() || p7501_hw_is_nsw5v_fault_latched()) {
        s_m0_keepalive_enabled = false;
        (void)p7501_push_emit_str("M0KEEPALIVE", "state=AUTO_OFF reason=TARGET_UNPOWERED");
        return;
    }
    (void)send_m0_keepalive_frame();
    s_m0_next_due_us = now_us + ((uint64_t)s_m0_interval_ms * 1000ULL);
}
