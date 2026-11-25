/**
 * @file test_pico_autonomous.c
 * @brief Host C11 tests covering push ring, Feistel, LZSS/zlib, traps, payloads, sector surgery,
 *        debounced hotplug FSM, 27-register SSOT MMIO decoding, and `@PICO_DIAG_DEEP`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_pico_autonomous.h"

#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "crypto/picofw_decomp.h"
#include "crypto/picofw_feistel.h"
#include "diag/picofw_boottrace_codec.h"
#include "diag/picofw_hdd_unlock.h"
#include "diag/picofw_hotplug_codec.h"
#include "diag/picofw_math_gps.h"
#include "diag/picofw_meihfs_probe.h"
#include "diag/picofw_microsurgery.h"
#include "diag/picofw_mmio_snapshot_codec.h"
#include "diag/picofw_pieos_xray.h"
#include "flash/picofw_fast_stream_codec.h"
#include "flash/picofw_flash.h"
#include "flash/picofw_payloads.h"
#include "flash/picofw_sector_ops.h"
#include "generated/picofw_ssot.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

typedef struct mock_push_sink_ctx {
    uint8_t captured[8192];
    size_t len;
    uint32_t calls;
} mock_push_sink_ctx_t;

static void mock_push_sink(const uint8_t *data, size_t len, void *raw_ctx) {
    mock_push_sink_ctx_t *const ctx = (mock_push_sink_ctx_t *)raw_ctx;
    ctx->calls += 1u;
    if ((ctx->len + len) <= sizeof(ctx->captured)) {
        memcpy(&ctx->captured[ctx->len], data, len);
        ctx->len += len;
    }
}

static void test_push_stream_ring_and_watermark(void) {
    static picofw_push_bus_t bus;
    static mock_push_sink_ctx_t sink_ctx;
    memset(&sink_ctx, 0, sizeof(sink_ctx));
    picofw_push_init(&bus, mock_push_sink, &sink_ctx);

    char payload[400];
    memset(payload, 'A', 399u);
    payload[399] = '\0';

    /* Each frame is ~455 B -> 6 frames (< 3072 B) stay buffered; 7th triggers auto-flush */
    for (size_t i = 0u; i < 6u; ++i) {
        assert(picofw_push_stage_frame(&bus, "ELEC", 1000ULL + i, payload, 399u));
    }
    assert(sink_ctx.calls == 0u);
    assert(bus.stats.buffered_bytes > 0u);

    assert(picofw_push_stage_frame(&bus, "ELEC", 2000ULL, payload, 399u));
    assert(sink_ctx.calls >= 1u);
    assert(strstr((const char *)sink_ctx.captured, "@PICO_FRAME seq=1 ts_us=1000 type=ELEC len=399 crc32=") != NULL);

    /* Explicit flush clears remaining buffered data */
    assert(picofw_push_stage_frame(&bus, "TAIL", 3000ULL, "ok=1\n", 5u));
    assert(picofw_push_flush(&bus));
    assert(bus.stats.buffered_bytes == 0u);
    assert(bus.stats.frames_staged == 8u);
    assert(bus.stats.drops == 0u);

    /* Oversized single frame drops cleanly without corrupting ring */
    memset(g_picofw_cmd_arena.raw, 'B', PICOFW_PUSH_RING_CAP + 8u);
    assert(!picofw_push_stage_frame(
        &bus, "BIG", 4000ULL, (const char *)g_picofw_cmd_arena.raw, PICOFW_PUSH_RING_CAP + 1u
    ));
    assert(bus.stats.drops == 1u);
    printf("  [PASS] test_push_stream_ring_and_watermark\n");
}

static void test_feistel_and_decomp_engines(void) {
    assert(picofw_feistel_selftest());

    /* Edge encrypt + decrypt on 32-byte buffer with 2013/hualu key 7 */
    uint8_t rk[32];
    picofw_feistel_derive_rk_8b(k_picofw_ssot_feistel_keys[7].key8, rk);
    uint8_t edge_buf[32];
    for (size_t i = 0u; i < sizeof(edge_buf); ++i) {
        edge_buf[i] = (uint8_t)(i * 7u);
    }
    picofw_feistel_crypt_edges(edge_buf, sizeof(edge_buf), 8u, rk, true);
    picofw_feistel_crypt_edges(edge_buf, sizeof(edge_buf), 8u, rk, false);
    for (size_t i = 0u; i < sizeof(edge_buf); ++i) {
        assert(edge_buf[i] == (uint8_t)(i * 7u));
    }

    assert(picofw_adler32_rfc1950((const uint8_t *)"PANADIGA", 8u) == 0x0A220236UL);
    assert(picofw_adler32_strided((const uint8_t *)"PANADIGA", 8u, 2u) != 0u);

    bool lz_ok = false;
    bool zl_ok = false;
    assert(picofw_decomp_selftest(g_picofw_cmd_arena.raw, &lz_ok, &zl_ok) && lz_ok && zl_ok);

    /* Corrupt Adler-32 rejection, raw DEFLATE stored, and LZSS backreference "PANPAN" */
    static const uint8_t k_bad_zlib[19] = {
        0x78u, 0x01u, 0x01u, 0x08u, 0x00u, 0xF7u, 0xFFu,
        'P', 'A', 'N', 'A', 'D', 'I', 'G', 'A',
        0x0Au, 0x22u, 0x02u, 0x37u
    };
    picofw_inflate_scratch_t *const scratch = (picofw_inflate_scratch_t *)(void *)g_picofw_cmd_arena.raw;
    uint8_t out_buf[64];
    size_t out_len = 0u;
    assert(!picofw_zlib_decompress(k_bad_zlib, sizeof(k_bad_zlib), out_buf, sizeof(out_buf), &out_len, scratch));

    static const uint8_t k_raw_vec[9] = {
        0x01u, 0x04u, 0x00u, 0xFBu, 0xFFu, 'T', 'E', 'S', 'T'
    };
    assert(picofw_inflate_raw(k_raw_vec, sizeof(k_raw_vec), out_buf, sizeof(out_buf), &out_len, scratch));
    assert((out_len == 4u) && (memcmp(out_buf, "TEST", 4u) == 0));

    static const uint8_t k_lzss_vec[6] = { 0x07u, 'P', 'A', 'N', 0xEEu, 0xF0u };
    uint8_t *const win4k = g_picofw_cmd_arena.raw;
    assert(picofw_lzss_decompress(k_lzss_vec, sizeof(k_lzss_vec), out_buf, sizeof(out_buf), &out_len, win4k));
    assert((out_len == 6u) && (memcmp(out_buf, "PANPAN", 6u) == 0));
    printf("  [PASS] test_feistel_and_decomp_engines\n");
}

