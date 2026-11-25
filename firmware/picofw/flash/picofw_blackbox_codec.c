/**
 * @file picofw_blackbox_codec.c
 * @brief Pure ISO C11 implementation of `P4_BLACKBOX` wear-leveled ring selection & CRC32 signing.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_blackbox_codec.h"
#include "common/picofw_hex.h"

#include <stddef.h>
#include <string.h>

static uint32_t blackbox_compute_crc(const picofw_blackbox_rec_t *rec) {
    return picofw_crc32_ieee((const uint8_t *)rec, offsetof(picofw_blackbox_rec_t, crc32));
}

static void copy_str_trunc(char *dst, size_t cap, const char *src) {
    if ((dst == NULL) || (cap == 0u)) {
        return;
    }
    dst[0] = '\0';
    if (src == NULL) {
        return;
    }
    size_t i = 0u;
    while ((i + 1u) < cap && (src[i] != '\0')) {
        dst[i] = src[i];
        ++i;
    }
    dst[i] = '\0';
}

void picofw_blackbox_build_record(
    picofw_blackbox_rec_t *out,
    uint32_t sequence,
    uint32_t uptime_ms,
    uint32_t vdd_mv,
    uint32_t nsw5v_drops,
    uint32_t event_mask,
    const char *model_tag,
    const char *summary
) {
    if (out == NULL) {
        return;
    }
    memset(out, 0, sizeof(*out));
    out->magic = PICOFW_BLACKBOX_MAGIC;
    out->version = PICOFW_BLACKBOX_VERSION;
    out->sequence = (sequence == 0u) ? 1u : sequence;
    out->uptime_ms = uptime_ms;
    out->vdd_mv = vdd_mv;
    out->nsw5v_drops = nsw5v_drops;
    out->event_mask = event_mask;
    copy_str_trunc(out->model_tag, sizeof(out->model_tag), (model_tag != NULL) ? model_tag : "UNKNOWN");
    copy_str_trunc(out->summary, sizeof(out->summary), (summary != NULL) ? summary : "SNAP");
    out->crc32 = blackbox_compute_crc(out);
}

bool picofw_blackbox_verify_record(const picofw_blackbox_rec_t *rec) {
    if ((rec == NULL) || (rec->magic != PICOFW_BLACKBOX_MAGIC) ||
        (rec->version != PICOFW_BLACKBOX_VERSION) || (rec->sequence == 0u)) {
        return false;
    }
    return rec->crc32 == blackbox_compute_crc(rec);
}

bool picofw_blackbox_page_qspi_offset(
    uint32_t total_qspi_bytes,
    uint8_t page_index,
    uint32_t *out_offset
) {
    if ((out_offset == NULL) || (page_index >= PICOFW_BLACKBOX_PAGES)) {
        return false;
    }
    const uint32_t off = PICOFW_QSPI_P4_BLACKBOX_OFFSET +
                         ((uint32_t)page_index * PICOFW_QSPI_SECTOR_BYTES);
    if (!picofw_qspi_is_writable_range(
            PICOFW_QSPI_PART_BLACKBOX, off, PICOFW_QSPI_SECTOR_BYTES, total_qspi_bytes)) {
        return false;
    }
    *out_offset = off;
    return true;
}

uint8_t picofw_blackbox_select_next_page(
    const uint32_t page_seqs[PICOFW_BLACKBOX_PAGES],
    uint32_t *out_next_seq
) {
    uint32_t max_seq = 0u;
    uint8_t max_idx = 0u;
    bool any_valid = false;

    if (page_seqs != NULL) {
        for (uint8_t i = 0u; i < PICOFW_BLACKBOX_PAGES; ++i) {
            if (page_seqs[i] > max_seq) {
                max_seq = page_seqs[i];
                max_idx = i;
                any_valid = true;
            }
        }
    }
    if (out_next_seq != NULL) {
        *out_next_seq = max_seq + 1u;
    }
    if (!any_valid) {
        return 0u;
    }
    return (uint8_t)((max_idx + 1u) % PICOFW_BLACKBOX_PAGES);
}
