/**
 * @file test_pana_logic.c
 * @brief Host C11 unit tests for `p7501_pana_rsp_codec` and `p7501_piologic_codec`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_pana_logic.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "common/p7501_probe_fsm.h"
#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_dumpnor.h"
#include "uart/p7501_pana_jedec.h"
#include "flash/p7501_bus_policy.h"
#include "flash/p7501_fast_stream_codec.h"
#include "flash/p7501_flash.h"
#include "flash/p7501_ftl_inspect.h"
#include "generated/p7501_ssot.h"
#include "hal/p7501_wireless_codec.h"
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
    assert(g8.bgo_hw_base == 0x84130000UL && g8.bgo_hw_end == 0x84400000UL);
    assert(g8.bgo_sector_count == 45u && P7501_BGO_PARTITION_COUNT == 45u);

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

    /* IEEE 802.3 CRC32 standard test vector + 0xFF scanner + DUMPNOR ff_blocks tracker */
    assert(p7501_crc32_ieee((const uint8_t *)"123456789", 9u) == 0xCBF43926UL);
    uint8_t ff_block[264];
    memset(ff_block, 0xFF, sizeof(ff_block));
    assert(p7501_buf_is_all_ff(&ff_block[1], 256u));
    assert(p7501_buf_count_non_ff(&ff_block[1], 256u) == 0u);
    ff_block[17] = 0xA5u;
    assert(!p7501_buf_is_all_ff(&ff_block[1], 256u));
    assert(p7501_buf_count_non_ff(&ff_block[1], 256u) == 1u);

    p7501_dumpnor_ff_tracker_t fft;
    p7501_dumpnor_ff_tracker_init(&fft, 256u);
    for (size_t i = 0u; i < 12u; ++i) {
        p7501_dumpnor_ff_tracker_feed(&fft, (uint8_t)i);
    }
    for (size_t i = 0u; i < 256u; ++i) {
        p7501_dumpnor_ff_tracker_feed(&fft, 0xFFu);
    }
    for (size_t i = 0u; i < 4u; ++i) {
        p7501_dumpnor_ff_tracker_feed(&fft, 0x11u);
    }
    for (size_t i = 0u; i < 256u; ++i) {
        p7501_dumpnor_ff_tracker_feed(&fft, (i == 9u) ? 0x5Au : 0xFFu);
    }
    for (size_t i = 0u; i < 4u; ++i) {
        p7501_dumpnor_ff_tracker_feed(&fft, 0x22u);
    }
    assert(fft.total_blocks == 2u && fft.ff_blocks == 1u);
}

static size_t find_bytes_offset(const uint8_t *hay, size_t hlen, const void *needle, size_t nlen) {
    if ((hay == NULL) || (needle == NULL) || (nlen == 0u) || (hlen < nlen)) {
        return hlen;
    }
    for (size_t i = 0u; i <= (hlen - nlen); ++i) {
        if (memcmp(&hay[i], needle, nlen) == 0) {
            return i;
        }
    }
    return hlen;
}

static bool buf_contains_bytes(const uint8_t *hay, size_t hlen, const void *needle, size_t nlen) {
    return find_bytes_offset(hay, hlen, needle, nlen) < hlen;
}

