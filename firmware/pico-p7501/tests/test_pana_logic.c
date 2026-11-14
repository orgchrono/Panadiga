/**
 * @file test_pana_logic.c
 * @brief Host C11 unit tests for `p7501_pana_rsp_codec` and `p7501_piologic_codec`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_pana_logic.h"
#include "uart/p7501_pana_rsp.h"
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

int p7501_run_pana_logic_tests(void) {
    test_pana_rsp_frame_codec();
    test_pana_rsp_arg_parser();
    test_piologic_analyzer();
    printf("[HOST-TEST] PANARSP codec + PIOLOGIC timing analyzer passed all assertions.\n");
    return 0;
}
