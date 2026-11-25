/**
 * @file test_uart_codec.c
 * @brief Host C11 assertions for baud math, pulse snapping, hex decode, and WAITTX triggers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_uart_codec.h"
#include "tests/test_pana_logic.h"
#include "diag/picofw_jigsweep_codec.h"
#include "diag/picofw_piologic_codec.h"
#include "flash/picofw_bus_policy.h"
#include "uart/picofw_fmu_codec.h"
#include "uart/picofw_uart_autobaud.h"
#include "uart/picofw_uart_codec.h"
#include "common/picofw_hex.h"
#include "common/picofw_wdt_logic.h"
#include "generated/picofw_ssot.h"
#include "hal/picofw_led_logic.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

static void test_timeout_and_tokens(void) {
    char tok[32];
    const char *cur = "  \"Hit Any Key\"  0D0A 4500 ";

    assert(picofw_parse_uint32_clamped(NULL, 5000u, 100u, 30000u) == 5000u);
    assert(picofw_parse_uint32_clamped("50", 5000u, 100u, 30000u) == 100u);
    assert(picofw_parse_uint32_clamped("999999", 5000u, 100u, 30000u) == 30000u);
    assert(picofw_parse_uint32_clamped("  7500 ", 5000u, 100u, 30000u) == 7500u);

    uint32_t strict_val = 99u;
    assert(picofw_parse_u32_strict(" 42 ", &strict_val) && strict_val == 42u);
    assert(!picofw_parse_u32_strict(NULL, &strict_val));
    assert(!picofw_parse_u32_strict("", &strict_val));
    assert(!picofw_parse_u32_strict("12abc", &strict_val));
    assert(!picofw_parse_u32_strict("4294967296", &strict_val));

    cur = picofw_next_token(cur, tok, sizeof(tok));
    assert(cur != NULL);
    assert(strcmp(tok, "Hit Any Key") == 0);
    cur = picofw_next_token(cur, tok, sizeof(tok));
    assert(cur != NULL);
    assert(strcmp(tok, "0D0A") == 0);
    cur = picofw_next_token(cur, tok, sizeof(tok));
    assert(cur != NULL);
    assert(strcmp(tok, "4500") == 0);
    assert(picofw_next_token(cur, tok, sizeof(tok)) == NULL);
}

static void test_hex_and_baud_math(void) {
    uint8_t buf[16];
    size_t len = 0u;
    uint16_t div_i = 0u;
    uint8_t div_f = 0u;
    uint32_t raw_bps = 0u;

    assert(picofw_decode_hex_bytes("1B 5B:48-0D0A", buf, sizeof(buf), &len));
    assert(len == 5u);
    assert(buf[0] == 0x1Bu && buf[3] == 0x0Du && buf[4] == 0x0Au);
    assert(!picofw_decode_hex_bytes("0D0", buf, sizeof(buf), &len));
    assert(!picofw_decode_hex_bytes("ZZ", buf, sizeof(buf), &len));

    assert(picofw_baud_is_valid(115200u));
    assert(!picofw_baud_is_valid(300u));
    assert(picofw_baud_to_bit_ns(115200u) == 8680u);

    picofw_calc_pio_clkdiv_fp16(125000000u, 115200u, &div_i, &div_f);
    assert(div_i == 135u && div_f == 162u);

    picofw_calc_pio_clkdiv_fp16(150000000u, 115200u, &div_i, &div_f);
    assert(div_i == 162u && div_f == 195u);

    assert(picofw_snap_standard_baud(9u, &raw_bps) == 115200u);
    assert(raw_bps == 111111u);
    assert(picofw_snap_standard_baud(17u, &raw_bps) == 57600u);
    assert(picofw_snap_standard_baud(32u, &raw_bps) == 31250u);
    assert(picofw_snap_standard_baud(104u, &raw_bps) == 9600u);

    /* Dual-polarity glitch-immune autobaud cluster verification */
    picofw_baud_hist_t hist;
    picofw_baud_estimate_t est;
    picofw_baud_hist_init(&hist);
    picofw_baud_hist_push_pulse(&hist, 1u, false); /* 1us power-on glitch ignored */
    picofw_baud_hist_push_pulse(&hist, 4u, false); /* isolated 230400 spike (1 hit) ignored when 115200 cluster dominates */
    for (int i = 0; i < 6; ++i) {
        picofw_baud_hist_push_pulse(&hist, 9u, (i % 2) == 1);
    }
    picofw_baud_hist_push_pulse(&hist, 17u, false); /* 2-bit run at 115200 */
    picofw_baud_hist_finish(&hist, &est);
    assert(hist.glitches_rejected == 1u);
    assert(est.snapped_baud == 115200u);
    assert(est.best_cluster_hits >= 6u);
    assert(est.confidence_pct >= 50u);

    /* BAUDSCAN 3-glitch 2T harmonic immunity: 3 fast 4us (230400) glitches must NOT hijack dominant 115200 (40 hits) */
    picofw_baud_hist_init(&hist);
    for (int i = 0; i < 40; ++i) {
        picofw_baud_hist_push_pulse(&hist, 9u, (i % 2) == 0);
    }
    for (int g = 0; g < 3; ++g) {
        picofw_baud_hist_push_pulse(&hist, 4u, false);
    }
    picofw_baud_hist_finish(&hist, &est);
    assert(est.snapped_baud == 115200u);
    assert(est.raw_bps == 111111u);
    assert(est.best_cluster_hits == 40u);

    /* Genuine 1T promotion when 57600 (2T) has 16 hits and 115200 (1T) has 4 hits (>= 3 and >= 16/8=2) */
    picofw_baud_hist_init(&hist);
    for (int i = 0; i < 16; ++i) {
        picofw_baud_hist_push_pulse(&hist, 17u, (i % 2) == 0);
    }
    for (int i = 0; i < 4; ++i) {
        picofw_baud_hist_push_pulse(&hist, 9u, (i % 2) == 0);
    }
    picofw_baud_hist_finish(&hist, &est);
    assert(est.snapped_baud == 115200u);
}

