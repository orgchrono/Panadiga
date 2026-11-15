/**
 * @file test_pana_logic.c
 * @brief Host C11 unit tests for `p7501_pana_rsp_codec` and `p7501_piologic_codec`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_pana_logic.h"
#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_dumpnor.h"
#include "uart/p7501_diag_watch_codec.h"
#include "flash/p7501_flash.h"
#include "diag/p7501_piologic_codec.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

static void test_pana_rsp_frame_codec(void) {
    char frame[64];
    assert(p7501_rsp_build_frame("g", frame, sizeof(frame)) == 5u);
    assert(strcmp(frame, "$g#67") == 0);

    assert(p7501_rsp_build_frame("Z0", frame, sizeof(frame)) == 6u);
    assert(strcmp(frame, "$Z0#8a") == 0);

    assert(p7501_rsp_build_frame("OK", frame, sizeof(frame)) == 6u);
    assert(strcmp(frame, "$OK#9a") == 0);

    const uint8_t ok_reply[] = "\x15+$OK#9A";
    p7501_rsp_frame_info_t info;
    assert(p7501_rsp_inspect_frame(ok_reply, sizeof(ok_reply) - 1u, &info));
    assert(info.found && info.ack_plus && info.raw_nak_0x15 && info.checksum_ok);
    assert(info.expected_csum == 0x9Au && info.payload_len == 2u);

    const uint8_t err_reply[] = "-$E01#A6";
    assert(p7501_rsp_inspect_frame(err_reply, sizeof(err_reply) - 1u, &info));
    assert(info.found && info.nak_minus && info.checksum_ok && info.payload_len == 3u);

    const uint8_t bad_csum[] = "+$OK#00";
    assert(p7501_rsp_inspect_frame(bad_csum, sizeof(bad_csum) - 1u, &info));
    assert(info.found && !info.checksum_ok);

    /* Ensure post-trigger slice ignores earlier bogus `$NO#9D` and finds real `$DEADBEEF#20` reply. */
    uint8_t mixed[] = "BOOT $NO#9D chatter... +$DEADBEEF#20";
    assert(p7501_rsp_inspect_frame_from(mixed, sizeof(mixed) - 1u, 15u, &info));
    assert(info.found && info.ack_plus && info.checksum_ok);
    assert(info.payload_offset == 25u && info.payload_len == 8u);
    assert(memcmp(mixed + info.payload_offset, "DEADBEEF", 8u) == 0);

    /* Raw binary `$x` reply containing embedded `#9A`, NUL byte, `$` and `+` before real `#xx`. */
    const uint8_t bin_payload[] = { 0x41u, 0x00u, '#', '9', 'A', '$', '+', 0xFFu, 0x15u };
    const uint8_t bin_csum = p7501_rsp_checksum(bin_payload, sizeof(bin_payload));
    uint8_t bin_frame[32];
    bin_frame[0] = (uint8_t)'+';
    bin_frame[1] = (uint8_t)'$';
    memcpy(&bin_frame[2], bin_payload, sizeof(bin_payload));
    bin_frame[2u + sizeof(bin_payload)] = (uint8_t)'#';
    bin_frame[3u + sizeof(bin_payload)] = (uint8_t)p7501_nibble_to_hex_char((uint8_t)(bin_csum >> 4));
    bin_frame[4u + sizeof(bin_payload)] = (uint8_t)p7501_nibble_to_hex_char((uint8_t)(bin_csum & 0x0Fu));
    const size_t bin_frame_len = 5u + sizeof(bin_payload);

    assert(p7501_rsp_inspect_frame(bin_frame, bin_frame_len, &info));
    assert(info.found && info.ack_plus && info.checksum_ok);
    assert(info.payload_len == sizeof(bin_payload));
    assert(memcmp(&bin_frame[info.payload_offset], bin_payload, sizeof(bin_payload)) == 0);

    /* If even the real trailer is corrupted, fall back cleanly to checksum mismatch. */
    bin_frame[3u + sizeof(bin_payload)] = (bin_frame[3u + sizeof(bin_payload)] == '0') ? '1' : '0';
    assert(p7501_rsp_inspect_frame(bin_frame, bin_frame_len, &info));
    assert(info.found && !info.checksum_ok);

    /* Verify 4-byte UART state-machine handshake & 32-bit LE `$M` write helper. */
    assert(k_p7501_rsp_handshake[0] == 0x0Eu && k_p7501_rsp_handshake[1] == 0x02u);
    assert(k_p7501_rsp_handshake[2] == 0x02u && k_p7501_rsp_handshake[3] == 0x04u);

    char wr_frame[48];
    const size_t wr_len = p7501_rsp_build_write_u32(
        P7501_AM34_SERIAL_CB_PTR_ADDR,
        0x51800000u,
        wr_frame,
        sizeof(wr_frame)
    );
    assert(wr_len > 0u);
    assert(strncmp(wr_frame, "$M515B2E40,4:00008051#", 22u) == 0);
    assert(p7501_rsp_inspect_frame((const uint8_t *)wr_frame, wr_len, &info));
    assert(info.found && info.checksum_ok);
    assert(p7501_rsp_build_write_u32(0x515B2E40u, 0x51800000u, wr_frame, 12u) == 0u);

    const uint8_t pass_branch[4] = { 0xCCu, 0xD0u, 0x00u, 0xCBu };
    const size_t wm_len = p7501_rsp_build_write_mem(
        0x574F7B8Cu,
        pass_branch,
        sizeof(pass_branch),
        wr_frame,
        sizeof(wr_frame)
    );
    assert(wm_len > 0u);
    assert(strncmp(wr_frame, "$M574F7B8C,4:ccd000cb#", 22u) == 0);
    assert(p7501_rsp_inspect_frame((const uint8_t *)wr_frame, wm_len, &info));
    assert(info.found && info.checksum_ok);
}

