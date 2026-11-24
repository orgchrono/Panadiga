/**
 * @file p7501_flash.c
 * @brief Parallel NOR Flash discovery and on-target MN103S CFI Query driver.
 *
 * Uses checked reads for passive address-aliasing size detection and routes
 * `p7501_hw_query_cfi()` through the verified MN103S SRAM CFI stub (`0x98`/`0xF0`)
 * instead of writing to Layer-1 protected `0x800000AA`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_flash.h"
#include "common/p7501_cmd_arena.h"
#include "flash/p7501_target_bus_panamon.h"
#include "hal/p7501_hw.h"
#include "gdb/gdb_rsp.h"
#include "uart/p7501_pana_jedec.h"
#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_rsp_report.h"
#include "uart/p7501_uart_pio_engine.h"
#include "pico/stdlib.h"
#include <stdio.h>
#include <string.h>

#define P7501_CFI_PANAMON_STUB_VA 0x9043C000UL

static uint8_t s_cfi_stub_buf[P7501_JEDEC_MAX_STUB_SIZE];

static bool query_cfi_via_panamon1_open(panarsp_rx_cap_t *cap, bool *io_pm_ok) {
    const size_t stub_len = p7501_jedec_build_autoselect_stub_ex(
        true, true, P7501_CFI_PANAMON_STUB_VA, s_cfi_stub_buf, sizeof(s_cfi_stub_buf)
    );
    if (stub_len == 0u) {
        return false;
    }
    if (!p7501_bus_panamon1_buf_matches_open(
            P7501_CFI_PANAMON_STUB_VA, s_cfi_stub_buf, stub_len, true, io_pm_ok
        ) &&
        !p7501_bus_panamon1_write_buf_open(
            P7501_CFI_PANAMON_STUB_VA, s_cfi_stub_buf, stub_len, true, io_pm_ok
        )) {
        return false;
    }
    char jcmd[24];
    snprintf(jcmd, sizeof(jcmd), "J %08lX\r", (unsigned long)P7501_CFI_PANAMON_STUB_VA);
    size_t rx_len = 0u;
    const bool ok = p7501_bus_panamon1_cmd_raw_open(
        jcmd, 1500u, true, cap->buf, sizeof(cap->buf), &rx_len, io_pm_ok
    );
    cap->len = rx_len;
    p7501_bus_panamon1_drain_tail(2u);
    return ok;
}

static bool p7501_fingerprint_matches(uint32_t base_a, uint32_t base_b) {
    for (uint32_t i = 0U; i < 4U; ++i) {
        uint32_t wa = 0UL;
        uint32_t wb = 0UL;
        if (!p7501_hw_read_mem32_checked(base_a + (i * 4U), &wa) ||
            !p7501_hw_read_mem32_checked(base_b + (i * 4U), &wb) ||
            (wa != wb)) {
            return false;
        }
    }
    return true;
}

uint32_t p7501_hw_autodetect_flash_size(void) {
    uint32_t w0 = 0UL;
    uint32_t w1 = 0UL;
    if (!p7501_hw_read_mem32_checked(P7501_SSOT_CS0_CACHED_BASE, &w0) ||
        !p7501_hw_read_mem32_checked(P7501_SSOT_CS0_CACHED_BASE + 4UL, &w1)) {
        return 0U;
    }

    if (((w0 == P7501_BUS_RESP_OPEN_HIGH) && (w1 == P7501_BUS_RESP_OPEN_HIGH)) ||
        ((w0 == P7501_BUS_RESP_OPEN_LOW)  && (w1 == P7501_BUS_RESP_OPEN_LOW))) {
        return P7501_FLASH_SIZE_8MB;
    }

    if (p7501_fingerprint_matches(P7501_SSOT_CS0_CACHED_BASE, P7501_FLASH_MIRROR_2MB_VA)) {
        return P7501_FLASH_SIZE_2MB;
    }

    if (p7501_fingerprint_matches(P7501_SSOT_CS0_CACHED_BASE, P7501_FLASH_MIRROR_4MB_VA)) {
        return P7501_FLASH_SIZE_4MB;
    }

    uint32_t u0 = P7501_BUS_RESP_OPEN_HIGH;
    uint32_t u1 = P7501_BUS_RESP_OPEN_HIGH;
    (void)p7501_hw_read_mem32_checked(P7501_FLASH_MIRROR_8MB_VA, &u0);
    (void)p7501_hw_read_mem32_checked(P7501_FLASH_MIRROR_8MB_VA + 4UL, &u1);
    if (p7501_fingerprint_matches(P7501_SSOT_CS0_CACHED_BASE, P7501_FLASH_MIRROR_8MB_VA) ||
        ((u0 == P7501_BUS_RESP_OPEN_HIGH) && (u1 == P7501_BUS_RESP_OPEN_HIGH)) ||
        ((u0 == P7501_BUS_RESP_OPEN_LOW)  && (u1 == P7501_BUS_RESP_OPEN_LOW))) {
        return P7501_FLASH_SIZE_8MB;
    }

    return P7501_FLASH_SIZE_16MB;
}

bool p7501_hw_query_cfi(uint32_t *out_size_bytes) {
    if (out_size_bytes != NULL) {
        *out_size_bytes = 0UL;
    }
    if (!p7501_hw_is_target_powered()) {
        return false;
    }

    panarsp_rx_cap_t *const cap = &g_p7501_cmd_arena.panarsp;
    memset(cap, 0, sizeof(*cap));
    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);

    bool pm_ok = false;
    bool done_seen = false;
    if (p7501_bus_panamon1_probe_open(true, &pm_ok)) {
        done_seen = query_cfi_via_panamon1_open(cap, &pm_ok);
    } else {
        const size_t stub_len = p7501_jedec_build_autoselect_stub(
            true, P7501_JEDEC_STUB_ADDR, s_cfi_stub_buf, sizeof(s_cfi_stub_buf)
        );
        if (stub_len > 0u) {
            p7501_pio_tx_bytes(k_p7501_rsp_handshake, P7501_RSP_HANDSHAKE_LEN, false);
            panarsp_drain_for_ms(cap, 30u, false, cap->len);

            bool hdr_seen = false;
            bool to_unused = false;
            uint32_t rx_bytes = 0u;
            (void)panarsp_stream_until_tokens(
                cap,
                P7501_JEDEC_STUB_ADDR,
                s_cfi_stub_buf,
                stub_len,
                2500u,
                P7501_CFI_QUERY_MAGIC,
                "PANADONE",
                NULL,
                &hdr_seen,
                &done_seen,
                &to_unused,
                &rx_bytes
            );
        }
    }
    p7501_pio_end_session();

    p7501_jedec_id_reply_t rep;
    if (!done_seen || !p7501_jedec_parse_autoselect_reply(cap->buf, cap->len, true, &rep)) {
        return false;
    }
    if (out_size_bytes != NULL) {
        *out_size_bytes = rep.size_bytes;
    }
    if (rep.size_bytes > 0UL) {
        (void)gdb_rsp_set_flash_size(rep.size_bytes);
    }
    return true;
}

p7501_flash_geometry_t p7501_probe_flash_geometry(void) {
    const uint32_t detected = p7501_hw_autodetect_flash_size();
    if (detected == 0U) {
        p7501_flash_geometry_t none = {0U};
        return none;
    }
    (void)gdb_rsp_set_flash_size(detected);
    return p7501_flash_get_geometry(detected);
}