static void test_traps_payloads_and_sector_ops(void) {
    const picofw_session_ctx_t *const ctx = picofw_session_default_ctx();
    const uint32_t flash8m = 8UL * 1024UL * 1024UL;
    picofw_trap_audit_result_t tr;

    /* Required TRAP checks + safe 0xDC0F0080 check */
    assert(picofw_trap_check_full(ctx, flash8m, 0x84000000UL, 0x1000UL, PICOFW_SSOT_TRAP_W, false, &tr));
    assert((tr.verdict == PICOFW_TRAP_DENY) && (strcmp(tr.trap->id, "IPL_L1_UNCACHED_RO") == 0));

    assert(picofw_trap_check_full(ctx, flash8m, 0xC000100CUL, 4UL, PICOFW_SSOT_TRAP_R, false, &tr));
    assert((tr.verdict == PICOFW_TRAP_DENY) && (strcmp(tr.trap->id, "LETHAL_MMIO_C000100C") == 0));

    assert(picofw_trap_check_full(ctx, flash8m, 0xDC0F0000UL, 4UL, PICOFW_SSOT_TRAP_R, false, &tr));
    assert((tr.verdict == PICOFW_TRAP_DENY) && (strcmp(tr.trap->id, "LETHAL_MMIO_DC0F0000") == 0));

    assert(picofw_trap_check_full(ctx, flash8m, 0xDC0F0080UL, 4UL, PICOFW_SSOT_TRAP_R, false, &tr));
    assert((tr.verdict == PICOFW_TRAP_ALLOW) && (tr.trap == NULL));

    assert(picofw_trap_audit(0x84200000UL, 0x10000UL, PICOFW_SSOT_TRAP_W, &tr));
    assert((tr.verdict == PICOFW_TRAP_WARN) && (strcmp(tr.trap->id, "BGO_4MB_ALIAS_TRAP") == 0));

    assert(picofw_trap_check_full(ctx, flash8m, 0x88000000UL, 4UL, PICOFW_SSOT_TRAP_R, false, &tr));
    assert(tr.verdict == PICOFW_TRAP_ALLOW);

    /* All 5 embedded MN103S payloads build and self-verify deterministically */
    assert(picofw_payload_count() == (size_t)PICOFW_PAY_COUNT);
    for (size_t i = 0u; i < PICOFW_PAY_COUNT; ++i) {
        picofw_payload_info_t info;
        assert(picofw_payload_build_and_verify(
            (picofw_payload_id_t)i, g_picofw_cmd_arena.raw, 2048u, &info
        ));
        assert(info.verified && (info.len > 0u) && (info.crc32 != 0u));
        if (i == PICOFW_PAY_FAST_BGO_STREAM) {
            assert(info.crc32 == PICOFW_FAST_STUB_CRC32);
        }
        if (i == PICOFW_PAY_VIDEO_MMIO_DUMP) {
            uint32_t hits[2];
            assert(picofw_sig_find_in_buf(g_picofw_cmd_arena.raw, info.len, 0u,
                                         (const uint8_t *)"PANAVID1", 8u, hits, 2u) == 1u);
            assert(picofw_sig_find_in_buf(g_picofw_cmd_arena.raw, info.len, 0u,
                                         (const uint8_t *)"PANADONE", 8u, hits, 2u) == 1u);
        }
    }

    /* SECTOR describe & patch planning */
    picofw_sector_info_t sec;
    assert(picofw_sector_describe(ctx, flash8m, 0x84130000UL, false, &sec));
    assert((sec.sa_index == 26) && (sec.sector_base == 0x84130000UL) &&
           (strcmp(sec.zone, "BGO_FTL_DATA") == 0) &&
           (sec.write_audit.verdict == PICOFW_TRAP_DENY));

    assert(picofw_sector_describe(ctx, flash8m, 0x847F0000UL, false, &sec));
    assert((sec.sa_index == 134) && (sec.sector_size == 0x2000UL) &&
           (strcmp(sec.zone, "NVRAM_EEMUL") == 0) &&
           (sec.write_audit.verdict == PICOFW_TRAP_ALLOW));

    const uint8_t old4[4] = { 0xFFu, 0xAAu, 0x00u, 0xFFu };
    const uint8_t good_new4[4] = { 0x5Au, 0x22u, 0x00u, 0x0Fu };
    const uint8_t bad_new4[4] = { 0x5Au, 0x22u, 0x01u, 0x0Fu };
    picofw_sector_patch_plan_t plan;

    assert(picofw_sector_plan_patch(ctx, flash8m, false, 0x847F0000UL, old4, good_new4, 4u, &plan));
    assert(plan.allowed && plan.bitclear_monotonic && !plan.needs_erase);

    assert(picofw_sector_plan_patch(ctx, flash8m, false, 0x847F0000UL, old4, bad_new4, 4u, &plan));
    assert(!plan.allowed && !plan.bitclear_monotonic && plan.needs_erase && (plan.first_bad_offset == 2u));

    assert(picofw_sector_plan_patch(ctx, flash8m, false, 0x84000000UL, old4, good_new4, 4u, &plan));
    assert(!plan.allowed && (plan.audit.verdict == PICOFW_TRAP_DENY));

    /* Multi-token whitespace splitter */
    char ws_buf[32] = "  SECTOR   INFO   84130000 ";
    char *toks[4] = { NULL, NULL, NULL, NULL };
    assert(picofw_split_ws_tokens(ws_buf, toks, 4u) == 3u);
    assert((strcmp(toks[0], "SECTOR") == 0) && (strcmp(toks[2], "84130000") == 0));
    printf("  [PASS] test_traps_payloads_and_sector_ops\n");
}

