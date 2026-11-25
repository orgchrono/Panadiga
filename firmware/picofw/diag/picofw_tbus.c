/**
 * @file picofw_tbus.c
 * @brief Passive Timer-MCU T-BUS PIO/GPIO sniffer (`TBUSSNIFF`) & keepalive injector (`M0KEEPALIVE`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_tbus_codec.h"
#include "cli/picofw_cli.h"
#include "cli/picofw_cli_sys.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "hal/picofw_hw.h"
#include "pico/time.h"
#include "uart/picofw_uart_capture.h"
#include "uart/picofw_uart_pio_engine.h"
#include <inttypes.h>
#include <stdio.h>

static bool s_m0_keepalive_enabled = false;
static uint8_t s_m0_seq = 0u;
static uint32_t s_m0_interval_ms = PICOFW_M0_KEEPALIVE_DEFAULT_MS;
static uint64_t s_m0_next_due_us = 0ULL;

static size_t send_m0_keepalive_frame(void) {
    uint8_t pkt[8];
    const size_t n = picofw_tbus_build_keepalive(++s_m0_seq, pkt, sizeof(pkt));
    if (n > 0u) {
        /* Transmit directly via TX PIO without touching GP26 SW_UART or isolating active sessions. */
        picofw_pio_tx_bytes_duplex(pkt, n, false, true, NULL, NULL);
    }
    return n;
}

void picofw_cmd_tbussniff(const char *arg) {
    const uint32_t dur_ms = picofw_parse_uint32_clamped(arg, 1000u, 100u, 15000u);
    picofw_hw_enforce_nsw5v_hiz();
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    uint8_t *buf = picofw_uart_rx_buffer(NULL);
    const uint32_t got = picofw_uart_capture_window(dur_ms * 1000UL, 2048u);
    picofw_pio_end_session();

    picofw_tbus_audit_t audit;
    picofw_tbus_audit_init(&audit);
    for (uint32_t i = 0u; i + 4u <= got; ++i) {
        picofw_tbus_frame_t frm;
        if (picofw_tbus_decode_frame(&buf[i], got - i, &frm)) {
            picofw_tbus_audit_record_frame(&audit, &frm);
            printf("[TBUSSNIFF] frame off=%lu op=0x%02X len=%u csum=0x%02X\n",
                   (unsigned long)i, frm.opcode, frm.len, frm.checksum);
        }
    }
    char audit_line[160];
    (void)picofw_tbus_audit_format_summary(&audit, audit_line, sizeof(audit_line));
    printf("%s\n", audit_line);
    printf("TBUSSNIFF: OK (transport=PICOFW_SC0_PASSIVE bytes=%" PRIu32 ", frames=%" PRIu32 ")\n",
           got, audit.frames_valid);
}

void picofw_tbus_keepalive_stop(void) {
    s_m0_keepalive_enabled = false;
}

void picofw_cmd_m0keepalive(const char *arg) {
    picofw_m0_keepalive_action_t action = PICOFW_M0_KA_ON;
    uint32_t req_ms = PICOFW_M0_KEEPALIVE_DEFAULT_MS;
    if (!picofw_tbus_parse_keepalive_args(arg, &action, &req_ms)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("M0KEEPALIVE: ERROR (Usage: M0KEEPALIVE [ON [1000..30000ms]|OFF|STATUS])\n");
        return;
    }

    if (action == PICOFW_M0_KA_OFF) {
        s_m0_keepalive_enabled = false;
        printf("M0KEEPALIVE: OK (state=OFF)\n");
        return;
    }
    if (action == PICOFW_M0_KA_STATUS) {
        printf("M0KEEPALIVE: OK (state=%s interval_ms=%" PRIu32 " seq=%u)\n",
               s_m0_keepalive_enabled ? "PERIODIC_SC0" : "OFF",
               s_m0_interval_ms,
               (unsigned)s_m0_seq);
        return;
    }
    if (!picofw_probe_dispatch_event(PICOFW_EV_ENGAGE, true)) {
        s_m0_keepalive_enabled = false;
        picofw_cli_set_result("ERR", "EOFFLINE");
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

void picofw_tbus_keepalive_poll(void) {
    if (!s_m0_keepalive_enabled) {
        return;
    }
    const picofw_ps_t ps = picofw_probe_get_state();
    if ((ps != PICOFW_PS_ENGAGED) && (ps != PICOFW_PS_ACTIVE_OP)) {
        s_m0_keepalive_enabled = false;
        return;
    }
    const uint64_t now_us = time_us_64();
    if (now_us < s_m0_next_due_us) {
        return;
    }
    /* Passive NSW5V sense avoids pulling GP26 (SW_UART) LOW during active sessions. */
    if (!picofw_hw_is_nsw5v_high() || picofw_hw_is_nsw5v_fault_latched()) {
        s_m0_keepalive_enabled = false;
        (void)picofw_push_emit_str("M0KEEPALIVE", "state=AUTO_OFF reason=TARGET_UNPOWERED");
        return;
    }
    (void)send_m0_keepalive_frame();
    s_m0_next_due_us = now_us + ((uint64_t)s_m0_interval_ms * 1000ULL);
}
