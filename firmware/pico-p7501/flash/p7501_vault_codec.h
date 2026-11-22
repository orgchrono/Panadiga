/**
 * @file p7501_vault_codec.h
 * @brief Pure C11 Pico QSPI Flash Forensic Vault manifest & A/B slot codec (`P2_VAULT`, Feature #8).
 *
 * Maps to Partition `P2_VAULT` (`0x100000..0x17FFFF`, 512 KiB = Bank A `0x100000` + Bank B `0x140000`),
 * with each 256 KiB bank containing 1 manifest sector block + 3 x 64 KiB forensic payload slots.
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_VAULT_CODEC_H
#define P7501_VAULT_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "flash/p7501_qspi_map.h"

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_VAULT_MAGIC           0x50564C54UL /* 'PVLT' */
#define P7501_VAULT_VERSION         1u
#define P7501_VAULT_TOTAL_BYTES     P7501_QSPI_VAULT_BANK_BYTES /* 256 KiB per bank */
#define P7501_VAULT_SLOT_BYTES      0x00010000UL /* 64 KiB per slot */
#define P7501_VAULT_PAYLOAD_SLOTS   3u           /* Slots 1..3; Slot 0 = manifest */
#define P7501_VAULT_BANK_COUNT      2u           /* Bank 0 (A) @ 0x100000, Bank 1 (B) @ 0x140000 */
#define P7501_VAULT_MIN_FLASH_BYTES P7501_QSPI_BASE_2MB_BYTES

typedef struct p7501_vault_slot_meta {
    uint32_t valid;
    uint32_t target_addr;
    uint32_t byte_len;
    uint32_t data_crc32;
    uint32_t sequence;
    char model_tag[16];
} p7501_vault_slot_meta_t;

typedef struct p7501_vault_manifest {
    uint32_t magic;
    uint32_t version;
    uint32_t write_count;
    p7501_vault_slot_meta_t slots[P7501_VAULT_PAYLOAD_SLOTS];
    uint32_t header_crc32;
} p7501_vault_manifest_t;

/** Initialize clean empty vault manifest with computed header CRC32. */
void p7501_vault_manifest_init(p7501_vault_manifest_t *m);

/** Compute P2_VAULT bank base offset (`bank_ab`: 0=A `0x100000`, 1=B `0x140000`), or `0` if invalid. */
uint32_t p7501_vault_bank_offset(uint32_t total_qspi_bytes, uint8_t bank_ab);

/** Compute primary P2_VAULT base offset (Bank A `0x100000`), or `0` if QSPI < 2 MiB. */
uint32_t p7501_vault_base_offset(uint32_t total_qspi_bytes);

/** Compute absolute QSPI byte offset for payload `slot_index` (`1..3`) in `bank_ab` (`0..1`). */
bool p7501_vault_slot_qspi_offset_ab(
    uint32_t total_qspi_bytes,
    uint8_t bank_ab,
    uint8_t slot_index,
    uint32_t *out_offset
);

/** Compute absolute QSPI byte offset for payload `slot_index` (`1..3`) in primary Bank A. */
bool p7501_vault_slot_qspi_offset(
    uint32_t total_qspi_bytes,
    uint8_t slot_index,
    uint32_t *out_offset
);

/** Update metadata for payload `slot_index` (`1..3`) with precomputed `data_crc32` and resign header. */
bool p7501_vault_manifest_record_slot_crc(
    p7501_vault_manifest_t *m,
    uint8_t slot_index,
    uint32_t target_addr,
    size_t payload_len,
    uint32_t data_crc32,
    const char *model_tag
);

/** Update metadata for payload `slot_index` (`1..3`) and recompute header CRC32. */
bool p7501_vault_manifest_record_slot(
    p7501_vault_manifest_t *m,
    uint8_t slot_index,
    uint32_t target_addr,
    const uint8_t *payload,
    size_t payload_len,
    const char *model_tag
);

/** Verify manifest magic, version, and IEEE CRC32 integrity. */
bool p7501_vault_manifest_verify(const p7501_vault_manifest_t *m);

/** Select newest valid bank between Bank A (0) and Bank B (1); returns 0, 1, or -1 if both invalid. */
int p7501_vault_select_ab(
    const p7501_vault_manifest_t *bank_a,
    const p7501_vault_manifest_t *bank_b,
    p7501_vault_manifest_t *out_active
);

/** Clear `slot_index` (`1..3`), increment `write_count`, and resign manifest CRC32 for atomic A/B erase. */
bool p7501_vault_manifest_clear_slot(p7501_vault_manifest_t *m, uint8_t slot_index);

/** Erase target `P2_VAULT` slot `1..3` in chunked 4 KiB sectors and return QSPI byte offset. */
bool p7501_vault_write_slot_begin_hw(
    uint8_t slot_index,
    size_t total_len,
    uint32_t *out_slot_off
);

/** Program a page-aligned chunk (`rel_off` multiple of 256 B) into an erased vault slot. */
bool p7501_vault_write_slot_chunk_hw(
    uint32_t slot_off,
    uint32_t rel_off,
    const uint8_t *data,
    size_t len
);

/** Commit manifest metadata and precomputed payload CRC32 after all chunks are written. */
bool p7501_vault_write_slot_commit_hw(
    uint8_t slot_index,
    uint32_t target_addr,
    size_t total_len,
    uint32_t data_crc32,
    const char *model_tag
);

/** Write verified target NVRAM snapshot into `P2_VAULT` slot `1..3` in hardware QSPI. */
bool p7501_vault_write_slot_hw(
    uint8_t slot_index,
    uint32_t target_addr,
    const uint8_t *payload,
    size_t payload_len,
    const char *model_tag
);

/** Callback computing IEEE CRC32 over `len` bytes at `buf` (in-memory or XIP-paged). */
typedef uint32_t (*p7501_vault_crc32_fn_t)(const uint8_t *buf, size_t len);

/** Verify `slot_index` (`1..3`) payload CRC32 + non-`0xFF` byte count using caller-supplied CRC32 fn. */
bool p7501_vault_verify_slot_with_crc(
    const p7501_vault_manifest_t *m,
    uint8_t slot_index,
    const uint8_t *slot_payload,
    size_t payload_buf_len,
    p7501_vault_crc32_fn_t crc_fn,
    uint32_t *out_crc32,
    uint32_t *out_non_ff_bytes,
    bool *out_crc_mismatch
);

/** Pure C11 slot payload CRC32 + non-`0xFF` byte verification on an in-memory buffer. */
bool p7501_vault_verify_slot_buffer(
    const p7501_vault_manifest_t *m,
    uint8_t slot_index,
    const uint8_t *slot_payload,
    size_t payload_buf_len,
    uint32_t *out_crc32,
    uint32_t *out_non_ff_bytes,
    bool *out_crc_mismatch
);

/** Verify an active `P2_VAULT` slot in hardware QSPI via XIP CRC32 + non-`0xFF` byte scan. */
bool p7501_vault_verify_slot(
    uint8_t slot_idx,
    uint32_t *out_crc32,
    uint32_t *out_non_ff_bytes
);

/** CLI command `VAULT [STATUS|LIST|VERIFY <1..3>|DUMP <1..3>|ERASE <1..3> CONFIRM]`. */
void p7501_cmd_vault(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_VAULT_CODEC_H */
