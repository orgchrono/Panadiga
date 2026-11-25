/**
 * @file picofw_qspi_map.c
 * @brief Pure ISO C11 7-Partition RP2040 (2 MiB) / RP2350 (4 MiB) QSPI layout & write-boundary validator.
 *
 * `P0..P5` occupy the universal 2 MiB baseline (`0x000000..0x1FFFFF`).
 * `P6_NOR_MIRROR` activates automatically on 4 MiB RP2350 (`0x200000..0x3FFFFF`)
 * and reports `size=0, writable=false` on 2 MiB RP2040.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_qspi_map.h"

static const picofw_qspi_part_info_t k_qspi_parts[PICOFW_QSPI_PART_COUNT] = {
    { PICOFW_QSPI_PART_FW_RO,      "P0_FW_RO",      PICOFW_QSPI_P0_FW_OFFSET,       PICOFW_QSPI_P0_FW_SIZE,       false },
    { PICOFW_QSPI_PART_STUBS,      "P1_STUBS",      PICOFW_QSPI_P1_STUBS_OFFSET,    PICOFW_QSPI_P1_STUBS_SIZE,    true  },
    { PICOFW_QSPI_PART_VAULT,      "P2_VAULT",      PICOFW_QSPI_P2_VAULT_OFFSET,    PICOFW_QSPI_P2_VAULT_SIZE,    true  },
    { PICOFW_QSPI_PART_RECIPE,     "P3_RECIPE",     PICOFW_QSPI_P3_RECIPE_OFFSET,   PICOFW_QSPI_P3_RECIPE_SIZE,   true  },
    { PICOFW_QSPI_PART_BLACKBOX,   "P4_BLACKBOX",   PICOFW_QSPI_P4_BLACKBOX_OFFSET, PICOFW_QSPI_P4_BLACKBOX_SIZE, true  },
    { PICOFW_QSPI_PART_PROBE_CFG,  "P5_PROBE_CFG",  PICOFW_QSPI_P5_CFG_OFFSET,      PICOFW_QSPI_P5_CFG_SIZE,      true  },
    { PICOFW_QSPI_PART_NOR_MIRROR, "P6_NOR_MIRROR", PICOFW_QSPI_P6_MIRROR_OFFSET,   0UL,                         false },
};

bool picofw_qspi_partition_info(
    picofw_qspi_part_id_t id,
    uint32_t total_qspi_bytes,
    picofw_qspi_part_info_t *out_info
) {
    if ((out_info == NULL) || ((size_t)id >= PICOFW_QSPI_PART_COUNT) ||
        (total_qspi_bytes < PICOFW_QSPI_BASE_2MB_BYTES)) {
        return false;
    }
    *out_info = k_qspi_parts[id];
    if ((id == PICOFW_QSPI_PART_NOR_MIRROR) && (total_qspi_bytes > PICOFW_QSPI_BASE_2MB_BYTES)) {
        out_info->size = total_qspi_bytes - PICOFW_QSPI_BASE_2MB_BYTES;
        out_info->writable = true;
    }
    return true;
}

bool picofw_qspi_is_writable_range(
    picofw_qspi_part_id_t part_id,
    uint32_t qspi_offset,
    uint32_t len,
    uint32_t total_qspi_bytes
) {
    picofw_qspi_part_info_t info;
    if ((len == 0UL) || !picofw_qspi_partition_info(part_id, total_qspi_bytes, &info)) {
        return false;
    }
    if (!info.writable || (info.size == 0UL) || (part_id == PICOFW_QSPI_PART_FW_RO) ||
        (qspi_offset < PICOFW_QSPI_P0_FW_SIZE)) {
        return false;
    }
    if (qspi_offset > (UINT32_MAX - len)) {
        return false;
    }
    const uint32_t end = qspi_offset + len;
    const uint32_t part_end = info.offset + info.size;
    return (qspi_offset >= info.offset) && (end <= part_end) && (end <= total_qspi_bytes);
}

const char *picofw_qspi_partition_name(picofw_qspi_part_id_t id) {
    if ((size_t)id >= PICOFW_QSPI_PART_COUNT) {
        return "UNKNOWN";
    }
    return k_qspi_parts[id].name;
}