static void test_trigger_and_waittx_parser(void) {
    uint8_t ring[16];
    size_t ring_len = 0u;
    const char *feed = "Loader ver0.61 -- Hit Any Key";
    bool hit = false;
    picofw_waittx_spec_t spec;

    for (size_t i = 0u; feed[i] != '\0'; ++i) {
        if (picofw_rolling_match_push(ring, &ring_len, sizeof(ring), (uint8_t)feed[i], "Hit Any Key")) {
            hit = true;
        }
    }
    assert(hit);

    assert(picofw_parse_waittx_args("20000 JIG \"Hit Any Key\" 1B0D 6000", &spec));
    assert(spec.timeout_ms == 20000u);
    assert(spec.boot_action == PICOFW_BOOT_ACTION_JIG && spec.jig_hold_ms == PICOFW_SSOT_JIG_DEFAULT_HOLD_MS);
    assert(strcmp(spec.trigger, "Hit Any Key") == 0);
    assert(spec.tx_len == 2u && spec.tx_bytes[0] == 0x1Bu && spec.tx_bytes[1] == 0x0Du);
    assert(spec.post_rx_ms == 6000u);
    assert(!spec.tx_break);

    assert(picofw_parse_waittx_args("5000 NORMAL wait BRK", &spec));
    assert(spec.boot_action == PICOFW_BOOT_ACTION_NORMAL);
    assert(spec.tx_break);
    assert(spec.post_rx_ms == 4000u);

    assert(!picofw_parse_waittx_args("5000 JIG trigger", &spec));

    assert(picofw_parse_waittx_args("8000 JIG:350 \"wait 1\" 0D 4000", &spec));
    assert(spec.boot_action == PICOFW_BOOT_ACTION_JIG);
    assert(spec.jig_hold_ms == 350u);
    assert(strcmp(spec.trigger, "wait 1") == 0);
    assert(spec.tx_len == 1u && spec.tx_bytes[0] == 0x0Du);

    assert(picofw_parse_waittx_args("45000 NORMAL \"FVU)...\" 46 45000", &spec));
    assert(spec.timeout_ms == 45000u);
    assert(spec.boot_action == PICOFW_BOOT_ACTION_NORMAL);
    assert(strcmp(spec.trigger, "FVU)...") == 0);
    assert(spec.tx_len == 1u && spec.tx_bytes[0] == 0x46u);
    assert(spec.post_rx_ms == 45000u);

    assert(picofw_parse_waittx_args("45000 NORMAL \"FVU)...\" 46 95000", &spec));
    assert(spec.post_rx_ms == 60000u);
}

