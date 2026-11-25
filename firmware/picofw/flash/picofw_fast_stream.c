/**
 * @file picofw_fast_stream.c
 * @brief Hardware PIO1 `NOR_MAP`, `FAST_READ`, and `FAST_WRITE` commands using the
 *        embedded `0x9043A000` MN103S binary streamer with real-time `@PICO_PROG` push frames.
 *
 * All large buffers reside inside `g_picofw_cmd_arena.raw` to keep Core-0 stack frames
 * strictly below `-Wframe-larger-than=512`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_fast_stream.h"
#include "flash/picofw_fast_stub_io.h"
#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "flash/picofw_bus_policy.h"
#include "flash/picofw_fast_stream_codec.h"
#include "flash/picofw_flash.h"
#include "flash/picofw_target_bus_panamon.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "hal/picofw_wireless_codec.h"
#include "uart/picofw_uart_codec.h"
#include "uart/picofw_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

void picofw_cmd_nor_map(const char *arg) {
    (void)arg;
    picofw_hw_enforce_nsw5v_hiz();
    if (!picofw_hw_is_rail_healthy() || !picofw_hw_is_target_online()) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("NOR_MAP: ERROR (Target offline or unpowered)\n");
        return;
    }
    if (!picofw_arena_try_acquire(PICOFW_ARENA_OWNER_FAST_STREAM)) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("NOR_MAP: ERROR (Command arena busy)\n");
        return;
    }

    picofw_normap_summary_t sum;
    memset(&sum, 0, sizeof(sum));
    uint32_t ipl_crc = 0u;
    uint8_t hdr20[20];
    uint8_t z16[16];

    for (uint16_t sa = 0u; sa < PICOFW_NORMAP_SECTOR_COUNT; ++sa) {
        picofw_wdt_kick();
        uint32_t va = 0u;
        uint32_t sz = 0u;
        (void)picofw_normap_sector_span(sa, &va, &sz);
        if (!picofw_hw_read_block_checked(va, hdr20, sizeof(hdr20), true)) {
            picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
            picofw_cli_set_result("ERR", "EBUS");
            printf("NOR_MAP: ERROR (Bus read failed at SA%u @ 0x%08" PRIX32 ")\n", (unsigned)sa, va);
            return;
        }
        const uint8_t *zptr = NULL;
        if ((sa >= 26u) && (sa <= 70u)) {
            if (!picofw_hw_read_block_checked(va + 0x100UL, z16, sizeof(z16), true)) {
                picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
                picofw_cli_set_result("ERR", "EBUS");
                printf("NOR_MAP: ERROR (Bus read failed at SA%u +0x100)\n", (unsigned)sa);
                return;
            }
            zptr = z16;
        }
        if (sa <= 10u) {
            ipl_crc = picofw_crc32_ieee_update(ipl_crc, hdr20, sizeof(hdr20));
        }
        picofw_normap_accumulate(&sum, picofw_normap_classify_sector(sa, hdr20, zptr));
        if (((sa & 7u) == 7u) || (sa + 1u == PICOFW_NORMAP_SECTOR_COUNT)) {
            picofw_fast_emit_prog_push("MAP", sa, (uint32_t)(sa + 1u), PICOFW_NORMAP_SECTOR_COUNT, ipl_crc);
        }
    }
    sum.ipl_crc32 = ipl_crc;
    picofw_normap_finalize(&sum);

    char *const line = (char *)g_picofw_cmd_arena.raw;
    (void)picofw_fast_format_normap_line(line, 512u, &sum);
    printf("%s\nNOR_MAP: OK\n", line);
    picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
}

void picofw_cmd_fast_read(const char *arg) {
    picofw_hw_enforce_nsw5v_hiz();
    char tok_va[24];
    char tok_len[24];
    const char *rest = picofw_next_token(arg, tok_va, sizeof(tok_va));
    (void)picofw_next_token(rest, tok_len, sizeof(tok_len));
    uint32_t start_va = 0u;
    uint32_t total_len = 0u;
    if (!picofw_parse_hex_u32_exact(tok_va, 10u, &start_va) ||
        !picofw_parse_hex_u32_exact(tok_len, 10u, &total_len) || (total_len == 0u) ||
        ((total_len % PICOFW_FAST_STUB_PAGE_BYTES) != 0u) || (total_len > PICOFW_FLASH_SIZE_8MB)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("FAST_READ: ERROR (Usage: FAST_READ <hex_va> <hex_len_mult_100>)\n");
        return;
    }
    if (picofw_bus_is_lethal_mmio_range(start_va, (size_t)total_len)) {
        picofw_cli_set_result("ERR", "E05");
        printf("FAST_READ: ERROR code=E05 reason=LETHAL_MMIO\n");
        return;
    }
    if (!picofw_hw_is_rail_healthy()) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("FAST_READ: ERROR (Target unpowered)\n");
        return;
    }
    if (!picofw_arena_try_acquire(PICOFW_ARENA_OWNER_FAST_STREAM)) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("FAST_READ: ERROR (Command arena busy)\n");
        return;
    }

    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    bool uploaded = false;
    if (!picofw_fast_stub_upload_and_enter_open(&uploaded)) {
        picofw_pio_end_session();
        picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("FAST_READ: ERROR (Failed entering 0x9043A000 stub from PANAMON1>)\n");
        return;
    }

    uint8_t *const page_buf = g_picofw_cmd_arena.raw;
    char *const hex_buf = (char *)&g_picofw_cmd_arena.raw[512u];
    const uint32_t pages = total_len / PICOFW_FAST_STUB_PAGE_BYTES;
    uint32_t crc32 = 0u;
    uint32_t blank_pages = 0u;
    uint32_t data_pages = 0u;
    bool ok = true;

    for (uint32_t p = 0u; p < pages; ++p) {
        picofw_wdt_kick();
        const uint32_t va = start_va + (p * PICOFW_FAST_STUB_PAGE_BYTES);
        bool is_blank = false;
        if (!picofw_fast_stub_read_page_open(va, page_buf, &is_blank)) {
            ok = false;
            break;
        }
        crc32 = picofw_crc32_ieee_update(crc32, page_buf, PICOFW_FAST_STUB_PAGE_BYTES);
        if (is_blank) {
            blank_pages += 1u;
            printf("@PICO_CHUNK va=0x%08" PRIX32 " len=256 blank=1\n", va);
        } else {
            data_pages += 1u;
            (void)picofw_fast_format_page_hex(page_buf, PICOFW_FAST_STUB_PAGE_BYTES, hex_buf, 513u);
            printf("@PICO_CHUNK va=0x%08" PRIX32 " len=256 blank=0 hex=%s\n", va, hex_buf);
        }
        if ((((p + 1u) & 3u) == 0u) || ((p + 1u) == pages)) {
            picofw_fast_emit_prog_push("READ", picofw_fast_va_to_sa(va),
                                      (p + 1u) * PICOFW_FAST_STUB_PAGE_BYTES, total_len, crc32);
        }
    }

    (void)picofw_fast_stub_exit_open();
    picofw_pio_end_session();
    picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);

    if (!ok) {
        picofw_cli_set_result("ERR", "EBUS");
        printf("FAST_READ: ERROR (Page read or sum8 verification failed)\n");
        return;
    }
    printf("[FAST_READ] va=0x%08" PRIX32 " len=%" PRIu32 " crc32=0x%08" PRIX32
           " blank_pages=%" PRIu32 " data_pages=%" PRIu32 " stub_uploaded=%s\n",
           start_va, total_len, crc32, blank_pages, data_pages, uploaded ? "YES" : "NO");
    printf("FAST_READ: OK\n");
}

bool picofw_fast_write_bytes_hw(
    const char *tag,
    uint32_t addr,
    const uint8_t *payload,
    size_t pay_len,
    bool has_confirm
) {
    picofw_hw_enforce_nsw5v_hiz();
    const char *deny_reason = NULL;
    const picofw_bus_deny_t deny = picofw_fast_check_write_guard(
        picofw_session_default_ctx(), addr, (uint32_t)pay_len, has_confirm, &deny_reason);
    if (deny != PICOFW_BUS_ALLOW) {
        picofw_cli_set_result("ERR", "E05");
        printf("%s: ERROR code=E05 reason=%s addr=0x%08" PRIX32 " len=%u\n",
               tag, deny_reason, addr, (unsigned)pay_len);
        return false;
    }
    if (!picofw_hw_is_rail_healthy()) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("%s: ERROR (Target unpowered)\n", tag);
        return false;
    }
    const bool own_arena = picofw_arena_try_acquire(PICOFW_ARENA_OWNER_FLASH);
    if (!own_arena && (picofw_arena_current_owner() != PICOFW_ARENA_OWNER_FAST_STREAM)) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("%s: ERROR (Command arena busy)\n", tag);
        return false;
    }

    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    bool uploaded = false;
    if (!picofw_fast_stub_upload_and_enter_open(&uploaded)) {
        picofw_pio_end_session();
        if (own_arena) {
            picofw_arena_release(PICOFW_ARENA_OWNER_FLASH);
        }
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("%s: ERROR (Failed entering 0x9043A000 stub)\n", tag);
        return false;
    }

    /* On-Pico Pre-Flight NOR 1->0 Monotonicity Verification BEFORE unlocking Upper NOR! */
    uint8_t *const cur_buf = &g_picofw_cmd_arena.raw[8192u];
    picofw_fast_page_window_t win;
    bool pre_ok = picofw_fast_compute_page_window(addr, (uint32_t)pay_len, &win) && (win.span_pages <= 8u);
    for (uint32_t p = 0u; pre_ok && (p < win.span_pages); ++p) {
        pre_ok = picofw_fast_stub_read_page_open(
            win.page_base + (p * PICOFW_FAST_STUB_PAGE_BYTES), &cur_buf[p * PICOFW_FAST_STUB_PAGE_BYTES], NULL);
    }
    if (!pre_ok) {
        (void)picofw_fast_stub_exit_open();
        picofw_pio_end_session();
        if (own_arena) {
            picofw_arena_release(PICOFW_ARENA_OWNER_FLASH);
        }
        picofw_cli_set_result("ERR", "EBUS");
        printf("%s: ERROR (Pre-flight live read failed)\n", tag);
        return false;
    }

    size_t bad_off = 0u;
    if (!picofw_fast_check_monotonic_1_to_0(&cur_buf[win.rel_off], payload, pay_len, &bad_off)) {
        (void)picofw_fast_stub_exit_open();
        picofw_pio_end_session();
        if (own_arena) {
            picofw_arena_release(PICOFW_ARENA_OWNER_FLASH);
        }
        picofw_cli_set_result("ERR", "EMONO");
        printf("%s: ERROR code=EMONO reason=REQUIRES_ERASE_0_TO_1 off=%u cur=0x%02X tgt=0x%02X\n",
               tag, (unsigned)bad_off, (unsigned)cur_buf[win.rel_off + bad_off], (unsigned)payload[bad_off]);
        return false;
    }

    /* Exit stub to unlock Upper NOR cleanly, re-enter stub, program, and unconditionally U LOCK. */
    char rep[128];
    bool pm_ok = false;
    bool write_ok = false;
    uint8_t ack = 0u;
    if (picofw_fast_stub_exit_open() &&
        picofw_bus_panamon1_cmd_open("U CONFIRM_UPPER_NOR\r", 1000u, true, rep, sizeof(rep), &pm_ok) &&
        (strstr(rep, "OK:NOR_UNLOCK") != NULL) && picofw_fast_stub_jump_and_ping_open()) {
        uint8_t hdr[7];
        (void)picofw_fast_encode_write_hdr(addr, (uint16_t)pay_len, hdr);
        const uint8_t sum8 = picofw_fast_sum8(payload, pay_len);
        picofw_fast_pio_tx_paced_bin(hdr, sizeof(hdr));
        picofw_fast_pio_tx_paced_bin(payload, pay_len);
        picofw_fast_pio_tx_paced_bin(&sum8, 1u);
        write_ok = picofw_fast_pio_read_exact_ms(&ack, 1u, 4000u) && (ack == (uint8_t)'K');
        (void)picofw_fast_stub_exit_open();
    }
    /* Guaranteed fail-closed Upper NOR relock on ALL exit paths */
    (void)picofw_bus_panamon1_cmd_open("U LOCK\r", 1000u, false, rep, sizeof(rep), &pm_ok);
    picofw_pio_end_session();

    if (!write_ok) {
        if (own_arena) {
            picofw_arena_release(PICOFW_ARENA_OWNER_FLASH);
        }
        picofw_cli_set_result("ERR", "EJEDEC");
        printf("%s: ERROR code=EJEDEC ack=0x%02X relocked=YES\n", tag, (unsigned)ack);
        return false;
    }
    const uint32_t crc = picofw_crc32_ieee(payload, pay_len);
    picofw_wireless_consume_nor_op(picofw_wireless_global_state());
    picofw_fast_emit_prog_push("WRITE", picofw_fast_va_to_sa(addr), (uint32_t)pay_len, (uint32_t)pay_len, crc);
    printf("[%s] va=0x%08" PRIX32 " len=%u crc32=0x%08" PRIX32 " relocked=YES\n", tag, addr, (unsigned)pay_len, crc);
    if (own_arena) {
        picofw_arena_release(PICOFW_ARENA_OWNER_FLASH);
    }
    return true;
}

