/**
 * @file test_pana_diag_drv.c
 * @brief Host C11 unit tests for parameterized `DIAGWATCH`, `PANARSP REGS`, `DRVPANA`, and shared flash guards.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "tests/test_pana_logic.h"
#include "uart/p7501_diag_watch_codec.h"
#include "uart/p7501_drv_pana_codec.h"
#include "uart/p7501_pana_rsp.h"
#include "flash/p7501_flash.h"
#include "common/p7501_hex.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

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

static void test_diag_watch_legacy_and_fsm(void) {
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
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_READ) == 'R');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_WRITE) == 'W');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_FORMAT) == 'F');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_DUMP) == 'D');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_JUMP) == 'J');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_ERASE) == 'E');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_QUIT) == 'Q');
    assert(p7501_diag_watch_action_char(P7501_DIAG_ACT_NONE) == '\0');
    assert(p7501_diag_watch_parse_args("20000 JIG:2500 READ RELEASE", &dw) && dw.action == P7501_DIAG_ACT_READ);
    assert(p7501_diag_watch_parse_args("20000 JIG:2500 WRITE RELEASE", &dw) && dw.action == P7501_DIAG_ACT_WRITE);
    assert(p7501_diag_watch_parse_args("20000 JIG:2500 JUMP RELEASE", &dw) && dw.action == P7501_DIAG_ACT_JUMP);
    assert(p7501_diag_watch_parse_args("20000 JIG:2500 ERASE RELEASE", &dw) && dw.action == P7501_DIAG_ACT_ERASE);
    assert(p7501_diag_watch_parse_args("20000 JIG:2500 QUIT RELEASE", &dw) && dw.action == P7501_DIAG_ACT_QUIT);

    assert(p7501_diag_watch_parse_args("25000 JIG:2500 AUTO RELEASE", &dw));
    p7501_diag_watch_state_t st;
    p7501_diag_watch_init(&st);
    int rel_count = 0;
    int tx_count = 0;
    char last_tx = '\0';

    feed_str_and_count(&st, &dw, "DDR CHECK: OK\r\n", &rel_count, &tx_count, &last_tx);
    assert(rel_count == 0 && !st.sw_uart_released);
    assert((st.seen_events & P7501_DIAG_EVT_DDR_OK) != 0u);

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

static void test_diag_watch_parameterized_tx_and_e05(void) {
    p7501_diag_watch_spec_t dw;
    char tx_buf[36];

    assert(p7501_diag_watch_parse_args("20000 JIG:2500 READ:847F0000,20 RELEASE", &dw));
    assert(dw.action == P7501_DIAG_ACT_READ);
    assert(p7501_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 13u);
    assert(strcmp(tx_buf, "R847F0000,20\r") == 0);

    assert(p7501_diag_watch_parse_args("20000 JIG:2500 READ:0x80000000", &dw));
    assert(p7501_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 10u);
    assert(strcmp(tx_buf, "R80000000\r") == 0);
    assert(!p7501_diag_watch_parse_args("20000 READ:80000000,0", &dw));
    assert(!p7501_diag_watch_parse_args("20000 READ:", &dw));

    /* RAM & NVRAM byte writes pass; Spansion IPL NOR writes fail E05 guard. */
    assert(p7501_diag_watch_parse_args("20000 WRITE:50401000,AA", &dw));
    assert(p7501_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 13u);
    assert(strcmp(tx_buf, "W50401000,AA\r") == 0);
    assert(p7501_diag_watch_parse_args("20000 WRITE:847F0000,55", &dw));
    assert(!p7501_diag_watch_parse_args("20000 WRITE:80000000,AA", &dw));
    assert(!p7501_diag_watch_parse_args("20000 WRITE:847EFFFF,FF", &dw));
    assert(!p7501_diag_watch_parse_args("20000 WRITE:847F0000,1FF", &dw));

    assert(p7501_diag_watch_parse_args("20000 JUMP:0x50401F3C", &dw));
    assert(p7501_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 10u);
    assert(strcmp(tx_buf, "J50401F3C\r") == 0);

    assert(p7501_diag_watch_parse_args("20000 ERASE:847F0000", &dw));
    assert(p7501_diag_watch_format_tx(&dw, tx_buf, sizeof(tx_buf)) == 10u);
    assert(strcmp(tx_buf, "E847F0000\r") == 0);
    assert(p7501_diag_watch_parse_args("20000 ERASE:807F2000", &dw));
    assert(!p7501_diag_watch_parse_args("20000 ERASE:847E0000", &dw));
    assert(!p7501_diag_watch_parse_args("20000 ERASE:50400000", &dw));

    /* Shared flash bounds helpers. */
    assert(p7501_flash_is_protected_ipl_range(0x80000000UL, 4u));
    assert(p7501_flash_is_protected_ipl_range(0x847EFFF0UL, 32u));
    assert(!p7501_flash_is_protected_ipl_range(0x847F0000UL, 16u));
    assert(p7501_flash_is_nvram_microsector_addr(0x807FFFFFUL));
    assert(!p7501_flash_is_nvram_microsector_addr(0x847EFFFFUL));
}