static void test_pana_rsp_jedec_and_nor_policy(void) {
    p7501_flash_set_upper_nor_unlocked(false);
    p7501_pana_rsp_spec_t spec;

    /* Locked default blocks Upper NOR SA4..SA133 in WMEM/WRITE32/JEDEC_* while permitting NVRAM */
    assert(!p7501_panarsp_parse_args("JEDEC_ERASE:84200000", &spec));
    assert(!p7501_panarsp_parse_args("JEDEC_PROG:84200010,A55A", &spec));
    assert(p7501_panarsp_parse_args("JEDEC_ERASE:807FE000", &spec));
    assert(spec.action == PANA_RSP_ACT_JEDEC_ERASE && spec.addr == 0x807FE000UL);
    assert(p7501_panarsp_parse_args("JEDEC_PROG:847F0010,A55A", &spec));
    assert(spec.action == PANA_RSP_ACT_JEDEC_PROG && spec.data_len == 2u);

    /* Bad unlock token MUST be rejected without unlocking */
    assert(!p7501_panarsp_parse_args("NORUNLOCK:WRONG_TOKEN JEDEC_ERASE:84200000", &spec));
    assert(!p7501_flash_is_upper_nor_unlocked());

    /* Explicit unlock enables SA11..SA133 while SA0..SA10 stays permanently blocked & parser stays pure (F3) */
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR JEDEC_ERASE:84000000", &spec));
    assert(!p7501_flash_is_upper_nor_unlocked()); /* atomic rollback on failure */
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR WMEM:8003FFFC,11223344", &spec));
    assert(p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR JEDEC_ERASE:80200000", &spec));
    assert(!p7501_flash_is_upper_nor_unlocked()); /* parse itself is 100% pure */
    assert(spec.policy_req == P7501_POLICY_REQ_UNLOCK);
    p7501_panarsp_apply_policy(&spec);
    assert(p7501_flash_is_upper_nor_unlocked());
    assert(p7501_panarsp_parse_args("JEDEC_PROG:84200011,5A", &spec));
    assert(p7501_panarsp_parse_args("WMEM:84400000,11223344", &spec));
    assert(p7501_panarsp_parse_args("WRITE32:80200000,DEADBEEF", &spec));
    assert(p7501_panarsp_parse_args("NORLOCK", &spec));
    assert(spec.action == PANA_RSP_ACT_NOR_POLICY && !spec.nor_unlocked_effective);
    assert(spec.policy_req == P7501_POLICY_REQ_LOCK);
    p7501_panarsp_apply_policy(&spec);
    assert(!p7501_flash_is_upper_nor_unlocked());
    assert(!p7501_panarsp_parse_args("JEDEC_ERASE:84200000", &spec));

    /* Stub generation invariants: uncached bus literals + completion/timeout banners */
    uint8_t stub[P7501_JEDEC_MAX_STUB_SIZE];
    assert(p7501_jedec_build_erase_stub(0x80200000UL, P7501_JEDEC_STUB_ADDR, stub, sizeof(stub)) == 0u);
    p7501_flash_set_upper_nor_unlocked(true);
    const uint8_t unlock1_le[4] = { 0xAAu, 0x0Au, 0x00u, 0x84u };
    const size_t elen = p7501_jedec_build_erase_stub(0x80200000UL, P7501_JEDEC_STUB_ADDR, stub, sizeof(stub));
    assert(elen > 64u && elen <= P7501_JEDEC_MAX_STUB_SIZE);
    assert(buf_contains_bytes(stub, elen, unlock1_le, sizeof(unlock1_le)));
    assert(buf_contains_bytes(stub, elen, P7501_JEDEC_OK_TOKEN, sizeof(P7501_JEDEC_OK_TOKEN) - 1u));
    assert(buf_contains_bytes(stub, elen, P7501_JEDEC_TO_TOKEN, sizeof(P7501_JEDEC_TO_TOKEN) - 1u));

    const uint8_t prog_data[3] = { 0xA5u, 0x5Au, 0xF0u };
    const size_t plen = p7501_jedec_build_prog_stub(0x84200010UL, prog_data, sizeof(prog_data), P7501_JEDEC_STUB_ADDR, stub, sizeof(stub));
    assert(plen > 80u && plen <= P7501_JEDEC_MAX_STUB_SIZE);
    assert(buf_contains_bytes(stub, plen, unlock1_le, sizeof(unlock1_le)));
    assert(buf_contains_bytes(stub, plen, prog_data, sizeof(prog_data)));
    assert(buf_contains_bytes(stub, plen, P7501_JEDEC_OK_TOKEN, sizeof(P7501_JEDEC_OK_TOKEN) - 1u));

    /* Verify upgraded erase budget >= 0x01000000, DQ5 poll (0x20), and 0xFFFF readback verify */
    const uint8_t erase_budget_le[4] = { 0x00u, 0x00u, 0x00u, 0x01u };
    const uint8_t readback_ffff_le[4] = { 0xFFu, 0xFFu, 0x00u, 0x00u };
    const uint8_t dq5_btst[3] = { 0xF8u, 0xECu, 0x20u };
    assert(buf_contains_bytes(stub, elen > plen ? elen : plen, dq5_btst, sizeof(dq5_btst)));
    const size_t elen2 = p7501_jedec_build_erase_stub(0x80200000UL, P7501_JEDEC_STUB_ADDR, stub, sizeof(stub));
    assert(buf_contains_bytes(stub, elen2, erase_budget_le, sizeof(erase_budget_le)));
    assert(buf_contains_bytes(stub, elen2, dq5_btst, sizeof(dq5_btst)));
    assert(buf_contains_bytes(stub, elen2, readback_ffff_le, sizeof(readback_ffff_le)));
    p7501_flash_set_upper_nor_unlocked(false);

    /* Parameterized 16 MiB JEDEC bounds + 128 KiB NVRAM (0x84FE0000..0x84FFFFFF) */
    uint32_t norm16 = 0u;
    assert(p7501_jedec_validate_erase_addr_for_flash(P7501_FLASH_SIZE_16MB, 0x80FE0000UL, false, &norm16));
    assert(norm16 == 0x84FE0000UL);
    assert(!p7501_jedec_validate_erase_addr_for_flash(P7501_FLASH_SIZE_16MB, 0x847F0000UL, false, &norm16));
    assert(p7501_jedec_validate_prog_range_for_flash(P7501_FLASH_SIZE_16MB, 0x80FE0010UL, 4u, false, &norm16));
    assert(norm16 == 0x84FE0010UL);
    assert(p7501_jedec_build_erase_stub_for_flash(P7501_FLASH_SIZE_16MB, 0x80FE0000UL, P7501_JEDEC_STUB_ADDR, stub, sizeof(stub)) > 64u);
    assert(p7501_jedec_build_prog_stub_for_flash(P7501_FLASH_SIZE_16MB, 0x80FE0010UL, prog_data, sizeof(prog_data), P7501_JEDEC_STUB_ADDR, stub, sizeof(stub)) > 80u);

    /* One-shot NORUNLOCK:CONFIRM_UPPER_NOR auto-relock on write/erase completion */
    p7501_flash_set_upper_nor_unlocked(false);
    assert(p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR JEDEC_ERASE:80200000", &spec));
    p7501_panarsp_apply_policy(&spec);
    assert(spec.auto_relock_nor && p7501_flash_is_upper_nor_unlocked());
    assert(p7501_panarsp_finish_action(&spec));
    assert(!p7501_flash_is_upper_nor_unlocked());
    assert(!p7501_panarsp_parse_args("JEDEC_ERASE:84200000", &spec) && spec.rejected_e05);

    /* RAW:M / RAW:X and HEX: IPL Layer 1 (SA0..SA10) E05 guards */
    assert(!p7501_panarsp_parse_args("RAW:M80000000,4:11223344", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR RAW:M8403FFFE,4:11223344", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR RAW:X80000000,4:ABCD", &spec) && spec.rejected_e05);
    assert(!p7501_flash_is_upper_nor_unlocked());
    assert(p7501_panarsp_parse_args("RAW:M807F0000,4:11223344", &spec));
    assert(p7501_panarsp_parse_args("RAW:m80000000,4", &spec));
    /* Adversarial whitespace prefix, chained $M after $m, JUMP to lethal MMIO, and >8-digit len */
    assert(!p7501_panarsp_parse_args("HEX:204538303030303030300D", &spec) && spec.rejected_e05); /* " E80000000\r" */
    assert(!p7501_panarsp_parse_args("RAW:m80000000,4$MC000100C,4:00000000", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("RAW:JC000100C", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("RAW:RC0001000,000000010", &spec) && spec.rejected_e05);
    /* D1: Even with CONFIRM_UPPER_NOR, bare W/E/J or non-hex address in RAW:/HEX: must fail E05 */
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR RAW:E", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR RAW:W", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR HEX:45202438303030303030300D", &spec) && spec.rejected_e05); /* "E $80000000\r" */
    /* HEX: `$M80000000,4:00000000#00` & `$X84000000,4:ABCD#00` */
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR HEX:244D38303030303030302C343A3030303030303030233030", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("HEX:245838343030303030302C343A41424344233030", &spec) && spec.rejected_e05);
    /* HEX: `W 84200000,AA\r` blocked when locked, allowed once unlocked and auto-relocked on finish */
    assert(!p7501_panarsp_parse_args("HEX:572038343230303030302C41410D", &spec) && spec.rejected_e05);
    assert(p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR HEX:572038343230303030302C41410D", &spec));
    p7501_panarsp_apply_policy(&spec);
    assert(spec.auto_relock_nor && p7501_panarsp_finish_action(&spec));
    assert(!p7501_flash_is_upper_nor_unlocked());
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

    /* Emit CMD17 single-block read @ byte addr 0x00010000 (SDSC LBA 128) and CMD18 @ LBA 0. */
    sds_emit_cmd_frame(&st, true, 17u, 0x00010000u);
    sds_emit_cmd_frame(&st, false, 17u, 0x00000900u);
    sds_emit_cmd_frame(&st, true, 18u, 0x00000000u);
    assert(st.cmd17_count == 1u && st.cmd18_count == 1u);
    assert(st.trace_count >= 2u);
    assert(st.trace[st.trace_count - 2u].cmd == 17u && st.trace[st.trace_count - 2u].arg == 0x00010000u);
    assert(st.trace[st.trace_count - 1u].cmd == 18u && st.trace[st.trace_count - 1u].arg == 0u);
}

static void test_bus_policy_hardening_and_bypass_guards(void) {
    p7501_flash_set_upper_nor_unlocked(false);
    p7501_wireless_state_init(p7501_wireless_global_state());
    p7501_pana_rsp_spec_t spec;

    /* 1. 0xC000100C lethal register blocked for READ & WRITE across PANARSP (MEM/XMEM/WMEM/RAW/HEX) */
    assert(!p7501_panarsp_parse_args("MEM:C000100C,4", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("XMEM:C0001008,8", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("WMEM:C000100C,11223344", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("WRITE32:C000100C,00000000", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("RAW:mC000100C,4", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("RAW:MC000100C,4:00000000", &spec) && spec.rejected_e05);
    /* HEX: `$mC000100C,4#00` */
    assert(!p7501_panarsp_parse_args("HEX:246D43303030313030432C34233030", &spec) && spec.rejected_e05);

    /* 2. 9-digit hex overflow attack (`080000000` -> `0x80000000` wrap) rejected in hex scanner & PANARSP */
    uint32_t scanned = 0u;
    assert(p7501_scan_hex_u32("080000000", 9u, 0u, &scanned) == 0u);
    assert(p7501_scan_hex_u32("807F0000", 8u, 0u, &scanned) == 8u && scanned == 0x807F0000UL);
    assert(!p7501_panarsp_parse_args("RAW:M080000000,4:11223344", &spec) && spec.rejected_e05);

    /* 3. Lowercase 'w'/'e' PANAMON1 commands inside PANARSP HEX: blocked on protected NOR */
    /* `w 80000000,AA\r` & `e 84000000\r` */
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR HEX:772038303030303030302C41410D", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("NORUNLOCK:CONFIRM_UPPER_NOR HEX:652038343030303030300D", &spec) && spec.rejected_e05);

    /* 4. CS0 alias mirrors (`0x80800000`, `0x84800000`, 4 MiB `0x80400000`) & straddles blocked */
    const p7501_session_ctx_t *sctx = p7501_session_default_ctx();
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0x80800000UL, 4u, true, false, true) == P7501_BUS_DENY_CS0_MIRROR);
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0x84800000UL, 4u, true, false, true) == P7501_BUS_DENY_CS0_MIRROR);
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_4MB, 0x80400000UL, 4u, true, false, true) == P7501_BUS_DENY_CS0_MIRROR);
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0x807FFFFCUL, 8u, true, false, true) != P7501_BUS_ALLOW);
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0x8003F000UL, 1u, true, true, true) == P7501_BUS_DENY_IPL_LAYER1);
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0xC000100CUL, 4u, false, false, true) == P7501_BUS_DENY_LETHAL_MMIO);

    /* 5. F14 wireless lockout on PANARSP PASSTHROUGH / CALL & quota consumption in finish_action */
    p7501_wireless_auth_state_t *ws = p7501_wireless_global_state();
    ws->active_origin = P7501_ORIGIN_BLE_NUS;
    assert(!p7501_panarsp_parse_args("PASSTHROUGH", &spec) && spec.rejected_e05);
    assert(!p7501_panarsp_parse_args("CALL:50401F3C", &spec) && spec.rejected_e05);
    assert(p7501_wireless_handle_usb_auth_cmd(ws, P7501_ORIGIN_USB_CDC, "CONFIRM 1"));
    assert(p7501_panarsp_parse_args("JEDEC_ERASE:807FE000", &spec));
    (void)p7501_panarsp_finish_action(&spec);
    assert(!p7501_wireless_nor_write_allowed(ws));
    p7501_wireless_state_init(ws);

    /* 6. JEDEC lookup requires full dev_cycles (3-cycle S29JL064H must not match on 1 cycle) */
    const uint16_t partial_dev[3] = { 0x227Eu, 0x0000u, 0x0000u };
    const uint16_t full_dev[3] = { 0x227Eu, 0x2202u, 0x2201u };
    assert(p7501_flash_find_chip_by_jedec(0x0001u, partial_dev, 1u) == NULL);
    assert(p7501_flash_find_chip_by_jedec(0x0001u, full_dev, 3u) != NULL);

    /* 7. BGO_BITCLEAR 1->0 allowed, 0->1 rejected */
    const uint8_t old_b[2] = { 0xFFu, 0x0Fu };
    const uint8_t ok_b[2]  = { 0xAAu, 0x05u };
    const uint8_t bad_b[2] = { 0xFFu, 0x1Fu };
    assert(p7501_jedec_validate_bitclear(old_b, ok_b, 2u));
    assert(!p7501_jedec_validate_bitclear(old_b, bad_b, 2u));
}