static void test_hotplug_mmio_and_diag_deep(void) {
    /* 1. 5-sample debounced HOTPLUG FSM & auto-heal */
    picofw_hotplug_fsm_t fsm;
    picofw_hotplug_fsm_init(&fsm);
    assert(!fsm.push_enabled && (fsm.state == PICOFW_HP_DETACHED));

    bool changed = false;
    bool clear_fault = false;
    for (int i = 0; i < 4; ++i) {
        picofw_hotplug_fsm_step(&fsm, 3260u, true, false, &changed, &clear_fault);
        assert(!changed && (fsm.state == PICOFW_HP_DETACHED));
    }
    picofw_hotplug_fsm_step(&fsm, 3260u, true, false, &changed, &clear_fault);
    assert(changed && !clear_fault && (fsm.state == PICOFW_HP_ONLINE_AWAKE));

    /* Instantaneous dropout fault fires immediately without waiting 5 samples */
    picofw_hotplug_fsm_step(&fsm, 2900u, false, true, &changed, &clear_fault);
    assert(changed && !clear_fault && (fsm.state == PICOFW_HP_DROPOUT_FAULT));

    /* Recover only on 5th stable ONLINE_AWAKE poll */
    for (int i = 0; i < 4; ++i) {
        picofw_hotplug_fsm_step(&fsm, 3250u, true, true, &changed, &clear_fault);
        assert(!changed && !clear_fault);
    }
    picofw_hotplug_fsm_step(&fsm, 3250u, true, true, &changed, &clear_fault);
    assert(changed && clear_fault && (fsm.state == PICOFW_HP_ONLINE_AWAKE) && (fsm.recoveries == 1u));

    char hp_kv[160];
    assert(picofw_hotplug_format_kv(&fsm, false, 1u, hp_kv, sizeof(hp_kv)) > 0u);
    assert(strstr(hp_kv, "event=ONLINE_AWAKE") != NULL);
    assert(strstr(hp_kv, "recoveries=1") != NULL);

    /* 2. 31-reg MMIO domain parser, safe guard (DC0F0000 vs DC0F0080), SSOT wire tags & video/storage decoder */
    assert(PICOFW_MMIO_SNAP_COUNT == 31u);
    assert(strcmp(PICOFW_WIRE_TAG_RES, "@PICO_RES") == 0);
    assert(strcmp(PICOFW_RES_CODE_EFAULT_LATCHED, "EFaultLatched") == 0);
    bool diff_only = false;
    assert(picofw_mmio_parse_domain(NULL, &diff_only) == PICOFW_SSOT_MMIO_DOM_CORE);
    assert(picofw_mmio_parse_domain("VIDEO DIFF", &diff_only) == PICOFW_SSOT_MMIO_DOM_VIDEO);
    assert(diff_only);
    assert(!picofw_mmio_is_safe_read_addr(0xC000100CUL));
    assert(!picofw_mmio_is_safe_read_addr(0xDC0F0000UL));
    assert(picofw_mmio_is_safe_read_addr(0xDC0F0080UL));

    char dec[64];
    assert(picofw_mmio_decode_video_reg(0x88000000UL, 0x00008021UL, dec, sizeof(dec)) > 0u);
    assert(strstr(dec, "PAL-M") != NULL);
    assert(picofw_mmio_decode_video_reg(0x88000064UL, 0x80010020UL, dec, sizeof(dec)) > 0u);
    assert(strstr(dec, "line_tbc=1 clk_start=1") != NULL);
    assert(picofw_mmio_decode_video_reg(0x89000C20UL, 0x00000000UL, dec, sizeof(dec)) > 0u);
    assert(strstr(dec, "ch1_rst=RELEASED") != NULL);
    assert(picofw_mmio_decode_video_reg(0x89001850UL, 0x00000004UL, dec, sizeof(dec)) > 0u);
    assert((strstr(dec, "sd_cfg4=0x00000004") != NULL) && (strstr(dec, "sd_bus=4BIT") != NULL));
    assert(picofw_mmio_decode_video_reg(0x89001850UL, 0x000080B8UL, dec, sizeof(dec)) > 0u);
    assert(strstr(dec, "sd_bus=1BIT") != NULL);

    /* 3. @PICO_DIAG_DEEP extraction from real EH55 3060NP UART trace lines */
    picofw_boot_classifier_t cls;
    picofw_boot_classifier_init(&cls);
    picofw_boot_classifier_feed_line(&cls, "[SYS_BOOT]RegionCode(EEPROM) [0x04]");
    picofw_boot_classifier_feed_line(&cls, "[SYS_BOOT]Timer Version [022]");
    picofw_boot_classifier_feed_line(&cls, "[SYS_BOOT]H/W System  = M6930LB");
    picofw_boot_classifier_feed_line(&cls, "[SYS_BOOT]market_code=PL(0x04)");
    picofw_boot_classifier_feed_line(&cls, "[SYS_BOOT]CAV:N/PAL");
    picofw_boot_classifier_feed_line(&cls, "[AVIN] target:0 input:0x0 --IIC send error retry 3 all err");
    picofw_boot_classifier_feed_line(&cls, "System assert!! File:src/avin_funcs_com.c Line:3756 Err:0");
    picofw_boot_classifier_feed_line(&cls, "[RTSD] Drive Revision : S211 ");
    picofw_boot_classifier_feed_line(&cls, "[SYS_CNTL] format=0x00000007  authent=0x00000004 ");
    picofw_boot_classifier_feed_line(&cls, "[SYS_CNTL]HDD is_writable=0x0");
    picofw_boot_classifier_feed_line(&cls, "[SYS_CNTL]SetGuiMode mode=6 task ID=0x70");
    picofw_boot_classifier_feed_line(&cls, "[DCS]dco_api :AVOUT_AV_Select: video=8, audio=1");
    picofw_boot_classifier_feed_line(&cls, "[SYS_CNTL]SYS_CNTL_SetSysState[VV]");

    char deep[256];
    assert(picofw_boot_classifier_format_deep(&cls, deep, sizeof(deep)) > 0u);
    assert(strstr(deep, "@PICO_DIAG_DEEP hw=M6930LB market=PL(0x04) timer=022 region=0x04 cav=N/PAL") != NULL);
    assert(strstr(deep, "avsel=8/1 gui=6 sys=VV hdd_wr=0 hdd_fmt=0x7 drv=S211 avin_iic=0:0x0@3756") != NULL);
    printf("  [PASS] test_hotplug_mmio_and_diag_deep\n");
}

