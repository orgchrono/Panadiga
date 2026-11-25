/**
 * @file test_gdb_rsp_host.c
 * @brief Host-side C11 unit test suite for Panadiga P7501 GDB RSP Server.
 *
 * Verifies RSP packet framing, memory read/write protection air-gaps,
 * non-overlapping XML memory-map generation, register file access,
 * and deterministic E05 error rejection on IPL writes.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <assert.h>

#include "../gdb/gdb_rsp.h"
#include "../flash/picofw_flash.h"
#include "../hal/picofw_hw.h"
#include "../hal/picofw_wireless_codec.h"
#include "../uart/picofw_pana_rsp_ssot.h"
#include "test_uart_codec.h"
#include "test_pico_autonomous.h"

_Static_assert(P7501_PIN_RXD == PICOFW_SSOT_GPIO_RXD_GP2, "GP2 RXD SSOT drift");
_Static_assert(P7501_PIN_NSW5V == PICOFW_SSOT_GPIO_NSW5V_HIZ_GP3, "GP3 NSW5V SSOT drift");
_Static_assert(P7501_PIN_TXD == PICOFW_SSOT_GPIO_TXD_GP4, "GP4 TXD SSOT drift");
_Static_assert(P7501_PIN_XRESET == PICOFW_SSOT_GPIO_XRESET_OD_GP5, "GP5 XRESET SSOT drift");
_Static_assert(P7501_PIN_SW_UART_ADC == PICOFW_SSOT_GPIO_SW_UART_GP26, "GP26 SW_UART SSOT drift");
_Static_assert(PICOFW_FLASH_CRITICAL_IPL_SIZE == PICOFW_SSOT_IPL_LAYER1_SIZE, "Layer 1 IPL size drift");

/* =========================================================================
 * Mock Hardware & Target Memory Subsystem
 * ========================================================================= */

#define MOCK_SRAM_SIZE      (256 * 1024)
#define MOCK_NVRAM_SIZE     (64 * 1024)
#define MOCK_FLASH_SIZE     (8 * 1024 * 1024)

static uint8_t s_mock_sram[MOCK_SRAM_SIZE];
static uint8_t s_mock_nvram[MOCK_NVRAM_SIZE];
static uint8_t s_mock_flash[MOCK_FLASH_SIZE];

static bool s_mock_halted = true;
static uint32_t s_mock_dcsr = 0x00000001UL; /* Halted flag set */
static uint32_t s_mock_pc = 0x80000000UL;

static bool resolve_mock_flash_offset(uint32_t addr, uint32_t *out_offset) {
    if ((addr >= PICOFW_FLASH_CACHED_BASE) && (addr < (PICOFW_FLASH_CACHED_BASE + MOCK_FLASH_SIZE))) {
        *out_offset = addr - PICOFW_FLASH_CACHED_BASE;
        return true;
    }
    if ((addr >= PICOFW_FLASH_HW_UNCACHED_BASE) &&
        (addr < (PICOFW_FLASH_HW_UNCACHED_BASE + MOCK_FLASH_SIZE))) {
        *out_offset = addr - PICOFW_FLASH_HW_UNCACHED_BASE;
        return true;
    }
    return false;
}

/* Mock HAL VTable callbacks registered cleanly on picofw_target_bus */
static uint32_t mock_bus_read_mem32(void *ctx, uint32_t addr) {
    (void)ctx;
    if (addr < MOCK_SRAM_SIZE) {
        uint32_t val;
        memcpy(&val, &s_mock_sram[addr], 4);
        return val;
    }
    uint32_t offset = 0UL;
    if (resolve_mock_flash_offset(addr, &offset)) {
        uint32_t val;
        if (offset >= PICOFW_FLASH_8MB_NVRAM_OFF) {
            memcpy(&val, &s_mock_nvram[offset - PICOFW_FLASH_8MB_NVRAM_OFF], 4);
        } else {
            memcpy(&val, &s_mock_flash[offset], 4);
        }
        return val;
    }
    return 0xFFFFFFFFUL;
}

