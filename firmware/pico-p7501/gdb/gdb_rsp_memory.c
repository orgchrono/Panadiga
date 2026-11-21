/**
 * @file gdb_rsp_memory.c
 * @brief Explicit-context GDB RSP target memory aperture, bounded cache, XML map, and IPL guard.
 *
 * Enforces OPUS-07 read-only isolation over protected Panasonic Bootloader IPL
 * ranges (`0x80000000 .. nvram_start - 1`), lethal MMIO `0xC000100C`, CS0 alias
 * mirrors, and F14 wireless quotas via `p7501_bus_check_access()`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "gdb/gdb_rsp_internal.h"
#include "flash/p7501_flash.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_wireless_codec.h"
#include "uart/p7501_pana_rsp_ssot.h"
#include <stdio.h>
#include <string.h>

static uint32_t normalized_flash_size(const gdb_rsp_ctx_t *ctx) {
    if (gdb_rsp_is_default_ctx(ctx)) {
        return p7501_flash_get_active_size();
    }
    if ((ctx != NULL) && (ctx->flash_size_bytes >= P7501_FLASH_SIZE_1MB)) {
        return p7501_flash_normalize_size(ctx->flash_size_bytes);
    }
    return P7501_FLASH_SIZE_8MB;
}

bool gdb_rsp_ctx_set_flash_size(gdb_rsp_ctx_t *ctx, uint32_t flash_size_bytes) {
    if (ctx == NULL) {
        return false;
    }
    if ((flash_size_bytes != P7501_FLASH_SIZE_1MB) &&
        (flash_size_bytes != P7501_FLASH_SIZE_2MB) &&
        (flash_size_bytes != P7501_FLASH_SIZE_4MB) &&
        (flash_size_bytes != P7501_FLASH_SIZE_8MB) &&
        (flash_size_bytes != P7501_FLASH_SIZE_16MB)) {
        return false;
    }
    ctx->flash_size_bytes = flash_size_bytes;
    if (gdb_rsp_is_default_ctx(ctx)) {
        (void)p7501_flash_set_active_size(flash_size_bytes);
    }
    gdb_rsp_ctx_invalidate_cache(ctx);
    return true;
}

uint32_t gdb_rsp_ctx_get_flash_size(const gdb_rsp_ctx_t *ctx) {
    return normalized_flash_size(ctx);
}

void gdb_rsp_ctx_invalidate_cache(gdb_rsp_ctx_t *ctx) {
    if (ctx == NULL) {
        return;
    }
    ctx->cache_valid = false;
    ctx->cache_base_addr = 0xFFFFFFFFUL;
}

static bool populate_cache(gdb_rsp_ctx_t *ctx, uint32_t aligned_addr) {
    if (!p7501_hw_read_block_checked(aligned_addr, ctx->sector_cache, GDB_SECTOR_CACHE_SIZE, false)) {
        ctx->cache_valid = false;
        return false;
    }
    ctx->cache_base_addr = aligned_addr;
    ctx->cache_valid = true;
    return true;
}

static bool memory_read_byte(gdb_rsp_ctx_t *ctx, uint32_t addr, uint8_t *out_byte) {
    if (ctx->cache_valid &&
        (addr >= ctx->cache_base_addr) &&
        (addr < (ctx->cache_base_addr + GDB_SECTOR_CACHE_SIZE))) {
        *out_byte = ctx->sector_cache[addr - ctx->cache_base_addr];
        return true;
    }

    const uint32_t flash_bytes = normalized_flash_size(ctx);
    if ((addr >= P7501_SSOT_CS0_CACHED_BASE) && (addr < (P7501_SSOT_CS0_CACHED_BASE + flash_bytes))) {
        const uint32_t aligned = addr & ~(GDB_SECTOR_CACHE_SIZE - 1U);
        if (!populate_cache(ctx, aligned)) {
            return false;
        }
        *out_byte = ctx->sector_cache[addr - aligned];
        return true;
    }

    const uint32_t aligned_addr = addr & ~0x03UL;
    uint32_t word = 0UL;
    if (!p7501_hw_read_mem32_checked(aligned_addr, &word)) {
        return false;
    }
    const uint32_t byte_idx = addr & 0x03UL;
    *out_byte = (uint8_t)((word >> (byte_idx * 8U)) & 0xFFU);
    return true;
}

static bool is_writable_nor_window(
    const gdb_rsp_ctx_t *ctx,
    uint32_t base,
    uint32_t flash_bytes,
    uint32_t addr,
    uint32_t end_addr
) {
    const bool unlocked = gdb_rsp_ctx_is_upper_nor_unlocked(ctx);
    const uint32_t flash_end = base + (flash_bytes - 1UL);
    const p7501_session_ctx_t *sess = gdb_rsp_is_default_ctx(ctx) ? p7501_session_default_ctx() : NULL;
    return (addr >= base) && (end_addr <= flash_end) &&
           !p7501_flash_is_write_blocked_in_session(
               sess, flash_bytes, addr, (size_t)(end_addr - addr + 1UL), unlocked
           );
}

static bool is_valid_write_range(const gdb_rsp_ctx_t *ctx, uint32_t addr, uint32_t length) {
    if ((length == 0U) || p7501_bus_is_lethal_mmio_range(addr, (size_t)length)) {
        return false;
    }
    const uint32_t end_addr = addr + (length - 1U);
    if (end_addr < addr) {
        return false;
    }
    if (!p7501_wireless_nor_write_allowed(p7501_wireless_global_state()) &&
        p7501_bus_touches_cs0_aperture(addr, (size_t)length)) {
        return false;
    }

    const uint32_t flash_bytes = normalized_flash_size(ctx);
    if (is_writable_nor_window(ctx, P7501_FLASH_CACHED_BASE, flash_bytes, addr, end_addr) ||
        is_writable_nor_window(ctx, P7501_FLASH_HW_UNCACHED_BASE, flash_bytes, addr, end_addr)) {
        return true;
    }
    if (p7501_bus_touches_cs0_aperture(addr, (size_t)length)) {
        return false;
    }

    if (end_addr <= 0x0003FFFFUL) { return true; }                             /* Internal SRAM */
    if ((addr >= 0x10000000UL) && (end_addr <= 0x1000FFFFUL)) { return true; } /* Core MMIO */
    if ((addr >= 0x40000000UL) && (end_addr <= 0x43FFFFFFUL)) { return true; } /* System SDRAM */
    if ((addr >= 0xC0000000UL) && (end_addr <= 0xC000FFFFUL)) { return true; } /* OCD MMIO */
    if ((addr >= 0xD4000000UL) && (end_addr <= 0xD4000FFFUL)) { return true; } /* PATA / IDE MMIO */

    return false;
}