static void test_fast_stream_codec_and_normap(void) {
    assert(p7501_fast_stub_crc32() == P7501_FAST_STUB_CRC32);
    const uint8_t sample[4] = { 0x10u, 0x20u, 0x30u, 0xA0u };
    assert(p7501_fast_sum8(sample, sizeof(sample)) == 0x00u);

    const p7501_session_ctx_t *sctx = p7501_session_default_ctx();
    const char *reason = NULL;
    /* Permanent E05 on Layer 1 IPL SA0..SA10 even with upper_nor_unlocked=true */
    assert(p7501_fast_check_write_guard(sctx, 0x84000000UL, 4u, true, &reason) == P7501_BUS_DENY_IPL_LAYER1);
    assert(p7501_fast_check_write_guard(sctx, 0x8003FFF0UL, 4u, true, &reason) == P7501_BUS_DENY_IPL_LAYER1);
    /* Blocked when locked, rejected when unaligned/outside BGO, allowed inside BGO when unlocked */
    assert(p7501_fast_check_write_guard(sctx, 0x84160000UL, 4u, false, &reason) == P7501_BUS_DENY_UPPER_LOCKED);
    assert(p7501_fast_check_write_guard(sctx, 0x84160001UL, 4u, true, &reason) != P7501_BUS_ALLOW);
    assert(p7501_fast_check_write_guard(sctx, 0x84160000UL, 3u, true, &reason) != P7501_BUS_ALLOW);
    assert(p7501_fast_check_write_guard(sctx, 0x84400000UL, 4u, true, &reason) != P7501_BUS_ALLOW);
    assert(p7501_fast_check_write_guard(sctx, 0x84160000UL, 4u, true, &reason) == P7501_BUS_ALLOW);

    /* FAST_ERASE guard: blocks IPL SA0..SA10 even unlocked, requires 64 KiB align + CONFIRM_UPPER_NOR */
    assert(p7501_fast_check_erase_guard(sctx, 0x84000000UL, true, &reason) == P7501_BUS_DENY_IPL_LAYER1);
    assert(p7501_fast_check_erase_guard(sctx, 0x84130000UL, false, &reason) == P7501_BUS_DENY_UPPER_LOCKED);
    assert(p7501_fast_check_erase_guard(sctx, 0x84130100UL, true, &reason) == P7501_BUS_DENY_PROTECTED_RO);
    assert(p7501_fast_check_erase_guard(sctx, 0x84400000UL, true, &reason) == P7501_BUS_DENY_PROTECTED_RO);
    assert(p7501_fast_check_erase_guard(sctx, 0x84130000UL, true, &reason) == P7501_BUS_ALLOW);

    /* On-Pico pre-flight 1->0 monotonicity check */
    const uint8_t cur_nor[4] = { 0xFFu, 0xF0u, 0x55u, 0x00u };
    const uint8_t ok_nor[4]  = { 0xAAu, 0xA0u, 0x51u, 0x00u };
    const uint8_t bad_nor[4] = { 0xAAu, 0xF1u, 0x55u, 0x00u };
    size_t bad_off = 99u;
    assert(p7501_fast_check_monotonic_1_to_0(cur_nor, ok_nor, 4u, &bad_off));
    assert(!p7501_fast_check_monotonic_1_to_0(cur_nor, bad_nor, 4u, &bad_off) && (bad_off == 1u));

    /* @P7501_PROG structured push line */
    char prog[128];
    assert(p7501_fast_format_prog_line(prog, sizeof(prog), "READ", 29u, 2048u, 65536u, 0x1234ABCDUL, 3270u) > 20u);
    assert(strstr(prog, "@P7501_PROG phase=READ sa=29 done=2048 total=65536 crc32=0x1234ABCD vdd_mv=3270") != NULL);

    /* 142-sector NOR_MAP span & classification */
    uint32_t va = 0u;
    uint32_t sz = 0u;
    assert(p7501_normap_sector_span(0u, &va, &sz) && (va == 0x84000000UL) && (sz == 0x2000UL));
    assert(p7501_normap_sector_span(26u, &va, &sz) && (va == 0x84130000UL) && (sz == 0x10000UL));
    assert(p7501_normap_sector_span(134u, &va, &sz) && (va == 0x847F0000UL) && (sz == 0x2000UL));
    assert(!p7501_normap_sector_span(142u, &va, &sz));

    uint8_t ff20[20];
    uint8_t z16[16];
    uint8_t bgo_p1[20];
    uint8_t bgo_p2[20];
    uint8_t fe_hdr[20];
    uint8_t ee_act[20];
    uint8_t ee_sp[20];
    memset(ff20, 0xFF, sizeof(ff20));
    memset(z16, 0x00, sizeof(z16));
    memset(bgo_p1, 0xFF, sizeof(bgo_p1));
    memset(bgo_p2, 0xFF, sizeof(bgo_p2));
    for (size_t off = 4u; off <= 16u; off += 4u) {
        bgo_p1[off] = 0x01u; bgo_p1[off + 1u] = 0xF1u;
        bgo_p2[off] = 0x01u; bgo_p2[off + 1u] = 0xF2u;
    }
    memcpy(fe_hdr, "EXTRHEADDRV \x01\x00\x00\x00\x00\x00\x16\x00", 20u);
    memcpy(ee_act, "\x0F\xF0\x00\x00\x00\x00" "1284\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", 20u);
    memcpy(ee_sp,  "\x0F\xF0\x00\x00\x00\x2D\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF", 20u);

    p7501_normap_summary_t sum;
    memset(&sum, 0, sizeof(sum));
    for (uint16_t sa = 0u; sa < P7501_NORMAP_SECTOR_COUNT; ++sa) {
        p7501_normap_kind_t k;
        if (sa == 15u) {
            k = p7501_normap_classify_sector(sa, fe_hdr, NULL);
        } else if (sa == 29u) {
            k = p7501_normap_classify_sector(sa, bgo_p1, z16);
        } else if ((sa >= 26u) && (sa <= 70u)) {
            k = (sa == 41u || sa == 67u || sa == 68u || sa == 70u)
                ? p7501_normap_classify_sector(sa, ff20, z16)
                : p7501_normap_classify_sector(sa, bgo_p2, z16);
        } else if ((sa >= 71u) && (sa <= 77u)) {
            k = p7501_normap_classify_sector(sa, ee_act, NULL);
        } else if ((sa >= 78u) && (sa <= 133u)) {
            k = p7501_normap_classify_sector(sa, ee_sp, NULL);
        } else {
            k = p7501_normap_classify_sector(sa, ff20, NULL);
        }
        p7501_normap_accumulate(&sum, k);
    }
    p7501_normap_finalize(&sum);
    assert(sum.healthy && sum.fe_extrhead_ok && (sum.bgo_p1 == 1u) && (sum.bgo_p2 == 40u));
    assert((sum.bgo_spares == 4u) && (sum.eemul_active_1284 == 7u) && (sum.nvram_blank == 8u));

    /* Exact Spansion S29JL064H VA -> SA mapper */
    assert(p7501_fast_va_to_sa(0x84000000UL) == 0u);
    assert(p7501_fast_va_to_sa(0x8000E000UL) == 7u);
    assert(p7501_fast_va_to_sa(0x84130000UL) == 26u);
    assert(p7501_fast_va_to_sa(0x807F0000UL) == 134u);
    assert(p7501_fast_va_to_sa(0x847FE000UL) == 141u);
    assert(p7501_fast_va_to_sa(0x90000000UL) == 0xFFFFu);

    /* Pure 'R' & 'W' packet encoders */
    uint8_t pkt[7];
    assert(p7501_fast_encode_read_pkt(0x84130000UL, 16u, pkt) == 7u);
    assert((pkt[0] == (uint8_t)'R') && (pkt[1] == 0x00u) && (pkt[2] == 0x00u) &&
           (pkt[3] == 0x13u) && (pkt[4] == 0x84u) && (pkt[5] == 16u) && (pkt[6] == 0u));
    assert(p7501_fast_encode_write_hdr(0x847F2004UL, 512u, pkt) == 7u);
    assert((pkt[0] == (uint8_t)'W') && (pkt[1] == 0x04u) && (pkt[2] == 0x20u) &&
           (pkt[3] == 0x7Fu) && (pkt[4] == 0x84u) && (pkt[5] == 0x00u) && (pkt[6] == 0x02u));

    /* 256-byte page window math */
    p7501_fast_page_window_t win;
    assert(p7501_fast_compute_page_window(0x84130010UL, 500u, &win));
    assert((win.page_base == 0x84130000UL) && (win.rel_off == 0x10u) && (win.span_pages == 3u));
    assert(!p7501_fast_compute_page_window(0x84130000UL, 0u, &win));

    /* NOR_MAP formatter & hex serializer */
    sum.ipl_crc32 = 0x1234ABCDUL;
    char normap_line[256];
    assert(p7501_fast_format_normap_line(normap_line, sizeof(normap_line), &sum) > 40u);
    assert(strstr(normap_line, "[NOR_MAP] sectors=142 ipl_crc32=0x1234ABCD fe_extrhead=YES") != NULL);
    assert(strstr(normap_line, "bgo_p1=1 bgo_p2=40 bgo_spares=4 bgo_corrupt=0") != NULL);
    assert(strstr(normap_line, "nvram_blank=8 healthy=YES") != NULL);

    const uint8_t sample_bytes[3] = { 0x0Au, 0xFEu, 0x55u };
    char sample_hex[16];
    assert(p7501_fast_format_page_hex(sample_bytes, sizeof(sample_bytes), sample_hex, sizeof(sample_hex)) == 6u);
    assert(strcmp(sample_hex, "0afe55") == 0);

    /* Pure chip capability descriptors */
    assert(strstr(p7501_describe_mcu_host(false, false), "RP2040") != NULL);
    assert(strstr(p7501_describe_mcu_host(false, true), "BLE") != NULL);
    assert(strstr(p7501_describe_mcu_host(true, false), "RP2350") != NULL);
    assert(strstr(p7501_describe_gdb_target(false, false), "RP2040") != NULL);
    assert(strstr(p7501_describe_gdb_target(true, false), "Cortex-M33") != NULL);
    assert(strstr(p7501_describe_gdb_target(true, true), "Hazard3") != NULL);
}

