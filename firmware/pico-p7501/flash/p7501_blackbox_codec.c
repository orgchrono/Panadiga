/**
 * @file p7501_blackbox_codec.c
 * @brief Pure ISO C11 implementation of `P4_BLACKBOX` wear-leveled ring selection & CRC32 signing.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_blackbox_codec.h"
#include "common/p7501_hex.h"

#include <stddef.h>
#include <string.h>

static uint32_t blackbox_compute_crc(const p7501_blackbox_rec_t *rec) {
    return p7501_crc32_ieee((const uint8_t *)rec, offsetof(p7501_blackbox_rec_t, crc32));
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

void p7501_blackbox_build_record(
    p7501_blackbox_rec_t *out,
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
    out->magic = P7501_BLACKBOX_MAGIC;
    out->version = P7501_BLACKBOX_VERSION;
    out->sequence = (sequence == 0u) ? 1u : sequence;
    out->uptime_ms = uptime_ms;
    out->vdd_mv = vdd_mv;
    out->nsw5v_drops = nsw5v_drops;
    out->event_mask = event_mask;
    copy_str_trunc(out->model_tag, sizeof(out->model_tag), (model_tag != NULL) ? model_tag : "UNKNOWN");
    copy_str_trunc(out->summary, sizeof(out->summary), (summary != NULL) ? summary : "SNAP");
    out->crc32 = blackbox_compute_crc(out);
}

bool p7501_blackbox_verify_record(const p7501_blackbox_rec_t *rec) {
    if ((rec == NULL) || (rec->magic != P7501_BLACKBOX_MAGIC) ||
        (rec->version != P7501_BLACKBOX_VERSION) || (rec->sequence == 0u)) {
        return false;
    }
    return rec->crc32 == blackbox_compute_crc(rec);
}

bool p7501_blackbox_page_qspi_offset(
    uint32_t total_qspi_bytes,
    uint8_t page_index,
    uint32_t *out_offset
) {
    if ((out_offset == NULL) || (page_index >= P7501_BLACKBOX_PAGES)) {
        return false;
    }
    const uint32_t off = P7501_QSPI_P4_BLACKBOX_OFFSET +
                         ((uint32_t)page_index * P7501_QSPI_SECTOR_BYTES);
    if (!p7501_qspi_is_writable_range(
            P7501_QSPI_PART_BLACKBOX, off, P7501_QSPI_SECTOR_BYTES, total_qspi_bytes)) {
        return false;
    }
    *out_offset = off;
    return true;
}

uint8_t p7501_blackbox_select_next_page(
    const uint32_t page_seqs[P7501_BLACKBOX_PAGES],
    uint32_t *out_next_seq
) {
    uint32_t max_seq = 0u;
    uint8_t max_idx = 0u;
    bool any_valid = false;

    if (page_seqs != NULL) {
        for (uint8_t i = 0u; i < P7501_BLACKBOX_PAGES; ++i) {
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
    return (uint8_t)((max_idx + 1u) % P7501_BLACKBOX_PAGES);
}