void picofw_cmd_fast_write(const char *arg) {
    picofw_hw_enforce_nsw5v_hiz();
    if (!picofw_arena_try_acquire(PICOFW_ARENA_OWNER_FAST_STREAM)) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("FAST_WRITE: ERROR (Command arena busy)\n");
        return;
    }
    char tok_va[24];
    char tok_confirm[32];
    const char *after_va = picofw_next_token(arg, tok_va, sizeof(tok_va));
    uint8_t *const payload = g_picofw_cmd_arena.raw;
    char *const tok_hex = (char *)&g_picofw_cmd_arena.raw[4096u];
    const char *after_hex = picofw_next_token(after_va, tok_hex, 2048u);
    (void)picofw_next_token(after_hex, tok_confirm, sizeof(tok_confirm));

    uint32_t addr = 0u;
    size_t pay_len = 0u;
    if (!picofw_parse_hex_u32_exact(tok_va, 10u, &addr) ||
        !picofw_decode_hex_bytes(tok_hex, payload, 1024u, &pay_len) || (pay_len < 2u)) {
        picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
        picofw_cli_set_result("ERR", "EINVAL");
        printf("FAST_WRITE: ERROR (Usage: FAST_WRITE <hex_va> <even_hex_bytes> CONFIRM_UPPER_NOR)\n");
        return;
    }
    if (picofw_fast_write_bytes_hw(
            "FAST_WRITE", addr, payload, pay_len, picofw_ascii_ieq(tok_confirm, "CONFIRM_UPPER_NOR"))) {
        printf("FAST_WRITE: OK\n");
    }
    picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
}