static void test_pana_rsp_arg_parser(void) {
    p7501_pana_rsp_spec_t spec;
    assert(p7501_panarsp_parse_args(NULL, &spec));
    assert(spec.timeout_ms == 4000u && spec.action == PANA_RSP_ACT_SEQ && spec.send_nak_prefix);

    assert(p7501_panarsp_parse_args("6500 JIG:350 MAGIC \"WAIT:wait 1\" MEM:515B2E38,10", &spec));
    assert(spec.timeout_ms == 6500u);
    assert(spec.boot_action == P7501_BOOT_ACTION_JIG && spec.jig_hold_ms == 350u);
    assert(spec.send_magic_prefix && spec.wait_trigger_enabled);
    assert(strcmp(spec.trigger, "wait 1") == 0);
    assert(spec.action == PANA_RSP_ACT_MEM);
    assert(strcmp(spec.payload, "m515B2E38,10") == 0);

    assert(p7501_panarsp_parse_args("4000 XMEM:0x84000000,0x440", &spec));
    assert(spec.timeout_ms == 4000u && spec.action == PANA_RSP_ACT_XMEM);
    assert(strcmp(spec.payload, "x84000000,440") == 0);
    assert(!p7501_panarsp_parse_args("XMEM:", &spec));
    assert(!p7501_panarsp_parse_args("XMEM:ZZZZ,10", &spec));

    assert(p7501_panarsp_parse_args("WMEM:574F7B8C,CCD000CB", &spec));
    assert(spec.action == PANA_RSP_ACT_WMEM && spec.addr == 0x574F7B8Cu && spec.data_len == 4u);
    assert(strcmp(spec.payload, "M574F7B8C,4:ccd000cb") == 0);

    assert(p7501_panarsp_parse_args("WRITE32:515B2E40,50401F3C", &spec));
    assert(spec.action == PANA_RSP_ACT_WRITE32 && spec.addr == 0x515B2E40u && spec.value_u32 == 0x50401F3Cu);
    assert(strcmp(spec.payload, "M515B2E40,4:3c1f4050") == 0);

    assert(p7501_panarsp_parse_args("PASSTHROUGH", &spec));
    assert(spec.action == PANA_RSP_ACT_PASSTHROUGH && spec.send_magic_prefix && spec.send_nak_prefix);

    assert(p7501_panarsp_parse_args("CALL:50401F3C", &spec));
    assert(spec.action == PANA_RSP_ACT_CALL && spec.addr == 0x50401F3Cu && spec.send_magic_prefix);
    assert(strcmp(spec.payload, "M515B2E40,4:3c1f4050") == 0);

    assert(!p7501_panarsp_parse_args("WMEM:", &spec));
    assert(!p7501_panarsp_parse_args("WRITE32:ZZZZ,1234", &spec));
    assert(!p7501_panarsp_parse_args("CALL:XYZ", &spec));

    assert(p7501_panarsp_parse_args("NONAK HEX:150E0402", &spec));
    assert(!spec.send_nak_prefix && spec.action == PANA_RSP_ACT_HEX && spec.raw_len == 4u);
    assert(spec.raw_bytes[0] == 0x15u && spec.raw_bytes[3] == 0x02u);

    assert(!p7501_panarsp_parse_args("BOGUS_CMD", &spec));
}