static void mock_bus_write_mem32(void *ctx, uint32_t addr, uint32_t data) {
    (void)ctx;
    if (addr < MOCK_SRAM_SIZE) {
        memcpy(&s_mock_sram[addr], &data, 4);
        return;
    }
    uint32_t offset = 0UL;
    if (resolve_mock_flash_offset(addr, &offset)) {
        if (offset >= PICOFW_FLASH_8MB_NVRAM_OFF) {
            memcpy(&s_mock_nvram[offset - PICOFW_FLASH_8MB_NVRAM_OFF], &data, 4);
        } else if (offset >= PICOFW_FLASH_CRITICAL_IPL_SIZE) {
            memcpy(&s_mock_flash[offset], &data, 4);
        }
        /* Critical Boot IPL (0x000000..0x03FFFF) is physically ignored in Flash */
    }
}

static bool mock_bus_is_online(void *ctx) { (void)ctx; return s_mock_dcsr != 0xFFFFFFFFUL; }
static bool mock_bus_is_halted(void *ctx) { (void)ctx; return s_mock_halted; }
static uint32_t mock_bus_read_dcsr(void *ctx) { (void)ctx; return s_mock_dcsr; }
static void mock_bus_set_pc(void *ctx, uint32_t pc_val) { (void)ctx; s_mock_pc = pc_val; }
static void mock_bus_resume_cpu(void *ctx) { (void)ctx; s_mock_halted = false; s_mock_dcsr = 0x00000000UL; }

static const picofw_target_bus_ops_t k_mock_bus_ops = {
    .read_mem32 = mock_bus_read_mem32,
    .write_mem32 = mock_bus_write_mem32,
    .is_online = mock_bus_is_online,
    .is_halted = mock_bus_is_halted,
    .read_dcsr = mock_bus_read_dcsr,
    .set_pc = mock_bus_set_pc,
    .resume_cpu = mock_bus_resume_cpu,
    .ctx = NULL,
};

void picofw_hw_reset_assert(void) {}
void picofw_hw_reset_release(void) {}
void picofw_hw_boot_sequence(picofw_boot_mode_t mode) { (void)mode; }
void picofw_hw_sw_uart_set_low(bool assert_low) { (void)assert_low; }
void sleep_ms(uint32_t ms) { (void)ms; }

/* =========================================================================
 * Test Verification Utilities
 * ========================================================================= */

static void reset_mock_state(void) {
    memset(s_mock_sram, 0xAA, sizeof(s_mock_sram));
    memset(s_mock_nvram, 0xFF, sizeof(s_mock_nvram));
    memset(s_mock_flash, 0x55, sizeof(s_mock_flash));
    s_mock_halted = true;
    s_mock_dcsr = 0x00000001UL;
    s_mock_pc = 0x80000000UL;
    picofw_target_bus_register_ops(&k_mock_bus_ops);
    gdb_rsp_init();
    gdb_rsp_set_active(true);
}

static void send_cmd_and_get_response(const char *cmd, char *resp_out, size_t max_len) {
    gdb_rsp_test_inject_packet(cmd);
    gdb_rsp_test_get_last_response(resp_out, max_len);
}

/* =========================================================================
 * Test Cases
 * ========================================================================= */

static void test_query_supported(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];
    send_cmd_and_get_response("qSupported", resp, sizeof(resp));
    assert(strstr(resp, "PacketSize=") != NULL);
    assert(strstr(resp, "qXfer:memory-map:read+") != NULL);
    printf("  [PASS] test_query_supported\n");
}

static void test_xml_memory_map_no_overlap(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];
    send_cmd_and_get_response("qXfer:memory-map:read::0,800", resp, sizeof(resp));

    /* Must contain non-overlapping ROM length 0x007F0000 */
    assert(strstr(resp, "start=\"0x80000000\" length=\"0x007F0000\"") != NULL);
    /* Must contain Flash NVRAM length 0x00010000 */
    assert(strstr(resp, "start=\"0x807F0000\" length=\"0x00010000\"") != NULL);
    /* Must contain SRAM at 0x00000000 */
    assert(strstr(resp, "start=\"0x00000000\" length=\"0x00040000\"") != NULL);
    /* Must not contain overlapping length 0x00800000 */
    assert(strstr(resp, "start=\"0x80000000\" length=\"0x00800000\"") == NULL);

    printf("  [PASS] test_xml_memory_map_no_overlap (OPUS-06 verified)\n");
}

static void test_sram_memory_read_write(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];

    /* Write 4 bytes to SRAM at 0x00001000 */
    send_cmd_and_get_response("M00001000,4:efbeadde", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);

    /* Read back 4 bytes from SRAM at 0x00001000 */
    send_cmd_and_get_response("m00001000,4", resp, sizeof(resp));
    assert(strcmp(resp, "efbeadde") == 0);

    printf("  [PASS] test_sram_memory_read_write\n");
}

