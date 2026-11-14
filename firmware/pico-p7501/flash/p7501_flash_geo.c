/**
 * @file p7501_flash_geo.c
 * @brief Pure C11 Spansion NOR Flash, EEMUL NVRAM & BGO/[nfrom] geometry helpers.
 *
 * Host-testable with zero Pico SDK hardware dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_flash.h"
#include "flash/ram_loader_payload.h"

uint32_t p7501_flash_normalize_size(uint32_t flash_size_bytes) {
    if ((flash_size_bytes == 0x00200000UL) ||
        (flash_size_bytes == 0x00400000UL) ||
        (flash_size_bytes == 0x00800000UL) ||
        (flash_size_bytes == 0x01000000UL)) {
        return flash_size_bytes;
    }
    return 0x00800000UL;
}

p7501_flash_geometry_t p7501_flash_get_geometry(uint32_t flash_size_bytes) {
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    const uint32_t nvram_size = (eff_size == 0x01000000UL) ? 0x00020000UL : 0x00010000UL;
    const uint32_t nvram_offset = eff_size - nvram_size;
    const bool has_bgo = (eff_size >= 0x00400000UL);

    p7501_flash_geometry_t geo;
    geo.bytes = eff_size;
    geo.mb = eff_size / (1024UL * 1024UL);
    geo.nvram_size = nvram_size;
    geo.nvram_base = P7501_FLASH_CACHED_BASE + nvram_offset;
    geo.nvram_end = P7501_FLASH_CACHED_BASE + eff_size;
    geo.hw_uncached_base = SPANSION_FLASH_HW_UNCACHED_BASE;
    geo.hw_nvram_base = SPANSION_FLASH_HW_UNCACHED_BASE + nvram_offset;
    geo.hw_nvram_end = SPANSION_FLASH_HW_UNCACHED_BASE + eff_size;
    geo.bgo_hw_base = has_bgo ? (SPANSION_FLASH_HW_UNCACHED_BASE + P7501_BGO_PARTITION_OFFSET) : 0U;
    geo.bgo_hw_end = has_bgo
        ? (SPANSION_FLASH_HW_UNCACHED_BASE + P7501_BGO_PARTITION_OFFSET + P7501_BGO_PARTITION_SIZE)
        : 0U;
    geo.bgo_sector_count = has_bgo ? P7501_BGO_SECTOR_COUNT : 0U;
    geo.sector_count = nvram_size / SPANSION_MICROSECTOR_SIZE_BYTES;
    geo.base_sector = (eff_size == 0x01000000UL) ? 254U :
                      (eff_size == 0x00400000UL) ? 70U :
                      (eff_size == 0x00200000UL) ? 38U : 134U;
    return geo;
}

bool p7501_flash_is_readonly_nor_range(uint32_t flash_size_bytes,
                                       uint32_t start_addr,
                                       uint32_t total_len) {
    if (total_len == 0U) {
        return false;
    }
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    const uint64_t s64 = (uint64_t)start_addr;
    const uint64_t e64 = s64 + (uint64_t)total_len;

    const uint64_t cached_lo = (uint64_t)P7501_FLASH_CACHED_BASE;
    const uint64_t cached_hi = cached_lo + (uint64_t)eff_size;
    const uint64_t hw_lo = (uint64_t)SPANSION_FLASH_HW_UNCACHED_BASE;
    const uint64_t hw_hi = hw_lo + (uint64_t)eff_size;

    return ((s64 >= cached_lo) && (e64 <= cached_hi)) ||
           ((s64 >= hw_lo) && (e64 <= hw_hi));
}
