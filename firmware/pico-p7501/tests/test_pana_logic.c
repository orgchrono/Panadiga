/**
 * @file test_pana_logic.c
 * @brief Host C11 unit tests for `p7501_pana_rsp_codec` and `p7501_piologic_codec`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_pana_logic.h"
#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_dumpnor.h"
#include "flash/p7501_flash.h"
#include "diag/p7501_piologic_codec.h"
#include "diag/p7501_sds_codec.h"

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

    /* Verify 3-byte `_dev_SERIAL` UART state-machine handshake (`0E 04 02`) & 32-bit LE `$M`. */
    assert(P7501_RSP_HANDSHAKE_LEN == 3u);
    assert(k_p7501_rsp_handshake[0] == 0x0Eu && k_p7501_rsp_handshake[1] == 0x04u);
    assert(k_p7501_rsp_handshake[2] == 0x02u);

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
    assert(!p7501_panarsp_parse_args("WMEM:80000000,11223344", &spec));
    assert(!p7501_panarsp_parse_args("WRITE32:847EFFF0,12345678", &spec));
    assert(p7501_panarsp_parse_args("WMEM:847F0000,11223344", &spec));
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

static void sds_emit_cmd_frame(p7501_sds_state_t *st, bool dir_host, uint8_t idx, uint32_t arg) {
    uint8_t raw5[5];
    raw5[0] = (uint8_t)((dir_host ? 0x40u : 0x00u) | (idx & 0x3Fu));
    raw5[1] = (uint8_t)(arg >> 24u);
    raw5[2] = (uint8_t)(arg >> 16u);
    raw5[3] = (uint8_t)(arg >> 8u);
    raw5[4] = (uint8_t)arg;
    const uint8_t crc7 = p7501_sds_crc7_bytes(raw5, 5u);
    const uint8_t frame[6] = {
        raw5[0], raw5[1], raw5[2], raw5[3], raw5[4], (uint8_t)((crc7 << 1u) | 1u)
    };
    p7501_sds_feed_sample(st, 0x1Fu); /* idle clock */
    for (size_t b = 0u; b < 6u; ++b) {
        for (int bit = 7; bit >= 0; --bit) {
            const uint8_t cmd_b = (uint8_t)((frame[b] >> (uint8_t)bit) & 1u);
            p7501_sds_feed_sample(st, (uint8_t)(0x1Eu | cmd_b)); /* DAT0..3 idle HIGH */
        }
    }
    p7501_sds_feed_sample(st, 0x1Fu);
}

static void sds_emit_4bit_block(
    p7501_sds_state_t *st,
    const uint8_t *payload,
    size_t len,
    uint8_t corrupt_lane_mask
) {
    uint16_t calc_crc[4] = { 0u, 0u, 0u, 0u };
    /* Start bit: all 4 data lanes go LOW while CMD stays HIGH (1). */
    p7501_sds_feed_sample(st, P7501_SDS_BIT_CMD);

    for (size_t i = 0u; i < len; ++i) {
        const uint8_t nibs[2] = {
            (uint8_t)((payload[i] >> 4u) & 0x0Fu),
            (uint8_t)(payload[i] & 0x0Fu)
        };
        for (size_t k = 0u; k < 2u; ++k) {
            for (uint8_t lane = 0u; lane < 4u; ++lane) {
                calc_crc[lane] = p7501_sds_crc16_step(
                    calc_crc[lane], (uint8_t)((nibs[k] >> lane) & 1u)
                );
            }
            p7501_sds_feed_sample(st, (uint8_t)(P7501_SDS_BIT_CMD | (nibs[k] << 1u)));
        }
    }

    for (int b = 15; b >= 0; --b) {
        uint8_t nib = 0u;
        for (uint8_t lane = 0u; lane < 4u; ++lane) {
            uint8_t bit = (uint8_t)((calc_crc[lane] >> (uint8_t)b) & 1u);
            if (b == 0 && ((corrupt_lane_mask >> lane) & 1u) != 0u) {
                bit ^= 1u;
            }
            nib |= (uint8_t)(bit << lane);
        }
        p7501_sds_feed_sample(st, (uint8_t)(P7501_SDS_BIT_CMD | (nib << 1u)));
    }
    p7501_sds_feed_sample(st, 0x1Fu); /* end bit HIGH */
}

