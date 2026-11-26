/**
 * @file test_pana_diag_drv.c
 * @brief Host C11 unit tests for parameterized `DIAGWATCH`, `PANARSP REGS`, `DRVPANA`, and shared flash guards.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_pana_logic.h"
#include "diag/picofw_boottrace_codec.h"
#include "diag/picofw_ir_codec.h"
#include "diag/picofw_tbus_codec.h"
#include "uart/picofw_diag_watch_codec.h"
#include "uart/picofw_drv_pana_codec.h"
#include "uart/picofw_pana_rsp.h"
#include "flash/picofw_flash.h"
#include "common/picofw_hex.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

static void feed_str_and_count(
    picofw_diag_watch_state_t *st,
    const picofw_diag_watch_spec_t *dw,
    const char *s,
    int *rel_count,
    int *tx_count,
    char *last_tx
) {
    for (size_t i = 0u; s[i] != '\0'; ++i) {
        char tx_ch = '\0';
        bool rel = false;
        (void)picofw_diag_watch_feed_byte(st, dw, (uint8_t)s[i], &tx_ch, &rel);
        if (rel) {
            *rel_count += 1;
        }
        if (tx_ch != '\0') {
            *tx_count += 1;
            *last_tx = tx_ch;
        }
    }
}

static void test_diag_watch_legacy_and_fsm(void) {
    picofw_diag_watch_spec_t dw;
    assert(picofw_diag_watch_parse_args("", &dw));
    assert(dw.timeout_ms == 15000u && dw.boot_action == PICOFW_BOOT_ACTION_JIG);
    assert(dw.jig_hold_ms == 2500u && dw.action == PICOFW_DIAG_ACT_AUTO && dw.release_on_ddr);

    /* Bare FORMAT/WRITE/ERASE fail closed without NORUNLOCK:CONFIRM_UPPER_NOR */
    assert(!picofw_diag_watch_parse_args("30000 NORMAL FORMAT HOLD", &dw));
    assert(picofw_diag_watch_parse_args("30000 NORMAL NORUNLOCK:CONFIRM_UPPER_NOR FORMAT HOLD", &dw));
    assert(dw.timeout_ms == 30000u && dw.boot_action == PICOFW_BOOT_ACTION_NORMAL);
    assert(dw.action == PICOFW_DIAG_ACT_FORMAT && !dw.release_on_ddr && dw.nor_unlock);

    assert(picofw_diag_watch_parse_args("180000 NOPULL DUMP RELEASE", &dw));
    assert(dw.timeout_ms == 180000u && dw.boot_action == PICOFW_BOOT_ACTION_PASSIVE);
    assert(dw.action == PICOFW_DIAG_ACT_DUMP && dw.release_on_ddr);
    assert(!picofw_diag_watch_parse_args("20000 BOGUS_DW", &dw));

    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_AUTO) == 'B');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_READ) == 'R');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_WRITE) == 'W');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_FORMAT) == 'F');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_DUMP) == 'D');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_JUMP) == 'J');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_ERASE) == 'E');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_QUIT) == 'Q');
    assert(picofw_diag_watch_action_char(PICOFW_DIAG_ACT_NONE) == '\0');
    assert(picofw_diag_watch_parse_args("20000 JIG:2500 READ RELEASE", &dw) && dw.action == PICOFW_DIAG_ACT_READ);
    assert(!picofw_diag_watch_parse_args("20000 JIG:2500 WRITE RELEASE", &dw));
    assert(picofw_diag_watch_parse_args("20000 JIG:2500 NORUNLOCK:CONFIRM_UPPER_NOR WRITE RELEASE", &dw) && dw.action == PICOFW_DIAG_ACT_WRITE);
    assert(picofw_diag_watch_parse_args("20000 JIG:2500 JUMP RELEASE", &dw) && dw.action == PICOFW_DIAG_ACT_JUMP);
    assert(!picofw_diag_watch_parse_args("20000 JIG:2500 ERASE RELEASE", &dw));
    assert(picofw_diag_watch_parse_args("20000 JIG:2500 NORUNLOCK:CONFIRM_UPPER_NOR ERASE RELEASE", &dw) && dw.action == PICOFW_DIAG_ACT_ERASE);
    assert(picofw_diag_watch_parse_args("20000 JIG:2500 QUIT RELEASE", &dw) && dw.action == PICOFW_DIAG_ACT_QUIT);
    /* Lethal register 0xC000100C blocked in DIAGWATCH READ/WRITE/TXN */
    assert(!picofw_diag_watch_parse_args("20000 READ:C000100C,4", &dw));
    assert(!picofw_diag_watch_parse_args("20000 WRITE:C000100C,AA", &dw));
    assert(!picofw_diag_watch_parse_args("20000 PANAMON_TXN:RC000100C", &dw));

    assert(picofw_diag_watch_parse_args("20000 PREJIG:3200 AUTO RELEASE WAKE:45000", &dw));
    assert(dw.boot_action == PICOFW_BOOT_ACTION_JIG && dw.jig_hold_ms == 3200u && dw.no_xreset);
    assert(dw.wait_for_wake && dw.wake_timeout_ms == 45000u);

    assert(picofw_diag_watch_parse_args("25000 JIG:2500 AUTO RELEASE", &dw));
    picofw_diag_watch_state_t st;
    picofw_diag_watch_init(&st);
    int rel_count = 0;
    int tx_count = 0;
    char last_tx = '\0';

    feed_str_and_count(&st, &dw, "DDR CHECK: OK\r\n", &rel_count, &tx_count, &last_tx);
    assert(rel_count == 0 && !st.sw_uart_released);
    assert((st.seen_events & PICOFW_DIAG_EVT_DDR_OK) != 0u);

    feed_str_and_count(
        &st, &dw,
        "***** serial boot mode *****\r\nSetBusWidth -91006\r\n** open tray ** 80040904\r\nPANAMON1>",
        &rel_count, &tx_count, &last_tx
    );
    assert(rel_count == 1 && st.sw_uart_released);
    assert(tx_count == 1 && last_tx == 'B' && st.tx_dispatched);
    assert((st.seen_events & PICOFW_DIAG_EVT_SERIAL_BOOT) != 0u);
    assert((st.seen_events & PICOFW_DIAG_EVT_SD_ERR) != 0u);
    assert((st.seen_events & PICOFW_DIAG_EVT_ODD_OPEN) != 0u);
    assert((st.seen_events & PICOFW_DIAG_EVT_CD_APC_ERR) != 0u);
    assert((st.seen_events & PICOFW_DIAG_EVT_PANAMON) != 0u);
    assert(picofw_diag_watch_diagnosis(st.seen_events) != NULL);

    assert(picofw_diag_watch_parse_args("12000 NORMAL NONE RELEASE", &dw));
    picofw_diag_watch_init(&st);
    rel_count = 0;
    tx_count = 0;
    feed_str_and_count(
        &st, &dw,
        "DDR CHECK: OK\r\n***** normal mode *****\r\nBGO driver init failed\r\n80040904\r\n",
        &rel_count, &tx_count, &last_tx
    );
    assert(rel_count == 1 && st.sw_uart_released && tx_count == 0);
    assert(strstr(picofw_diag_watch_diagnosis(st.seen_events), "BGO driver init failed") != NULL);
}