static void test_hex_window_and_led_logic(void) {
    uint8_t comma_buf[4] = { 0 };
    size_t comma_len = 0u;
    assert(picofw_decode_hex_bytes("1B,0D,0A", comma_buf, sizeof(comma_buf), &comma_len));
    assert(comma_len == 3u && comma_buf[0] == 0x1Bu && comma_buf[1] == 0x0Du && comma_buf[2] == 0x0Au);
    assert(picofw_ascii_ieq("ConFirm", "CONFIRM"));

    char upper[16];
    picofw_ascii_upper_copy(upper, sizeof(upper), "jig:350");
    assert(strcmp(upper, "JIG:350") == 0);

    picofw_window_boot_spec_t win;
    picofw_parse_window_boot_args("8000 JIG:350", 5000u, 200u, 30000u, &win);
    assert(win.window_ms == 8000u && win.boot_action == PICOFW_BOOT_ACTION_JIG && win.jig_hold_ms == 350u && !win.sticky_jig && !win.no_xreset && !win.wait_for_wake);
    picofw_parse_window_boot_args("45000 JIG:3000 STICKY", 5000u, 200u, 90000u, &win);
    assert(win.window_ms == 45000u && win.boot_action == PICOFW_BOOT_ACTION_JIG && win.jig_hold_ms == 3000u && win.sticky_jig);
    picofw_parse_window_boot_args("4000 NOPULL", 5000u, 200u, 30000u, &win);
    assert(win.window_ms == 4000u && win.boot_action == PICOFW_BOOT_ACTION_PASSIVE && win.jig_hold_ms == 0u && !win.sticky_jig);
    picofw_parse_window_boot_args("12000 PREJIG:2800 WAKE:45000", 5000u, 200u, 90000u, &win);
    assert(win.window_ms == 12000u && win.boot_action == PICOFW_BOOT_ACTION_JIG && win.jig_hold_ms == 2800u && win.no_xreset && win.wait_for_wake && win.wake_timeout_ms == 45000u);
    picofw_parse_window_boot_args("6000 PASSIVE WAKE", 5000u, 200u, 90000u, &win);
    assert(win.window_ms == 6000u && win.boot_action == PICOFW_BOOT_ACTION_PASSIVE && !win.no_xreset && win.wait_for_wake && win.wake_timeout_ms == 30000u);

    picofw_bridge_spec_t bspec;
    picofw_parse_bridge_args("115200 PASSIVE CTRL", 115200u, &bspec);
    assert(bspec.baud == 115200u && bspec.boot_action == PICOFW_BOOT_ACTION_PASSIVE && bspec.log_ctrl_edges);
    picofw_parse_bridge_args("JIG:500 CTRL", 115200u, &bspec);
    assert(bspec.baud == 115200u && bspec.boot_action == PICOFW_BOOT_ACTION_JIG && bspec.jig_hold_ms == 500u && bspec.log_ctrl_edges);
    picofw_parse_bridge_args("", 57600u, &bspec);
    assert(bspec.baud == 57600u && bspec.boot_action == PICOFW_BOOT_ACTION_PASSIVE && !bspec.log_ctrl_edges);

    picofw_led_state_t led;
    picofw_led_state_init(&led);
    assert(!led.manual_lock && led.mode == LED_MODE_HEARTBEAT_IDLE);
    assert(picofw_led_pattern_output(LED_MODE_HEARTBEAT_IDLE, 0u));
    assert(!picofw_led_pattern_output(LED_MODE_HEARTBEAT_IDLE, 3u));
    assert(picofw_led_pattern_output(LED_MODE_BUS_FAULT, 4u));
    assert(!picofw_led_pattern_output(LED_MODE_OFF, 0u));

    picofw_led_state_set_manual(&led, LED_MODE_GDB_RSP);
    assert(led.manual_lock && led.mode == LED_MODE_GDB_RSP);
    assert(!picofw_led_state_set_auto(&led, LED_MODE_HEARTBEAT_IDLE));
    assert(led.mode == LED_MODE_GDB_RSP && led.manual_lock);

    const bool next_on = picofw_led_state_step_10hz(&led);
    assert(!next_on && led.phase == 1u);

    picofw_led_state_unlock_auto(&led, LED_MODE_HEARTBEAT_IDLE);
    assert(!led.manual_lock && led.mode == LED_MODE_HEARTBEAT_IDLE);
    assert(picofw_led_state_set_auto(&led, LED_MODE_BUS_ACTIVE));
    assert(led.mode == LED_MODE_BUS_ACTIVE);

    pico_led_mode_t parsed_mode = LED_MODE_OFF;
    bool is_auto = false;
    bool is_query = false;
    assert(picofw_led_parse_arg("off", &parsed_mode, &is_auto, &is_query));
    assert(parsed_mode == LED_MODE_OFF && !is_auto && !is_query);
    assert(picofw_led_parse_arg("AUTO", &parsed_mode, &is_auto, &is_query));
    assert(is_auto && !is_query);
    assert(picofw_led_parse_arg("status", &parsed_mode, &is_auto, &is_query));
    assert(is_query);
    assert(!picofw_led_parse_arg("bogus", &parsed_mode, &is_auto, &is_query));
}

