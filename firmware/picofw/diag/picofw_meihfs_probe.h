/**
 * @file picofw_meihfs_probe.h
 * @brief Lightweight MEIHFS Sector 0 ('SH' MBR) micro-triage for Pico P7501.
 *
 * Host-testable pure C11 (`~80 LOC`); deep partition recovery belongs in Rust/SATA.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_MEIHFS_PROBE_H
#define PICOFW_MEIHFS_PROBE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_MEIHFS_JOURNAL_LBA 16384u
#define PICOFW_MEIHFS_MIRROR_LBA  57344u

typedef struct picofw_meihfs_mbr_info {
    bool valid_sh_magic;
    bool byte_swapped_hs;
    bool all_ff;
    bool all_zero;
    uint32_t crc32;
    uint32_t journal_lba;
    uint32_t mirror_lba;
    uint8_t ver_hi;
    uint8_t ver_lo;
} picofw_meihfs_mbr_info_t;

bool picofw_meihfs_inspect_sector0(const uint8_t sec[512], picofw_meihfs_mbr_info_t *out);
size_t picofw_meihfs_format_report(
    uint32_t src_va,
    const picofw_meihfs_mbr_info_t *info,
    char *out,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_MEIHFS_PROBE_H */