static void test_diag_watch_parameterized_tx_and_e05(void) {
    picofw_diag_watch_spec_t dw;
    char tx_buf[36];

    assert(picofw_diag_watch_parse_args("20000 JIG:2500 READ:847F0000,20 RELEASE", &dw));
    assert(dw.action == PICOFW_DIAG_ACT_READ);
    assert(picofw_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 13u);
    assert(strcmp(tx_buf, "R847F0000,20\r") == 0);

    assert(picofw_diag_watch_parse_args("20000 JIG:2500 READ:0x80000000", &dw));
    assert(picofw_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 10u);
    assert(strcmp(tx_buf, "R80000000\r") == 0);
    assert(!picofw_diag_watch_parse_args("20000 READ:80000000,0", &dw));
    assert(!picofw_diag_watch_parse_args("20000 READ:", &dw));

    /* RAM & NVRAM byte writes pass; Spansion IPL NOR writes fail E05 guard. */
    assert(picofw_diag_watch_parse_args("20000 WRITE:50401000,AA", &dw));
    assert(picofw_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 13u);
    assert(strcmp(tx_buf, "W50401000,AA\r") == 0);
    assert(picofw_diag_watch_parse_args("20000 WRITE:847F0000,55", &dw));
    assert(!picofw_diag_watch_parse_args("20000 WRITE:80000000,AA", &dw));
    assert(!picofw_diag_watch_parse_args("20000 WRITE:847EFFFF,FF", &dw));
    assert(!picofw_diag_watch_parse_args("20000 WRITE:847F0000,1FF", &dw));

    assert(picofw_diag_watch_parse_args("20000 JUMP:0x50401F3C", &dw));
    assert(picofw_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 10u);
    assert(strcmp(tx_buf, "J50401F3C\r") == 0);

    assert(picofw_diag_watch_parse_args("20000 ERASE:847F0000", &dw));
    assert(picofw_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 10u);
    assert(strcmp(tx_buf, "E847F0000\r") == 0);
    assert(picofw_diag_watch_parse_args("20000 ERASE:807F2000", &dw));
    assert(!picofw_diag_watch_parse_args("20000 ERASE:847E0000", &dw));
    assert(!picofw_diag_watch_parse_args("20000 ERASE:50400000", &dw));

    /* Shared flash bounds helpers. */
    assert(picofw_flash_is_protected_ipl_range(0x80000000UL, 4u));
    assert(picofw_flash_is_protected_ipl_range(0x847EFFF0UL, 32u));
    assert(!picofw_flash_is_protected_ipl_range(0x847F0000UL, 16u));
    assert(picofw_flash_is_nvram_microsector_addr(0x807FFFFFUL));
    assert(!picofw_flash_is_nvram_microsector_addr(0x847EFFFFUL));

    /* DIAGWATCH 2-layer unlock/lock + two-step 'U' -> 'W'/'E' dispatch + SA0..SA3/RAM guards */
    picofw_flash_set_upper_nor_unlocked(false);
    assert(!picofw_diag_watch_parse_args("20000 WRITE:84200010,A5", &dw));
    assert(!picofw_diag_watch_parse_args("20000 ERASE:84200000", &dw));
    assert(!picofw_diag_watch_parse_args("20000 NORUNLOCK:BAD WRITE:84200010,A5", &dw));
    assert(!picofw_diag_watch_parse_args("20000 NORUNLOCK:CONFIRM_UPPER_NOR WRITE:84000100,AA", &dw));
    assert(!picofw_diag_watch_parse_args("20000 NORUNLOCK:CONFIRM_UPPER_NOR ERASE:84030000", &dw));
    assert(!picofw_diag_watch_parse_args("20000 NORUNLOCK:CONFIRM_UPPER_NOR ERASE:50400000", &dw));
    /* Adversarial PANAMON_TXN whitespace after ';' and >8-digit read span */
    assert(!picofw_diag_watch_parse_args("20000 \"PANAMON_TXN:R80000000; E80000000\"", &dw));
    assert(!picofw_diag_watch_parse_args("20000 PANAMON_TXN:RC0001000,000000010", &dw));
    assert(!picofw_diag_watch_parse_args("20000 PANAMON_TXN:J", &dw));

    assert(picofw_diag_watch_parse_args("20000 JIG NORUNLOCK:CONFIRM_UPPER_NOR WRITE:84200010,A5", &dw));
    assert(dw.nor_unlock && dw.action == PICOFW_DIAG_ACT_WRITE);
    assert(!picofw_flash_is_upper_nor_unlocked()); /* F3 pure parser */
    picofw_diag_watch_apply_policy(&dw);
    assert(picofw_flash_is_upper_nor_unlocked());
    picofw_diag_watch_state_t dwst;
    picofw_diag_watch_init(&dwst);
    int rc = 0, tc = 0;
    char ltx = '\0';
    feed_str_and_count(&dwst, &dw, "PANAMON1>", &rc, &tc, &ltx);
    assert(tc == 1 && ltx == 'U' && dwst.unlock_dispatched && !dwst.tx_dispatched);
    feed_str_and_count(&dwst, &dw, "\r\nOK:NOR_UNLOCK\r\nPANAMON1>", &rc, &tc, &ltx);
    assert(tc == 2 && ltx == 'W' && dwst.tx_dispatched);
    assert(picofw_diag_watch_parse_args("20000 ERASE:80200000 NORLOCK", &dw) == false);
    assert(picofw_diag_watch_parse_args("20000 NORLOCK WRITE:847F0010,55", &dw));
    assert(!dw.nor_unlock && dw.policy_req == PICOFW_POLICY_REQ_LOCK);
    picofw_diag_watch_apply_policy(&dw);
    assert(!picofw_flash_is_upper_nor_unlocked());

    /* One-shot NORUNLOCK auto-relock on 3rd PANAMON1> prompt emits 'L' + relock_now flag purely */
    assert(picofw_diag_watch_parse_args("20000 JIG NORUNLOCK:CONFIRM_UPPER_NOR WRITE:84200010,A5", &dw));
    assert(dw.nor_unlock && dw.policy_req == PICOFW_POLICY_REQ_UNLOCK);
    picofw_diag_watch_apply_policy(&dw);
    assert(picofw_flash_is_upper_nor_unlocked());
    picofw_diag_watch_init(&dwst);
    rc = 0; tc = 0; ltx = '\0';
    feed_str_and_count(&dwst, &dw, "PANAMON1>", &rc, &tc, &ltx);
    assert(tc == 1 && ltx == 'U');
    feed_str_and_count(&dwst, &dw, "\r\nOK:NOR_UNLOCK\r\nPANAMON1>", &rc, &tc, &ltx);
    assert(tc == 2 && ltx == 'W');
    feed_str_and_count(&dwst, &dw, "\r\nOK:WRITE\r\nPANAMON1>", &rc, &tc, &ltx);
    assert(tc == 3 && ltx == 'L' && dwst.relock_dispatched);
    bool relock_evt = false;
    picofw_diag_watch_state_t dwst2;
    picofw_diag_watch_init(&dwst2);
    const char *seq = "PANAMON1>PANAMON1>PANAMON1>";
    for (size_t k = 0u; seq[k] != '\0'; ++k) {
        bool rel = false;
        (void)picofw_diag_watch_feed_byte_ex(&dwst2, &dw, (uint8_t)seq[k], NULL, NULL, &rel);
        if (rel) relock_evt = true;
    }
    assert(relock_evt && picofw_flash_is_upper_nor_unlocked());
    picofw_flash_set_upper_nor_unlocked(false);
    assert(!picofw_diag_watch_parse_args("20000 WRITE:84200010,A5", &dw));
}

