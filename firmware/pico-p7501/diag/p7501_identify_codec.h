/**
 * @file p7501_identify_codec.h
 * @brief Pure ISO C11 4-stage `IDENTIFY`/`AUTOPROBE` target classifier & `@P7501_IDENT` serializer.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 * Auto-locks `DMR-ES35V` `MAIN_XIP` (`0x130000..0x40FFFF`) Read-Only when `"ver0.18"` is matched.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_IDENTIFY_CODEC_H
#define P7501_IDENTIFY_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "flash/p7501_flash.h"

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_CAP_NVRAM_WRITE       (1u << 0)
#define P7501_CAP_UPPER_NOR_UNLOCK  (1u << 1)
#define P7501_CAP_VAULT_BACKUP      (1u << 2)
#define P7501_CAP_XIP_MAIN_RO_LOCK  (1u << 3)
#define P7501_CAP_FULL_NOR_RO_LOCK  (1u << 4)

typedef struct p7501_ident_report {
    uint32_t vdd_mv;
    uint32_t sw_pu_mv;
    bool nsw5v_high;
    const char *elec_state;
    char loader_banner[32];
    uint16_t jedec_mfg;
    uint16_t jedec_dev0;
    const char *flash_part;
    uint32_t ipl_crc32;
    const char *hw_board;
    const char *tv_system;
    const char *target_state;
    const char *model_name;
    const char *tier;
    uint32_t caps_mask;
    const char *ssot_hash;
    p7501_ssot_target_profile_id_t matched_profile;
    bool matched_from_banner;
    bool xip_main_ro_locked;
} p7501_ident_report_t;

/**
 * Evaluate 4-stage target identification from electrical readings, optional UART banner bytes,
 * JEDEC IDs, and IPL header CRC32. If `apply_to_session` is true, updates `p7501_flash_set_target_profile_by_id`.
 */
void p7501_identify_evaluate(
    uint32_t vdd_mv,
    uint32_t sw_pu_mv,
    bool nsw5v_high,
    bool rxd_high,
    bool txd_high,
    const uint8_t *uart_rx,
    size_t uart_rx_len,
    uint16_t jedec_mfg,
    uint16_t jedec_dev0,
    uint32_t ipl_crc32,
    bool apply_to_session,
    p7501_ident_report_t *out_rep
);

/**
 * Format canonical `@P7501_IDENT ...` single-line telemetry token.
 * Returns number of bytes written (excluding NUL).
 */
size_t p7501_identify_format_line(
    const p7501_ident_report_t *rep,
    char *out_buf,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_IDENTIFY_CODEC_H */
