/**
 * @file picofw_qspi_map.h
 * @brief Pure ISO C11 6-Partition 2 MiB / 4 MiB RP2040/RP2350 QSPI Flash Map (`P0..P5`).
 *
 * Partition Table (2 MiB Baseline):
 * - P0 (`0x000000..0x03FFFF`, 256 KiB): `P0_FW_RO` — Firmware `.text` + `.rodata` (PERMANENTLY READ-ONLY)
 * - P1 (`0x040000..0x0FFFFF`, 768 KiB): `P1_STUBS` — Target Stubs / `ifodev2` / `DIGADIAG` payloads
 * - P2 (`0x100000..0x17FFFF`, 512 KiB): `P2_VAULT` — A/B 2 x 256 KiB Forensic NVRAM Vault (`0x100000`, `0x140000`)
 * - P3 (`0x180000..0x1BFFFF`, 256 KiB): `P3_RECIPE` — Signed repair recipes & profile overlays
 * - P4 (`0x1C0000..0x1EFFFF`, 192 KiB): `P4_BLACKBOX` — 48 x 4 KiB wear-leveled diagnostic telemetry ring
 * - P5 (`0x1F0000..0x1FFFFF`,  64 KiB): `P5_PROBE_CFG` — A/B 2 x 32 KiB probe calibration & SSOT hash
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_QSPI_MAP_H
#define PICOFW_QSPI_MAP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_QSPI_SECTOR_BYTES          0x00001000UL /* 4 KiB erase sector */
#define PICOFW_QSPI_PAGE_BYTES            0x00000100UL /* 256 B program page */
#define PICOFW_QSPI_BASE_2MB_BYTES        0x00200000UL /* 2 MiB RP2040 */
#define PICOFW_QSPI_EXP_4MB_BYTES         0x00400000UL /* 4 MiB RP2350 */

#define PICOFW_QSPI_P0_FW_OFFSET          0x00000000UL
#define PICOFW_QSPI_P0_FW_SIZE            0x00040000UL

#define PICOFW_QSPI_P1_STUBS_OFFSET       0x00040000UL
#define PICOFW_QSPI_P1_STUBS_SIZE         0x000C0000UL

#define PICOFW_QSPI_P2_VAULT_OFFSET       0x00100000UL
#define PICOFW_QSPI_P2_VAULT_SIZE         0x00080000UL
#define PICOFW_QSPI_VAULT_BANK_BYTES      0x00040000UL
#define PICOFW_QSPI_VAULT_BANK_A_OFFSET   0x00100000UL
#define PICOFW_QSPI_VAULT_BANK_B_OFFSET   0x00140000UL

#define PICOFW_QSPI_P3_RECIPE_OFFSET      0x00180000UL
#define PICOFW_QSPI_P3_RECIPE_SIZE        0x00040000UL

#define PICOFW_QSPI_P4_BLACKBOX_OFFSET    0x001C0000UL
#define PICOFW_QSPI_P4_BLACKBOX_SIZE      0x00030000UL
#define PICOFW_QSPI_BLACKBOX_PAGES        48u

#define PICOFW_QSPI_P5_CFG_OFFSET         0x001F0000UL
#define PICOFW_QSPI_P5_CFG_SIZE           0x00010000UL
#define PICOFW_QSPI_CFG_BANK_BYTES        0x00008000UL
#define PICOFW_QSPI_CFG_BANK_A_OFFSET     0x001F0000UL
#define PICOFW_QSPI_CFG_BANK_B_OFFSET     0x001F8000UL

#define PICOFW_QSPI_P6_MIRROR_OFFSET      0x00200000UL
#define PICOFW_QSPI_P6_MIRROR_SIZE_4MB    0x00200000UL /* Upper 2 MiB on 4 MiB RP2350 Pico 2 */

typedef enum picofw_qspi_part_id {
    PICOFW_QSPI_PART_FW_RO = 0,
    PICOFW_QSPI_PART_STUBS = 1,
    PICOFW_QSPI_PART_VAULT = 2,
    PICOFW_QSPI_PART_RECIPE = 3,
    PICOFW_QSPI_PART_BLACKBOX = 4,
    PICOFW_QSPI_PART_PROBE_CFG = 5,
    PICOFW_QSPI_PART_NOR_MIRROR = 6,
    PICOFW_QSPI_PART_COUNT = 7
} picofw_qspi_part_id_t;

typedef struct picofw_qspi_part_info {
    picofw_qspi_part_id_t id;
    const char *name;
    uint32_t offset;
    uint32_t size;
    bool writable;
} picofw_qspi_part_info_t;

/** Query partition descriptor for `id` given `total_qspi_bytes` (>= 2 MiB). */
bool picofw_qspi_partition_info(
    picofw_qspi_part_id_t id,
    uint32_t total_qspi_bytes,
    picofw_qspi_part_info_t *out_info
);

/**
 * Validate that `[qspi_offset .. qspi_offset + len)` lies strictly inside `part_id`
 * and is permitted to be erased/programmed (`P0_FW_RO` is permanently blocked).
 */
bool picofw_qspi_is_writable_range(
    picofw_qspi_part_id_t part_id,
    uint32_t qspi_offset,
    uint32_t len,
    uint32_t total_qspi_bytes
);

/** Return canonical partition name string. */
const char *picofw_qspi_partition_name(picofw_qspi_part_id_t id);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_QSPI_MAP_H */