static void test_jedec_autoselect_and_cfi_stubs(void) {
    uint8_t stub_halt[P7501_JEDEC_MAX_STUB_SIZE];
    uint8_t stub_ret[P7501_JEDEC_MAX_STUB_SIZE];
    const size_t len_halt = p7501_jedec_build_autoselect_stub(
        true, 0x51800000UL, stub_halt, sizeof(stub_halt)
    );
    const size_t len_ret = p7501_jedec_build_autoselect_stub_ex(
        true, true, 0x9043C000UL, stub_ret, sizeof(stub_ret)
    );
    assert((len_halt > 100u) && (len_ret == (len_halt + 10u)) && (len_ret <= P7501_JEDEC_MAX_STUB_SIZE));
    assert((stub_ret[0] == 0xCFu) && (stub_ret[1] == 0xF0u) &&
           (stub_ret[2] == 0xF8u) && (stub_ret[3] == 0xFEu) && (stub_ret[4] == 0xFCu));

    static const uint8_t k_emit_hw[6] = { 0xF8u, 0xFEu, 0xFCu, 0xF0u, 0x68u, 0x8Bu };
    const size_t halt_hw_off = find_bytes_offset(stub_halt, len_halt, k_emit_hw, sizeof(k_emit_hw));
    const size_t ret_hw_off = find_bytes_offset(stub_ret, len_ret, k_emit_hw, sizeof(k_emit_hw));
    assert((halt_hw_off >= 2u) && (halt_hw_off < len_halt));
    assert((stub_halt[halt_hw_off - 2u] == 0xCAu) && (stub_halt[halt_hw_off - 1u] == 0x00u));
    assert((ret_hw_off >= 7u) && (ret_hw_off < len_ret));
    assert((stub_ret[ret_hw_off - 7u] == 0xF8u) && (stub_ret[ret_hw_off - 6u] == 0xFEu) &&
           (stub_ret[ret_hw_off - 5u] == 0x04u) && (stub_ret[ret_hw_off - 4u] == 0xCEu) &&
           (stub_ret[ret_hw_off - 3u] == 0xF0u) && (stub_ret[ret_hw_off - 2u] == 0xF0u) &&
           (stub_ret[ret_hw_off - 1u] == 0xFCu));
    static const uint8_t k_span_hdr[6] = { 0xF8u, 0xFEu, 0xFCu, 0xF0u, 0x40u, 0x83u };
    static const uint8_t k_span_tail[11] = {
        0x20u, 0x01u, 0xB4u, 0xC9u, 0xF5u, 0xF8u, 0xFEu, 0x04u, 0xDFu, 0x00u, 0x00u
    };
    assert(buf_contains_bytes(stub_ret, len_ret, k_span_hdr, sizeof(k_span_hdr)) &&
           buf_contains_bytes(stub_ret, len_ret, k_span_tail, sizeof(k_span_tail)));
    assert(buf_contains_bytes(stub_halt, len_halt, P7501_CFI_QUERY_MAGIC, 8u) &&
           buf_contains_bytes(stub_halt, len_halt, "PANADONE", 8u));
    assert(buf_contains_bytes(stub_ret, len_ret, P7501_CFI_QUERY_MAGIC, 8u) &&
           buf_contains_bytes(stub_ret, len_ret, "PANADONE", 8u));

    static const uint8_t k_cfi_reply[] =
        "J 9043C000\r\nPANACFI1\x51\x00\x52\x00\x59\x00\x17\x00PANADONE\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n";
    p7501_jedec_id_reply_t rep;
    assert(p7501_jedec_parse_autoselect_reply(k_cfi_reply, sizeof(k_cfi_reply) - 1u, true, &rep));
    assert(rep.valid && (rep.qry[0] == 'Q') && (rep.qry[1] == 'R') && (rep.qry[2] == 'Y') &&
           (rep.size_bytes == 8388608UL));

    const size_t len_jid = p7501_jedec_build_autoselect_stub_ex(
        false, true, 0x9043C000UL, stub_ret, sizeof(stub_ret)
    );
    assert((len_jid > 100u) && (len_jid <= P7501_JEDEC_MAX_STUB_SIZE) &&
           buf_contains_bytes(stub_ret, len_jid, P7501_JEDEC_ID_MAGIC, 8u));
    static const uint8_t k_jid_reply[] =
        "PANAJID1\x01\x00\x7E\x22\x02\x22\x01\x22PANADONE\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n";
    assert(p7501_jedec_parse_autoselect_reply(k_jid_reply, sizeof(k_jid_reply) - 1u, false, &rep));
    assert(rep.valid && (rep.mfg == 0x0001u) && (rep.size_bytes == 8388608UL));
}