static void test_framing_ctrl_and_jigsweep(void) {
    picofw_uart_framing_t f = PICOFW_FRAMING_8N1;
    assert(picofw_parse_framing("8e1", &f) && f == PICOFW_FRAMING_8E1);
    assert(strcmp(picofw_framing_name(f), "8E1") == 0);
    assert(picofw_parse_framing("8O1", &f) && f == PICOFW_FRAMING_8O1);
    assert(!picofw_parse_framing("7E1", &f));

    /* 0x55 = 0b01010101 (4 ones -> even parity bit 0, odd parity bit 1) */
    assert(picofw_uart_even_parity_bit(0x55u) == 0u);
    assert(picofw_uart_even_parity_bit(0x57u) == 1u);
    assert(picofw_uart_encode_tx_word(0x55u, PICOFW_FRAMING_8N1) == 0x055u);
    assert(picofw_uart_encode_tx_word(0x55u, PICOFW_FRAMING_8E1) == 0x055u);
    assert(picofw_uart_encode_tx_word(0x55u, PICOFW_FRAMING_8O1) == 0x155u);
    assert(picofw_uart_encode_tx_word(0x57u, PICOFW_FRAMING_8E1) == 0x157u);

    uint8_t rx_b = 0u;
    bool rx_ok = false;
    picofw_uart_decode_rx_word(0xAA000000u, PICOFW_FRAMING_8N1, &rx_b, &rx_ok);
    assert(rx_b == 0xAAu && rx_ok);
    /* Valid 8E1: byte=0x57 in [30:23], parity=1 in bit31 */
    const uint32_t raw_57_p1 = (1u << 31u) | ((uint32_t)0x57u << 23u);
    picofw_uart_decode_rx_word(raw_57_p1, PICOFW_FRAMING_8E1, &rx_b, &rx_ok);
    assert(rx_b == 0x57u && rx_ok);
    /* Corrupt parity bit -> 8O1 expects 0 for 0x57 */
    picofw_uart_decode_rx_word(raw_57_p1, PICOFW_FRAMING_8O1, &rx_b, &rx_ok);
    assert(rx_b == 0x57u && !rx_ok);

    picofw_window_boot_spec_t boot;
    uint32_t khz = 0u;
    bool ctrl_only = false;
    picofw_logic_parse_args_ex("4000 JIG:2500 CTRL", &boot, &khz, &ctrl_only);
    assert(boot.window_ms == 4000u && boot.jig_hold_ms == 2500u && khz == 1000u && ctrl_only);
    picofw_logic_parse_args_ex("4000 JIG:2500 500 CTRL", &boot, &khz, &ctrl_only);
    assert(khz == 500u && ctrl_only);

    picofw_jigsweep_spec_t sw;
    assert(picofw_jigsweep_parse_args("", &sw) && sw.mode == PICOFW_JIGSWEEP_HOLD && sw.point_count == 8u);
    assert(picofw_jigsweep_parse_args("DELAY 0 12 25", &sw));
    assert(sw.mode == PICOFW_JIGSWEEP_DELAY && sw.point_count == 3u && sw.points_ms[1] == 12u);
    assert(picofw_jigsweep_parse_args("PREBOOT", &sw) && sw.mode == PICOFW_JIGSWEEP_PREBOOT);
    assert(!picofw_jigsweep_parse_args("DELAY bogus", &sw));

    static const uint8_t k_ser[] = "DDR CHECK: OK\nserial boot mode\n";
    static const uint8_t k_norm[] = "DDR CHECK: OK\nnormal mode\n";
    static const uint8_t k_early[] = { 0x55u, 0xAAu };
    assert(picofw_jigsweep_classify_rx(NULL, 0u) == PICOFW_SWEEP_OUTCOME_SILENT);
    assert(picofw_jigsweep_classify_rx(k_ser, sizeof(k_ser) - 1u) == PICOFW_SWEEP_OUTCOME_SERIAL);
    assert(picofw_jigsweep_classify_rx(k_norm, sizeof(k_norm) - 1u) == PICOFW_SWEEP_OUTCOME_NORMAL);
    assert(picofw_jigsweep_classify_rx(k_early, sizeof(k_early)) == PICOFW_SWEEP_OUTCOME_EARLY_OTHER);

    /* Watchdog functional core: scratch packing, window clamping, CDC Ctrl-C abort */
    const uint32_t w0 = picofw_wdt_pack_scratch0(42u, PICOFW_WDT_FLAG_IN_CMD | PICOFW_WDT_FLAG_RECOVERED, 7u);
    uint8_t dec_cmd = 0u;
    uint8_t dec_flags = 0u;
    uint16_t dec_rec = 0u;
    assert(picofw_wdt_unpack_scratch0(w0, &dec_cmd, &dec_flags, &dec_rec));
    assert(dec_cmd == 42u && dec_flags == (PICOFW_WDT_FLAG_IN_CMD | PICOFW_WDT_FLAG_RECOVERED) && dec_rec == 7u);
    assert(!picofw_wdt_unpack_scratch0(w0, NULL, &dec_flags, &dec_rec));
    assert(picofw_wdt_clamp_window_ms(900000u, 15000u, 1000u) == PICOFW_WDT_MAX_CMD_WINDOW_MS);
    assert(picofw_wdt_clamp_window_ms(0u, 15000u, 1000u) == 15000u);
    assert(picofw_wdt_clamp_window_ms(50u, 15000u, 1000u) == 1000u);
    assert(picofw_wdt_is_abort_char(PICOFW_CDC_ABORT_CTRL_C));
    assert(!picofw_wdt_is_abort_char('B'));

    /* Distinguish UF2/BOOTSEL reboots from genuine enabled watchdog timeouts */
    assert(!picofw_wdt_is_genuine_timeout_recovery(true, false, false));
    assert(!picofw_wdt_is_genuine_timeout_recovery(true, false, true));
    assert(!picofw_wdt_is_genuine_timeout_recovery(true, true, false));
    assert(!picofw_wdt_is_genuine_timeout_recovery(false, true, true));
    assert(picofw_wdt_is_genuine_timeout_recovery(true, true, true));

    /* JIGPIN argument parser & Standby / Wakeup classifier */
    picofw_jigpin_mode_t jmode = PICOFW_JIGPIN_STATUS;
    assert(picofw_parse_jigpin_arg("", &jmode) && jmode == PICOFW_JIGPIN_STATUS);
    assert(picofw_parse_jigpin_arg("low", &jmode) && jmode == PICOFW_JIGPIN_LOW);
    assert(picofw_parse_jigpin_arg("GND", &jmode) && jmode == PICOFW_JIGPIN_LOW);
    assert(picofw_parse_jigpin_arg("HiZ", &jmode) && jmode == PICOFW_JIGPIN_HIZ);
    assert(picofw_parse_jigpin_arg("release", &jmode) && jmode == PICOFW_JIGPIN_HIZ);
    assert(!picofw_parse_jigpin_arg("pushpull_5v", &jmode));

    assert(strstr(picofw_classify_standby_state_mv(3250u, 3260u, true, true, false), "POWERED / AWAKE") != NULL);
    assert(strstr(picofw_classify_standby_state_mv(30u, 30u, true, true, false), "NSW5V HIGH") != NULL);
    assert(strstr(picofw_classify_standby_state_mv(30u, 400u, false, false, false), "CLAMPED LOW") != NULL);
    assert(strstr(picofw_classify_standby_state_mv(30u, 3250u, false, false, false), "CABLE UNPLUGGED") != NULL);
    assert(strstr(picofw_classify_standby_state_mv(0u, 0u, false, false, true), "JIGPIN_LOW LATCHED") != NULL);
}