static void test_piologic_analyzer(void) {
    p7501_window_boot_spec_t boot;
    uint32_t khz = 0u;
    p7501_logic_parse_args("3000 JIG:350 1500", &boot, &khz);
    assert(boot.window_ms == 3000u && boot.boot_action == P7501_BOOT_ACTION_JIG);
    assert(boot.jig_hold_ms == 350u && khz == 1500u);

    /* Initial state: RXD=1, NSW5V=1, TXD=1, XRESET=0 -> 0b0111 (7) */
    const p7501_logic_edge_t edges[] = {
        { 1000u, 0x0Fu }, /* XRESET rising (0->1) */
        { 1500u, 0x0Eu }, /* RXD falling (first UART byte after reset: +500 us) */
        { 1508u, 0x0Fu }, /* RXD rising (8 us low bit!) */
        { 2000u, 0x0Bu }, /* TXD falling */
        { 2040u, 0x0Fu }, /* TXD rising */
        { 2120u, 0x0Eu }, /* RXD falling (response turnaround: +120 us from TXD fall) */
    };
    p7501_logic_stats_t st;
    p7501_logic_analyze_edges(edges, sizeof(edges) / sizeof(edges[0]), 0x07u, &st);

    assert(st.edges[P7501_LOGIC_CH_NSW5V] == 0u);
    assert(st.edges[P7501_LOGIC_CH_XRESET] == 1u);
    assert(st.edges[P7501_LOGIC_CH_RXD] == 3u);
    assert(st.min_low_us[P7501_LOGIC_CH_RXD] == 8u);
    assert(st.min_low_us[P7501_LOGIC_CH_TXD] == 40u);
    assert(st.saw_reset_release && st.reset_to_first_rxd_us == 500u);
    assert(st.saw_tx_to_rx && st.min_tx_to_rx_us == 120u && st.max_tx_to_rx_us == 120u);
}

static void test_flash_geometry_and_bgo(void) {
    const p7501_flash_geometry_t g8 = p7501_flash_get_geometry(8u * 1024u * 1024u);
    assert(g8.bytes == 0x00800000UL && g8.mb == 8u);
    assert(g8.nvram_base == 0x807F0000UL && g8.nvram_end == 0x80800000UL);
    assert(g8.hw_uncached_base == 0x84000000UL);
    assert(g8.hw_nvram_base == 0x847F0000UL && g8.hw_nvram_end == 0x84800000UL);
    assert(g8.bgo_hw_base == 0x84200000UL && g8.bgo_hw_end == 0x84400000UL);
    assert(g8.bgo_sector_count == 32u);

    assert(p7501_flash_is_readonly_nor_range(g8.bytes, 0x80000000UL, 0x00800000UL));
    assert(p7501_flash_is_readonly_nor_range(g8.bytes, 0x84000000UL, 0x00800000UL));
    assert(p7501_flash_is_readonly_nor_range(g8.bytes, 0x84200000UL, 0x00200000UL));
    assert(!p7501_flash_is_readonly_nor_range(g8.bytes, 0x84000000UL, 0u));
    assert(!p7501_flash_is_readonly_nor_range(g8.bytes, 0x847F0000UL, 0x00020000UL));
    assert(!p7501_flash_is_readonly_nor_range(g8.bytes, 0x50400000UL, 0x00010000UL));
}