static void test_pana_rsp_regs_codec(void) {
    picofw_pana_rsp_spec_t spec;
    assert(picofw_panarsp_parse_args("5000 JIG:350 REGS", &spec));
    assert(spec.action == PANA_RSP_ACT_REGS && strcmp(spec.payload, "g") == 0);

    const uint32_t expected[12] = {
        0x11223344UL, 0x00000001UL, 0xCAFEBABEUL, 0xDEAD0003UL,
        0x50400000UL, 0x515B2E40UL, 0x847F0000UL, 0xA0001000UL,
        0x000000A5UL, 0x00000F00UL, 0x44007BA5UL, 0x51BFFFC0UL
    };
    char hex_payload[97];
    for (size_t i = 0u; i < 12u; ++i) {
        const uint32_t v = expected[i];
        const uint8_t le[4] = {
            (uint8_t)(v & 0xFFu),
            (uint8_t)((v >> 8) & 0xFFu),
            (uint8_t)((v >> 16) & 0xFFu),
            (uint8_t)((v >> 24) & 0xFFu)
        };
        for (size_t b = 0u; b < 4u; ++b) {
            hex_payload[(i * 8u) + (b * 2u)] = picofw_nibble_to_hex_char((uint8_t)(le[b] >> 4));
            hex_payload[(i * 8u) + (b * 2u) + 1u] = picofw_nibble_to_hex_char((uint8_t)(le[b] & 0x0Fu));
        }
    }
    hex_payload[96] = '\0';

    uint32_t decoded[12];
    assert(picofw_rsp_decode_mn103_regs((const uint8_t *)hex_payload, 96u, decoded));
    for (size_t i = 0u; i < 12u; ++i) {
        assert(decoded[i] == expected[i]);
    }
    assert(!picofw_rsp_decode_mn103_regs((const uint8_t *)hex_payload, 94u, decoded));

    /* Accept 224-hex-char (0x70 bytes = 28 words from `_dev_SERIAL` stub) `$g` replies */
    char hex_224[225];
    memcpy(hex_224, hex_payload, 96u);
    for (size_t i = 96u; i < 224u; ++i) {
        hex_224[i] = '0';
    }
    hex_224[224] = '\0';
    memset(decoded, 0, sizeof(decoded));
    assert(picofw_rsp_decode_mn103_regs((const uint8_t *)hex_224, 224u, decoded));
    for (size_t i = 0u; i < 12u; ++i) {
        assert(decoded[i] == expected[i]);
    }
    hex_224[220] = 'Z';
    assert(!picofw_rsp_decode_mn103_regs((const uint8_t *)hex_224, 224u, decoded));

    hex_payload[5] = 'Z';
    assert(!picofw_rsp_decode_mn103_regs((const uint8_t *)hex_payload, 96u, decoded));
}

static void test_drv_pana_codec(void) {
    picofw_drv_pana_spec_t drv;
    char wire[104];

    assert(picofw_drv_pana_parse_args("DSCREG", &drv));
    assert(drv.timeout_ms == 3000u && drv.boot_action == PICOFW_BOOT_ACTION_PASSIVE);
    assert(drv.send_pass && strcmp(drv.cmd, "DSCREG") == 0);
    assert(picofw_drv_pana_format_cmd(&drv, wire, sizeof(wire)) == 8u);
    assert(strcmp(wire, "DSCREG\r\n") == 0);

    assert(picofw_drv_pana_parse_args("4500 JIG:350 NOPASS START", &drv));
    assert(drv.timeout_ms == 4500u && drv.boot_action == PICOFW_BOOT_ACTION_JIG);
    assert(drv.jig_hold_ms == 350u && !drv.send_pass && strcmp(drv.cmd, "START") == 0);

    assert(picofw_drv_pana_parse_args("PASS 0x4d415453 0x48495441", &drv));
    assert(!drv.send_pass && strcmp(drv.cmd, "PASS 0x4d415453 0x48495441") == 0);

    assert(picofw_drv_pana_parse_args("PASS", &drv));
    assert(!drv.send_pass && strcmp(drv.cmd, "PASS") == 0);

    assert(picofw_drv_pana_parse_args("2000 \"FEP 10\"", &drv));
    assert(drv.timeout_ms == 2000u && drv.send_pass && strcmp(drv.cmd, "FEP 10") == 0);

    assert(!picofw_drv_pana_parse_args("", &drv));
    assert(!picofw_drv_pana_parse_args("4000 JIG:350", &drv));
}

static void test_diag_watch_stage_funnel_no_bgo_masking(void) {
    picofw_diag_watch_spec_t dw;
    assert(picofw_diag_watch_parse_args("10000 JIG NONE", &dw));

    /* Real COM10 scenario: SD 4-bit HW succeeds, Stage-1 fails SD diag load[0], then reboots to BGO fail + tray fail. */
    picofw_diag_watch_state_t st;
    picofw_diag_watch_init(&st);
    int rel_count = 0;
    int tx_count = 0;
    char last_tx = '\0';
    feed_str_and_count(
        &st, &dw,
        "DDR CHECK: OK\r\n"
        "***** Loader ver0.61 [06/03/06] *****\r\n"
        "***** serial boot mode *****\r\n"
        "SD = 0x3BB000[LBA]\r\n"
        "... SD init end\r\n"
        "SD diag load failed[0].\r\n"
        "***** normal mode *****\r\n"
        "BGO driver init failed[1].\r\n"
        "Main hung-up time changed 255[s].\r\n"
        "** can not close tray **\r\n",
        &rel_count, &tx_count, &last_tx
    );
    assert(rel_count == 1 && tx_count == 0);
    assert((st.seen_events & PICOFW_DIAG_EVT_SD_HW_OK) != 0u);
    assert((st.seen_events & PICOFW_DIAG_EVT_SD_DIAG_LOAD_FAIL) != 0u);
    assert((st.seen_events & PICOFW_DIAG_EVT_BGO_FAIL) != 0u);
    assert((st.seen_events & PICOFW_DIAG_EVT_ODD_TRAY_FAIL) != 0u);
    const char *diag = picofw_diag_watch_diagnosis(st.seen_events);
    assert(strstr(diag, "SD diag load failed[0]") != NULL);
    assert(strstr(diag, "SD HW OK") != NULL);
    assert(strstr(diag, "fat_read 0x20000 failed") != NULL);

    /* Uncompress failure stage also beats post-reset BGO. */
    picofw_diag_watch_init(&st);
    feed_str_and_count(
        &st, &dw,
        "***** serial boot mode *****\r\n... SD init end\r\ndiag loader uncompress failed...\r\nBGO driver init failed[1].\r\n",
        &rel_count, &tx_count, &last_tx
    );
    assert((st.seen_events & PICOFW_DIAG_EVT_SD_UNCOMPRESS_FAIL) != 0u);
    assert(strstr(picofw_diag_watch_diagnosis(st.seen_events), "EXTRHEAD/zlib unpack failed") != NULL);

    /* Stage-1 *** start diag *** sets DIAG_EXEC. */
    picofw_diag_watch_init(&st);
    feed_str_and_count(&st, &dw, "***** serial boot mode *****\r\n... SD init end\r\n*** start diag ***\r\n", &rel_count, &tx_count, &last_tx);
    assert((st.seen_events & PICOFW_DIAG_EVT_DIAG_EXEC) != 0u);
    assert(strstr(picofw_diag_watch_diagnosis(st.seen_events), "decompressed") != NULL);
}