static void test_wdt_stage_and_fmu_codec(void) {
    /* WDT stage traceability packing in scratch[3] */
    const uint32_t w3 = picofw_wdt_pack_scratch3(PICOFW_WDT_STAGE_PROBE_SWEEP, 0x00123456u);
    uint8_t stage = 0u;
    uint32_t up_ms = 0u;
    assert(picofw_wdt_unpack_scratch3(w3, &stage, &up_ms));
    assert(stage == PICOFW_WDT_STAGE_PROBE_SWEEP && up_ms == 0x00123456u);
    assert(!picofw_wdt_unpack_scratch3(w3, NULL, &up_ms));
    assert(strcmp(picofw_wdt_stage_name(PICOFW_WDT_STAGE_PROBE_SWEEP), "PROBE_SWEEP") == 0);
    assert(strcmp(picofw_wdt_stage_name(99u), "UNKNOWN") == 0);

    /* FMU rolling trigger matcher + echo false-positive guard */
    picofw_fmu_phase_state_t st;
    picofw_fmu_state_init(&st);
    const char *msg = "DDR CHECK: OK\r\nserial boot mode\r\nwait 1 second.\r\nSD init start";
    bool any_rel = false, any_ser = false, any_w1 = false;
    for (size_t i = 0u; msg[i] != '\0'; ++i) {
        bool rel = false, ser = false, w1 = false;
        picofw_fmu_feed_byte(&st, 2u, (uint8_t)msg[i], (uint32_t)(i * 100u), &rel, &ser, &w1);
        any_rel = any_rel || rel;
        any_ser = any_ser || ser;
        any_w1  = any_w1 || w1;
    }
    assert(any_rel && any_ser && any_w1 && st.sw_uart_flipped && st.wait1_seen && st.sd_started);
    assert(!picofw_fmu_is_echo_hit(256u, true, true, 100u));
    assert(!picofw_fmu_is_echo_hit(256u, true, false, PICOFW_FMU_ECHO_WINDOW_US + 1u));
    assert(!picofw_fmu_is_echo_hit(0u, true, false, 100u));
    assert(picofw_fmu_is_echo_hit(1u, true, false, PICOFW_FMU_ECHO_WINDOW_US));

    uint8_t tx_buf[8];
    assert(picofw_fmu_decode_or_default_cr("", tx_buf, sizeof(tx_buf)) == 1u && tx_buf[0] == 0x0Du);
    assert(picofw_fmu_decode_or_default_cr("A55A", tx_buf, sizeof(tx_buf)) == 2u && tx_buf[0] == 0xA5u);

    uint32_t fmode = 0u;
    bool rep = false;
    const char *hex = NULL;
    picofw_fmu_parse_test_args("13 CAFE", &fmode, &rep, &hex);
    assert(fmode == 3u && rep && strcmp(hex, "CAFE") == 0);
    picofw_fmu_parse_test_args("BEEF", &fmode, &rep, &hex);
    assert(fmode == 0u && !rep && strcmp(hex, "BEEF") == 0);

    uint32_t bs = 0u, bc = 0u, bo = 0u;
    picofw_fmu_parse_batch_args("250 32 50", &bs, &bc, &bo);
    assert(bs == 250u && bc == 6u && bo == 50u);

    /* Strict numeric & hex token parsers (zero atoi/strtoul UB) */
    uint32_t u32 = 0u;
    const char *tail = NULL;
    assert(picofw_parse_u32_strict(" 4294967295 ", &u32) && u32 == 0xFFFFFFFFu);
    assert(!picofw_parse_u32_strict("4294967296", &u32));
    assert(!picofw_parse_u32_strict("-1", &u32));
    assert(!picofw_parse_u32_strict("12ms", &u32));
    assert(picofw_parse_u32_auto("0x807F0000", &u32) && u32 == 0x807F0000u);
    assert(picofw_parse_u32_auto("16", &u32) && u32 == 16u);
    assert(!picofw_parse_u32_auto("0x100000000", &u32));
    assert(picofw_parse_hex_u32_prefix("  deadbeef 32", &u32, &tail));
    assert(u32 == 0xDEADBEEFu && strcmp(tail, "32") == 0);
    assert(!picofw_parse_hex_u32_prefix("80000000z", &u32, &tail));
}