static void test_nvram_memory_write_allowed(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];

    /* NVRAM microsectors SA134..SA141 at 0x807F0000 are writable for U99 recovery */
    send_cmd_and_get_response("M807F0000,4:00112233", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);

    send_cmd_and_get_response("m807F0000,4", resp, sizeof(resp));
    assert(strcmp(resp, "00112233") == 0);

    printf("  [PASS] test_nvram_memory_write_allowed\n");
}

static void test_ipl_write_rejection_e05(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];

    /* OPUS-07: Writing to IPL root-of-trust (0x80000000) must return E05 error code */
    send_cmd_and_get_response("M80000000,4:00000000", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    /* Writing near end of IPL (0x807EFFF0) must also return E05 */
    send_cmd_and_get_response("M807EFFF0,4:ffffffff", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    /* Writing that spans across the boundary into NVRAM must be rejected */
    send_cmd_and_get_response("M807EFFFE,4:11223344", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    /* Writing across unmapped boundary at end of SRAM (0x0003FFFE..0x00040001) must be rejected */
    send_cmd_and_get_response("M0003FFFE,4:11223344", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    /* Malformed read packet without comma delimiter must return E02 */
    send_cmd_and_get_response("m80000000", resp, sizeof(resp));
    assert(strcmp(resp, "E02") == 0);

    printf("  [PASS] test_ipl_write_rejection_e05 (OPUS-07 verified)\n");
}

static void test_register_access_no_ub(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];

    /* Read registers: must return exactly 12 * 8 = 96 hex characters */
    send_cmd_and_get_response("g", resp, sizeof(resp));
    assert(strlen(resp) == (AM34_REG_COUNT * 8));

    /* Write registers with known pattern */
    char write_cmd[GDB_RSP_MAX_PACKET_LEN];
    snprintf(write_cmd, sizeof(write_cmd), "G");
    for (size_t i = 0; i < AM34_REG_COUNT; i++) {
        strcat(write_cmd, "01020304");
    }
    send_cmd_and_get_response(write_cmd, resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);

    /* Verify written PC was passed to picofw_hw_set_pc */
    assert(s_mock_pc == 0x04030201UL);

    printf("  [PASS] test_register_access_no_ub (OPUS-14 verified)\n");
}

static void test_halt_and_continue_status(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];

    /* CPU halted: '?' must return S05 */
    s_mock_halted = true;
    s_mock_dcsr = 0x00000001UL;
    send_cmd_and_get_response("?", resp, sizeof(resp));
    assert(strcmp(resp, "S05") == 0);

    /* CPU running: '?' must return S00 */
    s_mock_halted = false;
    s_mock_dcsr = 0x00000000UL;
    send_cmd_and_get_response("?", resp, sizeof(resp));
    assert(strcmp(resp, "S00") == 0);

    printf("  [PASS] test_halt_and_continue_status (OPUS-05 verified)\n");
}

static void test_binary_x_write_and_ipl_guard(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];
    /* Binary write of raw `}\x00#$` into NVRAM at 0x807F0010 using GDB $X escaping */
    static const uint8_t x_pkt[] = {
        'X', '8', '0', '7', 'F', '0', '0', '1', '0', ',', '4', ':',
        0x7Du, 0x5Du, 0x00u, 0x7Du, 0x03u, 0x7Du, 0x04u
    };
    gdb_rsp_test_inject_packet_len(x_pkt, sizeof(x_pkt));
    gdb_rsp_test_get_last_response(resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);

    send_cmd_and_get_response("m807F0010,4", resp, sizeof(resp));
    assert(strcmp(resp, "7d002324") == 0);

    /* $X into read-only IPL NOR must fail closed with E05 */
    send_cmd_and_get_response("X80000000,4:ABCD", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    printf("  [PASS] test_binary_x_write_and_ipl_guard\n");
}

static void test_isolated_gdb_ctx_no_aliasing(void) {
    gdb_rsp_ctx_t ctx_4mb;
    gdb_rsp_ctx_t ctx_16mb;
    char resp_a[GDB_RSP_MAX_PACKET_LEN];
    char resp_b[GDB_RSP_MAX_PACKET_LEN];

    gdb_rsp_ctx_init(&ctx_4mb);
    gdb_rsp_ctx_init(&ctx_16mb);
    assert(gdb_rsp_ctx_set_flash_size(&ctx_4mb, 0x00400000UL));
    assert(gdb_rsp_ctx_set_flash_size(&ctx_16mb, 0x01000000UL));
    gdb_rsp_ctx_set_active(&ctx_4mb, true);
    gdb_rsp_ctx_set_active(&ctx_16mb, true);

    assert(gdb_rsp_ctx_get_flash_size(&ctx_4mb) == 0x00400000UL);
    assert(gdb_rsp_ctx_get_flash_size(&ctx_16mb) == 0x01000000UL);

    gdb_rsp_ctx_test_inject_packet(&ctx_4mb, "qXfer:memory-map:read::0,400");
    gdb_rsp_ctx_test_get_last_response(&ctx_4mb, resp_a, sizeof(resp_a));
    gdb_rsp_ctx_test_inject_packet(&ctx_16mb, "qXfer:memory-map:read::0,400");
    gdb_rsp_ctx_test_get_last_response(&ctx_16mb, resp_b, sizeof(resp_b));

    assert(strstr(resp_a, "0x803F0000") != NULL);
    assert(strstr(resp_b, "start=\"0x80FE0000\" length=\"0x00020000\"") != NULL);

    /* 16 MiB target: 0x807F0000 is Upper NOR (E05 when locked), while 0x80FE0000..0x80FFFFFF (128 KiB) is writable NVRAM */
    gdb_rsp_ctx_test_inject_packet(&ctx_16mb, "M807F0000,4:11223344");
    gdb_rsp_ctx_test_get_last_response(&ctx_16mb, resp_b, sizeof(resp_b));
    assert(strcmp(resp_b, "E05") == 0);

    gdb_rsp_ctx_test_inject_packet(&ctx_16mb, "M80FE0000,4:11223344");
    gdb_rsp_ctx_test_get_last_response(&ctx_16mb, resp_b, sizeof(resp_b));
    assert(strcmp(resp_b, "OK") == 0);

    printf("  [PASS] test_isolated_gdb_ctx_no_aliasing (incl. 16 MiB 128 KiB NVRAM)\n");
}

static void test_upper_nor_unlock_allows_app_keeps_ipl_e05(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];

    /* Locked by default: Upper NOR App/BGO writes fail closed with E05 */
    assert(!gdb_rsp_is_upper_nor_unlocked());
    send_cmd_and_get_response("M80040000,4:11223344", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);
    send_cmd_and_get_response("M84200000,4:aabbccdd", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    /* Wrong or empty unlock token returns E01 and keeps lock engaged */
    send_cmd_and_get_response("qPanadigaNorUnlock:WRONG", resp, sizeof(resp));
    assert(strcmp(resp, "E01") == 0);
    assert(!gdb_rsp_is_upper_nor_unlocked());
    send_cmd_and_get_response("qPanadigaNorUnlock", resp, sizeof(resp));
    assert(strcmp(resp, "E01") == 0);
    assert(!gdb_rsp_is_upper_nor_unlocked());
    send_cmd_and_get_response("M80400000,4:deadbeef", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    /* Explicit unlock token unlocks SA4..SA133 while SA0..SA3 remains E05 */
    send_cmd_and_get_response("qPanadigaNorUnlock:CONFIRM_UPPER_NOR", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    assert(gdb_rsp_is_upper_nor_unlocked());

    /* Critical Boot IPL (SA0..SA3: 0x80000000..0x8003FFFF / 0x84000000..0x8403FFFF) permanently E05 */
    static const char *k_ipl_cmds[] = {
        "M80000000,4:00000000",
        "M8003FFFC,4:00000000",
        "M8003FFFE,4:11223344",
        "X80000000,4:ABCD",
        "M84000000,4:00000000",
        "M8403FFFE,4:11223344",
    };
    for (size_t i = 0u; i < (sizeof(k_ipl_cmds) / sizeof(k_ipl_cmds[0])); ++i) {
        send_cmd_and_get_response(k_ipl_cmds[i], resp, sizeof(resp));
        assert(strcmp(resp, "E05") == 0);
    }

    /* Upper NOR App & BGO writes succeed and read back cleanly */
    send_cmd_and_get_response("M80040000,4:11223344", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    send_cmd_and_get_response("m80040000,4", resp, sizeof(resp));
    assert(strcmp(resp, "11223344") == 0);

    send_cmd_and_get_response("M84200000,4:cafebabe", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    send_cmd_and_get_response("m84200000,4", resp, sizeof(resp));
    assert(strcmp(resp, "cafebabe") == 0);

    send_cmd_and_get_response("M80400000,4:deadbeef", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    send_cmd_and_get_response("m80400000,4", resp, sizeof(resp));
    assert(strcmp(resp, "deadbeef") == 0);

    /* NVRAM stays writable and out-of-bounds remains E05 */
    send_cmd_and_get_response("M807F0000,4:55667788", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    send_cmd_and_get_response("M80800000,4:00000000", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);
    /* D2: Oversized $m read (> 1023 bytes) returns E02 without overflowing tx_resp_buf[2048] */
    send_cmd_and_get_response("m80000000,FFFF", resp, sizeof(resp));
    assert(strcmp(resp, "E02") == 0);
    /* D4: Full-RO and ES35V MAIN_XIP remain denied even if flash_size_bytes differs from profile */
    picofw_session_ctx_t prof_ctx;
    picofw_session_ctx_reset(&prof_ctx);
    prof_ctx.profile_selected = true;
    prof_ctx.active_profile = PICOFW_TARGET_PROFILE_DMR_EZ485V;
    assert(picofw_bus_check_access(&prof_ctx, PICOFW_FLASH_SIZE_16MB, 0x80040000UL, 4u, true, false, true) == PICOFW_BUS_DENY_FULL_RO);
    prof_ctx.active_profile = PICOFW_TARGET_PROFILE_DMR_ES35V;
    assert(picofw_bus_check_access(&prof_ctx, PICOFW_FLASH_SIZE_16MB, 0x80200000UL, 4u, true, false, true) == PICOFW_BUS_DENY_PROTECTED_RO);

    /* Re-locking restores full NvramOnly E05 protection over Upper NOR */
    send_cmd_and_get_response("qPanadigaNorLock", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    assert(!gdb_rsp_is_upper_nor_unlocked());
    send_cmd_and_get_response("M80400000,4:01020304", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);

    /* Explicit context setter/getter isolation + centralized picofw_flash_geo sync */
    assert(!picofw_flash_is_upper_nor_unlocked());
    assert(picofw_flash_is_write_blocked_with_policy(0x84200000UL, 4u, false));
    assert(!picofw_flash_is_write_blocked_with_policy(0x84200000UL, 4u, true));
    assert(picofw_flash_is_write_blocked_with_policy(0x84000000UL, 4u, true));

    gdb_rsp_ctx_t isolated;
    gdb_rsp_ctx_init(&isolated);
    assert(!gdb_rsp_ctx_is_upper_nor_unlocked(&isolated));
    gdb_rsp_ctx_set_upper_nor_unlocked(&isolated, true);
    assert(gdb_rsp_ctx_is_upper_nor_unlocked(&isolated));
    assert(!gdb_rsp_is_upper_nor_unlocked());
    assert(!picofw_flash_is_upper_nor_unlocked());

    printf("  [PASS] test_upper_nor_unlock_allows_app_keeps_ipl_e05\n");
}

static void test_flash_sector_geo_64k_vs_8k(void) {
    assert(PICOFW_FLASH_CRITICAL_IPL_SIZE == 0x00040000UL);
    assert(SPANSION_MAIN_SECTOR_SIZE_BYTES == 65536UL);

    assert(picofw_flash_is_critical_boot_ipl_range(0x80000000UL, 4u));
    assert(picofw_flash_is_critical_boot_ipl_range(0x8003FFFEUL, 4u));
    assert(picofw_flash_is_critical_boot_ipl_range(0x84000000UL, 4u));
    assert(picofw_flash_is_critical_boot_ipl_range(0x8403FFFFUL, 1u));
    assert(!picofw_flash_is_critical_boot_ipl_range(0x80040000UL, 4u));
    assert(!picofw_flash_is_critical_boot_ipl_range(0x84040000UL, 4u));
    assert(!picofw_flash_is_critical_boot_ipl_range(0x807F0000UL, 4u));
    assert(!picofw_flash_is_critical_boot_ipl_range(0x80000000UL, 0u));

    /* 8 MiB S29JL064H: 8x8 KiB bottom SA0..SA7 (<0x10000), 126x64 KiB main, 8x8 KiB top NVRAM */
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x80000000UL) == 0x2000UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x8000E000UL) == 0x2000UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x80010000UL) == 65536UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x80040000UL) == 65536UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x84200000UL) == 65536UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x807EFFFFUL) == 65536UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x807F0000UL) == 8192UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x847F2000UL) == 8192UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x807FFFFFUL) == 8192UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x80800000UL) == 0UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_8MB, 0x40000000UL) == 0UL);

    /* Walk complete 8 MiB S29JL064H sector layout (8x8K bottom + 126x64K main + 8x8K top = 127x64K + 8x8K span) */
    uint32_t walk_off = 0UL;
    uint32_t cnt_8k_bottom = 0u;
    uint32_t cnt_64k_main = 0u;
    uint32_t cnt_8k_top = 0u;
    while (walk_off < PICOFW_FLASH_SIZE_8MB) {
        const uint32_t sz = picofw_flash_get_sector_size_at(
            PICOFW_FLASH_SIZE_8MB, PICOFW_FLASH_CACHED_BASE + walk_off
        );
        assert(sz == 8192UL || sz == 65536UL);
        if (walk_off < 0x010000UL) {
            assert(sz == 8192UL);
            cnt_8k_bottom++;
        } else if (walk_off < PICOFW_FLASH_8MB_NVRAM_OFF) {
            assert(sz == 65536UL);
            cnt_64k_main++;
        } else {
            assert(sz == 8192UL);
            cnt_8k_top++;
        }
        walk_off += sz;
    }
    assert(walk_off == PICOFW_FLASH_SIZE_8MB);
    assert(cnt_8k_bottom == 8u && cnt_64k_main == 126u && cnt_8k_top == 8u);

    /* 2 MiB (S29AL016D Top Boot: 31x64K + 32K + 2x8K + 16K) / 4 MiB / 16 MiB (S29GL128N uniform 128K) */
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_2MB, 0x80002000UL) == 65536UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_2MB, 0x801EFFFFUL) == 65536UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_2MB, 0x801F0000UL) == 32768UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_2MB, 0x801F8000UL) == 8192UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_2MB, 0x801FC000UL) == 16384UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_4MB, 0x803EFFFFUL) == 65536UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_4MB, 0x803F0000UL) == 8192UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_16MB, 0x80FC0000UL) == 131072UL);
    assert(picofw_flash_get_sector_size_at(PICOFW_FLASH_SIZE_16MB, 0x80FE0000UL) == 131072UL);
    const picofw_flash_geometry_t g16 = picofw_flash_get_geometry(PICOFW_FLASH_SIZE_16MB);
    assert(g16.base_sector == 127u && g16.sector_count == 1u);

    printf("  [PASS] test_flash_sector_geo_64k_vs_8k\n");
}