static void test_diag_watch_wake_and_ir_kaseikyo(void) {
    picofw_diag_watch_spec_t dw;
    assert(picofw_diag_watch_parse_args("20000 JIG:2500 AUTO RELEASE WAKE:45000", &dw));
    assert(dw.wait_for_wake && dw.wake_timeout_ms == 45000u);
    assert(picofw_diag_watch_parse_args("15000 NOPULL NONE WAKE", &dw));
    assert(dw.wait_for_wake && dw.wake_timeout_ms == 30000u);
    assert(!picofw_diag_watch_parse_args("15000 WAKE:bad", &dw));

    /* Verify Panasonic Kaseikyo IR frame synthesis vs Flipper-IRDB captures & pin safety. */
    picofw_ir_spec_t ir;
    assert(picofw_ir_parse_args("RED", &ir));
    assert(ir.gpio == 6u && ir.repeat == 2u);
    assert(ir.frame[0] == 0x02u && ir.frame[1] == 0x20u && ir.frame[2] == 0xB0u);
    assert(ir.frame[3] == 0x01u && ir.frame[4] == 0x41u && ir.frame[5] == 0xF0u);

    assert(picofw_ir_parse_args("AUDIO GP16 3", &ir));
    assert(ir.gpio == 16u && ir.repeat == 3u);
    assert(ir.frame[3] == 0x00u && ir.frame[4] == 0x33u && ir.frame[5] == 0x83u);

    assert(picofw_ir_parse_args("ND2 22 1", &ir));
    assert(ir.gpio == 22u && ir.repeat == 1u);
    assert(ir.frame[3] == 0x01u && ir.frame[4] == 0x82u && ir.frame[5] == 0x33u);

    assert(picofw_ir_parse_args("POWER", &ir));
    assert(ir.frame[3] == 0x00u && ir.frame[4] == 0x3Du && ir.frame[5] == 0x8Du);

    assert(picofw_ir_parse_args("B001:41 7", &ir));
    assert(ir.frame[2] == 0xB0u && ir.frame[3] == 0x01u && ir.frame[4] == 0x41u && ir.frame[5] == 0xF0u);

    /* Remote Modes 2/3 preserve upper bits of sub_byte (`0x20`/`0x21`/`0x30`) and default `dev = 0xB0` for <hex2>:<cmd> */
    uint8_t kmode_frame[PICOFW_IR_KASEIKYO_FRAME_BYTES];
    picofw_ir_encode_kaseikyo(0xB0u, 0x20u, 0x3Du, kmode_frame);
    assert(kmode_frame[3] == 0x20u && kmode_frame[5] == (uint8_t)(0xB0u ^ 0x20u ^ 0x3Du));

    assert(picofw_ir_parse_args("20:3D", &ir));
    assert(ir.frame[2] == 0xB0u && ir.frame[3] == 0x20u && ir.frame[4] == 0x3Du && ir.frame[5] == 0xADu);

    assert(picofw_ir_parse_args("21:41", &ir));
    assert(ir.frame[2] == 0xB0u && ir.frame[3] == 0x21u && ir.frame[4] == 0x41u && ir.frame[5] == 0xD0u);

    assert(picofw_ir_parse_args("30:3D", &ir));
    assert(ir.frame[2] == 0xB0u && ir.frame[3] == 0x30u && ir.frame[4] == 0x3Du && ir.frame[5] == 0xBDu);

    assert(picofw_ir_parse_args("B021:41", &ir));
    assert(ir.frame[2] == 0xB0u && ir.frame[3] == 0x21u && ir.frame[4] == 0x41u && ir.frame[5] == 0xD0u);

    /* P7501 GP2..5, SD GP10..15, and ADC/LED GP25..26 MUST be blocked. */
    assert(!picofw_ir_parse_args("RED GP3", &ir));
    assert(!picofw_ir_parse_args("RED GP5", &ir));
    assert(!picofw_ir_parse_args("RED GP10", &ir));
    assert(!picofw_ir_parse_args("RED GP26", &ir));

    uint32_t pulses[PICOFW_IR_PULSE_COUNT];
    assert(picofw_ir_build_pulse_table(ir.frame, pulses, PICOFW_IR_PULSE_COUNT) == PICOFW_IR_PULSE_COUNT);
    picofw_ir_decode_result_t dec;
    assert(picofw_ir_decode_pulses(pulses, PICOFW_IR_PULSE_COUNT, &dec));
    assert(dec.valid && dec.vendor_ok && dec.parity_ok);
    assert(memcmp(dec.frame, ir.frame, PICOFW_IR_KASEIKYO_FRAME_BYTES) == 0);

    /* IR VERIFY 48-bit hex validator: valid frame, bad checksum, bad OEM/nibble, malformed length */
    assert(picofw_ir_verify_hex_frame("0220B00141F0", &dec));
    assert(dec.valid && dec.vendor_ok && dec.parity_ok && dec.dev == 0xB0u && dec.cmd == 0x41u);
    assert(picofw_ir_verify_hex_frame("0220B00141FF", &dec));
    assert(!dec.valid && dec.vendor_ok && !dec.parity_ok);
    assert(picofw_ir_verify_hex_frame("0221B00141F0", &dec));
    assert(!dec.valid && !dec.vendor_ok);
    assert(!picofw_ir_verify_hex_frame("0220B0", &dec));
}

#include "common/picofw_cmd_arena.h"
#include "common/picofw_spsc_ring.h"
#include "diag/picofw_i2c_codec.h"
#include "diag/picofw_identify_codec.h"
#include "diag/picofw_mmio_snapshot_codec.h"
#include "diag/picofw_pwrosc_codec.h"
#include "diag/picofw_sds_codec.h"
#include "diag/picofw_serv_macro_codec.h"
#include "diag/picofw_tbus_codec.h"
#include "flash/picofw_blackbox_codec.h"
#include "flash/picofw_ftl_inspect.h"
#include "flash/picofw_job_codec.h"
#include "flash/picofw_probe_cfg_codec.h"
#include "flash/picofw_qspi_map.h"
#include "flash/picofw_vault_codec.h"
#include "hal/picofw_bootsel_logic.h"
#include "hal/picofw_wireless_codec.h"
#include "uart/picofw_pana_dumpnor.h"
#include "uart/picofw_pana_jedec.h"