static void test_math_gps_and_pieos_xray(void) {
    static uint32_t s_hist[256];
    static uint8_t s_buf[256];
    static picofw_gps_env_t s_env;
    static picofw_boot_classifier_t s_cls;
    static picofw_xray_trace_t s_tr;

    picofw_math_block_stats_t st;

    /* 1. Fixed-point Q10 Shannon entropy & classification invariants */
    memset(s_buf, 0xFF, sizeof(s_buf));
    assert(picofw_math_analyze_block(s_buf, sizeof(s_buf), s_hist, &st));
    assert((st.entropy_mbit == 0u) && (st.ff_pct == 100u) && (st.kind == PICOFW_BLK_ERASED_FF));

    memset(s_buf, 0x00, sizeof(s_buf));
    assert(picofw_math_analyze_block(s_buf, sizeof(s_buf), s_hist, &st));
    assert((st.entropy_mbit == 0u) && (st.zero_pct == 100u) && (st.kind == PICOFW_BLK_EMPTY_ZERO));

    for (size_t i = 0u; i < 256u; ++i) {
        s_buf[i] = (uint8_t)i;
    }
    assert(picofw_math_analyze_block(s_buf, 256u, s_hist, &st));
    assert((st.entropy_mbit >= 7990u) && (st.entropy_mbit <= 8010u));

    /* zlib header detection (0x789C % 31 == 0) + compressed class */
    s_buf[0] = 0x78u;
    s_buf[1] = 0x9Cu;
    assert(picofw_math_analyze_block(s_buf, 256u, s_hist, &st));
    assert(st.zlib_header && (st.kind == PICOFW_BLK_COMPRESSED));

    /* ASCII text detection */
    static const char k_ascii[] =
        "PiE-OS System Version : 3060NP\r\nH/W System  = M6930LB\r\n"
        "AVOUT_init() success.\r\nnfrom back up ok.\r\n";
    assert(picofw_math_analyze_block(
        (const uint8_t *)k_ascii, sizeof(k_ascii) - 1u, s_hist, &st
    ));
    assert((st.ascii_pct >= 80u) && (st.kind == PICOFW_BLK_ASCII_TEXT));

    /* 2. Pure MN103S prologue, CALL16/CALL32, and 0xFC AM34 MMIO edge decoder */
    static const uint8_t k_mn103_sample[] = {
        0xCFu, 0x0Eu,                               /* MOVM [regs],(SP) */
        0xF8u, 0xFEu, 0xE8u,                        /* ADD -24,SP */
        0xCDu, 0x69u, 0x00u, 0x00u, 0x00u,          /* CALL d16 -> +0x69 -> 0x5711226E */
        0xDDu, 0x00u, 0x01u, 0x00u, 0x00u, 0x00u, 0x00u, /* CALL d32 -> +0x100 -> 0x5711230A */
        0xFCu, 0xDCu, 0x00u, 0x20u, 0x01u, 0xDCu    /* 0xDC012000 MMIO ref */
    };
    picofw_mn103_edge_t edges[8];
    const size_t n_edges = picofw_mn103_scan_edges(
        k_mn103_sample, sizeof(k_mn103_sample), 0x57112200UL, edges, 8u
    );
    assert(n_edges == 5u);
    assert((edges[0].kind == PICOFW_EDGE_PROLOGUE) && (strcmp(edges[0].label, "MOVM_SP") == 0));
    assert((edges[1].kind == PICOFW_EDGE_PROLOGUE) && (strcmp(edges[1].label, "ADD_IMM8_SP") == 0));
    assert((edges[2].kind == PICOFW_EDGE_CALL) && (edges[2].target_va == 0x5711226EUL));
    assert((edges[3].kind == PICOFW_EDGE_CALL) && (edges[3].target_va == 0x5711230AUL));
    assert((edges[4].kind == PICOFW_EDGE_MMIO) && (edges[4].target_va == 0xDC012000UL));

    /* 3. Line accumulator parity (CR/LF termination & bounds) */
    char acc[16];
    size_t acc_len = 0u;
    assert(!picofw_line_accum_push_byte(acc, &acc_len, sizeof(acc), (uint8_t)'A'));
    assert(!picofw_line_accum_push_byte(acc, &acc_len, sizeof(acc), 0x01u));
    assert(!picofw_line_accum_push_byte(acc, &acc_len, sizeof(acc), (uint8_t)'B'));
    assert(picofw_line_accum_push_byte(acc, &acc_len, sizeof(acc), (uint8_t)'\r'));
    acc[acc_len] = '\0';
    assert(strcmp(acc, "AB") == 0);

    /* 4. Live SSOT anchor resolution, affine calibration (-0x78E2), and ROUTE synthesis */
    picofw_gps_env_init(&s_env);
    picofw_boot_classifier_init(&s_cls);
    picofw_boot_classifier_feed_line(&s_cls, "System Version : 3060NP");
    picofw_boot_classifier_feed_line(&s_cls, "[SYS_BOOT]H/W System  = M6930LB");
    picofw_boot_classifier_feed_line(&s_cls, "[SYS_BOOT]RegionCode(EEPROM) [0x04]");
    picofw_boot_classifier_feed_line(&s_cls, "[AVIN]IIC send error");
    picofw_boot_classifier_feed_line(&s_cls, "BGO driver init failed[1]");
    picofw_gps_calibrate_from_classifier(&s_env, &s_cls);
    assert((s_env.main_build == (uint8_t)PICOFW_MAIN_BUILD_3060NP) && s_env.has_region);
    assert((s_env.region_code == 4u) && (s_env.avin_iic_err == 1u) && (s_env.bgo_err == 1u));

    size_t idx_nfd = 0u;
    size_t idx_rtsd = 0u;
    size_t idx_bgo = 0u;
    assert(picofw_gps_find_anchor_by_name("NFD_Init", &idx_nfd));
    assert(picofw_gps_find_anchor_by_name("rtsd_ata_hard_reset_wait", &idx_rtsd));
    assert(picofw_gps_find_anchor_by_name("BGO_Init_entry", &idx_bgo));
    assert(picofw_gps_resolve_anchor(&s_env, idx_nfd) != 0u);
    assert(picofw_gps_resolve_anchor(&s_env, idx_rtsd) == 0x5711226EUL);
    const uint32_t bgo_before = picofw_gps_resolve_anchor(&s_env, idx_bgo);
    assert(bgo_before == 0x574AF2F0UL);

    assert(picofw_gps_calibrate_affine(&s_env, idx_rtsd, 0x5710A98CUL));
    assert(s_env.affine_valid && (s_env.affine_delta == -0x78E2));
    assert(picofw_gps_resolve_anchor(&s_env, idx_rtsd) == 0x5710A98CUL);
    assert(picofw_gps_resolve_anchor(&s_env, idx_bgo) == (uint32_t)((int32_t)bgo_before - 0x78E2));

    char out[256];
    assert(picofw_gps_format_env(&s_env, out, sizeof(out)) > 0u);
    assert(strstr(out, "@PICO_GPS_ENV build=3060NP") != NULL);
    assert(strstr(out, "affine=1 delta=-30946") != NULL);

    assert(picofw_gps_build_route(&s_env, PICOFW_ROUTE_VIDEO, out, sizeof(out)) > 0u);
    assert(strstr(out, "@PICO_ROUTE dom=VIDEO build=3060NP") != NULL);
    assert(picofw_gps_build_route(&s_env, PICOFW_ROUTE_BGO, out, sizeof(out)) > 0u);
    assert(strstr(out, "@PICO_ROUTE dom=BGO build=3060NP") != NULL);
    assert(picofw_gps_build_route(&s_env, PICOFW_ROUTE_BOOT, out, sizeof(out)) > 0u);
    assert(strstr(out, "@PICO_ROUTE dom=BOOT build=3060NP") != NULL);

    /* 5. 12-stage PiE-OS X-Ray timeline, ANSI hygiene, and summary envelope */
    picofw_xray_init(&s_tr, false);
    assert(!s_tr.color_enabled);

    picofw_xray_event_t ev;
    assert(picofw_xray_feed_line(&s_tr, &s_env, 1000ULL, "DDR CHECK OK", &ev));
    assert(ev.stage == PICOFW_XRAY_S1_IPL_DDR);
    assert(picofw_xray_feed_line(&s_tr, &s_env, 5200ULL, "Loader ver 0.61", &ev));
    assert((ev.stage == PICOFW_XRAY_S2_LOADER) && (ev.delta_us == 4200ULL));
    assert(picofw_xray_feed_line(&s_tr, &s_env, 7500ULL, "Main hung-up time changed 255[s].", &ev));
    assert((ev.stage == PICOFW_XRAY_S2_LOADER) && (ev.level == PICOFW_XRAY_LVL_INFO) && (s_tr.errors_seen == 0u));
    assert(picofw_xray_feed_line(&s_tr, &s_env, 9000ULL, "PANAMON1>[B/R/W/D/F/J/E/Q]", &ev));
    assert((ev.stage == PICOFW_XRAY_S4_SD_DIAG_FVU) && (ev.level == PICOFW_XRAY_LVL_OK));
    assert(picofw_xray_feed_line(&s_tr, &s_env, 12000ULL, "System Version : 3060NP", &ev));
    assert(ev.stage == PICOFW_XRAY_S5_PIE_CRT0);
    assert(picofw_xray_feed_line(&s_tr, &s_env, 25000ULL, "AVOUT_init() success.", &ev));
    assert(ev.stage == PICOFW_XRAY_S8_AVIN_AVOUT);
    assert(picofw_xray_feed_line(&s_tr, &s_env, 40000ULL, "BGO driver init failed[1]", &ev));
    assert((ev.stage == PICOFW_XRAY_S7_BGO_NFROM) && (ev.level == PICOFW_XRAY_LVL_CRIT));

    assert(picofw_xray_format_event(&ev, false, out, sizeof(out)) > 0u);
    assert(strstr(out, "[S7_BGO_NFROM]") != NULL);
    assert(strstr(out, "[CRT]") != NULL);
    assert(strstr(out, "\x1b[") == NULL);

    assert(picofw_xray_format_event(&ev, true, out, sizeof(out)) > 0u);
    assert(strstr(out, "\x1b[") != NULL);

    assert(picofw_xray_format_summary(&s_tr, &s_env, out, sizeof(out)) > 0u);
    assert(strstr(out, "@PICO_XRAY ") != NULL);
    assert(strstr(out, "fault_stage=S7_BGO_NFROM") != NULL);
    assert(strstr(out, "errors=1") != NULL);
    assert(strstr(out, "color=OFF") != NULL);
    assert(strstr(out, "\x1b[") == NULL);

    printf("  [PASS] test_math_gps_and_pieos_xray\n");
}