static void test_lethal_register_c000100c_e05_block(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];
    static const char *k_lethal_cmds[] = {
        "mC000100C,4",
        "mC000100E,1",
        "mC0001008,8",
        "MC000100C,4:00000000",
        "MC0001008,8:0011223344556677",
        "XC000100C,4:ABCD",
        "MC000100F,1:AA",
    };
    for (size_t i = 0u; i < (sizeof(k_lethal_cmds) / sizeof(k_lethal_cmds[0])); ++i) {
        send_cmd_and_get_response(k_lethal_cmds[i], resp, sizeof(resp));
        assert(strcmp(resp, "E05") == 0);
    }
    /* Adjacent non-lethal system status register (0xC0001008..0xC000100B) remains accessible */
    send_cmd_and_get_response("mC0001008,4", resp, sizeof(resp));
    assert(strcmp(resp, "ffffffff") == 0);
    send_cmd_and_get_response("MC0001008,4:00000000", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    printf("  [PASS] test_lethal_register_c000100c_e05_block\n");
}

static bool mock_bus_read_mem32_fail(void *ctx, uint32_t addr, uint32_t *out_val) {
    (void)ctx;
    if (addr == 0x00002000UL) {
        return false;
    }
    *out_val = mock_bus_read_mem32(NULL, addr);
    return true;
}

