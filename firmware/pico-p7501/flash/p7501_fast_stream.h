/**
 * @file p7501_fast_stream.h
 * @brief Hardware PIO1 runtime commands (`NOR_MAP`, `FAST_READ`, `FAST_WRITE`)
 *        for the embedded `0x9043A000` MN103S binary streamer & 142-sector scanner.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_FAST_STREAM_H
#define P7501_FAST_STREAM_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** CLI `NOR_MAP`: scan all 142 `S29JL064H` sectors (`SA0..SA141`) emitting live `@P7501_PROG`. */
void p7501_cmd_nor_map(const char *arg);

/** CLI `FAST_READ <hex_va> <hex_len>`: page-bitmap binary NOR reader via `0x9043A000` stub. */
void p7501_cmd_fast_read(const char *arg);

/** CLI `FAST_WRITE <hex_va> <hex_payload> [CONFIRM_UPPER_NOR]`: guarded `1->0` JEDEC writer. */
void p7501_cmd_fast_write(const char *arg);

/**
 * @brief Shared physical guarded `1->0` NOR writer (enforces E05, pre-image check, and unconditional `U LOCK`).
 */
bool p7501_fast_write_bytes_hw(
    const char *tag,
    uint32_t addr,
    const uint8_t *payload,
    size_t pay_len,
    bool has_confirm
);

/** CLI `FAST_ERASE <hex_sector_va> CONFIRM_UPPER_NOR`: guarded 64 KiB BGO sector erase + verify. */
void p7501_cmd_fast_erase(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_FAST_STREAM_H */