static void test_smart_probe_qspi_arena_and_zero_float(void) {
    assert(sizeof(g_picofw_cmd_arena) == PICOFW_CMD_ARENA_BYTES);
    char vbuf[8];
    assert(strcmp(picofw_fmt_mv_volts(0u, vbuf), "0.00") == 0);
    assert(strcmp(picofw_fmt_mv_volts(3295u, vbuf), "3.30") == 0);
    assert(strcmp(picofw_fmt_mv_volts(3281u, vbuf), "3.28") == 0);

    /* P0..P5 QSPI Partition Map: P0 permanently read-only */
    assert(!picofw_qspi_is_writable_range(
        PICOFW_QSPI_PART_FW_RO, 0x000000UL, 4096UL, PICOFW_QSPI_BASE_2MB_BYTES));
    assert(!picofw_qspi_is_writable_range(
        PICOFW_QSPI_PART_VAULT, 0x000000UL, 4096UL, PICOFW_QSPI_BASE_2MB_BYTES));
    assert(picofw_qspi_is_writable_range(
        PICOFW_QSPI_PART_VAULT, PICOFW_QSPI_VAULT_BANK_A_OFFSET, PICOFW_QSPI_VAULT_BANK_BYTES, PICOFW_QSPI_BASE_2MB_BYTES));
    assert(picofw_qspi_is_writable_range(
        PICOFW_QSPI_PART_VAULT, PICOFW_QSPI_VAULT_BANK_B_OFFSET, PICOFW_QSPI_VAULT_BANK_BYTES, PICOFW_QSPI_BASE_2MB_BYTES));

    /* P2_VAULT A/B offsets & manifest CRC32 */
    assert(picofw_vault_bank_offset(PICOFW_QSPI_BASE_2MB_BYTES, 0u) == 0x100000UL);
    assert(picofw_vault_bank_offset(PICOFW_QSPI_BASE_2MB_BYTES, 1u) == 0x140000UL);
    uint32_t slot_off = 0UL;
    assert(picofw_vault_slot_qspi_offset_ab(PICOFW_QSPI_BASE_2MB_BYTES, 1u, 2u, &slot_off));
    assert(slot_off == 0x160000UL);
    picofw_vault_manifest_t vm;
    picofw_vault_manifest_init(&vm);
    const uint8_t sample_nv[16] = { 0x12u, 0x34u, 0x56u, 0x78u };
    assert(picofw_vault_manifest_record_slot(&vm, 1u, 0x807F0000UL, sample_nv, sizeof(sample_nv), "DMR-EH55"));
    assert(picofw_vault_manifest_verify(&vm));

    /* P4_BLACKBOX wear-leveled ring & CRC32 */
    uint32_t seqs[PICOFW_BLACKBOX_PAGES];
    memset(seqs, 0, sizeof(seqs));
    seqs[0] = 10u;
    seqs[1] = 11u;
    uint32_t next_seq = 0u;
    assert(picofw_blackbox_select_next_page(seqs, &next_seq) == 2u);
    assert(next_seq == 12u);
    picofw_blackbox_rec_t rec;
    picofw_blackbox_build_record(&rec, next_seq, 1500u, 3290u, 0u, 0x05u, "DMR-EH55", "TEST_SNAP");
    assert(picofw_blackbox_verify_record(&rec));

    /* P5_PROBE_CFG A/B selection, ADC0 mV calibration, and P3_RECIPE CRC32 */
    picofw_probe_cfg_t cfg_a, cfg_b, cfg_act;
    picofw_probe_cfg_init(&cfg_a, PICOFW_SSOT_HASH_STR);
    picofw_probe_cfg_init(&cfg_b, PICOFW_SSOT_HASH_STR);
    cfg_b.sequence = 5u;
    cfg_b.adc0_gain_ppm = 1010000UL;
    cfg_b.adc0_offset_mv = -15;
    picofw_probe_cfg_resign(&cfg_b);
    assert(picofw_probe_cfg_select_ab(&cfg_a, &cfg_b, &cfg_act) == 1);
    assert(picofw_probe_cfg_apply_adc_cal_mv(3300u, &cfg_act) == 3318u);

    picofw_recipe_hdr_t rcp;
    picofw_recipe_hdr_init(&rcp, "EH55_BGO_V1", "DMR-EH55", sample_nv, sizeof(sample_nv));
    assert(picofw_recipe_hdr_verify(&rcp, sample_nv, sizeof(sample_nv)));
}