static void test_sds_bus_sniffer_codec(void) {
    const uint8_t cmd0[5] = { 0x40u, 0x00u, 0x00u, 0x00u, 0x00u };
    assert(((p7501_sds_crc7_bytes(cmd0, 5u) << 1u) | 1u) == 0x95u);
    const uint8_t cmd8[5] = { 0x48u, 0x00u, 0x00u, 0x01u, 0xAAu };
    assert(((p7501_sds_crc7_bytes(cmd8, 5u) << 1u) | 1u) == 0x87u);

    p7501_sds_spec_t sds_arg;
    assert(p7501_sds_parse_args("5000 NORMAL LEN:64", &sds_arg));
    assert(sds_arg.boot.window_ms == 5000u && sds_arg.default_block_bytes == 64u);
    assert(!p7501_sds_parse_args("4000 NORMAL LEN:128", &sds_arg));

    p7501_sds_state_t st;
    p7501_sds_init(&st, 0u);
    sds_emit_cmd_frame(&st, true, 0u, 0u);
    sds_emit_cmd_frame(&st, true, 55u, 0x12340000u);
    sds_emit_cmd_frame(&st, false, 55u, 0x00000120u);
    sds_emit_cmd_frame(&st, true, 6u, 0x00000002u); /* ACMD6 4-bit */
    sds_emit_cmd_frame(&st, false, 6u, 0x00000920u);
    assert(st.bus_width == 4u && st.saw_cmd55 && st.saw_acmd6_4bit && st.cmd_crc_err == 0u);

    sds_emit_cmd_frame(&st, true, 55u, 0x12340000u);
    sds_emit_cmd_frame(&st, false, 55u, 0x00000920u);
    sds_emit_cmd_frame(&st, true, 13u, 0u); /* ACMD13 */
    sds_emit_cmd_frame(&st, false, 13u, 0x00000920u);
    assert(st.saw_acmd13 && st.expecting_acmd13);

    uint8_t sd_status[64];
    memset(sd_status, 0, sizeof(sd_status));
    sd_status[0] = 0x80u; /* DAT_BUS_WIDTH = 2 (4-bit) */
    sd_status[2] = 0x00u; /* SD_CARD_TYPE = 0x0000 (regular RW SD) */
    sd_status[3] = 0x00u;
    sds_emit_4bit_block(&st, sd_status, sizeof(sd_status), 0x00u);
    assert(st.data_blocks_ok == 1u && st.data_crc_err == 0u);
    assert(st.acmd13_captured && st.acmd13_crc_ok);
    assert(st.acmd13_dat_bus_width == 2u && st.acmd13_sd_card_type == 0x0000u);

    /* Now fault ONLY DAT2 (bit 2 -> mask 0x04) on a second ACMD13 and confirm isolated lane error. */
    sds_emit_cmd_frame(&st, true, 55u, 0x12340000u);
    sds_emit_cmd_frame(&st, true, 13u, 0u);
    sds_emit_4bit_block(&st, sd_status, sizeof(sd_status), 0x04u);
    assert(st.data_crc_err == 1u && st.last_fail_lane_mask == 0x04u);
    assert(st.lane_crc_err[0] == 0u && st.lane_crc_err[1] == 0u);
    assert(st.lane_crc_err[2] == 1u && st.lane_crc_err[3] == 0u);
}

int p7501_run_pana_logic_tests(void) {
    test_pana_rsp_frame_codec();
    test_rsp_boot_prose_does_not_trip_nak();
    test_pana_rsp_arg_parser();
    test_piologic_analyzer();
    test_flash_geometry_and_bgo();
    test_pana_dumpnor_stub_and_args();
    p7501_run_pana_diag_drv_tests();
    test_sds_bus_sniffer_codec();
    printf("[HOST-TEST] PANARSP + REGS + DUMPNOR + DIAGWATCH + DRVPANA + FlashGeo + PIOLOGIC + SDSNIFF passed.\n");
    return 0;
}