static void test_pana_dumpnor_stub_and_args(void) {
    uint8_t stub[P7501_DUMPNOR_STUB_SIZE];
    assert(p7501_dumpnor_build_stub(0x84000000UL, 0x00800000UL, 0x51800000UL, stub, sizeof(stub)) == P7501_DUMPNOR_STUB_SIZE);
    assert(stub[110] == 0xFCu && stub[111] == 0xACu && stub[116] == 0x1Cu && stub[131] == 0xDFu);
    assert(memcmp(&stub[134], "PANADMP1", 8u) == 0);
    assert(memcmp(&stub[154], "PANADONE", 8u) == 0);
    assert(p7501_dumpnor_build_stub(0x00000000UL, 0x00010000UL, 0x51800000UL, stub, sizeof(stub)) == 0u);
    assert(p7501_dumpnor_build_stub(0x84000000UL, 0x00800000UL, 0x51800000UL, stub, 64u) == 0u);

    p7501_pana_rsp_spec_t spec;
    assert(p7501_panarsp_parse_args("60000 JIG:350 DUMPNOR 84000000 10000", &spec));
    assert(spec.action == PANA_RSP_ACT_DUMPNOR && spec.timeout_ms == 60000u);
    assert(spec.boot_action == P7501_BOOT_ACTION_JIG && spec.jig_hold_ms == 350u);
    assert(spec.addr == 0x84000000UL && spec.value_u32 == 0x00010000UL && spec.send_magic_prefix);

    assert(p7501_panarsp_parse_args("DUMPNOR:847F0000,10000", &spec));
    assert(spec.action == PANA_RSP_ACT_DUMPNOR && spec.addr == 0x847F0000UL && spec.value_u32 == 0x00010000UL);

    assert(!p7501_panarsp_parse_args("DUMPNOR 00000000 1000", &spec));
    assert(!p7501_panarsp_parse_args("DUMPNOR 847F0000", &spec));
}

static void feed_str_and_count(
    p7501_diag_watch_state_t *st,
    const p7501_diag_watch_spec_t *dw,
    const char *s,
    int *rel_count,
    int *tx_count,
    char *last_tx
) {
    for (size_t i = 0u; s[i] != '\0'; ++i) {
        char tx_ch = '\0';
        bool rel = false;
        (void)p7501_diag_watch_feed_byte(st, dw, (uint8_t)s[i], &tx_ch, &rel);
        if (rel) {
            *rel_count += 1;
        }
        if (tx_ch != '\0') {
            *tx_count += 1;
            *last_tx = tx_ch;
        }
    }
}