static uint64_t next_xorshift64(uint64_t *state) {
    uint64_t x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    return x * 0x2545F4914F6CDD1DULL;
}

static p7501_bridge_filter_action_t feed_slice_through_bridge_filter(
    const char *s,
    p7501_bus_deny_t *out_reason
) {
    p7501_bridge_tx_filter_t filt;
    p7501_bridge_tx_filter_init(&filt);
    uint8_t out_buf[P7501_BRIDGE_RSP_CAP];
    size_t out_len = 0u;
    p7501_bridge_filter_action_t last = P7501_BRIDGE_ACT_HOLD;
    const size_t slen = strlen(s);
    for (size_t i = 0u; i < slen; ++i) {
        last = p7501_bridge_tx_filter_feed(
            &filt, (uint8_t)s[i], out_buf, sizeof(out_buf), &out_len, out_reason
        );
        if ((i + 1u) < slen) {
            assert(last == P7501_BRIDGE_ACT_HOLD);
        }
    }
    return last;
}

static void test_probe_fsm_exhaustive(void) {
    const p7501_rail_snap_t snaps[4] = {
        { .vdd_mv = 3240u, .dropouts = 0u, .acked_dropouts = 0u, .nsw5v_high = true, .jig_strapped_low = false },
        { .vdd_mv = 0u, .dropouts = 0u, .acked_dropouts = 0u, .nsw5v_high = false, .jig_strapped_low = false },
        { .vdd_mv = 3240u, .dropouts = 2u, .acked_dropouts = 1u, .nsw5v_high = true, .jig_strapped_low = false },
        { .vdd_mv = 0u, .dropouts = 0u, .acked_dropouts = 0u, .nsw5v_high = true, .jig_strapped_low = true },
    };
    const uint32_t iso_all =
        P7501_EFF_HIZ_ALL | P7501_EFF_PIO_END | P7501_EFF_KEEPALIVE_OFF | P7501_EFF_UPPER_NOR_LOCK;

    for (unsigned st = 0u; st < (unsigned)P7501_PS_COUNT; ++st) {
        for (unsigned ev = 0u; ev < (unsigned)P7501_EV_COUNT; ++ev) {
            for (unsigned si = 0u; si < 4u; ++si) {
                const p7501_step_res_t r =
                    p7501_probe_step((p7501_ps_t)st, (p7501_ev_t)ev, &snaps[si]);
                assert((unsigned)r.next < (unsigned)P7501_PS_COUNT);
                if (si == 2u) {
                    if (ev == (unsigned)P7501_EV_FAULT_ACK) {
                        assert(r.allowed && (r.next == P7501_PS_ISOLATED) &&
                               ((r.effects & P7501_EFF_ACK_FAULT) != 0u));
                    } else {
                        assert((r.next == P7501_PS_FAULT) && ((r.effects & iso_all) == iso_all));
                    }
                } else if (si == 3u) {
                    if (ev == (unsigned)P7501_EV_FAULT_ACK) {
                        assert(!r.allowed);
                    } else if ((ev == (unsigned)P7501_EV_ENGAGE) ||
                               (ev == (unsigned)P7501_EV_BRIDGE_ENTER) ||
                               (ev == (unsigned)P7501_EV_GDB_ENTER) ||
                               (ev == (unsigned)P7501_EV_OP_BEGIN)) {
                        assert(r.allowed);
                    }
                } else if (si == 1u) {
                    if ((ev == (unsigned)P7501_EV_ENGAGE) ||
                        (ev == (unsigned)P7501_EV_BRIDGE_ENTER) ||
                        (ev == (unsigned)P7501_EV_GDB_ENTER) ||
                        (ev == (unsigned)P7501_EV_OP_BEGIN) ||
                        (ev == (unsigned)P7501_EV_FAULT_ACK)) {
                        assert(!r.allowed && ((r.effects & iso_all) == iso_all));
                    }
                }
            }
        }
    }
}

