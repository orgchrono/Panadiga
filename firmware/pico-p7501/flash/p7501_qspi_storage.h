/**
 * @file p7501_qspi_storage.h
 * @brief Runtime RP2040/RP2350 QSPI storage driver for `P3_RECIPE`, `P4_BLACKBOX`, and `P5_PROBE_CFG`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_QSPI_STORAGE_H
#define P7501_QSPI_STORAGE_H

#include <stdbool.h>
#include <stdint.h>

#include "flash/p7501_blackbox_codec.h"
#include "flash/p7501_probe_cfg_codec.h"
#include "flash/p7501_qspi_map.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Return total onboard QSPI flash capacity in bytes (2 MiB RP2040 / 4 MiB RP2350). */
uint32_t p7501_qspi_total_bytes(void);

/** Return XIP read pointer for a QSPI flash byte offset. */
const uint8_t *p7501_qspi_xip_ptr(uint32_t qspi_offset);

/** Compute IEEE CRC32 over active `P0_FW_RO` binary image up to `__flash_binary_end`. */
uint32_t p7501_qspi_firmware_crc32(uint32_t *out_bytes);

/** Safely erase `[qspi_offset .. qspi_offset + len)` inside `part_id` in 4 KiB IRQ-guarded sectors with WDT kicks. */
bool p7501_qspi_erase_range_safe(
    p7501_qspi_part_id_t part_id,
    uint32_t qspi_offset,
    uint32_t len
);

/** Safely program `data[0..len)` into `part_id` in 256 B pages with periodic WDT kicks and binary-end check. */
bool p7501_qspi_program_range_safe(
    p7501_qspi_part_id_t part_id,
    uint32_t qspi_offset,
    const uint8_t *data,
    size_t len
);

/** Erase one 4 KiB sector and program one <=256 B header/struct page safely. */
bool p7501_qspi_write_sector_page(
    p7501_qspi_part_id_t part_id,
    uint32_t sector_off,
    const void *struct_ptr,
    size_t struct_len
);

/** Append one wear-leveled telemetry record into `P4_BLACKBOX` (`0x1C0000..0x1EFFFF`). */
bool p7501_blackbox_append_hw(
    uint32_t vdd_mv,
    uint32_t nsw5v_drops,
    uint32_t event_mask,
    const char *model_tag,
    const char *summary
);

/** Load active A/B probe configuration from `P5_PROBE_CFG` (or defaults if unformatted). */
void p7501_probe_cfg_load_active_hw(p7501_probe_cfg_t *out_cfg);

/** Verify and optionally return the stored `P3_RECIPE` header from QSPI flash. */
bool p7501_recipe_load_active_hw(p7501_recipe_hdr_t *out_hdr);

/** CLI command `BLACKBOX [STATUS|DUMP|SNAP [msg]|CLEAR CONFIRM]`. */
void p7501_cmd_blackbox(const char *arg);

/** CLI command `PROBE_CFG [STATUS|CAL_ADC <gain_ppm> <offset_mv>|RESET CONFIRM]`. */
void p7501_cmd_probe_cfg(const char *arg);

/** CLI command `RECIPE [STATUS|LIST|VERIFY]`. */
void p7501_cmd_recipe(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_QSPI_STORAGE_H */