static bool memory_write_byte(gdb_rsp_ctx_t *ctx, uint32_t addr, uint8_t byte_val, bool *out_bus_err) {
    if (out_bus_err != NULL) {
        *out_bus_err = false;
    }
    if (!is_valid_write_range(ctx, addr, 1U)) {
        return false;
    }

    const uint32_t aligned_addr = addr & ~0x03UL;
    const uint32_t byte_idx = addr & 0x03UL;
    uint32_t current_word = 0UL;
    /* Checked RMW read: never merge into a fake 0xFFFFFFFF bus-error word! */
    if (!p7501_hw_read_mem32_checked(aligned_addr, &current_word)) {
        if (out_bus_err != NULL) {
            *out_bus_err = true;
        }
        return false;
    }

    const uint32_t mask = 0xFFUL << (byte_idx * 8U);
    const uint32_t new_word = (current_word & ~mask) | (((uint32_t)byte_val) << (byte_idx * 8U));

    p7501_hw_write_mem32(aligned_addr, new_word);

    if (ctx->cache_valid &&
        (addr >= ctx->cache_base_addr) &&
        (addr < (ctx->cache_base_addr + GDB_SECTOR_CACHE_SIZE))) {
        ctx->sector_cache[addr - ctx->cache_base_addr] = byte_val;
    }
    return true;
}

void gdb_mem_send_memory_map_xml(gdb_rsp_ctx_t *ctx, char *resp_buf, size_t resp_cap) {
    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(normalized_flash_size(ctx));
    const uint32_t rom_len = geo.bytes - geo.nvram_size;
    if (geo.nvram_size == 0UL) {
        /* Full-RO profile (DMR-EZ485V / DMR-ES10, nvram_size == 0): omit <memory type="flash"> block. */
        snprintf(resp_buf, resp_cap,
            "l<memory-map>"
            "<memory type=\"ram\" start=\"0x00000000\" length=\"0x00040000\"/>"
            "<memory type=\"ram\" start=\"0x10000000\" length=\"0x00010000\"/>"
            "<memory type=\"ram\" start=\"0x40000000\" length=\"0x04000000\"/>"
            "<memory type=\"rom\" start=\"0x80000000\" length=\"0x%08lX\"/>"
            "</memory-map>",
            (unsigned long)rom_len);
    } else {
        const uint32_t nvram_base = P7501_FLASH_CACHED_BASE + rom_len;
        snprintf(resp_buf, resp_cap,
            "l<memory-map>"
            "<memory type=\"ram\" start=\"0x00000000\" length=\"0x00040000\"/>"
            "<memory type=\"ram\" start=\"0x10000000\" length=\"0x00010000\"/>"
            "<memory type=\"ram\" start=\"0x40000000\" length=\"0x04000000\"/>"
            "<memory type=\"rom\" start=\"0x80000000\" length=\"0x%08lX\"/>"
            "<memory type=\"flash\" start=\"0x%08lX\" length=\"0x%08lX\">"
            "<property name=\"blocksize\">0x2000</property>"
            "</memory>"
            "</memory-map>",
            (unsigned long)rom_len,
            (unsigned long)nvram_base,
            (unsigned long)geo.nvram_size);
    }
    gdb_rsp_ctx_send_packet(ctx, resp_buf);
}