static void test_wrap_and_protocol_bypass_matrix(void) {
    p7501_flash_set_upper_nor_unlocked(false);
    const p7501_session_ctx_t *sctx = p7501_session_default_ctx();
    p7501_bus_deny_t deny = P7501_BUS_ALLOW;

    /* 1. Exact u32 wraparound boundaries and lethal MMIO priority */
    assert(!p7501_bus_span_wraps_u32(0xFFFFFFFFUL, 1u));
    assert(p7501_bus_span_wraps_u32(0xFFFFFFFFUL, 2u));
    assert(p7501_bus_span_wraps_u32(0x80000000UL, 0x80000001UL));
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0xFFFFFFFFUL, 2u, false, false, true) ==
           P7501_BUS_DENY_CS0_MIRROR);
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0xC000100CUL, 0xFFFFFFFFu, false, false, true) ==
           P7501_BUS_DENY_LETHAL_MMIO);
    assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0xD0000000UL, 0xF000100Du, false, false, true) ==
           P7501_BUS_DENY_LETHAL_MMIO);

    /* 2. A01 UART TX / BRIDGE-POLICY bypass vectors */
    static const char *const k_blocked[] = {
        "$m080000000,4#00",
        "$M080000000,4:11223344#00",
        "$vFlashWrite:80000000:AABB#00",
        "$vFlashErase:8003E000,2000#00",
        "MW 00000000C000100C 0\r",
        "W 80000000 11\r",
        "E 80000000\r",
        "w 80000000,1\r",
        "e 84000000\r",
        "MD C000100C\r",
    };
    for (size_t i = 0u; i < (sizeof(k_blocked) / sizeof(k_blocked[0])); ++i) {
        deny = P7501_BUS_ALLOW;
        assert(feed_slice_through_bridge_filter(k_blocked[i], &deny) == P7501_BRIDGE_ACT_BLOCKED);
        assert(deny != P7501_BUS_ALLOW);
    }

    assert(feed_slice_through_bridge_filter("MD 80000000\r", &deny) == P7501_BRIDGE_ACT_FORWARD);
    assert(feed_slice_through_bridge_filter("MW 807F0000 00\r", &deny) == P7501_BRIDGE_ACT_FORWARD);

    /* Fail-closed overflow (>255B line) and PASS_CTL */
    p7501_bridge_tx_filter_t filt;
    p7501_bridge_tx_filter_init(&filt);
    uint8_t out[P7501_BRIDGE_RSP_CAP];
    size_t out_len = 0u;
    assert(p7501_bridge_tx_filter_feed(&filt, 0x03u, out, sizeof(out), &out_len, &deny) == P7501_BRIDGE_ACT_PASS_CTL);
    for (size_t i = 0u; i < 260u; ++i) {
        assert(p7501_bridge_tx_filter_feed(&filt, (uint8_t)'A', out, sizeof(out), &out_len, &deny) == P7501_BRIDGE_ACT_HOLD);
    }
    assert(p7501_bridge_tx_filter_feed(&filt, (uint8_t)'\r', out, sizeof(out), &out_len, &deny) == P7501_BRIDGE_ACT_BLOCKED);

    /* 3. 10,000-vector pseudo-random differential check on bus policy */
    uint64_t rng = 0x9E3779B97F4A7C15ULL;
    for (uint32_t i = 0u; i < 10000u; ++i) {
        const uint32_t rnd_a = (uint32_t)(next_xorshift64(&rng) & 0xFFFFFFFFULL);
        const size_t rnd_l = (size_t)((next_xorshift64(&rng) % 0x20000u) + 1u);
        const bool wr = (i & 1u) != 0u;
        const p7501_bus_deny_t res =
            p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, rnd_a, rnd_l, wr, false, false);
        if (p7501_bus_is_lethal_mmio_range(rnd_a, rnd_l)) {
            assert(res == P7501_BUS_DENY_LETHAL_MMIO);
        } else if (p7501_bus_span_wraps_u32(rnd_a, rnd_l)) {
            assert(res == P7501_BUS_DENY_CS0_MIRROR);
        }
        /* Cached <-> uncached equivalence across CS0 primary 8 MiB */
        const uint32_t off = (uint32_t)(next_xorshift64(&rng) & 0x007FFF00ULL);
        const size_t span = (size_t)((next_xorshift64(&rng) % 256u) + 1u);
        assert(p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0x80000000UL + off, span, true, false, false) ==
               p7501_bus_check_access(sctx, P7501_FLASH_SIZE_8MB, 0x84000000UL + off, span, true, false, false));
    }
}

