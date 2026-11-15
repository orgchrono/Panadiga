/**
 * @file p7501_sds_codec.h
 * @brief Pure ISO C11 passive SD 1-bit/4-bit bus sniffer decoder (`SDSNIFF`).
 *
 * Decodes SD CMD frames (CRC7), CMD55 + ACMD6 bus-width switches, ACMD13 SD Status
 * blocks (64 bytes), and independent per-lane CRC16-CCITT (`x^16 + x^12 + x^5 + 1`)
 * across `DAT0..DAT3` to isolate broken/shorted SD traces causing DMR-EH55 `-91006`.
 *
 * Compiled identically on Pico ARM/RISC-V targets and x86_64 CI hosts (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_SDS_CODEC_H
#define P7501_SDS_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "uart/p7501_uart_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_SDS_BIT_CMD   0x01u
#define P7501_SDS_BIT_DAT0  0x02u
#define P7501_SDS_BIT_DAT1  0x04u
#define P7501_SDS_BIT_DAT2  0x08u
#define P7501_SDS_BIT_DAT3  0x10u
#define P7501_SDS_MAX_BLOCK 512u

typedef enum p7501_sds_data_phase {
    P7501_SDS_DATA_IDLE = 0,
    P7501_SDS_DATA_PAYLOAD = 1,
    P7501_SDS_DATA_CRC16 = 2,
    P7501_SDS_DATA_END = 3
} p7501_sds_data_phase_t;

typedef struct p7501_sds_spec {
    p7501_window_boot_spec_t boot;
    uint16_t default_block_bytes;
} p7501_sds_spec_t;

typedef struct p7501_sds_state {
    bool has_prev;
    uint8_t prev_sample;
    uint32_t total_clocks;
    uint32_t transitions[5]; /* 0=CMD, 1=DAT0, 2=DAT1, 3=DAT2, 4=DAT3 */

    uint8_t cmd_bits;
    uint8_t cmd_target_bits;
    uint8_t cmd_buf[17];
    bool in_app_cmd;
    bool expect_r2_resp;
    uint8_t bus_width;
    bool expecting_acmd13;
    uint16_t next_block_bytes;
    uint16_t default_block_bytes;

    bool saw_cmd55;
    bool saw_acmd6_4bit;
    bool saw_acmd13;
    uint8_t last_cmd_idx;
    uint32_t last_cmd_arg;
    uint32_t host_cmds_ok;
    uint32_t card_resps_ok;
    uint32_t cmd_crc_err;

    p7501_sds_data_phase_t data_phase;
    uint16_t active_block_bytes;
    uint16_t data_clock_pos;
    uint8_t data_active_width;
    uint8_t start_bit_bad_mask;
    uint8_t data_buf[P7501_SDS_MAX_BLOCK];
    uint16_t lane_calc_crc[4];
    uint16_t lane_rx_crc[4];

    uint32_t data_blocks_ok;
    uint32_t data_crc_err;
    uint32_t lane_crc_err[4];
    uint8_t last_fail_lane_mask;

    bool acmd13_captured;
    bool acmd13_crc_ok;
    uint8_t acmd13_dat_bus_width;
    uint16_t acmd13_sd_card_type;
    uint8_t acmd13_raw[64];
} p7501_sds_state_t;

/** Compute 7-bit SD CRC7 (`poly 0x09`) over `len` bytes. */
uint8_t p7501_sds_crc7_bytes(const uint8_t *data, size_t len);

/** Advance one bit into SD CRC16-CCITT (`poly 0x1021`). */
uint16_t p7501_sds_crc16_step(uint16_t crc, uint8_t bit);

/** Parse `SDSNIFF [ms] [NORMAL|JIG[:ms]|NOPULL] [LEN:64|512]`. */
bool p7501_sds_parse_args(const char *arg, p7501_sds_spec_t *out_spec);

/** Initialize SD bus sniffer state (`default_block_bytes`: 0=auto 512/64 on ACMD13, or 64/512). */
void p7501_sds_init(p7501_sds_state_t *st, uint16_t default_block_bytes);

/** Feed one SD_CLK rising-edge sample (`bits[4:0] = DAT3,DAT2,DAT1,DAT0,CMD`). */
void p7501_sds_feed_sample(p7501_sds_state_t *st, uint8_t sample);

/** Feed a contiguous buffer of SD_CLK rising-edge sample bytes. */
void p7501_sds_feed_samples(p7501_sds_state_t *st, const uint8_t *samples, size_t count);

#ifdef __cplusplus
}
#endif

#endif /* P7501_SDS_CODEC_H */