static void test_diag_watch_codec(void) {
    p7501_diag_watch_spec_t dw;
    assert(p7501_diag_watch_parse_args("", &dw));
    assert(dw.timeout_ms == 15000u && dw.boot_action == P7501_BOOT_ACTION_JIG);
    assert(dw.jig_hold_ms == 2500u && dw.action == P7501_DIAG_ACT_AUTO && dw.release_on_ddr);

    assert(p7501_diag_watch_parse_args("30000 NORMAL FORMAT HOLD", &dw));
    assert(dw.timeout_ms == 30000u && dw.boot_action == P7501_BOOT_ACTION_NORMAL);
    assert(dw.action == P7501_DIAG_ACT_FORMAT && !dw.release_on_ddr);

    assert(p7501_diag_watch_parse_args("180000 NOPULL DUMP RELEASE", &dw));
    assert(dw.timeout_ms == 180000u && dw.boot_action == P7501_BOOT_ACTION_PASSIVE);
    assert(dw.action == P7501_DIAG_ACT_DUMP && dw.release_on_ddr);
    assert(!p7501_diag_watch_parse_args("20000 BOGUS_DW", &dw));

    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_AUTO) == 'B');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_FORMAT) == 'F');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_DUMP) == 'D');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_NONE) == '\0');

    assert(p7501_diag_watch_parse_args("25000 JIG:2500 AUTO RELEASE", &dw));
    p7501_diag_watch_state_t st;
    p7501_diag_watch_init(&st);
    int rel_count = 0;
    int tx_count = 0;
    char last_tx = '\0';

    /* In JIG mode, DDR CHECK: OK arrives BEFORE P50 is sampled: must keep SW_UART LOW! */
    feed_str_and_count(&st, &dw, "DDR CHECK: OK\r\n", &rel_count, &tx_count, &last_tx);
    assert(rel_count == 0 && !st.sw_uart_released);
    assert((st.seen_events & P7501_DIAG_EVT_DDR_OK) != 0u);

    /* Release happens cleanly once "serial boot mode" confirms /BOOT.BIN sampled P50. */
    feed_str_and_count(
        &st, &dw,
        "***** serial boot mode *****\r\nSetBusWidth -91006\r\n** open tray ** 80040904\r\nPANAMON1>",
        &rel_count, &tx_count, &last_tx
    );
    assert(rel_count == 1 && st.sw_uart_released);
    assert(tx_count == 1 && last_tx == 'B' && st.tx_dispatched);
    assert((st.seen_events & P7501_DIAG_EVT_SERIAL_BOOT) != 0u);
    assert((st.seen_events & P7501_DIAG_EVT_SD_ERR) != 0u);
    assert((st.seen_events & P7501_DIAG_EVT_ODD_OPEN) != 0u);
    assert((st.seen_events & P7501_DIAG_EVT_CD_APC_ERR) != 0u);
    assert((st.seen_events & P7501_DIAG_EVT_PANAMON) != 0u);
    assert(p7501_diag_watch_diagnosis(st.seen_events) != NULL);

    /* In NORMAL mode, DDR CHECK: OK releases immediately and BGO beats ODD noise in diagnosis. */
    assert(p7501_diag_watch_parse_args("12000 NORMAL NONE RELEASE", &dw));
    p7501_diag_watch_init(&st);
    rel_count = 0;
    tx_count = 0;
    feed_str_and_count(
        &st, &dw,
        "DDR CHECK: OK\r\n***** normal mode *****\r\nBGO driver init failed\r\n80040904\r\n",
        &rel_count, &tx_count, &last_tx
    );
    assert(rel_count == 1 && st.sw_uart_released && tx_count == 0);
    assert(strstr(p7501_diag_watch_diagnosis(st.seen_events), "BGO driver init failed") != NULL);
}
static void test_rsp_boot_prose_does_not_trip_nak(void) {
    const char *boot_log =
        "Main hung-up time changed 255[s].\r\n"
        "[SDCD]SetBusWidth:SD Status ERROR -91006\r\n"
        "SD init failed[-90002].\r\n";
    p7501_rsp_frame_info_t info;
    assert(!p7501_rsp_inspect_frame_from(
        (const uint8_t *)boot_log, strlen(boot_log), 0u, &info
    ));
    assert(!info.found && !info.ack_plus && !info.nak_minus && !info.raw_nak_0x15);

    const char *real_ack = "+$OK#9a";
    assert(p7501_rsp_inspect_frame_from(
        (const uint8_t *)real_ack, strlen(real_ack), 0u, &info
    ));
    assert(info.found && info.checksum_ok && info.ack_plus && !info.nak_minus);

    const char *real_nak = "-$E01#A6";
    assert(p7501_rsp_inspect_frame_from(
        (const uint8_t *)real_nak, strlen(real_nak), 0u, &info
    ));
    assert(info.found && info.checksum_ok && !info.ack_plus && info.nak_minus);

    const char *after_csum_nak = "#9a-";
    assert(!p7501_rsp_inspect_frame_from(
        (const uint8_t *)after_csum_nak, strlen(after_csum_nak), 0u, &info
    ));
    assert(!info.found && info.nak_minus && !info.ack_plus);
}

int p7501_run_pana_logic_tests(void) {
    test_pana_rsp_frame_codec();
    test_rsp_boot_prose_does_not_trip_nak();
    test_pana_rsp_arg_parser();
    test_piologic_analyzer();
    test_flash_geometry_and_bgo();
    test_pana_dumpnor_stub_and_args();
    test_diag_watch_codec();
    printf("[HOST-TEST] PANARSP + DUMPNOR + DIAGWATCH + FlashGeo + PIOLOGIC passed.\n");
    return 0;
}