void gdb_mem_handle_read_cmd(gdb_rsp_ctx_t *ctx, const char *arg, char *resp_buf) {
    uint32_t addr = 0U;
    uint32_t length = 0U;
    if ((gdb_parse_addr_and_len(arg, &addr, &length, '\0') == NULL) ||
        (length > ((GDB_RSP_MAX_PACKET_LEN - 2U) / 2U))) {
        gdb_rsp_ctx_send_packet(ctx, "E02");
        return;
    }
    if (p7501_bus_is_lethal_mmio_range(addr, (size_t)length)) {
        gdb_rsp_ctx_send_packet(ctx, "E05");
        return;
    }

    char *out = resp_buf;
    for (uint32_t i = 0U; i < length; i++) {
        uint8_t b = 0u;
        if (!memory_read_byte(ctx, addr + i, &b)) {
            gdb_rsp_ctx_send_packet(ctx, "E01");
            return;
        }
        *out++ = gdb_nibble_to_hex_char((uint8_t)(b >> 4));
        *out++ = gdb_nibble_to_hex_char((uint8_t)(b & 0x0FU));
    }
    *out = '\0';
    gdb_rsp_ctx_send_packet(ctx, resp_buf);
}

static const char *parse_and_validate_write_header(
    gdb_rsp_ctx_t *ctx,
    const char *arg,
    uint32_t *out_addr,
    uint32_t *out_len
) {
    const char *p = gdb_parse_addr_and_len(arg, out_addr, out_len, ':');
    if (p == NULL) {
        gdb_rsp_ctx_send_packet(ctx, "E03");
        return NULL;
    }
    if (!is_valid_write_range(ctx, *out_addr, *out_len)) {
        gdb_rsp_ctx_send_packet(ctx, "E05");
        return NULL;
    }
    return p;
}

void gdb_mem_handle_write_cmd(gdb_rsp_ctx_t *ctx, const char *arg) {
    uint32_t addr = 0U;
    uint32_t length = 0U;
    const char *p = parse_and_validate_write_header(ctx, arg, &addr, &length);
    if (p == NULL) {
        return;
    }

    if (strlen(p) < (length * 2U)) {
        gdb_rsp_ctx_send_packet(ctx, "E04");
        return;
    }

    bool write_ok = true;
    bool bus_err = false;
    uint32_t written = 0U;
    for (uint32_t i = 0U; i < length; i++) {
        const int h = gdb_hex_char_to_nibble(*p++);
        const int l = gdb_hex_char_to_nibble(*p++);
        if ((h >= 0) && (l >= 0)) {
            const uint8_t byte = (uint8_t)((h << 4) | l);
            if (!memory_write_byte(ctx, addr + i, byte, &bus_err)) {
                write_ok = false;
                break;
            }
            written++;
        } else {
            if ((written > 0U) && p7501_bus_touches_cs0_aperture(addr, (size_t)length)) {
                p7501_wireless_consume_nor_op(p7501_wireless_global_state());
            }
            gdb_rsp_ctx_send_packet(ctx, "E04");
            return;
        }
    }

    if ((write_ok || (written > 0U)) && p7501_bus_touches_cs0_aperture(addr, (size_t)length)) {
        p7501_wireless_consume_nor_op(p7501_wireless_global_state());
    }
    gdb_rsp_ctx_send_packet(ctx, write_ok ? "OK" : (bus_err ? "E01" : "E05"));
}

void gdb_mem_handle_write_bin_cmd(gdb_rsp_ctx_t *ctx, const char *arg, size_t arg_len) {
    uint32_t addr = 0U;
    uint32_t length = 0U;
    const char *p = parse_and_validate_write_header(ctx, arg, &addr, &length);
    if (p == NULL) {
        return;
    }
    const size_t hdr_len = (size_t)(p - arg);
    if (hdr_len > arg_len) {
        gdb_rsp_ctx_send_packet(ctx, "E03");
        return;
    }

    size_t rx_pos = hdr_len;
    uint32_t written = 0U;
    bool write_ok = true;
    bool bus_err = false;
    while ((written < length) && (rx_pos < arg_len)) {
        uint8_t byte = (uint8_t)arg[rx_pos++];
        if (byte == 0x7DU) {
            if (rx_pos >= arg_len) {
                if ((written > 0U) && p7501_bus_touches_cs0_aperture(addr, (size_t)length)) {
                    p7501_wireless_consume_nor_op(p7501_wireless_global_state());
                }
                gdb_rsp_ctx_send_packet(ctx, "E04");
                return;
            }
            byte = (uint8_t)((uint8_t)arg[rx_pos++] ^ 0x20U);
        }
        if (!memory_write_byte(ctx, addr + written, byte, &bus_err)) {
            write_ok = false;
            break;
        }
        written++;
    }
    if ((write_ok || (written > 0U)) && p7501_bus_touches_cs0_aperture(addr, (size_t)length)) {
        p7501_wireless_consume_nor_op(p7501_wireless_global_state());
    }
    if (written != length) {
        gdb_rsp_ctx_send_packet(ctx, bus_err ? "E01" : "E04");
        return;
    }
    gdb_rsp_ctx_send_packet(ctx, write_ok ? "OK" : (bus_err ? "E01" : "E05"));
}
