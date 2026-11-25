/**
 * @file picofw_hdd_unlock.c
 * @brief Pure host-testable ATA IDENTIFY decoder, MEIHFS full-disk sizer, RTSD unlocker & TBUS 5-stage builder.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_hdd_unlock.h"

#include "common/picofw_hex.h"
#include "diag/picofw_microsurgery.h"

#include <string.h>

static uint16_t read_ident_w(const uint8_t *raw, size_t idx, bool pio1024) {
    const size_t stride = pio1024 ? 4u : 2u;
    return (uint16_t)raw[idx * stride] | (uint16_t)((uint16_t)raw[idx * stride + 1u] << 8u);
}

static void decode_ata_ascii(
    const uint8_t *raw,
    bool pio1024,
    size_t start_w,
    size_t n_words,
    char *dst,
    size_t cap
) {
    size_t pos = 0u;
    for (size_t i = 0u; (i < n_words) && ((pos + 2u) < cap); ++i) {
        const uint16_t w = read_ident_w(raw, start_w + i, pio1024);
        const char hi = (char)((w >> 8u) & 0xFFu);
        const char lo = (char)(w & 0xFFu);
        dst[pos++] = ((hi >= 0x20) && (hi <= 0x7E)) ? hi : ' ';
        dst[pos++] = ((lo >= 0x20) && (lo <= 0x7E)) ? lo : ' ';
    }
    while ((pos > 0u) && ((dst[pos - 1u] == ' ') || (dst[pos - 1u] == '\0'))) {
        --pos;
    }
    dst[pos] = '\0';
}

bool picofw_ata_parse_identify(
    const uint8_t *raw,
    size_t len,
    bool is_davis_pio_1024,
    picofw_ata_ident_t *out
) {
    const size_t expected = is_davis_pio_1024 ? 1024u : 512u;
    if ((raw == NULL) || (len != expected) || (out == NULL)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    const uint16_t w0 = read_ident_w(raw, 0u, is_davis_pio_1024);
    if ((w0 == 0x0000u) || (w0 == 0xFFFFu)) {
        return false;
    }
    bool all_same = true;
    for (size_t i = 1u; i < 256u; ++i) {
        if (read_ident_w(raw, i, is_davis_pio_1024) != w0) {
            all_same = false;
            break;
        }
    }
    if (all_same) {
        return false;
    }

    out->word0 = w0;
    out->is_atapi = (w0 & 0xC000u) == 0x8000u;
    decode_ata_ascii(raw, is_davis_pio_1024, 10u, 10u, out->serial, sizeof(out->serial));
    decode_ata_ascii(raw, is_davis_pio_1024, 23u, 4u, out->fw_rev, sizeof(out->fw_rev));
    decode_ata_ascii(raw, is_davis_pio_1024, 27u, 20u, out->model, sizeof(out->model));

    out->lba28_sectors = (uint32_t)read_ident_w(raw, 60u, is_davis_pio_1024) |
                         ((uint32_t)read_ident_w(raw, 61u, is_davis_pio_1024) << 16u);
    out->lba48_supported = (read_ident_w(raw, 83u, is_davis_pio_1024) & (1u << 10u)) != 0u;
    const uint64_t lba48 = (uint64_t)read_ident_w(raw, 100u, is_davis_pio_1024) |
                           ((uint64_t)read_ident_w(raw, 101u, is_davis_pio_1024) << 16u) |
                           ((uint64_t)read_ident_w(raw, 102u, is_davis_pio_1024) << 32u) |
                           ((uint64_t)read_ident_w(raw, 103u, is_davis_pio_1024) << 48u);
    out->effective_sectors = (out->lba48_supported && (lba48 > 0ULL)) ? lba48 : (uint64_t)out->lba28_sectors;
    const uint64_t aligned = out->effective_sectors & ~3ULL;
    out->mirror_fw_lba = (aligned > PICOFW_SSOT_MEIHFS_MIRROR_DELTA_LBA)
        ? (aligned - PICOFW_SSOT_MEIHFS_MIRROR_DELTA_LBA)
        : 0ULL;
    out->capacity_mib = (uint32_t)(out->effective_sectors >> 11u);
    const uint64_t blk2kb = out->effective_sectors >> 2u;
    out->target_blocks_2kb = (blk2kb > PICOFW_SSOT_HDD_DYNAMIC_MAX_2KB)
        ? PICOFW_SSOT_HDD_DYNAMIC_MAX_2KB
        : (uint32_t)blk2kb;
    return true;
}

bool picofw_hdd_compute_full_plan(
    uint64_t total_lba,
    uint32_t fw_bytes,
    picofw_hdd_full_plan_t *out
) {
    if ((out == NULL) || (fw_bytes == 0u)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    const uint64_t aligned_total = total_lba & ~3ULL;
    const uint64_t min_limit = (uint64_t)PICOFW_SSOT_MEIHFS_PRIMARY_FW_LBA +
                               (uint64_t)PICOFW_SSOT_MEIHFS_MIRROR_DELTA_LBA;
    if (aligned_total <= min_limit) {
        return false;
    }
    const uint64_t fw_sectors = ((uint64_t)fw_bytes + 511ULL) >> 9u;
    const uint64_t mirror_fw = aligned_total - (uint64_t)PICOFW_SSOT_MEIHFS_MIRROR_DELTA_LBA;
    const uint64_t raw_end = (uint64_t)PICOFW_SSOT_MEIHFS_PRIMARY_FW_LBA + fw_sectors;
    const uint64_t data_start = (raw_end + 3ULL) & ~3ULL;
    if ((fw_sectors == 0ULL) || (data_start >= mirror_fw)) {
        return false;
    }
    const uint64_t usable_2kb = (mirror_fw - data_start) >> 2u;
    const uint64_t total_2kb = aligned_total >> 2u;
    out->total_lba = aligned_total;
    out->primary_fw_lba = PICOFW_SSOT_MEIHFS_PRIMARY_FW_LBA;
    out->mirror_fw_lba = mirror_fw;
    out->fw_sectors = (uint32_t)fw_sectors;
    out->data_start_lba = data_start;
    out->usable_blocks_2kb = (usable_2kb > PICOFW_SSOT_HDD_DYNAMIC_MAX_2KB)
        ? PICOFW_SSOT_HDD_DYNAMIC_MAX_2KB : (uint32_t)usable_2kb;
    out->target_rtsd_blocks_2kb = (total_2kb > PICOFW_SSOT_HDD_DYNAMIC_MAX_2KB)
        ? PICOFW_SSOT_HDD_DYNAMIC_MAX_2KB : (uint32_t)total_2kb;
    out->valid = out->usable_blocks_2kb > 0u;
    return out->valid;
}

bool picofw_hdd_find_clamp_site(const uint8_t *buf, size_t len, size_t *out_off) {
    if ((buf == NULL) || (len < PICOFW_SSOT_HDD_CLAMP_WIN_LEN) || (out_off == NULL)) {
        return false;
    }
    static const uint8_t k_op_fc81[2] = { 0xFCu, 0x81u };
    static const uint8_t k_op_fccc[2] = { 0xFCu, 0xCCu };
    static const uint8_t k_op_fca5[2] = { 0xFCu, 0xA5u };
    static const uint8_t k_op_fca4[2] = { 0xFCu, 0xA4u };
    static const uint8_t k_tail_5c[5] = { 0x0Cu, 0x29u, 0xFFu, 0xFCu, 0x85u };
    static const uint8_t k_tail_65[6] = { 0xCAu, 0x0Au, 0x28u, 0xFFu, 0xFCu, 0x81u };

    for (size_t pos = 0u; (pos + PICOFW_SSOT_HDD_CLAMP_WIN_LEN) <= len; ++pos) {
        const uint8_t *const w = &buf[pos];
        const uint8_t br = w[PICOFW_SSOT_HDD_CLAMP_OFF_BRANCH];
        if ((memcmp(w, k_picofw_ssot_hdd_clamp_prefix, 14u) == 0) &&
            (memcmp(&w[0x12], k_op_fc81, 2u) == 0) &&
            (memcmp(&w[0x27], k_op_fccc, 2u) == 0) &&
            (memcmp(&w[0x2D], k_op_fc81, 2u) == 0) &&
            (memcmp(&w[0x42], k_op_fccc, 2u) == 0) &&
            (memcmp(&w[0x48], k_op_fc81, 2u) == 0) &&
            (memcmp(&w[0x4E], k_op_fca5, 2u) == 0) &&
            (memcmp(&w[0x54], k_op_fca4, 2u) == 0) &&
            (w[0x5A] == 0xA1u) &&
            ((br == PICOFW_SSOT_HDD_OPCODE_BLS) || (br == PICOFW_SSOT_HDD_OPCODE_BRA)) &&
            (memcmp(&w[0x5C], k_tail_5c, 5u) == 0) &&
            (memcmp(&w[0x65], k_tail_65, 6u) == 0)) {
            *out_off = pos;
            return true;
        }
    }
    return false;
}

bool picofw_hdd_plan_rtsd_unlock(
    uint32_t site_va,
    const uint8_t win[PICOFW_SSOT_HDD_CLAMP_WIN_LEN],
    uint32_t target_blocks_2kb,
    picofw_hdd_unlock_plan_t *out
) {
    size_t off = 0u;
    if ((win == NULL) || (out == NULL) || (target_blocks_2kb == 0u) ||
        !picofw_surg_is_live_ram_span(site_va, PICOFW_SSOT_HDD_CLAMP_WIN_LEN) ||
        !picofw_hdd_find_clamp_site(win, PICOFW_SSOT_HDD_CLAMP_WIN_LEN, &off) || (off != 0u)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->site_va = site_va;
    out->target_blocks_2kb = target_blocks_2kb;
    out->prev_200gb_2kb = picofw_read_u32_le(&win[PICOFW_SSOT_HDD_CLAMP_OFF_200GB]);
    out->prev_branch_op = win[PICOFW_SSOT_HDD_CLAMP_OFF_BRANCH];

    static const uint8_t k_slot_offs[3] = {
        PICOFW_SSOT_HDD_CLAMP_OFF_400GB,
        PICOFW_SSOT_HDD_CLAMP_OFF_200GB,
        PICOFW_SSOT_HDD_CLAMP_OFF_FALLBACK,
    };
    for (size_t i = 0u; i < 3u; ++i) {
        out->ops[i].va = site_va + k_slot_offs[i];
        out->ops[i].len = 4u;
        picofw_write_u32_le(out->ops[i].data, target_blocks_2kb);
    }
    out->ops[3].va = site_va + PICOFW_SSOT_HDD_CLAMP_OFF_BRANCH;
    out->ops[3].len = 1u;
    out->ops[3].data[0] = PICOFW_SSOT_HDD_OPCODE_BRA;
    out->valid = true;
    return true;
}

static uint8_t tbus_csum15(const uint8_t body15[15]) {
    uint32_t s = 0u;
    for (size_t i = 0u; i < 15u; ++i) {
        s += body15[i];
    }
    return (uint8_t)((s - 0x5Bu) & 0xFFu);
}

bool picofw_hdd_build_stage_tbus(
    uint8_t stage_idx,
    uint8_t out_vfd16[16],
    uint8_t out_wdt16[16],
    const char **out_label
) {
    static const char *const k_vfd[5] = { "M FIRM", "MAIN", "UPDATE", "UPDOK", "FINISH" };
    if ((stage_idx >= 5u) || (out_vfd16 == NULL) || (out_wdt16 == NULL)) {
        return false;
    }
    memset(out_vfd16, 0, 16u);
    out_vfd16[0] = 0xA3u;
    memset(&out_vfd16[3], ' ', 7u);
    const char *const lbl = k_vfd[stage_idx];
    for (size_t i = 0u; (i < 7u) && (lbl[i] != '\0'); ++i) {
        out_vfd16[3u + i] = (uint8_t)lbl[i];
    }
    out_vfd16[15] = tbus_csum15(out_vfd16);

    memset(out_wdt16, 0, 16u);
    out_wdt16[0] = 0x8Au;
    out_wdt16[1] = 0x02u;
    out_wdt16[3] = 0xFFu;
    out_wdt16[15] = tbus_csum15(out_wdt16);
    if (out_label != NULL) {
        *out_label = lbl;
    }
    return true;
}
