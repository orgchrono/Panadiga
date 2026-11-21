/**
 * @file p7501_probe_cfg_codec.h
 * @brief Pure ISO C11 A/B Probe Configuration (`P5_PROBE_CFG`) & Recipe Header (`P3_RECIPE`) codec.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PROBE_CFG_CODEC_H
#define P7501_PROBE_CFG_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "flash/p7501_qspi_map.h"

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_PROBE_CFG_MAGIC        0x50434647UL /* 'PCFG' */
#define P7501_PROBE_CFG_VERSION      1u
#define P7501_ADC_GAIN_UNITY_PPM     1000000UL
#define P7501_ADC_GAIN_MIN_PPM       800000UL
#define P7501_ADC_GAIN_MAX_PPM       1200000UL
#define P7501_ADC_OFFSET_MAX_ABS_MV  500

#define P7501_RECIPE_MAGIC           0x50524350UL /* 'PRCP' */
#define P7501_RECIPE_VERSION         1u

typedef struct p7501_probe_cfg {
    uint32_t magic;
    uint32_t version;
    uint32_t sequence;
    uint32_t adc0_gain_ppm;
    int32_t adc0_offset_mv;
    uint32_t default_baud;
    char ssot_hash[16];
    uint32_t crc32;
} p7501_probe_cfg_t;

typedef struct p7501_recipe_hdr {
    uint32_t magic;
    uint32_t version;
    uint32_t payload_bytes;
    uint32_t payload_crc32;
    char recipe_id[24];
    char target_model[16];
    uint32_t header_crc32;
} p7501_recipe_hdr_t;

/** Initialize default calibrated probe configuration with CRC32. */
void p7501_probe_cfg_init(p7501_probe_cfg_t *cfg, const char *ssot_hash);

/** Re-sign configuration struct after mutating calibration or sequence fields. */
void p7501_probe_cfg_resign(p7501_probe_cfg_t *cfg);

/** Verify magic, bounds, and IEEE CRC32 of a probe configuration bank. */
bool p7501_probe_cfg_verify(const p7501_probe_cfg_t *cfg);

/**
 * Select active configuration from A/B banks (`0x1F0000` vs `0x1F8000`).
 * Returns 0 if Bank A wins, 1 if Bank B wins, or -1 if neither is valid.
 */
int p7501_probe_cfg_select_ab(
    const p7501_probe_cfg_t *bank_a,
    const p7501_probe_cfg_t *bank_b,
    p7501_probe_cfg_t *out_active
);

/** Apply linear integer ADC0 calibration (`(raw_mv * gain_ppm) / 1e6 + offset_mv`, clamped >= 0). */
uint32_t p7501_probe_cfg_apply_adc_cal_mv(
    uint32_t raw_mv,
    const p7501_probe_cfg_t *cfg
);

/** Build and sign a P3_RECIPE header. */
void p7501_recipe_hdr_init(
    p7501_recipe_hdr_t *hdr,
    const char *recipe_id,
    const char *target_model,
    const uint8_t *payload,
    size_t payload_len
);

/** Verify a P3_RECIPE header and optional payload buffer against IEEE CRC32. */
bool p7501_recipe_hdr_verify(
    const p7501_recipe_hdr_t *hdr,
    const uint8_t *payload,
    size_t payload_len
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PROBE_CFG_CODEC_H */
