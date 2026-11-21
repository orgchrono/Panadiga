/**
 * @file p7501_blackbox_codec.h
 * @brief Pure ISO C11 wear-leveled 48-page Flight-Recorder codec (`P4_BLACKBOX`, `0x1C0000..0x1EFFFF`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_BLACKBOX_CODEC_H
#define P7501_BLACKBOX_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "flash/p7501_qspi_map.h"

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_BLACKBOX_MAGIC    0x50424258UL /* 'PBBX' */
#define P7501_BLACKBOX_VERSION  1u
#define P7501_BLACKBOX_PAGES    P7501_QSPI_BLACKBOX_PAGES

typedef struct p7501_blackbox_rec {
    uint32_t magic;
    uint32_t version;
    uint32_t sequence;
    uint32_t uptime_ms;
    uint32_t vdd_mv;
    uint32_t nsw5v_drops;
    uint32_t event_mask;
    char model_tag[16];
    char summary[96];
    uint32_t crc32;
} p7501_blackbox_rec_t;

/** Populate and sign a blackbox flight-recorder entry with IEEE CRC32. */
void p7501_blackbox_build_record(
    p7501_blackbox_rec_t *out,
    uint32_t sequence,
    uint32_t uptime_ms,
    uint32_t vdd_mv,
    uint32_t nsw5v_drops,
    uint32_t event_mask,
    const char *model_tag,
    const char *summary
);

/** Verify magic, version, and CRC32 of a blackbox record. */
bool p7501_blackbox_verify_record(const p7501_blackbox_rec_t *rec);

/** Compute validated QSPI offset for blackbox page `page_index` (`0..47`). */
bool p7501_blackbox_page_qspi_offset(
    uint32_t total_qspi_bytes,
    uint8_t page_index,
    uint32_t *out_offset
);

/**
 * Wear-leveled ring selector: given `page_seqs[0..47]` (0 = blank/invalid, >0 = valid sequence),
 * returns the next page index (`0..47`) and next monotonic sequence number (`>= 1`).
 */
uint8_t p7501_blackbox_select_next_page(
    const uint32_t page_seqs[P7501_BLACKBOX_PAGES],
    uint32_t *out_next_seq
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_BLACKBOX_CODEC_H */
