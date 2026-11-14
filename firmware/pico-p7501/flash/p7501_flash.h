/**
 * @file p7501_flash.h
 * @brief Parallel NOR Flash geometry discovery & Spansion EEMUL NVRAM recovery engine.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_FLASH_H
#define P7501_FLASH_H

#include <stddef.h>
#include "p7501_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Cached / GDB RSP Spansion NOR aperture base address (`0x80000000`). */
#define P7501_FLASH_CACHED_BASE        0x80000000UL
/** Real MN103S hardware uncached NOR aperture base used by BOOT/PROG061 (`0x84000000`). */
#define P7501_FLASH_HW_UNCACHED_BASE   0x84000000UL
/** BGO/[nfrom] partition base offset (2 MiB, `0x84200000..0x843FFFFF`, 32 x 64 KB). */
#define P7501_BGO_PARTITION_OFFSET     0x00200000UL
#define P7501_BGO_PARTITION_SIZE       0x00200000UL
#define P7501_BGO_SECTOR_SIZE          0x00010000UL
#define P7501_BGO_SECTOR_COUNT         32U

typedef struct p7501_flash_geometry {
    uint32_t bytes;        /**< Total Flash size in bytes (2MB/4MB/8MB/16MB) */
    uint32_t mb;           /**< Total Flash size in MiB */
    uint32_t nvram_size;   /**< EEMUL NVRAM aperture size (64KB or 128KB for 16MB) */
    uint32_t nvram_base;   /**< Cached/GDB RSP NVRAM start address (0x80xxxxxx) */
    uint32_t nvram_end;    /**< Cached/GDB RSP NVRAM exclusive end address */
    uint32_t hw_uncached_base; /**< Real MN103S uncached bus window base (0x84000000) */
    uint32_t hw_nvram_base;    /**< Physical uncached NVRAM start (0x84000000 + offset) */
    uint32_t hw_nvram_end;     /**< Physical uncached NVRAM exclusive end */
    uint32_t bgo_hw_base;      /**< BGO/[nfrom] partition start VA (0x84200000 if >= 4MB) */
    uint32_t bgo_hw_end;       /**< BGO/[nfrom] partition exclusive end VA */
    uint32_t bgo_sector_count; /**< Number of 64 KB BGO sectors (32 for 8MB NOR) */
    uint32_t base_sector;  /**< First Spansion micro-sector index (e.g. SA134) */
    uint32_t sector_count; /**< Number of 8 KB micro-sectors in NVRAM partition */
} p7501_flash_geometry_t;

uint32_t p7501_flash_normalize_size(uint32_t flash_size_bytes);
p7501_flash_geometry_t p7501_flash_get_geometry(uint32_t flash_size_bytes);
bool p7501_flash_is_readonly_nor_range(uint32_t flash_size_bytes,
                                       uint32_t start_addr,
                                       uint32_t total_len);
uint32_t p7501_hw_autodetect_flash_size(void);
bool p7501_hw_query_cfi(uint32_t *out_size_bytes);
p7501_flash_geometry_t p7501_probe_flash_geometry(void);

uint32_t ram_loader_calc_crc32(const uint8_t *data, size_t length);
uint32_t ram_loader_calc_sum32(const uint32_t *words, size_t count);

p7501_status_t ram_loader_inject_aperture(
    uint32_t target_sram_addr,
    bool verify,
    uint32_t flash_size_bytes,
    bool use_hw_uncached
);
p7501_status_t ram_loader_inject_for_flash_size(
    uint32_t target_sram_addr,
    bool verify,
    uint32_t flash_size_bytes
);
p7501_status_t ram_loader_inject(uint32_t target_sram_addr, bool verify);
p7501_status_t ram_loader_execute_erase_for_flash_size(uint32_t flash_size_bytes);
p7501_status_t ram_loader_execute_erase(void);

#ifdef __cplusplus
}
#endif

#endif /* P7501_FLASH_H */