static uint64_t splitmix64_next(uint64_t *st) {
    *st += 0x9E3779B97F4A7C15ULL;
    uint64_t z = *st;
    z = (z ^ (z >> 30u)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27u)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31u);
}

static void test_microsurgery_patterns_and_autocal(void) {
    static uint32_t s_hist[256];
    static uint8_t s_buf[512];
    static picofw_math_pattern_report_t s_rep;
    static picofw_surg_plan_t s_plan;
    static picofw_surg_journal_t s_jrn;
    static picofw_gps_env_t s_env;
    static picofw_meihfs_mbr_info_t s_mbr;

    /* 1. Statistical & structural pattern detectors (chi2, ECB-64, pin-mirror, S-box) */
    for (size_t i = 0u; i < 256u; ++i) {
        s_buf[i] = (uint8_t)i;
    }
    assert(picofw_math_analyze_block(s_buf, 256u, s_hist, &s_rep.stats));
    assert(picofw_math_chi2_q8(s_hist, 256u) == 0u);
    assert(picofw_math_chi2_q8(s_hist, 128u) == 0u); /* small-window guard */
    assert(!picofw_math_is_crypto_sbox256(s_buf));     /* reject identity ramp */
    for (size_t i = 0u; i < 256u; ++i) {
        s_buf[i] = (uint8_t)(255u - i);
    }
    assert(!picofw_math_is_crypto_sbox256(s_buf));     /* reject reverse ramp */
    for (size_t i = 0u; i < 256u; ++i) {
        s_buf[i] = (uint8_t)(i ^ 0x5Au);
    }
    assert(!picofw_math_is_crypto_sbox256(s_buf));     /* reject XOR-affine table */
    assert(picofw_math_is_crypto_sbox256(k_picofw_ssot_feistel_sbox));

    memset(s_buf, 0x00, 256u);
    assert(picofw_math_analyze_block(s_buf, 256u, s_hist, &s_rep.stats));
    assert(picofw_math_chi2_q8(s_hist, 256u) == (65280UL * 256UL));

    for (size_t i = 0u; i < 64u; ++i) {
        s_buf[i] = (uint8_t)(i * 3u + 1u);
        s_buf[64u + i] = s_buf[i];
    }
    assert(picofw_math_detect_addr_mirror(s_buf, 128u) == 64u);
    assert(picofw_math_detect_ecb64(s_buf, 128u) >= 8u);

    memcpy(&s_buf[128], k_picofw_ssot_feistel_sbox, 256u);
    assert(picofw_math_scan_patterns(s_buf, 512u, s_hist, &s_rep));
    assert(s_rep.sbox_256_found && (s_rep.sbox_offset == 128u));

    /* 2. UB-free bitwise engine + splitmix64 differential algebraic identities */
    uint32_t bv = 0u;
    assert(picofw_bit_eval(PICOFW_BIT_FIELD, 0x80010020UL, 16u, 1u, 0u, &bv) && (bv == 0x80000020UL));
    assert(picofw_bit_eval(PICOFW_BIT_EXT, 0xA55A0F00UL, 8u, 4u, 0u, &bv) && (bv == 0x0Fu));
    assert(picofw_bit_eval(PICOFW_BIT_SET, 0u, 31u, 0u, 0u, &bv) && (bv == 0x80000000UL));
    assert(picofw_bit_eval(PICOFW_BIT_CLR, 0xFFFFFFFFUL, 0u, 0u, 0u, &bv) && (bv == 0xFFFFFFFEUL));
    assert(picofw_bit_eval(PICOFW_BIT_TOG, 0x0F0F0F0FUL, 4u, 0u, 0u, &bv) && (bv == 0x0F0F0F1FUL));
    assert(!picofw_bit_eval(PICOFW_BIT_SET, 0u, 32u, 0u, 0u, &bv));
    assert(!picofw_bit_eval(PICOFW_BIT_FIELD, 0u, 28u, 5u, 0u, &bv));
    assert(!picofw_bit_eval(PICOFW_BIT_EXT, 0u, 0u, 0u, 0u, &bv));

    uint64_t rng = 0x50414E4144494741ULL;
    for (size_t i = 0u; i < 1000u; ++i) {
        const uint32_t x = (uint32_t)splitmix64_next(&rng);
        const uint32_t k = (uint32_t)(splitmix64_next(&rng) % 32ULL);
        uint32_t r1 = 0u, r2 = 0u, p1 = 0u, p2 = 0u, s1 = 0u, s2 = 0u;
        assert(picofw_bit_eval(PICOFW_BIT_ROTL, x, k, 0u, 0u, &r1));
        assert(picofw_bit_eval(PICOFW_BIT_ROTR, r1, k, 0u, 0u, &r2) && (r2 == x));
        assert(picofw_bit_eval(PICOFW_BIT_BSWAP32, x, 0u, 0u, 0u, &s1));
        assert(picofw_bit_eval(PICOFW_BIT_BSWAP32, s1, 0u, 0u, 0u, &s2) && (s2 == x));
        assert(picofw_bit_eval(PICOFW_BIT_POPCNT, x, 0u, 0u, 0u, &p1));
        assert(picofw_bit_eval(PICOFW_BIT_POPCNT, ~x, 0u, 0u, 0u, &p2) && ((p1 + p2) == 32u));
    }

    /* 3. u64 [lo, hi) wraparound defense & RAM-only microsurgery safety gates */
    picofw_trap_audit_result_t ta;
    assert(picofw_trap_audit(0x80050000UL, 0xFFFF0000UL, PICOFW_SSOT_TRAP_W, &ta));
    assert(ta.verdict == PICOFW_TRAP_DENY);
    assert(!picofw_surg_is_live_ram_span(0x84000000UL, 4u));
    assert(!picofw_surg_is_live_ram_span(0xC000100CUL, 4u));
    assert(!picofw_surg_is_live_ram_span(0x80050000UL, 0xFFFF0000UL));
    assert(picofw_surg_is_live_ram_span(0x50400200UL, 16u));

    /* 4. Wildcard search, MN103S disasm, timeout in-place patch & hook planner */
    static const uint8_t k_code[12] = {
        0xFCu, 0xCCu, 0x88u, 0x13u, 0x00u, 0x00u, /* MOV 5000 (0x1388), D0 */
        0xCBu,                                     /* NOP */
        0xF0u, 0xFCu,                              /* RETS */
        0xCAu, 0x04u, 0xCBu                         /* BRA +4, NOP */
    };
    uint8_t pat[16], mask[16];
    size_t plen = 0u;
    uint32_t hits[4] = {0};
    assert(picofw_surg_parse_wildcard_hex("FCCC??1300??", pat, mask, &plen) && (plen == 6u));
    assert(picofw_surg_search_buf(k_code, sizeof(k_code), 0x50400200UL, pat, mask, plen, hits, 4u) == 1u);
    assert(hits[0] == 0x50400200UL);

    assert(picofw_surg_plan_timeout_patch(0x50400200UL, k_code, sizeof(k_code), 1000u, &s_plan));
    assert((s_plan.site_len == 6u) && (s_plan.patch[2] == 0xE8u) && (s_plan.patch[3] == 0x03u));

    assert(picofw_surg_plan_hook(0x50400200UL, k_code, 8u, 0x50401000UL, PICOFW_HOOK_D0_CONT, 0x11223344UL, &s_plan));
    assert((s_plan.site_len == 6u) && (s_plan.patch[0] == 0xDCu) && (s_plan.patch[5] == 0xCBu));
    assert((s_plan.tramp_len == 17u) && (s_plan.tramp[0] == 0xFCu) && (s_plan.tramp[12] == 0xDCu));

    assert(picofw_surg_plan_hook(0x50400200UL, k_code, 8u, 0x50401000UL, PICOFW_HOOK_CALL, 0x50402000UL, &s_plan));
    assert((s_plan.tramp_len == 18u) && (s_plan.tramp[0] == 0xDDu));

    /* Fail-closed rejections: NOR Flash, overlapping tramp, non-relocatable BRA */
    static const uint8_t k_rel_branch[6] = { 0xCAu, 0x04u, 0xCBu, 0xCBu, 0xCBu, 0xCBu };
    assert(!picofw_surg_plan_hook(0x84040000UL, k_code, 8u, 0x50401000UL, PICOFW_HOOK_D0_CONT, 1u, &s_plan));
    assert(!picofw_surg_plan_hook(0x50400200UL, k_code, 8u, 0x50400204UL, PICOFW_HOOK_D0_CONT, 1u, &s_plan));
    assert(!picofw_surg_plan_hook(0x50400200UL, k_rel_branch, sizeof(k_rel_branch), 0x50401000UL, PICOFW_HOOK_D0_CONT, 1u, &s_plan));

    /* 5. Time-travel journal algebraic round-trip laws */
    picofw_surg_journal_init(&s_jrn);
    picofw_surg_entry_t e1 = { PICOFW_SURG_MEM_PATCH, 4u, 0x50400200UL, 0u, 0u, 0u, 100ULL,
                              { 1, 2, 3, 4 }, { 5, 6, 7, 8 }, "PATCH" };
    picofw_surg_entry_t e2 = { PICOFW_SURG_PC_JUMP, 0u, 0u, 0u, 0x50400000UL, 0x50400200UL, 200ULL,
                              {0}, {0}, "JUMP" };
    assert(picofw_surg_journal_push(&s_jrn, &e1));
    assert(picofw_surg_journal_push(&s_jrn, &e2));
    assert((s_jrn.count == 2u) && (s_jrn.cursor == 2u));
    assert(picofw_surg_journal_peek_undo(&s_jrn)->pc_before == 0x50400000UL);
    assert(picofw_surg_journal_commit_undo(&s_jrn) && (s_jrn.cursor == 1u));
    assert(picofw_surg_journal_peek_undo(&s_jrn)->site_va == 0x50400200UL);
    assert(picofw_surg_journal_commit_redo(&s_jrn) && (s_jrn.cursor == 2u));
    assert(picofw_surg_journal_commit_undo(&s_jrn) && (s_jrn.cursor == 1u));
    assert(picofw_surg_journal_push(&s_jrn, &e1) && (s_jrn.count == 2u) && (s_jrn.cursor == 2u));

    /* 6. Declarative SSOT model+suffix auto-calibration */
    picofw_gps_env_init(&s_env);
    assert(picofw_gps_autocal_model_suffix(&s_env, "DMR-EH55LB-S"));
    assert((strcmp(s_env.tv_standard, "PAL-M") == 0) && (s_env.aiof_vmodesel == 0x0002u));
    assert((s_env.dvd_region == 4u) && (s_env.ic3001_expected_addr7 == 0x48u));

    assert(picofw_gps_autocal_model_suffix(&s_env, "DMR-EH55EC"));
    assert((strcmp(s_env.tv_standard, "PAL") == 0) && (s_env.aiof_vmodesel == 0x0001u));
    assert((s_env.dvd_region == 2u) && (s_env.ic3001_expected_addr7 == 0x4Au));

    assert(picofw_gps_autocal_model_suffix(&s_env, "DMR-EH55P"));
    assert((strcmp(s_env.tv_standard, "NTSC") == 0) && (s_env.aiof_vmodesel == 0x0000u));
    assert((s_env.dvd_region == 1u) && (s_env.ic3001_expected_addr7 == 0x48u));

    /* 7. MEIHFS Sector 0 micro-triage */
    memset(s_buf, 0x11, 512u);
    s_buf[0] = 'S';
    s_buf[1] = 'H';
    s_buf[2] = 0x02u;
    s_buf[3] = 0x00u;
    assert(picofw_meihfs_inspect_sector0(s_buf, &s_mbr) && s_mbr.valid_sh_magic && !s_mbr.byte_swapped_hs);
    s_buf[0] = 'H';
    s_buf[1] = 'S';
    assert(picofw_meihfs_inspect_sector0(s_buf, &s_mbr) && !s_mbr.valid_sh_magic && s_mbr.byte_swapped_hs);
    char rep_str[192];
    assert(picofw_meihfs_format_report(0x50404000UL, &s_mbr, rep_str, sizeof(rep_str)) > 0u);
    assert(strstr(rep_str, "@PICO_SURG op=MEIHFS") != NULL);

    /* 8. Two-way MN103S ASM <-> DISASM round-trip proofs */
    static const char *const k_asm_vec[] = {
        "NOP", "RETS", "MOV 1000, D0", "MOV 0x88000064, A0",
        "STORE D0, 0x88000064", "ADD -16, SP", "JMP 0x50401000",
        "CALL 0x5711226E", "BRA 0x50400210", "MOVM_PUSH 0x30", "MOVM_POP 0x30"
    };
    for (size_t i = 0u; i < (sizeof(k_asm_vec) / sizeof(k_asm_vec[0])); ++i) {
        uint8_t mc[8] = {0};
        uint8_t mlen = 0u;
        picofw_mn103_insn_info_t dec;
        assert(picofw_mn103_assemble_insn(k_asm_vec[i], 0x50400200UL, mc, &mlen));
        assert(picofw_mn103_decode_insn(mc, mlen, 0x50400200UL, &dec));
        assert(dec.valid && (dec.len == mlen));
    }

    /* 9. Two-way LZSS & zlib round-trip with splitmix64 entropy/pattern vectors */
    static uint8_t s_comp[512];
    static uint8_t s_rec[512];
    static uint8_t s_win4k[4096];
    static picofw_inflate_scratch_t s_inf;
    static const char k_phrase[] = "PICOFW_DIGAOS_M_FIRM_MAIN_UPDATE_UPDOK_FINISH_PICOFW_DIGAOS";
    const size_t p_len = sizeof(k_phrase) - 1u;
    size_t c_len = 0u, d_len = 0u;
    assert(picofw_lzss_compress((const uint8_t *)k_phrase, p_len, s_comp, sizeof(s_comp), &c_len, s_win4k));
    assert(c_len < p_len);
    assert(picofw_lzss_decompress(s_comp, c_len, s_rec, sizeof(s_rec), &d_len, s_win4k));
    assert((d_len == p_len) && (memcmp(s_rec, k_phrase, p_len) == 0));

    assert(picofw_zlib_compress((const uint8_t *)k_phrase, p_len, s_comp, sizeof(s_comp), &c_len));
    assert(picofw_zlib_decompress(s_comp, c_len, s_rec, sizeof(s_rec), &d_len, &s_inf));
    assert((d_len == p_len) && (memcmp(s_rec, k_phrase, p_len) == 0));

    /* Deterministic 256-byte PRNG round-trip */
    for (size_t i = 0u; i < 256u; ++i) {
        s_buf[i] = (uint8_t)splitmix64_next(&rng);
    }
    assert(picofw_lzss_compress(s_buf, 256u, s_comp, sizeof(s_comp), &c_len, s_win4k));
    assert(picofw_lzss_decompress(s_comp, c_len, s_rec, sizeof(s_rec), &d_len, s_win4k));
    assert((d_len == 256u) && (memcmp(s_rec, s_buf, 256u) == 0));

    /* 10. ATA IDENTIFY 512B/1024B decode, full-HDD plan, RTSD clamp patch & 5-stage TBUS */
    static uint8_t s_id512[512];
    static uint8_t s_id1024[1024];
    static picofw_ata_ident_t s_ata;
    static picofw_hdd_full_plan_t s_hp;
    static picofw_hdd_unlock_plan_t s_up;
    memset(s_id512, 0, sizeof(s_id512));
    s_id512[0] = 0x5Au; s_id512[1] = 0x04u; /* non-ATAPI HDD */
    s_id512[166] = 0x00u; s_id512[167] = 0x04u; /* word83 bit10 LBA48 */
    /* 976,773,168 sectors (500 GB) = 0x3A386030 */
    s_id512[200] = 0x30u; s_id512[201] = 0x60u; s_id512[202] = 0x38u; s_id512[203] = 0x3Au;
    /* Swapped ATA ASCII model at word 27 (byte 54): "WDC WD5000AAJB" */
    static const char k_mdl[] = "DW CMW5D00A0JA B";
    memcpy(&s_id512[54], k_mdl, 16u);
    assert(picofw_ata_parse_identify(s_id512, 512u, false, &s_ata));
    assert(s_ata.lba48_supported && (s_ata.effective_sectors == 976773168ULL));
    assert(s_ata.mirror_fw_lba == (976773168ULL - PICOFW_SSOT_MEIHFS_MIRROR_DELTA_LBA));
    for (size_t w = 0u; w < 256u; ++w) {
        s_id1024[w * 4u] = s_id512[w * 2u];
        s_id1024[w * 4u + 1u] = s_id512[w * 2u + 1u];
        s_id1024[w * 4u + 2u] = 0u;
        s_id1024[w * 4u + 3u] = 0u;
    }
    assert(picofw_ata_parse_identify(s_id1024, 1024u, true, &s_ata));
    assert(s_ata.effective_sectors == 976773168ULL);

    assert(picofw_hdd_compute_full_plan(s_ata.effective_sectors, 0x400000UL, &s_hp) && s_hp.valid);
    assert(s_hp.target_rtsd_blocks_2kb == (uint32_t)(976773168ULL >> 2u));

    static uint8_t s_clamp[PICOFW_SSOT_HDD_CLAMP_WIN_LEN];
    memset(s_clamp, 0, sizeof(s_clamp));
    memcpy(s_clamp, k_picofw_ssot_hdd_clamp_prefix, 14u);
    s_clamp[0x12] = 0xFCu; s_clamp[0x13] = 0x81u;
    s_clamp[0x27] = 0xFCu; s_clamp[0x28] = 0xCCu;
    s_clamp[PICOFW_SSOT_HDD_CLAMP_OFF_200GB] = 0xBAu;
    s_clamp[PICOFW_SSOT_HDD_CLAMP_OFF_200GB + 1u] = 0x1Du;
    s_clamp[PICOFW_SSOT_HDD_CLAMP_OFF_200GB + 2u] = 0xD2u;
    s_clamp[PICOFW_SSOT_HDD_CLAMP_OFF_200GB + 3u] = 0x05u;
    s_clamp[0x2D] = 0xFCu; s_clamp[0x2E] = 0x81u;
    s_clamp[0x42] = 0xFCu; s_clamp[0x43] = 0xCCu;
    s_clamp[0x48] = 0xFCu; s_clamp[0x49] = 0x81u;
    s_clamp[0x4E] = 0xFCu; s_clamp[0x4F] = 0xA5u;
    s_clamp[0x54] = 0xFCu; s_clamp[0x55] = 0xA4u;
    s_clamp[0x5A] = 0xA1u;
    s_clamp[PICOFW_SSOT_HDD_CLAMP_OFF_BRANCH] = PICOFW_SSOT_HDD_OPCODE_BLS;
    s_clamp[0x5C] = 0x0Cu; s_clamp[0x5D] = 0x29u; s_clamp[0x5E] = 0xFFu; s_clamp[0x5F] = 0xFCu; s_clamp[0x60] = 0x85u;
    s_clamp[0x65] = 0xCAu; s_clamp[0x66] = 0x0Au; s_clamp[0x67] = 0x28u; s_clamp[0x68] = 0xFFu; s_clamp[0x69] = 0xFCu; s_clamp[0x6A] = 0x81u;

    size_t clamp_off = 99u;
    assert(picofw_hdd_find_clamp_site(s_clamp, sizeof(s_clamp), &clamp_off) && (clamp_off == 0u));
    assert(picofw_hdd_plan_rtsd_unlock(0x57110000UL, s_clamp, s_hp.target_rtsd_blocks_2kb, &s_up) && s_up.valid);
    assert((s_up.prev_200gb_2kb == 0x05D21DBAUL) && (s_up.ops[3].data[0] == PICOFW_SSOT_HDD_OPCODE_BRA));
    /* Must fail-closed on NOR Flash */
    assert(!picofw_hdd_plan_rtsd_unlock(0x84110000UL, s_clamp, s_hp.target_rtsd_blocks_2kb, &s_up));

    static const char *const k_vfd_expected[5] = { "M FIRM", "MAIN", "UPDATE", "UPDOK", "FINISH" };
    for (uint8_t st = 0u; st < 5u; ++st) {
        uint8_t vfd16[16], wdt16[16];
        const char *lbl = NULL;
        assert(picofw_hdd_build_stage_tbus(st, vfd16, wdt16, &lbl));
        assert((strcmp(lbl, k_vfd_expected[st]) == 0) && (vfd16[0] == 0xA3u) && (wdt16[15] == 0x30u));
    }

    printf("  [PASS] test_microsurgery_patterns_and_autocal\n");
}

int picofw_run_pico_autonomous_tests(void) {
    test_push_stream_ring_and_watermark();
    test_feistel_and_decomp_engines();
    test_traps_payloads_and_sector_ops();
    test_hotplug_mmio_and_diag_deep();
    test_math_gps_and_pieos_xray();
    test_microsurgery_patterns_and_autocal();
    return 0;
}