static void test_cmd_arena_invariants(void) {
    assert(sizeof(g_p7501_cmd_arena) == P7501_CMD_ARENA_BYTES);
    assert(p7501_arena_current_owner() == P7501_ARENA_FREE);
    assert(!p7501_arena_try_acquire(P7501_ARENA_FREE));
    assert(p7501_arena_try_acquire(P7501_ARENA_OWNER_FLASH));
    assert(!p7501_arena_try_acquire(P7501_ARENA_OWNER_DIAG));
    p7501_arena_release(P7501_ARENA_OWNER_DIAG);
    assert(p7501_arena_current_owner() == P7501_ARENA_OWNER_FLASH);
    p7501_arena_release(P7501_ARENA_OWNER_FLASH);
    assert(p7501_arena_current_owner() == P7501_ARENA_FREE);
}

static void test_ftl_bgo_norfrom_inspect(void) {
    uint8_t blk[P7501_SSOT_NOR_AUTOBOOT_HDR_BYTES];
    memset(blk, 0xFF, sizeof(blk));
    uint32_t crc = 0u;
    assert(p7501_norfrom_classify_prefix(blk, sizeof(blk), &crc) == P7501_NORFROM_ERASED_FF);
    assert(strcmp(p7501_norfrom_state_name(P7501_NORFROM_ERASED_FF), "ERASED_FF") == 0);
    assert(crc != 0u);

    /* Non-FF header with canonical "NAUT" (0x5455414E) stamped at +0xFC */
    blk[0] = 0xE0u;
    blk[P7501_SSOT_NOR_AUTOBOOT_MAGIC_OFF + 0u] = 0x4Eu;
    blk[P7501_SSOT_NOR_AUTOBOOT_MAGIC_OFF + 1u] = 0x41u;
    blk[P7501_SSOT_NOR_AUTOBOOT_MAGIC_OFF + 2u] = 0x55u;
    blk[P7501_SSOT_NOR_AUTOBOOT_MAGIC_OFF + 3u] = 0x54u;
    assert(p7501_norfrom_classify_prefix(blk, sizeof(blk), &crc) == P7501_NORFROM_AUTOBOOT_CAPSULE);
    assert(strcmp(p7501_norfrom_state_name(P7501_NORFROM_AUTOBOOT_CAPSULE), "NOR_AUTOBOOT") == 0);

    /* Putting "NAUT" at +0x00 with 0xFF at +0xFC must NOT trigger capsule detection */
    memset(blk, 0xFF, sizeof(blk));
    memcpy(blk, "NAUT", 4u);
    assert(p7501_norfrom_classify_prefix(blk, sizeof(blk), &crc) == P7501_NORFROM_NON_EMPTY);

    memcpy(blk, "EXTRHEAD", 8u);
    assert(p7501_norfrom_classify_prefix(blk, sizeof(blk), &crc) == P7501_NORFROM_EXTRHEAD);
    blk[0] = 0x12u;
    assert(p7501_norfrom_classify_prefix(blk, sizeof(blk), &crc) == P7501_NORFROM_NON_EMPTY);

    uint8_t hdrs[45u * P7501_BGO_HEADER_BYTES];
    memset(hdrs, 0xFF, sizeof(hdrs));
    uint16_t next_p2 = 0xF201u;
    for (size_t i = 0u; i < 45u; ++i) {
        if (i >= 41u) {
            continue; /* 4 erased spares */
        }
        const uint16_t lbn = (i == 3u) ? 0xF101u : next_p2++;
        uint8_t *const h = &hdrs[i * P7501_BGO_HEADER_BYTES];
        memset(h, 0x00, 4u);
        for (size_t w = 4u; w < 20u; w += 4u) {
            h[w] = (uint8_t)(lbn & 0xFFu);
            h[w + 1u] = (uint8_t)(lbn >> 8u);
            h[w + 2u] = 0xFFu;
            h[w + 3u] = 0xFFu;
        }
    }
    p7501_bgo_report_t rep;
    assert(p7501_bgo_inspect_headers(hdrs, 45u, &rep));
    assert(rep.healthy_bgo_ftl);
    assert((rep.valid_count == 41u) && (rep.erased_count == 4u) && (rep.p1_sa_index == 29u));
}

int p7501_run_pana_logic_tests(void) {
    test_pana_rsp_frame_codec();
    test_rsp_boot_prose_does_not_trip_nak();
    test_pana_rsp_arg_parser();
    test_piologic_analyzer();
    test_flash_geometry_and_bgo();
    test_pana_dumpnor_stub_and_args();
    test_jedec_autoselect_and_cfi_stubs();
    test_pana_rsp_jedec_and_nor_policy();
    test_bus_policy_hardening_and_bypass_guards();
    test_probe_fsm_exhaustive();
    test_wrap_and_protocol_bypass_matrix();
    test_cmd_arena_invariants();
    test_fast_stream_codec_and_normap();
    test_ftl_bgo_norfrom_inspect();
    p7501_run_pana_diag_drv_tests();
    test_sds_bus_sniffer_codec();
    printf("[HOST-TEST] PANARSP + BusPolicy + ProbeFSM + Arena + FastStream + NOR_MAP + REGS + DUMPNOR + DIAGWATCH + DRVPANA + FlashGeo + PIOLOGIC + SDSNIFF passed.\n");
    return 0;
}