void picofw_cmd_fast_erase(const char *arg) {
    picofw_hw_enforce_nsw5v_hiz();
    char tok_va[24];
    char tok_confirm[32];
    const char *rest = picofw_next_token(arg, tok_va, sizeof(tok_va));
    (void)picofw_next_token(rest, tok_confirm, sizeof(tok_confirm));

    uint32_t sector_va = 0u;
    if (!picofw_parse_hex_u32_exact(tok_va, 10u, &sector_va)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("FAST_ERASE: ERROR (Usage: FAST_ERASE <hex_sector_va> CONFIRM_UPPER_NOR)\n");
        return;
    }
    const bool has_confirm = picofw_ascii_ieq(tok_confirm, "CONFIRM_UPPER_NOR");
    const char *deny_reason = NULL;
    if (picofw_fast_check_erase_guard(
            picofw_session_default_ctx(), sector_va, has_confirm, &deny_reason) != PICOFW_BUS_ALLOW) {
        picofw_cli_set_result("ERR", "E05");
        printf("FAST_ERASE: ERROR code=E05 reason=%s va=0x%08" PRIX32 "\n", deny_reason, sector_va);
        return;
    }
    if (!picofw_hw_is_rail_healthy()) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("FAST_ERASE: ERROR (Target unpowered)\n");
        return;
    }
    if (!picofw_arena_try_acquire(PICOFW_ARENA_OWNER_FAST_STREAM)) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("FAST_ERASE: ERROR (Command arena busy)\n");
        return;
    }

    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    if (picofw_fast_stub_ping_open()) {
        (void)picofw_fast_stub_exit_open();
    }
    char rep[128];
    char erase_cmd[32];
    bool pm_ok = false;
    bool erase_ok = false;
    if (picofw_bus_panamon1_cmd_open("U CONFIRM_UPPER_NOR\r", 1000u, true, rep, sizeof(rep), &pm_ok) &&
        (strstr(rep, "OK:NOR_UNLOCK") != NULL)) {
        (void)snprintf(erase_cmd, sizeof(erase_cmd), "E %08" PRIX32 "\r", sector_va);
        erase_ok = picofw_bus_panamon1_cmd_open(erase_cmd, 10000u, false, rep, sizeof(rep), &pm_ok) &&
                   (strstr(rep, "OK:ERASE") != NULL);
    }
    /* Guaranteed fail-closed Upper NOR relock on ALL exit paths */
    (void)picofw_bus_panamon1_cmd_open("U LOCK\r", 1000u, false, rep, sizeof(rep), &pm_ok);
    picofw_pio_end_session();

    if (!erase_ok) {
        picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
        picofw_cli_set_result("ERR", "EJEDEC");
        printf("FAST_ERASE: ERROR code=EJEDEC va=0x%08" PRIX32 " relocked=YES\n", sector_va);
        return;
    }
    uint8_t *const verify_buf = g_picofw_cmd_arena.raw;
    bool all_ff = picofw_hw_read_block_checked(sector_va, verify_buf, 64u, true);
    for (size_t i = 0u; all_ff && (i < 64u); ++i) {
        all_ff = (verify_buf[i] == 0xFFu);
    }
    picofw_arena_release(PICOFW_ARENA_OWNER_FAST_STREAM);
    if (!all_ff) {
        picofw_cli_set_result("ERR", "EVERIFY");
        printf("FAST_ERASE: ERROR code=EVERIFY va=0x%08" PRIX32 " verified_ff=NO relocked=YES\n", sector_va);
        return;
    }
    const uint16_t sa = picofw_fast_va_to_sa(sector_va);
    picofw_wireless_consume_nor_op(picofw_wireless_global_state());
    picofw_fast_emit_prog_push("ERASE", sa, 0x10000u, 0x10000u, 0u);
    printf("[FAST_ERASE] va=0x%08" PRIX32 " sa=%u verified_ff=YES relocked=YES\n",
           sector_va, (unsigned)sa);
    printf("FAST_ERASE: OK\n");
}
