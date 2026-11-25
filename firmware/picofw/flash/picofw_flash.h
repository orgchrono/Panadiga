/**
 * @file picofw_flash.h
 * @brief Parallel NOR Flash geometry discovery & Spansion EEMUL NVRAM recovery engine.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_FLASH_H
#define PICOFW_FLASH_H

#include <stddef.h>
#include "picofw_types.h"
#include "flash/picofw_bus_policy.h"
#include "hal/picofw_wireless_codec.h"
#include "uart/picofw_pana_rsp_ssot.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Supported Spansion NOR Flash apertures and NVRAM partition bounds. */
#define PICOFW_FLASH_SIZE_1MB           0x00100000UL
#define PICOFW_FLASH_SIZE_2MB           0x00200000UL
#define PICOFW_FLASH_SIZE_4MB           0x00400000UL
#define PICOFW_FLASH_SIZE_8MB           0x00800000UL
#define PICOFW_FLASH_SIZE_16MB          0x01000000UL
#define PICOFW_FLASH_NVRAM_SIZE_64KB    0x00010000UL
#define PICOFW_FLASH_NVRAM_SIZE_128KB   0x00020000UL
#define PICOFW_FLASH_8MB_IPL_END_OFF    0x007EFFFFUL
#define PICOFW_FLASH_8MB_NVRAM_OFF      0x007F0000UL
#define PICOFW_FLASH_8MB_END_OFF        0x007FFFFFUL

/** Pure parser output request for mutating or resetting Upper NOR unlock state. */
typedef enum picofw_policy_req {
    PICOFW_POLICY_REQ_NONE = 0,
    PICOFW_POLICY_REQ_LOCK = 1,
    PICOFW_POLICY_REQ_UNLOCK = 2
} picofw_policy_req_t;

/** Unified Probe Session Context shared across CLI, PANARSP, DIAGWATCH, and GDB RSP (F5 + #22). */
typedef struct picofw_session_ctx {
    uint32_t active_flash_size;
    picofw_ssot_target_profile_id_t active_profile;
    bool profile_selected;
    bool upper_nor_unlocked;
    bool jedec_verified;
    uint16_t jedec_mfg;
    uint16_t jedec_dev[3];
} picofw_session_ctx_t;

/** Cached / GDB RSP Spansion NOR aperture base address (`0x80000000`, from SSOT). */
#define PICOFW_FLASH_CACHED_BASE        PICOFW_SSOT_CS0_CACHED_BASE
/** Real MN103S hardware uncached NOR aperture base used by BOOT/PROG061 (`0x84000000`, from SSOT). */
#define PICOFW_FLASH_HW_UNCACHED_BASE   PICOFW_SSOT_CS0_UNCACHED_BASE
/** Aliasing mirror probes (`0x80200000`, `0x80400000`, `0x80800000`). */
#define PICOFW_FLASH_MIRROR_2MB_VA      (PICOFW_FLASH_CACHED_BASE + PICOFW_FLASH_SIZE_2MB)
#define PICOFW_FLASH_MIRROR_4MB_VA      (PICOFW_FLASH_CACHED_BASE + PICOFW_FLASH_SIZE_4MB)
#define PICOFW_FLASH_MIRROR_8MB_VA      (PICOFW_FLASH_CACHED_BASE + PICOFW_FLASH_SIZE_8MB)
/** Standard 16-bit JEDEC CFI query & reset commands. */
#define PICOFW_FLASH_JEDEC_CFI_ADDR     (PICOFW_FLASH_CACHED_BASE + 0x000000AAUL)
#define PICOFW_FLASH_JEDEC_QRY0_ADDR    (PICOFW_FLASH_CACHED_BASE + 0x00000020UL)
#define PICOFW_FLASH_JEDEC_QRY1_ADDR    (PICOFW_FLASH_CACHED_BASE + 0x00000024UL)
#define PICOFW_FLASH_JEDEC_SIZE_ADDR    (PICOFW_FLASH_CACHED_BASE + 0x0000004CUL)
#define PICOFW_FLASH_JEDEC_CFI_CMD      0x00980098UL
#define PICOFW_FLASH_JEDEC_RESET_CMD    0x00F000F0UL
/** Real DMR-EH55 `Loader ver0.61` `IC5001` table `0x440337A4` places BGO at `0x84130000..0x843FFFFF`,
 * 45 x 64 KiB `SA26..SA70`, and Upper EEMUL across `SA71..SA141` (`0x84400000..0x847FFFFF`). */
#define PICOFW_BGO_PARTITION_OFFSET     PICOFW_SSOT_BGO_OFFSET
#define PICOFW_BGO_PARTITION_SIZE       PICOFW_SSOT_BGO_SIZE
#define PICOFW_BGO_SECTOR_SIZE          0x00010000UL
#define PICOFW_BGO_SECTOR_COUNT         PICOFW_SSOT_BGO_SECTOR_COUNT
#define PICOFW_BGO_PARTITION_COUNT      PICOFW_BGO_SECTOR_COUNT

