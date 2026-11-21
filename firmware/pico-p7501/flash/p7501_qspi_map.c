/**
 * @file p7501_qspi_map.c
 * @brief Pure ISO C11 7-Partition RP2040 (2 MiB) / RP2350 (4 MiB) QSPI layout & write-boundary validator.
 *
 * `P0..P5` occupy the universal 2 MiB baseline (`0x000000..0x1FFFFF`).
 * `P6_NOR_MIRROR` activates automatically on 4 MiB RP2350 (`0x200000..0x3FFFFF`)
 * and reports `size=0, writable=false` on 2 MiB RP2040.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_qspi_map.h"

static const p7501_qspi_part_info_t k_qspi_parts[P7501_QSPI_PART_COUNT] = {
    { P7501_QSPI_PART_FW_RO,      "P0_FW_RO",      P7501_QSPI_P0_FW_OFFSET,       P7501_QSPI_P0_FW_SIZE,       false },
    { P7501_QSPI_PART_STUBS,      "P1_STUBS",      P7501_QSPI_P1_STUBS_OFFSET,    P7501_QSPI_P1_STUBS_SIZE,    true  },
    { P7501_QSPI_PART_VAULT,      "P2_VAULT",      P7501_QSPI_P2_VAULT_OFFSET,    P7501_QSPI_P2_VAULT_SIZE,    true  },
    { P7501_QSPI_PART_RECIPE,     "P3_RECIPE",     P7501_QSPI_P3_RECIPE_OFFSET,   P7501_QSPI_P3_RECIPE_SIZE,   true  },
    { P7501_QSPI_PART_BLACKBOX,   "P4_BLACKBOX",   P7501_QSPI_P4_BLACKBOX_OFFSET, P7501_QSPI_P4_BLACKBOX_SIZE, true  },
    { P7501_QSPI_PART_PROBE_CFG,  "P5_PROBE_CFG",  P7501_QSPI_P5_CFG_OFFSET,      P7501_QSPI_P5_CFG_SIZE,      true  },
    { P7501_QSPI_PART_NOR_MIRROR, "P6_NOR_MIRROR", P7501_QSPI_P6_MIRROR_OFFSET,   0UL,                         false },
};

bool p7501_qspi_partition_info(
    p7501_qspi_part_id_t id,
    uint32_t total_qspi_bytes,
    p7501_qspi_part_info_t *out_info
) {
    if ((out_info == NULL) || ((size_t)id >= P7501_QSPI_PART_COUNT) ||
        (total_qspi_bytes < P7501_QSPI_BASE_2MB_BYTES)) {
        return false;
    }
    *out_info = k_qspi_parts[id];
    if ((id == P7501_QSPI_PART_NOR_MIRROR) && (total_qspi_bytes > P7501_QSPI_BASE_2MB_BYTES)) {
        out_info->size = total_qspi_bytes - P7501_QSPI_BASE_2MB_BYTES;
        out_info->writable = true;
    }
    return true;
}

bool p7501_qspi_is_writable_range(
    p7501_qspi_part_id_t part_id,
    uint32_t qspi_offset,
    uint32_t len,
    uint32_t total_qspi_bytes
) {
    p7501_qspi_part_info_t info;
    if ((len == 0UL) || !p7501_qspi_partition_info(part_id, total_qspi_bytes, &info)) {
        return false;
    }
    if (!info.writable || (info.size == 0UL) || (part_id == P7501_QSPI_PART_FW_RO) ||
        (qspi_offset < P7501_QSPI_P0_FW_SIZE)) {
        return false;
    }
    if (qspi_offset > (UINT32_MAX - len)) {
        return false;
    }
    const uint32_t end = qspi_offset + len;
    const uint32_t part_end = info.offset + info.size;
    return (qspi_offset >= info.offset) && (end <= part_end) && (end <= total_qspi_bytes);
}

const char *p7501_qspi_partition_name(p7501_qspi_part_id_t id) {
    if ((size_t)id >= P7501_QSPI_PART_COUNT) {
        return "UNKNOWN";
    }
    return k_qspi_parts[id].name;
}
