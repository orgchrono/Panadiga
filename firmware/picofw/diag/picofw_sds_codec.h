/**
 * @file picofw_sds_codec.h
 * @brief Pure ISO C11 passive SD 1-bit/4-bit bus sniffer decoder (`SDSNIFF`).
 *
 * Decodes SD CMD frames (CRC7), CMD55 + ACMD6 bus-width switches, ACMD13 SD Status
 * blocks (64 bytes), and independent per-lane CRC16-CCITT (`x^16 + x^12 + x^5 + 1`)
 * across `DAT0..DAT3` to isolate broken/shorted SD traces causing DMR-EH55 `-91006`.
 *
 * Compiled identically on Pico ARM/RISC-V targets and x86_64 CI hosts (`PICO_HOST_TEST`).
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_SDS_CODEC_H
#define PICOFW_SDS_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "uart/picofw_uart_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_SDS_BIT_CMD   0x01u
#define PICOFW_SDS_BIT_DAT0  0x02u
#define PICOFW_SDS_BIT_DAT1  0x04u
#define PICOFW_SDS_BIT_DAT2  0x08u
#define PICOFW_SDS_BIT_DAT3    0x10u
#define PICOFW_SDS_MAX_BLOCK   512u
#define PICOFW_SDS_TRACE_CAP   32u

typedef struct picofw_sds_trace_entry {
    uint32_t clk;
    uint32_t arg;
    uint8_t cmd;
    bool is_app;
} picofw_sds_trace_entry_t;

typedef enum picofw_sds_data_phase {
    PICOFW_SDS_DATA_IDLE = 0,
    PICOFW_SDS_DATA_PAYLOAD = 1,
    PICOFW_SDS_DATA_CRC16 = 2,
    PICOFW_SDS_DATA_END = 3
} picofw_sds_data_phase_t;

typedef enum picofw_sds_trigger {
    PICOFW_SDS_TRIG_NONE = 0,
    PICOFW_SDS_TRIG_ACMD13 = 1,
    PICOFW_SDS_TRIG_CRCERR = 2,
    PICOFW_SDS_TRIG_91006 = 3
} picofw_sds_trigger_t;

typedef struct picofw_sds_spec {
    picofw_window_boot_spec_t boot;
    uint16_t default_block_bytes;
    picofw_sds_trigger_t trigger;
} picofw_sds_spec_t;

typedef struct picofw_sds_state {
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

    picofw_sds_data_phase_t data_phase;
    uint16_t active_block_bytes;
    uint16_t data_clock_pos;
    uint8_t data_active_width;
    uint8_t start_bit_bad_mask;
    uint8_t data_buf[PICOFW_SDS_MAX_BLOCK];
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

    uint32_t cmd17_count;
    uint32_t cmd18_count;
    uint32_t trace_total;
    uint8_t trace_count;
    picofw_sds_trace_entry_t trace[PICOFW_SDS_TRACE_CAP];

    picofw_sds_trigger_t active_trigger;
    bool trigger_fired;
    uint32_t trigger_clk;
} picofw_sds_state_t;

/** Compute 7-bit SD CRC7 (`poly 0x09`) over `len` bytes. */
uint8_t picofw_sds_crc7_bytes(const uint8_t *data, size_t len);

/** Advance one bit into SD CRC16-CCITT (`poly 0x1021`). */
uint16_t picofw_sds_crc16_step(uint16_t crc, uint8_t bit);

/** Parse `SDSNIFF [ms] [NORMAL|JIG[:ms]|NOPULL] [LEN:64|512] [TRIG:ACMD13|CRCERR|91006]`. */
bool picofw_sds_parse_args(const char *arg, picofw_sds_spec_t *out_spec);

/** Configure active trigger mode on initialized sniffer state. */
void picofw_sds_set_trigger(picofw_sds_state_t *st, picofw_sds_trigger_t trig);

/** Return true if configured trigger condition has fired. */
bool picofw_sds_check_trigger(const picofw_sds_state_t *st);

/** Return canonical uppercase trigger name. */
const char *picofw_sds_trigger_name(picofw_sds_trigger_t trig);

/** Initialize SD bus sniffer state (`default_block_bytes`: 0=auto 512/64 on ACMD13, or 64/512). */
void picofw_sds_init(picofw_sds_state_t *st, uint16_t default_block_bytes);

/** Feed one SD_CLK rising-edge sample (`bits[4:0] = DAT3,DAT2,DAT1,DAT0,CMD`). */
void picofw_sds_feed_sample(picofw_sds_state_t *st, uint8_t sample);

/** Feed a contiguous buffer of SD_CLK rising-edge sample bytes. */
void picofw_sds_feed_samples(picofw_sds_state_t *st, const uint8_t *samples, size_t count);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_SDS_CODEC_H */