typedef struct picofw_flash_geometry {
    uint32_t bytes;        /**< Total Flash size in bytes (2MB/4MB/8MB/16MB) */
    uint32_t mb;           /**< Total Flash size in MiB */
    uint32_t nvram_size;   /**< EEMUL NVRAM aperture size (64KB or 128KB for 16MB) */
    uint32_t nvram_base;   /**< Cached/GDB RSP NVRAM start address (0x80xxxxxx) */
    uint32_t nvram_end;    /**< Cached/GDB RSP NVRAM exclusive end address */
    uint32_t hw_uncached_base; /**< Real MN103S uncached bus window base (0x84000000) */
    uint32_t hw_nvram_base;    /**< Physical uncached NVRAM start (0x84000000 + offset) */
    uint32_t hw_nvram_end;     /**< Physical uncached NVRAM exclusive end */
    uint32_t bgo_hw_base;      /**< PROG061/ver0.63 BGO/[nfrom] start VA (0x84200000 if >= 4MB; EH55 ver0.61 = 0x84130000) */
    uint32_t bgo_hw_end;       /**< BGO/[nfrom] partition exclusive end VA (`0x84400000`) */
    uint32_t bgo_sector_count; /**< Number of 64 KB BGO sectors (`32` PROG061/ver0.63; `45` on real EH55 ver0.61) */
    uint32_t base_sector;  /**< First Spansion micro-sector index (e.g. SA134) */
    uint32_t sector_count; /**< Number of 8 KB micro-sectors in NVRAM partition */
} picofw_flash_geometry_t;

picofw_session_ctx_t *picofw_session_default_ctx(void);
void picofw_session_ctx_reset(picofw_session_ctx_t *ctx);
const picofw_ssot_target_profile_t *picofw_flash_get_target_profile(void);
bool picofw_flash_set_target_profile_by_id(picofw_ssot_target_profile_id_t profile_id);
bool picofw_flash_set_target_profile_name(const char *model_name);
const picofw_ssot_flash_chip_t *picofw_flash_find_chip_for_session(
    const picofw_session_ctx_t *eff,
    uint32_t eff_size
);
const picofw_ssot_flash_chip_t *picofw_flash_find_chip_by_size(uint32_t flash_size_bytes);
const picofw_ssot_flash_chip_t *picofw_flash_find_chip_by_jedec(
    uint16_t mfg,
    const uint16_t dev[3],
    uint8_t dev_cycles
);
bool picofw_flash_get_sector_span_at(
    uint32_t flash_size_bytes,
    uint32_t addr,
    uint32_t *out_base_addr,
    uint32_t *out_size
);
bool picofw_flash_is_write_blocked_in_session(
    const picofw_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked
);

uint32_t picofw_flash_normalize_size(uint32_t flash_size_bytes);
uint32_t picofw_flash_get_active_size(void);
bool picofw_flash_set_active_size(uint32_t flash_size_bytes);
picofw_flash_geometry_t picofw_flash_get_geometry(uint32_t flash_size_bytes);
bool picofw_flash_is_readonly_nor_range(uint32_t flash_size_bytes,
                                       uint32_t start_addr,
                                       uint32_t total_len);
bool picofw_flash_is_protected_ipl_range_for_flash(uint32_t flash_size_bytes, uint32_t addr, size_t len);
bool picofw_flash_is_protected_ipl_range(uint32_t addr, size_t len);
bool picofw_flash_is_critical_boot_ipl_range(uint32_t addr, size_t len);
bool picofw_flash_is_upper_nor_unlocked(void);
void picofw_flash_set_upper_nor_unlocked(bool unlocked);
void picofw_flash_apply_policy_req(picofw_policy_req_t req);
bool picofw_flash_is_write_blocked_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked
);
bool picofw_flash_is_write_blocked_with_policy(uint32_t addr, size_t len, bool upper_nor_unlocked);
bool picofw_flash_is_write_blocked(uint32_t addr, size_t len);
bool picofw_flash_is_erasable_nor_addr_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    bool upper_nor_unlocked
);
bool picofw_flash_is_erasable_nor_addr(uint32_t addr, bool upper_nor_unlocked);
uint32_t picofw_flash_normalize_uncached_addr(uint32_t addr);
int picofw_flash_eval_policy_token(const char *tok);
bool picofw_flash_consume_policy_tok(const char *tok, int *io_override, bool *out_invalid);
bool picofw_flash_effective_policy(int policy_override);
uint32_t picofw_flash_get_sector_size_at(uint32_t flash_size_bytes, uint32_t addr);
bool picofw_flash_is_nvram_microsector_addr_for_flash(uint32_t flash_size_bytes, uint32_t addr);
bool picofw_flash_is_nvram_microsector_addr(uint32_t addr);
uint32_t picofw_hw_autodetect_flash_size(void);
bool picofw_hw_query_jedec_id(uint16_t *out_mfg, uint16_t out_dev[3], bool *out_verified);
bool picofw_hw_query_cfi(uint32_t *out_size_bytes);
picofw_flash_geometry_t picofw_probe_flash_geometry(void);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_FLASH_H */