static void test_gdb_hardening_relock_ro_xml_bus_err_and_f14(void) {
    char resp[GDB_RSP_MAX_PACKET_LEN];

    /* 1. Compact gdb_rsp_ctx_t size (fixed 68 KiB RP2350 BSS bloat; <12 KiB even with host test capture buf) */
    assert(sizeof(gdb_rsp_ctx_t) < 12288u);

    /* 2. Detach ($D), Kill ($k), and Restart ($R) auto-relock Upper NOR */
    static const char *k_exit_pkts[3] = { "D", "k", "R00" };
    for (size_t i = 0u; i < 3u; ++i) {
        gdb_rsp_set_active(true);
        send_cmd_and_get_response("qPanadigaNorUnlock:CONFIRM_UPPER_NOR", resp, sizeof(resp));
        assert(strcmp(resp, "OK") == 0 && gdb_rsp_is_upper_nor_unlocked());
        send_cmd_and_get_response(k_exit_pkts[i], resp, sizeof(resp));
        assert(!gdb_rsp_is_upper_nor_unlocked());
        assert(!picofw_flash_is_upper_nor_unlocked());
    }

    /* 3. Full-RO profile omits empty 0-length <memory type="flash"/> tag in XML memory-map */
    gdb_rsp_set_active(true);
    assert(picofw_flash_set_target_profile_by_id(PICOFW_TARGET_PROFILE_DMR_EZ485V));
    send_cmd_and_get_response("qXfer:memory-map:read::0,800", resp, sizeof(resp));
    assert(strstr(resp, "length=\"0x00000000\"") == NULL);
    assert(strstr(resp, "type=\"flash\"") == NULL);
    assert(picofw_flash_set_target_profile_by_id(PICOFW_TARGET_PROFILE_DMR_EH55));

    /* 4. Checked bus read failure returns E01 (never silent 0xFFFFFFFF or corrupt RMW) */
    picofw_target_bus_ops_t fail_ops = k_mock_bus_ops;
    fail_ops.read_mem32_checked = mock_bus_read_mem32_fail;
    picofw_target_bus_register_ops(&fail_ops);
    send_cmd_and_get_response("m00002000,4", resp, sizeof(resp));
    assert(strcmp(resp, "E01") == 0);
    send_cmd_and_get_response("M00002001,1:AA", resp, sizeof(resp));
    assert(strcmp(resp, "E01") == 0);
    picofw_target_bus_register_ops(&k_mock_bus_ops);

    /* 5. F14 wireless 1-op quota consumed on GDB NOR write */
    picofw_wireless_auth_state_t *ws = picofw_wireless_global_state();
    picofw_wireless_state_init(ws);
    ws->active_origin = PICOFW_ORIGIN_BLE_NUS;
    assert(picofw_wireless_handle_usb_auth_cmd(ws, PICOFW_ORIGIN_USB_CDC, "CONFIRM 1"));
    send_cmd_and_get_response("M807F0000,4:12345678", resp, sizeof(resp));
    assert(strcmp(resp, "OK") == 0);
    send_cmd_and_get_response("M807F0004,4:87654321", resp, sizeof(resp));
    assert(strcmp(resp, "E05") == 0);
    picofw_wireless_state_init(ws);

    printf("  [PASS] test_gdb_hardening_relock_ro_xml_bus_err_and_f14\n");
}