static void test_identify_job_f14_and_bootsel(void) {
    picofw_session_ctx_reset(picofw_session_default_ctx());
    picofw_wireless_state_init(picofw_wireless_global_state());

    /* IDENTIFY Stage 2: "ver0.18" banner auto-selects DMR-ES35V and locks MAIN_XIP Read-Only */
    const char *es35_banner = "DDR CHECK: OK\r\n***** Loader ver0.18 *****\r\n";
    picofw_ident_report_t rep;
    picofw_identify_evaluate(
        3290u, 3280u, true, true, true,
        (const uint8_t *)es35_banner, strlen(es35_banner),
        0x0001u, 0x227Eu, 0x11223344UL, true, &rep
    );
    assert(rep.matched_profile == PICOFW_TARGET_PROFILE_DMR_ES35V);
    assert(rep.xip_main_ro_locked);
    /* DMR-ES35V MAIN_XIP (0x130000..0x40FFFF) blocked even if Upper NOR unlocked */
    assert(picofw_flash_is_write_blocked_with_policy(0x84200000UL, 4u, true));
    /* Restore default EH55 profile */
    assert(picofw_flash_set_target_profile_by_id(PICOFW_TARGET_PROFILE_DMR_EH55));

    char ident_line[320];
    assert(picofw_identify_format_line(&rep, ident_line, sizeof(ident_line)) > 40u);
    assert(strstr(ident_line, "@PICO_IDENT ") == ident_line);

    /* F14 Wireless NOR Guard + JOB Policy */
    picofw_wireless_auth_state_t *ws = picofw_wireless_global_state();
    picofw_job_spec_t job;
    const char *err_code = NULL;
    const char *reason = NULL;
    assert(picofw_job_parse_args("NVRAM_CLEAN", &job) && !job.confirmed);
    assert(!picofw_job_check_policy(&job, NULL, ws, &err_code, &reason));
    assert(strcmp(err_code, "E05") == 0);

    assert(picofw_job_parse_args("NVRAM_CLEAN CONFIRM", &job) && job.confirmed);
    assert(picofw_job_check_policy(&job, NULL, ws, &err_code, &reason));

    /* Switch origin to BLE_NUS: NOR writes MUST fail-closed (E05) */
    ws->active_origin = PICOFW_ORIGIN_BLE_NUS;
    assert(!picofw_job_check_policy(&job, NULL, ws, &err_code, &reason));
    assert(strcmp(err_code, "E05") == 0);
    assert(picofw_flash_is_write_blocked(0x807F0000UL, 4u));
    /* Wireless origin cannot self-authorize */
    assert(!picofw_wireless_handle_usb_auth_cmd(ws, PICOFW_ORIGIN_BLE_NUS, "CONFIRM 1"));
    /* USB CDC grants 1 op, then auto-relocks after consumption */
    assert(picofw_wireless_handle_usb_auth_cmd(ws, PICOFW_ORIGIN_USB_CDC, "CONFIRM 1"));
    assert(picofw_job_check_policy(&job, NULL, ws, &err_code, &reason));
    picofw_wireless_consume_nor_op(ws);
    assert(!picofw_wireless_nor_write_allowed(ws));
    picofw_wireless_state_init(ws);

    /* BOOTSEL short-press vs 3s long-press FSM */
    picofw_bootsel_fsm_t bfsm;
    picofw_bootsel_fsm_init(&bfsm);
    assert(picofw_bootsel_fsm_step(&bfsm, true, 100u) == PICOFW_BOOTSEL_EVT_NONE);
    assert(picofw_bootsel_fsm_step(&bfsm, false, 350u) == PICOFW_BOOTSEL_EVT_SHORT_IDENTIFY);
    assert(picofw_bootsel_fsm_step(&bfsm, true, 1000u) == PICOFW_BOOTSEL_EVT_NONE);
    assert(picofw_bootsel_fsm_step(&bfsm, true, 4100u) == PICOFW_BOOTSEL_EVT_LONG_VAULT_BACKUP);
    assert(picofw_bootsel_fsm_step(&bfsm, false, 4200u) == PICOFW_BOOTSEL_EVT_NONE);

    /* MMIO_SNAP 0xC000100C exclusion, SERVMACRO, SDSNIFF triggers, DRVPANA DIAG, SPSC ring */
    assert(!picofw_mmio_is_safe_read_addr(0xC000100CUL));
    assert(picofw_mmio_is_safe_read_addr(0xC0001008UL));
    picofw_serv_macro_id_t mid;
    uint32_t mgpio = 0u;
    assert(picofw_serv_macro_parse("SELF_CHECK GP16", &mid, &mgpio) && mgpio == 16u);
    assert(!picofw_serv_macro_parse("SELF_CHECK GP3", &mid, &mgpio));
    picofw_sds_spec_t sds_spec;
    assert(picofw_sds_parse_args("4000 JIG TRIG:91006", &sds_spec));
    assert(sds_spec.trigger == PICOFW_SDS_TRIG_91006);
    picofw_drv_pana_spec_t dspec;
    assert(picofw_drv_pana_parse_args("DIAG", &dspec) && dspec.diag_macro);
    picofw_spsc_ring_t ring;
    uint8_t ring_buf[16];
    assert(picofw_spsc_init(&ring, ring_buf, sizeof(ring_buf)));
    assert(picofw_spsc_push(&ring, 0x5Au));
    uint8_t out_b = 0u;
    assert(picofw_spsc_pop(&ring, &out_b) && out_b == 0x5Au);

    /* BGO 20B EH55 header format, 16-bit EEPROM offset, offline IDENTIFY, and NVRAM range stub */
    uint8_t bgo_hdrs[3u * PICOFW_BGO_HEADER_BYTES];
    memset(bgo_hdrs, 0xFF, sizeof(bgo_hdrs));
    for (size_t k = 0u; k < 4u; ++k) {
        const size_t o = 4u + (k * 4u);
        bgo_hdrs[o] = 0x01u;
        bgo_hdrs[o + 1u] = 0xF1u;
        bgo_hdrs[PICOFW_BGO_HEADER_BYTES + o] = 0x01u;
        bgo_hdrs[PICOFW_BGO_HEADER_BYTES + o + 1u] = 0xF2u;
    }
    picofw_bgo_report_t bgo_rep;
    assert(picofw_bgo_inspect_headers(bgo_hdrs, 3u, &bgo_rep));
    assert(bgo_rep.healthy_bgo_ftl && bgo_rep.valid_count == 2u && bgo_rep.erased_count == 1u);
    bgo_hdrs[8] = 0x99u;
    assert(picofw_bgo_inspect_headers(bgo_hdrs, 3u, &bgo_rep));
    assert(!bgo_rep.healthy_bgo_ftl && bgo_rep.corrupt_count >= 1u);
    memset(bgo_hdrs, 0xFF, sizeof(bgo_hdrs));
    assert(picofw_bgo_inspect_headers(bgo_hdrs, 3u, &bgo_rep));
    assert(!bgo_rep.healthy_bgo_ftl && bgo_rep.valid_count == 0u);

    picofw_eeprom_spec_t espec;
    assert(picofw_eeprom_parse_args("READ 0x50 0x1000 16", &espec));
    assert(espec.addr_16bit && espec.offset == 0x1000u && espec.len == 16u);

    picofw_identify_evaluate(0u, 0u, false, false, false, NULL, 0u, 0u, 0u, 0u, false, &rep);
    assert(rep.caps_mask == 0u && strcmp(rep.target_state, "STANDBY_OR_OFF") == 0);

    picofw_m0_keepalive_action_t ka_act = PICOFW_M0_KA_STATUS;
    uint32_t ka_ms = 0u;
    assert(picofw_tbus_parse_keepalive_args("", &ka_act, &ka_ms) &&
           ka_act == PICOFW_M0_KA_ON && ka_ms == PICOFW_M0_KEEPALIVE_DEFAULT_MS);
    assert(picofw_tbus_parse_keepalive_args("ON 2500", &ka_act, &ka_ms) &&
           ka_act == PICOFW_M0_KA_ON && ka_ms == 2500u);
    assert(picofw_tbus_parse_keepalive_args("500", &ka_act, &ka_ms) &&
           ka_act == PICOFW_M0_KA_ON && ka_ms == PICOFW_M0_KEEPALIVE_MIN_MS);
    assert(picofw_tbus_parse_keepalive_args("OFF", &ka_act, &ka_ms) &&
           ka_act == PICOFW_M0_KA_OFF);
    assert(picofw_tbus_parse_keepalive_args("STATUS", &ka_act, &ka_ms) &&
           ka_act == PICOFW_M0_KA_STATUS);
    assert(!picofw_tbus_parse_keepalive_args("BOGUS", &ka_act, &ka_ms));

    uint8_t rstub[PICOFW_JEDEC_MAX_STUB_SIZE];
    const size_t rlen = picofw_jedec_build_erase_range_stub(
        0x807F0000UL, 8192UL, 8u, PICOFW_JEDEC_STUB_ADDR, rstub, sizeof(rstub)
    );
    assert(rlen > 120u && rlen <= PICOFW_JEDEC_MAX_STUB_SIZE);
    bool seen_add_a2 = false;
    bool seen_cmp_a2 = false;
    for (size_t i = 0u; (i + 1u) < rlen; ++i) {
        if (rstub[i] == 0xFCu && rstub[i + 1u] == 0xD2u) {
            seen_add_a2 = true;
        } else if (rstub[i] == 0xFCu && rstub[i + 1u] == 0xDAu) {
            seen_cmp_a2 = true;
        }
    }
    assert(seen_add_a2 && seen_cmp_a2);
    assert(picofw_jedec_build_erase_range_stub(0x80000000UL, 8192UL, 4u, PICOFW_JEDEC_STUB_ADDR, rstub, sizeof(rstub)) == 0u);
    assert(picofw_jedec_build_erase_range_stub(0x80200000UL, 65536UL, 2u, PICOFW_JEDEC_STUB_ADDR, rstub, sizeof(rstub)) == 0u);
}

