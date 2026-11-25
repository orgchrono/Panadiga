/**
 * @file picofw_qspi_hal.c
 * @brief Low-level RP2040/RP2350 QSPI Flash erase/program HAL with fail-closed multicore lockout.
 *
 * Extracted from monolithic `picofw_qspi_storage.c`:
 *  - Enforces `picofw_qspi_is_writable_range` and `__flash_binary_end` (`P0_FW_RO` immutable).
 *  - Isolates `P7501_PIN_TXD` (`GP4`) Hi-Z before QSPI sector erase (`AUD-12` anti-backpowering).
 *  - Uses bounded fail-closed `picofw_multicore_lockout_begin()` (50 ms timeout) so QSPI ops
 *    never deadlock into a WDT reset.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_qspi_storage.h"
#include "common/picofw_hex.h"
#include "flash/picofw_qspi_map.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_multicore.h"
#include "hal/picofw_watchdog.h"

#include "hardware/flash.h"
#include "hardware/sync.h"
#include "pico/stdlib.h"

#include <string.h>

uint32_t picofw_qspi_total_bytes(void) {
#ifdef PICO_FLASH_SIZE_BYTES
    return (uint32_t)PICO_FLASH_SIZE_BYTES;
#else
    return PICOFW_QSPI_BASE_2MB_BYTES;
#endif
}

const uint8_t *picofw_qspi_xip_ptr(uint32_t qspi_offset) {
    return (const uint8_t *)(XIP_BASE + qspi_offset);
}

static uint32_t qspi_binary_end_offset(void) {
#if !defined(PICO_HOST_TEST)
    extern char __flash_binary_end[];
    const uintptr_t bin_end = (uintptr_t)__flash_binary_end;
    if (bin_end >= (uintptr_t)XIP_BASE) {
        const uint32_t total = picofw_qspi_total_bytes();
        const uint32_t end_off = (uint32_t)(bin_end - (uintptr_t)XIP_BASE);
        return (end_off <= total) ? end_off : total;
    }
#endif
    return 0UL;
}

uint32_t picofw_qspi_crc32_fast(const uint8_t *buf, size_t len) {
    if ((buf == NULL) || (len == 0u)) {
        return 0UL;
    }
    uint32_t crc = 0UL;
    for (size_t off = 0u; off < len; off += 4096u) {
        const size_t chunk = ((len - off) > 4096u) ? 4096u : (len - off);
        crc = picofw_crc32_ieee_update(crc, &buf[off], chunk);
        picofw_wdt_kick();
    }
    return crc;
}

uint32_t picofw_qspi_firmware_crc32(uint32_t *out_bytes) {
    const uint32_t fw_len = qspi_binary_end_offset();
    if (out_bytes != NULL) {
        *out_bytes = fw_len;
    }
    if (fw_len == 0UL) {
        return 0UL;
    }
    return picofw_qspi_crc32_fast(picofw_qspi_xip_ptr(0UL), (size_t)fw_len);
}

static bool qspi_check_binary_boundary(uint32_t qspi_offset) {
    const uint32_t end_off = qspi_binary_end_offset();
    return qspi_offset >= end_off;
}

bool picofw_qspi_erase_range_safe(
    picofw_qspi_part_id_t part_id,
    uint32_t qspi_offset,
    uint32_t len
) {
    const uint32_t total = picofw_qspi_total_bytes();
    if ((len == 0UL) ||
        ((qspi_offset % FLASH_SECTOR_SIZE) != 0UL) ||
        ((len % FLASH_SECTOR_SIZE) != 0UL) ||
        !picofw_qspi_is_writable_range(part_id, qspi_offset, len, total) ||
        !qspi_check_binary_boundary(qspi_offset)) {
        return false;
    }
    /* AUD-12: float GP4 TXD before IRQ-disabled QSPI erase so NSW5V drop cannot backpower target. */
    picofw_hw_release_txd_hiz(false);
    picofw_hw_enforce_nsw5v_hiz();

    for (uint32_t off = 0UL; off < len; off += FLASH_SECTOR_SIZE) {
        picofw_wdt_kick();
        if (!picofw_multicore_lockout_begin()) {
            return false;
        }
        const uint32_t irq = save_and_disable_interrupts();
        flash_range_erase(qspi_offset + off, FLASH_SECTOR_SIZE);
        restore_interrupts(irq);
        picofw_multicore_lockout_end();
        picofw_wdt_kick();
    }
    return true;
}

bool picofw_qspi_program_range_safe(
    picofw_qspi_part_id_t part_id,
    uint32_t qspi_offset,
    const uint8_t *data,
    size_t len
) {
    const uint32_t total = picofw_qspi_total_bytes();
    if ((data == NULL) || (len == 0u) ||
        ((qspi_offset % FLASH_PAGE_SIZE) != 0UL) ||
        (len > PICOFW_QSPI_P1_STUBS_SIZE) ||
        !picofw_qspi_is_writable_range(part_id, qspi_offset, (uint32_t)len, total) ||
        !qspi_check_binary_boundary(qspi_offset)) {
        return false;
    }
    size_t off = 0u;
    uint32_t pages = 0u;
    while (off < len) {
        uint8_t page_buf[FLASH_PAGE_SIZE];
        memset(page_buf, 0xFF, sizeof(page_buf));
        const size_t chunk = ((len - off) > FLASH_PAGE_SIZE) ? FLASH_PAGE_SIZE : (len - off);
        memcpy(page_buf, &data[off], chunk);
        if ((pages & 0x0Fu) == 0u) {
            picofw_wdt_kick();
        }
        if (!picofw_multicore_lockout_begin()) {
            return false;
        }
        const uint32_t irq = save_and_disable_interrupts();
        flash_range_program(qspi_offset + (uint32_t)off, page_buf, FLASH_PAGE_SIZE);
        restore_interrupts(irq);
        picofw_multicore_lockout_end();
        off += chunk;
        pages += 1u;
    }
    picofw_wdt_kick();
    return true;
}

bool picofw_qspi_write_sector_page(
    picofw_qspi_part_id_t part_id,
    uint32_t sector_off,
    const void *struct_ptr,
    size_t struct_len
) {
    if ((struct_ptr == NULL) || (struct_len == 0u) || (struct_len > FLASH_PAGE_SIZE)) {
        return false;
    }
    return picofw_qspi_erase_range_safe(part_id, sector_off, FLASH_SECTOR_SIZE) &&
           picofw_qspi_program_range_safe(part_id, sector_off, (const uint8_t *)struct_ptr, struct_len);
}
