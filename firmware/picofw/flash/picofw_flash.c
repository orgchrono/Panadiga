/**
 * @file picofw_flash.c
 * @brief Parallel NOR Flash discovery and on-target MN103S CFI Query driver.
 *
 * Uses checked reads for passive address-aliasing size detection and routes
 * `picofw_hw_query_cfi()` through the verified MN103S SRAM CFI stub (`0x98`/`0xF0`)
 * instead of writing to Layer-1 protected `0x800000AA`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_flash.h"
#include "common/picofw_cmd_arena.h"
#include "flash/picofw_target_bus_panamon.h"
#include "hal/picofw_hw.h"
#include "gdb/gdb_rsp.h"
#include "uart/picofw_pana_jedec.h"
#include "uart/picofw_pana_rsp.h"
#include "uart/picofw_pana_rsp_report.h"
#include "uart/picofw_uart_pio_engine.h"
#include "pico/stdlib.h"
#include <stdio.h>
#include <string.h>

#define PICOFW_CFI_PANAMON_STUB_VA 0x9043C000UL

static uint8_t s_cfi_stub_buf[PICOFW_JEDEC_MAX_STUB_SIZE];

static bool query_flash_id_via_panamon1_open(bool is_cfi, panarsp_rx_cap_t *cap, bool *io_pm_ok) {
    const size_t stub_len = picofw_jedec_build_autoselect_stub_ex(
        is_cfi, true, PICOFW_CFI_PANAMON_STUB_VA, s_cfi_stub_buf, sizeof(s_cfi_stub_buf)
    );
    if (stub_len == 0u) {
        return false;
    }
    if (!picofw_bus_panamon1_buf_matches_open(
            PICOFW_CFI_PANAMON_STUB_VA, s_cfi_stub_buf, stub_len, true, io_pm_ok
        ) &&
        !picofw_bus_panamon1_write_buf_open(
            PICOFW_CFI_PANAMON_STUB_VA, s_cfi_stub_buf, stub_len, true, io_pm_ok
        )) {
        return false;
    }
    char jcmd[24];
    snprintf(jcmd, sizeof(jcmd), "J %08lX\r", (unsigned long)PICOFW_CFI_PANAMON_STUB_VA);
    size_t rx_len = 0u;
    const bool ok = picofw_bus_panamon1_cmd_raw_open(
        jcmd, 1500u, true, cap->buf, sizeof(cap->buf), &rx_len, io_pm_ok
    );
    cap->len = rx_len;
    picofw_bus_panamon1_drain_tail(2u);
    return ok;
}

static bool picofw_fingerprint_matches(uint32_t base_a, uint32_t base_b) {
    for (uint32_t i = 0U; i < 4U; ++i) {
        uint32_t wa = 0UL;
        uint32_t wb = 0UL;
        if (!picofw_hw_read_mem32_checked(base_a + (i * 4U), &wa) ||
            !picofw_hw_read_mem32_checked(base_b + (i * 4U), &wb) ||
            (wa != wb)) {
            return false;
        }
    }
    return true;
}

uint32_t picofw_hw_autodetect_flash_size(void) {
    uint32_t w0 = 0UL;
    uint32_t w1 = 0UL;
    if (!picofw_hw_read_mem32_checked(PICOFW_SSOT_CS0_CACHED_BASE, &w0) ||
        !picofw_hw_read_mem32_checked(PICOFW_SSOT_CS0_CACHED_BASE + 4UL, &w1)) {
        return 0U;
    }

    if (((w0 == PICOFW_BUS_RESP_OPEN_HIGH) && (w1 == PICOFW_BUS_RESP_OPEN_HIGH)) ||
        ((w0 == PICOFW_BUS_RESP_OPEN_LOW)  && (w1 == PICOFW_BUS_RESP_OPEN_LOW))) {
        return PICOFW_FLASH_SIZE_8MB;
    }

    if (picofw_fingerprint_matches(PICOFW_SSOT_CS0_CACHED_BASE, PICOFW_FLASH_MIRROR_2MB_VA)) {
        return PICOFW_FLASH_SIZE_2MB;
    }

    if (picofw_fingerprint_matches(PICOFW_SSOT_CS0_CACHED_BASE, PICOFW_FLASH_MIRROR_4MB_VA)) {
        return PICOFW_FLASH_SIZE_4MB;
    }

    uint32_t u0 = PICOFW_BUS_RESP_OPEN_HIGH;
    uint32_t u1 = PICOFW_BUS_RESP_OPEN_HIGH;
    (void)picofw_hw_read_mem32_checked(PICOFW_FLASH_MIRROR_8MB_VA, &u0);
    (void)picofw_hw_read_mem32_checked(PICOFW_FLASH_MIRROR_8MB_VA + 4UL, &u1);
    if (picofw_fingerprint_matches(PICOFW_SSOT_CS0_CACHED_BASE, PICOFW_FLASH_MIRROR_8MB_VA) ||
        ((u0 == PICOFW_BUS_RESP_OPEN_HIGH) && (u1 == PICOFW_BUS_RESP_OPEN_HIGH)) ||
        ((u0 == PICOFW_BUS_RESP_OPEN_LOW)  && (u1 == PICOFW_BUS_RESP_OPEN_LOW))) {
        return PICOFW_FLASH_SIZE_8MB;
    }

    return PICOFW_FLASH_SIZE_16MB;
}

static bool picofw_hw_probe_flash_id(bool is_cfi, picofw_jedec_id_reply_t *out_reply) {
    if (out_reply != NULL) {
        memset(out_reply, 0, sizeof(*out_reply));
    }
    if (!picofw_hw_is_rail_healthy()) {
        return false;
    }
    if (!picofw_arena_try_acquire(PICOFW_ARENA_OWNER_FLASH)) {
        return false;
    }

    panarsp_rx_cap_t *const cap = &g_picofw_cmd_arena.panarsp;
    memset(cap, 0, sizeof(*cap));
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);

    bool pm_ok = false;
    bool done_seen = false;
    if (picofw_bus_panamon1_probe_open(true, &pm_ok)) {
        done_seen = query_flash_id_via_panamon1_open(is_cfi, cap, &pm_ok);
    } else {
        const size_t stub_len = picofw_jedec_build_autoselect_stub(
            is_cfi, PICOFW_JEDEC_STUB_ADDR, s_cfi_stub_buf, sizeof(s_cfi_stub_buf)
        );
        if (stub_len > 0u) {
            picofw_pio_tx_bytes(k_picofw_rsp_handshake, PICOFW_RSP_HANDSHAKE_LEN, false);
            panarsp_drain_for_ms(cap, 30u, false, cap->len);

            bool hdr_seen = false;
            bool to_unused = false;
            uint32_t rx_bytes = 0u;
            (void)panarsp_stream_until_tokens(
                cap,
                PICOFW_JEDEC_STUB_ADDR,
                s_cfi_stub_buf,
                stub_len,
                2500u,
                is_cfi ? PICOFW_CFI_QUERY_MAGIC : PICOFW_JEDEC_ID_MAGIC,
                "PANADONE",
                NULL,
                &hdr_seen,
                &done_seen,
                &to_unused,
                &rx_bytes
            );
        }
    }
    picofw_pio_end_session();

    picofw_jedec_id_reply_t rep;
    const bool ok = done_seen && picofw_jedec_parse_autoselect_reply(cap->buf, cap->len, is_cfi, &rep);
    picofw_arena_release(PICOFW_ARENA_OWNER_FLASH);
    if (!ok) {
        return false;
    }

    picofw_session_ctx_t *const ctx = picofw_session_default_ctx();
    if (is_cfi) {
        if (rep.size_bytes > 0UL) {
            (void)gdb_rsp_set_flash_size(rep.size_bytes);
        }
    } else if (ctx != NULL) {
        ctx->jedec_mfg = rep.mfg;
        memcpy(ctx->jedec_dev, rep.dev, sizeof(ctx->jedec_dev));
        ctx->jedec_verified = picofw_jedec_verify_chip_match(rep.mfg, rep.dev, ctx->active_flash_size);
    }
    if (out_reply != NULL) {
        *out_reply = rep;
    }
    return true;
}

bool picofw_hw_query_jedec_id(uint16_t *out_mfg, uint16_t out_dev[3], bool *out_verified) {
    picofw_jedec_id_reply_t rep;
    if (!picofw_hw_probe_flash_id(false, &rep)) {
        return false;
    }
    if (out_mfg != NULL) {
        *out_mfg = rep.mfg;
    }
    if (out_dev != NULL) {
        memcpy(out_dev, rep.dev, sizeof(rep.dev));
    }
    if (out_verified != NULL) {
        const picofw_session_ctx_t *const ctx = picofw_session_default_ctx();
        *out_verified = (ctx != NULL) ? ctx->jedec_verified : false;
    }
    return true;
}

bool picofw_hw_query_cfi(uint32_t *out_size_bytes) {
    if (out_size_bytes != NULL) {
        *out_size_bytes = 0UL;
    }
    picofw_jedec_id_reply_t rep;
    if (!picofw_hw_probe_flash_id(true, &rep)) {
        return false;
    }
    if (out_size_bytes != NULL) {
        *out_size_bytes = rep.size_bytes;
    }
    return true;
}

picofw_flash_geometry_t picofw_probe_flash_geometry(void) {
    const uint32_t detected = picofw_hw_autodetect_flash_size();
    if (detected == 0U) {
        picofw_flash_geometry_t none = {0U};
        return none;
    }
    (void)gdb_rsp_set_flash_size(detected);
    return picofw_flash_get_geometry(detected);
}