static void test_vault_ab_p6_mmio_pwrosc_and_disconnect_relock(void) {
    /* 1. Atomic A/B Vault selection & clear_slot */
    picofw_vault_manifest_t va, vb, vout;
    picofw_vault_manifest_init(&va);
    picofw_vault_manifest_init(&vb);
    const uint8_t d1[8] = { 1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u };
    const uint8_t d2[8] = { 9u, 8u, 7u, 6u, 5u, 4u, 3u, 2u };
    assert(picofw_vault_manifest_record_slot(&va, 1u, 0x807F0000UL, d1, sizeof(d1), "DMR-EH55"));
    vb = va;
    assert(picofw_vault_manifest_record_slot(&vb, 1u, 0x807F0000UL, d2, sizeof(d2), "DMR-EH55"));
    assert(va.write_count == 1u && vb.write_count == 2u);
    assert(picofw_vault_select_ab(&va, &vb, &vout) == 1);
    assert(vout.write_count == vb.write_count);
    /* Corrupt Bank B CRC -> automatic fallback to intact Bank A */
    vb.header_crc32 ^= 0xFFu;
    assert(picofw_vault_select_ab(&va, &vb, &vout) == 0);
    assert(vout.write_count == va.write_count);

    /* Vault slot payload verify + non-0xFF count + ECRC detection */
    uint8_t slot_pay[16];
    memset(slot_pay, 0xFF, sizeof(slot_pay));
    slot_pay[0] = 0x12u; slot_pay[1] = 0x34u;
    assert(picofw_vault_manifest_record_slot(&va, 2u, 0x807F0000UL, slot_pay, sizeof(slot_pay), "DMR-EH55"));
    uint32_t out_crc = 0u, out_non_ff = 0u;
    bool crc_bad = false;
    assert(picofw_vault_verify_slot_buffer(&va, 2u, slot_pay, sizeof(slot_pay), &out_crc, &out_non_ff, &crc_bad));
    assert(out_non_ff == 2u && !crc_bad && out_crc == va.slots[1].data_crc32);
    slot_pay[3] = 0x55u;
    assert(!picofw_vault_verify_slot_buffer(&va, 2u, slot_pay, sizeof(slot_pay), &out_crc, &out_non_ff, &crc_bad));
    assert(crc_bad && out_non_ff == 3u);
    assert(!picofw_vault_verify_slot_buffer(&va, 3u, slot_pay, sizeof(slot_pay), &out_crc, &out_non_ff, &crc_bad));
    assert(!crc_bad);

    assert(picofw_vault_manifest_clear_slot(&va, 1u));
    assert(va.slots[0].valid == 0u && picofw_vault_manifest_verify(&va));

    /* 2. P6_NOR_MIRROR active on 4 MiB RP2350, disabled (size=0) on 2 MiB RP2040 */
    picofw_qspi_part_info_t r2, r4;
    assert(picofw_qspi_partition_info(PICOFW_QSPI_PART_NOR_MIRROR, PICOFW_QSPI_BASE_2MB_BYTES, &r2));
    assert(r2.size == 0u && !r2.writable);
    assert(picofw_qspi_partition_info(PICOFW_QSPI_PART_NOR_MIRROR, PICOFW_QSPI_EXP_4MB_BYTES, &r4));
    assert(r4.offset == 0x200000UL && r4.size == 0x200000UL && r4.writable);

    /* 3. MMIO_SNAP table strictly 32-bit aligned (SC0CTR @ 0xD4002000, never 0xD4002009) */
    size_t mmio_cnt = 0u;
    const picofw_mmio_reg_desc_t *tbl = picofw_mmio_safe_table(&mmio_cnt);
    assert(mmio_cnt == PICOFW_MMIO_SNAP_COUNT);
    for (size_t i = 0u; i < mmio_cnt; ++i) {
        assert((tbl[i].addr & 0x03UL) == 0u);
        assert(picofw_mmio_is_safe_read_addr(tbl[i].addr));
    }

    /* 4. PWROSC zero-sample finalize resets min_mv from sentinel to 0 */
    picofw_pwrosc_stats_t ps;
    picofw_pwrosc_stats_init(&ps, 25000);
    picofw_pwrosc_stats_finalize(&ps);
    assert(ps.sample_count == 0u && ps.min_mv == 0u && ps.max_mv == 0u && ps.avg_mv == 0u);

    /* 5. Wireless REVOKE and disconnect auto-relock Upper NOR */
    picofw_wireless_auth_state_t *ws = picofw_wireless_global_state();
    picofw_wireless_state_init(ws);
    picofw_flash_set_upper_nor_unlocked(true);
    assert(picofw_wireless_handle_usb_auth_cmd(ws, PICOFW_ORIGIN_USB_CDC, "REVOKE"));
    assert(!picofw_flash_is_upper_nor_unlocked());
    picofw_flash_set_upper_nor_unlocked(true);
    picofw_wireless_on_disconnect(ws);
    assert(!picofw_flash_is_upper_nor_unlocked());
}

static void test_boottrace_baremetal_classifier(void) {
    picofw_boot_classifier_t cls;
    char buf[224];

    /* 1. Clean MAIN 3060NP HDD autoboot */
    picofw_boot_classifier_init(&cls);
    picofw_boot_classifier_feed_line(&cls, "[HDD_AUTOBOOT] Boot MAIN @ 0x57000000");
    picofw_boot_classifier_feed_line(&cls, "  DVD Recorder DMR5x Series System Version : 3060NP (2006/04/27)");
    picofw_boot_classifier_feed_line(&cls, "[SYS_BOOT]AVOUT_init() success.");
    picofw_boot_classifier_feed_line(&cls, "[DCS]dco_api :AVOUT_V_Mute: onoff=0");
    picofw_boot_classifier_feed_line(&cls, ":::SYS_RSV::: >> nfrom back up ok.");
    assert(picofw_boot_classifier_verdict(&cls) == PICOFW_BOOT_VERDICT_CLEAN_MAIN_OK);
    (void)picofw_boot_classifier_format_summary(&cls, buf, sizeof(buf));
    assert(strstr(buf, "verdict=CLEAN_MAIN_OK build=3060NP hdd_autoboot=1 avout_ok=1 vmute0=1") != NULL);
    assert(strstr(buf, "rsv_ok=1 hdd_fail=0 bgo_err=0 fpc_err=0") != NULL);
    assert(strstr(buf, "op7f_cnt=0 op7f=NONE") != NULL);

    /* 2. European MAIN 3410PQ on Brazilian IC7501 (MN67788RA) */
    picofw_boot_classifier_init(&cls);
    picofw_boot_classifier_feed_line(&cls, "[HDD_AUTOBOOT] Boot MAIN @ 0x57000000");
    picofw_boot_classifier_feed_line(&cls, "  DVD Recorder DMR5x Series System Version : 3410PQ (2006/10/20)");
    picofw_boot_classifier_feed_line(&cls, "[FPC]FPC_ReciveTASK() : 7F A9 01 01 00 00 0E 06 00 02 00 00 00 00 00 40");
    picofw_boot_classifier_feed_line(&cls, "[FPC]FPC_ReciveTASK() : 7F F9 02 11 00 06 82 32 00 00 00 00 00 00 00 45");
    picofw_boot_classifier_feed_line(&cls, "[FPC]FPC_ReciveTASK() : 7F A9 01 01 00 00 0E 06 00 02 00 00 00 00 00 40");
    picofw_boot_classifier_feed_line(&cls, "[SYS_CNTL]FPC_EMERGENCY_ERROR");
    picofw_boot_classifier_feed_line(&cls, "[SYS_CNTL]FPC_EMERGENCY_TIMEOUT");
    picofw_boot_classifier_feed_line(&cls, "[AVIN] target:0 input:0x0 --IIC send error retry 3 all err");
    picofw_boot_classifier_feed_line(&cls, "[RTSD] === DRIVE ACCESS ERROR === ( 80040904 )");
    assert(picofw_boot_classifier_verdict(&cls) == PICOFW_BOOT_VERDICT_FPC_IC7501_MISMATCH);
    (void)picofw_boot_classifier_format_summary(&cls, buf, sizeof(buf));
    assert(strstr(buf, "verdict=FPC_IC7501_MISMATCH build=3410PQ") != NULL);
    assert(strstr(buf, "fpc_err=1 fpc_to=1 iic_err=1 odd_err=1 op7f_cnt=2 op7f=A9,F9") != NULL);

    /* 3. L1489 BGO/NFROM failure */
    picofw_boot_classifier_init(&cls);
    picofw_boot_classifier_feed_line(&cls, "[nfrom] bgo_writeD(ID=0xF07) return error(19).");
    picofw_boot_classifier_feed_line(&cls, "[SYS_CNTL]##### NFROMError Power off !!!!! #####");
    assert(picofw_boot_classifier_verdict(&cls) == PICOFW_BOOT_VERDICT_BGO_NFROM_FAULT);

    /* 4. Cold SATA bridge timeout without HDD autoboot hook */
    picofw_boot_classifier_init(&cls);
    picofw_boot_classifier_feed_line(&cls, "HDD_MAX_LBA get failed[0xE8000000].");
    assert(picofw_boot_classifier_verdict(&cls) == PICOFW_BOOT_VERDICT_HDD_BRIDGE_WAIT_FAIL);
}