static void test_pana_rsp_regs_codec(void) {
    p7501_pana_rsp_spec_t spec;
    assert(p7501_panarsp_parse_args("5000 JIG:350 REGS", &spec));
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
            hex_payload[(i * 8u) + (b * 2u)] = p7501_nibble_to_hex_char((uint8_t)(le[b] >> 4));
            hex_payload[(i * 8u) + (b * 2u) + 1u] = p7501_nibble_to_hex_char((uint8_t)(le[b] & 0x0Fu));
        }
    }
    hex_payload[96] = '\0';

    uint32_t decoded[12];
    assert(p7501_rsp_decode_mn103_regs((const uint8_t *)hex_payload, 96u, decoded));
    for (size_t i = 0u; i < 12u; ++i) {
        assert(decoded[i] == expected[i]);
    }
    assert(!p7501_rsp_decode_mn103_regs((const uint8_t *)hex_payload, 94u, decoded));
    hex_payload[5] = 'Z';
    assert(!p7501_rsp_decode_mn103_regs((const uint8_t *)hex_payload, 96u, decoded));
}

static void test_drv_pana_codec(void) {
    p7501_drv_pana_spec_t drv;
    char wire[104];

    assert(p7501_drv_pana_parse_args("DSCREG", &drv));
    assert(drv.timeout_ms == 3000u && drv.boot_action == P7501_BOOT_ACTION_PASSIVE);
    assert(drv.send_pass && strcmp(drv.cmd, "DSCREG") == 0);
    assert(p7501_drv_pana_format_cmd(&drv, wire, sizeof(wire)) == 8u);
    assert(strcmp(wire, "DSCREG\r\n") == 0);

    assert(p7501_drv_pana_parse_args("4500 JIG:350 NOPASS START", &drv));
    assert(drv.timeout_ms == 4500u && drv.boot_action == P7501_BOOT_ACTION_JIG);
    assert(drv.jig_hold_ms == 350u && !drv.send_pass && strcmp(drv.cmd, "START") == 0);

    assert(p7501_drv_pana_parse_args("PASS 0x4d415453 0x48495441", &drv));
    assert(!drv.send_pass && strcmp(drv.cmd, "PASS 0x4d415453 0x48495441") == 0);

    assert(p7501_drv_pana_parse_args("PASS", &drv));
    assert(!drv.send_pass && strcmp(drv.cmd, "PASS") == 0);

    assert(p7501_drv_pana_parse_args("2000 \"FEP 10\"", &drv));
    assert(drv.timeout_ms == 2000u && drv.send_pass && strcmp(drv.cmd, "FEP 10") == 0);

    assert(!p7501_drv_pana_parse_args("", &drv));
    assert(!p7501_drv_pana_parse_args("4000 JIG:350", &drv));
}

void p7501_run_pana_diag_drv_tests(void) {
    test_diag_watch_legacy_and_fsm();
    test_diag_watch_parameterized_tx_and_e05();
    test_pana_rsp_regs_codec();
    test_drv_pana_codec();
}