static void test_uart_tx_bus_policy(void) {
    picofw_bus_deny_t deny = PICOFW_BUS_ALLOW;
    const char *safe_help = "HELP";
    const char *safe_ipl_read = "MD 80000000 10";
    const char *safe_nvram_w = "MW 807F0000 1234";
    const char *safe_mmio_ok = "MD DC0F0080";
    const char *safe_rsp_r = "$m80000000,4#00";
    const char *safe_rsp_nvram = "$M807F0000,4:deadbeef#00";
    const char *lethal_r = "MD C000100C";
    const char *lethal_w = "MW 0xDC0F0000 0";
    const char *lethal_rsp = "$mC000100C,4#00";
    const char *ipl_write = "MW 80000000 DEADBEEF";
    const char *ipl_uncached = "ERASE 0x84000000";
    const char *ipl_rsp = "$M80000000,4:deadbeef#00";
    const char *cs0_mirror_rsp = "$M80800000,4:deadbeef#00";

    assert(picofw_bus_check_uart_tx_payload((const uint8_t *)safe_help, strlen(safe_help), false, &deny));
    assert(deny == PICOFW_BUS_ALLOW);
    assert(picofw_bus_check_uart_tx_payload((const uint8_t *)safe_ipl_read, strlen(safe_ipl_read), true, &deny));
    assert(picofw_bus_check_uart_tx_payload((const uint8_t *)safe_nvram_w, strlen(safe_nvram_w), false, &deny));
    assert(picofw_bus_check_uart_tx_payload((const uint8_t *)safe_mmio_ok, strlen(safe_mmio_ok), false, &deny));
    assert(picofw_bus_check_uart_tx_payload((const uint8_t *)safe_rsp_r, strlen(safe_rsp_r), true, &deny));
    assert(picofw_bus_check_uart_tx_payload((const uint8_t *)safe_rsp_nvram, strlen(safe_rsp_nvram), true, &deny));

    assert(!picofw_bus_check_uart_tx_payload((const uint8_t *)lethal_r, strlen(lethal_r), false, &deny));
    assert(deny == PICOFW_BUS_DENY_LETHAL_MMIO);
    assert(!picofw_bus_check_uart_tx_payload((const uint8_t *)lethal_w, strlen(lethal_w), true, &deny));
    assert(deny == PICOFW_BUS_DENY_LETHAL_MMIO);
    assert(!picofw_bus_check_uart_tx_payload((const uint8_t *)lethal_rsp, strlen(lethal_rsp), true, &deny));
    assert(deny == PICOFW_BUS_DENY_LETHAL_MMIO);
    assert(!picofw_bus_check_uart_tx_payload((const uint8_t *)ipl_write, strlen(ipl_write), true, &deny));
    assert(deny == PICOFW_BUS_DENY_IPL_LAYER1);
    assert(!picofw_bus_check_uart_tx_payload((const uint8_t *)ipl_uncached, strlen(ipl_uncached), false, &deny));
    assert(deny == PICOFW_BUS_DENY_IPL_LAYER1);
    assert(!picofw_bus_check_uart_tx_payload((const uint8_t *)ipl_rsp, strlen(ipl_rsp), true, &deny));
    assert(deny == PICOFW_BUS_DENY_IPL_LAYER1);
    assert(!picofw_bus_check_uart_tx_payload((const uint8_t *)cs0_mirror_rsp, strlen(cs0_mirror_rsp), true, &deny));
    assert(deny == PICOFW_BUS_DENY_CS0_MIRROR);
}

int picofw_run_uart_codec_tests(void) {
    test_timeout_and_tokens();
    test_hex_and_baud_math();
    test_trigger_and_waittx_parser();
    test_hex_window_and_led_logic();
    test_framing_ctrl_and_jigsweep();
    test_wdt_stage_and_fmu_codec();
    test_uart_tx_bus_policy();
    printf("[HOST-TEST] test_uart_codec + LED + hex + framing + jigsweep + wdt + fmu + tx_policy passed.\n");
    return picofw_run_pana_logic_tests();
}