static void test_boot_cycle_follow_fsm(void) {
    picofw_boot_cycle_t cy;
    const uint64_t ms = 1000u;

    /* Attach while target is already running: cycle 1 opens at attach time (idempotent start). */
    picofw_boot_cycle_init(&cy, 8000u, PICOFW_CTRL3_NSW5V | PICOFW_CTRL3_XRESET);
    assert(!cy.active);
    picofw_boot_cycle_attach(&cy, 5000u * ms);
    assert(cy.active && (cy.cycle_no == 1u) && (cy.start_reason == PICOFW_CYCLE_REASON_ATTACH));
    assert(!cy.had_previous);
    assert(picofw_boot_cycle_on_line(&cy, 5100u * ms, "[HDD_AUTOBOOT] Boot MAIN") == PICOFW_CYCLE_EV_NONE);
    assert(cy.lines_in_cycle == 1u);
    assert(picofw_boot_cycle_rel_ms(&cy, 5100u * ms) == 100u);
    assert(picofw_boot_cycle_rel_ms(&cy, 4000u * ms) == 0u);

    /* Real reset: XRESET falls -> new cycle, previous snapshot preserved, clock restarts at +0. */
    assert(picofw_boot_cycle_on_ctrl(&cy, 6000u * ms, PICOFW_CTRL3_NSW5V) == PICOFW_CYCLE_EV_START);
    assert(cy.cycle_no == 2u && cy.had_previous && (cy.prev_cycle_no == 1u) && (cy.prev_lines == 1u));
    assert(cy.start_reason == PICOFW_CYCLE_REASON_XRESET_FALL);
    assert(picofw_boot_cycle_rel_us(&cy, 6000u * ms + 10u * ms) == 10000u);

    /* XRESET release (rising) must not open a cycle; banner right after the edge only confirms it. */
    assert(picofw_boot_cycle_on_ctrl(&cy, 6050u * ms, PICOFW_CTRL3_NSW5V | PICOFW_CTRL3_XRESET) == PICOFW_CYCLE_EV_NONE);
    assert(picofw_boot_cycle_on_line(&cy, 6060u * ms, "DDR CHECK: OK") == PICOFW_CYCLE_EV_NONE);
    assert(cy.cycle_no == 2u && cy.lines_in_cycle == 1u);

    /* Banner without any control edge (reset not wired/observed) still restarts the cycle at +0. */
    assert(picofw_boot_cycle_on_line(&cy, 7000u * ms, "Chip select = 1") == PICOFW_CYCLE_EV_NONE);
    assert(picofw_boot_cycle_on_line(&cy, 9000u * ms, "DDR CHECK: OK") == PICOFW_CYCLE_EV_START);
    assert(cy.cycle_no == 3u && cy.had_previous && (cy.prev_cycle_no == 2u) && (cy.prev_lines == 2u));
    assert(cy.start_reason == PICOFW_CYCLE_REASON_BOOT_BANNER);
    assert(picofw_boot_cycle_rel_ms(&cy, 9000u * ms) == 0u);

    /* Idle report fires once after silence, then stays quiet until new lines arrive. */
    assert(picofw_boot_cycle_tick(&cy, 12000u * ms) == PICOFW_CYCLE_EV_NONE);
    assert(picofw_boot_cycle_tick(&cy, 17000u * ms) == PICOFW_CYCLE_EV_IDLE_REPORT);
    assert(picofw_boot_cycle_tick(&cy, 30000u * ms) == PICOFW_CYCLE_EV_NONE);
    assert(picofw_boot_cycle_on_line(&cy, 31000u * ms, "late line") == PICOFW_CYCLE_EV_NONE);
    assert(picofw_boot_cycle_tick(&cy, 40000u * ms) == PICOFW_CYCLE_EV_IDLE_REPORT);

    /* Standby (NSW5V falls) ends the cycle; UART text afterwards opens an ACTIVITY cycle. */
    assert(picofw_boot_cycle_on_ctrl(&cy, 41000u * ms, PICOFW_CTRL3_XRESET) == PICOFW_CYCLE_EV_END);
    assert(!cy.active && (cy.end_reason == PICOFW_CYCLE_REASON_STANDBY_FALL));
    assert(picofw_boot_cycle_on_ctrl(&cy, 41500u * ms, PICOFW_CTRL3_XRESET) == PICOFW_CYCLE_EV_NONE);
    assert(picofw_boot_cycle_tick(&cy, 99000u * ms) == PICOFW_CYCLE_EV_NONE);

    /* Wake from standby (NSW5V rises) opens the next cycle. */
    assert(picofw_boot_cycle_on_ctrl(&cy, 50000u * ms, PICOFW_CTRL3_XRESET | PICOFW_CTRL3_NSW5V) == PICOFW_CYCLE_EV_START);
    assert(cy.cycle_no == 4u && !cy.had_previous && (cy.start_reason == PICOFW_CYCLE_REASON_WAKE_RISE));

    /* Attach in standby: nothing open until a real event; text while closed opens ACTIVITY. */
    picofw_boot_cycle_init(&cy, 8000u, 0u);
    assert(picofw_boot_cycle_on_line(&cy, 100u * ms, "stray line") == PICOFW_CYCLE_EV_START);
    assert(cy.cycle_no == 1u && (cy.start_reason == PICOFW_CYCLE_REASON_ACTIVITY));
    assert(strcmp(picofw_boot_cycle_reason_name(PICOFW_CYCLE_REASON_XRESET_FALL), "XRESET_FALL") == 0);
}

static void test_tbus_br_ic7501_audit(void) {
    static const uint8_t k_expected_rejected[13] = {
        0x85u, 0x89u, 0x9Bu, 0xA9u, 0xAAu, 0xC9u, 0xDAu,
        0xE3u, 0xE4u, 0xECu, 0xF0u, 0xF9u, 0xFAu
    };
    for (size_t i = 0u; i < sizeof(k_expected_rejected); ++i) {
        assert(picofw_tbus_opcode_rejected_by_br_ic7501(k_expected_rejected[i]));
    }
    assert(!picofw_tbus_opcode_rejected_by_br_ic7501(0x10u));
    assert(!picofw_tbus_opcode_rejected_by_br_ic7501(0x20u));

    picofw_tbus_audit_t audit;
    picofw_tbus_audit_init(&audit);

    picofw_tbus_frame_t ok_frm = { PICOFW_TBUS_SYNC_BYTE, 0x10u, 2u, { 0x01u, 0x01u }, 0u };
    picofw_tbus_frame_t eu_frm = { PICOFW_TBUS_SYNC_BYTE, 0xA9u, 1u, { 0x01u }, 0u };
    picofw_tbus_frame_t nak_frm = { PICOFW_TBUS_SYNC_BYTE, 0x7Fu, 1u, { 0xF9u }, 0u };
    picofw_tbus_audit_record_frame(&audit, &ok_frm);
    picofw_tbus_audit_record_frame(&audit, &eu_frm);
    picofw_tbus_audit_record_frame(&audit, &nak_frm);

    char line[160];
    const size_t len = picofw_tbus_audit_format_summary(&audit, line, sizeof(line));
    assert(len > 0u);
    assert(strcmp(line, "@PICO_TBUS_AUDIT frames=3 br_rejected=2 nak7f=1 ops=A9,F9") == 0);
}

void picofw_run_pana_diag_drv_tests(void) {
    test_diag_watch_legacy_and_fsm();
    test_diag_watch_parameterized_tx_and_e05();
    test_diag_watch_stage_funnel_no_bgo_masking();
    test_pana_rsp_regs_codec();
    test_drv_pana_codec();
    test_diag_watch_wake_and_ir_kaseikyo();
    test_smart_probe_qspi_arena_and_zero_float();
    test_identify_job_f14_and_bootsel();
    test_vault_ab_p6_mmio_pwrosc_and_disconnect_relock();
    test_boottrace_baremetal_classifier();
    test_boot_cycle_follow_fsm();
    test_tbus_br_ic7501_audit();
}