int main(void) {
    printf("======================================================================\n");
    printf(">> Running PicoFW P7501 C11 Host Test Suite (PICO_HOST_TEST=1)\n");
    printf("======================================================================\n");

    reset_mock_state();
    test_query_supported();

    reset_mock_state();
    test_xml_memory_map_no_overlap();

    reset_mock_state();
    test_sram_memory_read_write();

    reset_mock_state();
    test_nvram_memory_write_allowed();

    reset_mock_state();
    test_ipl_write_rejection_e05();

    reset_mock_state();
    test_binary_x_write_and_ipl_guard();

    reset_mock_state();
    test_isolated_gdb_ctx_no_aliasing();

    reset_mock_state();
    test_upper_nor_unlock_allows_app_keeps_ipl_e05();

    reset_mock_state();
    test_flash_sector_geo_64k_vs_8k();

    reset_mock_state();
    test_lethal_register_c000100c_e05_block();

    reset_mock_state();
    test_gdb_hardening_relock_ro_xml_bus_err_and_f14();

    reset_mock_state();
    test_register_access_no_ub();

    reset_mock_state();
    test_halt_and_continue_status();

    if (picofw_run_uart_codec_tests() != 0) {
        return 1;
    }
    if (picofw_run_pico_autonomous_tests() != 0) {
        return 1;
    }

    printf("======================================================================\n");
    printf("[SUCCESS] All 8 C11 Pico Firmware Host Unit Tests Passed Cleanly!\n");
    printf("======================================================================\n");
    return 0;
}
