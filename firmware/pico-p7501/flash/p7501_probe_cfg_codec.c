/**
 * @file p7501_probe_cfg_codec.c
 * @brief Pure ISO C11 implementation of `P5_PROBE_CFG` A/B calibration & `P3_RECIPE` verification.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_probe_cfg_codec.h"
#include "common/p7501_hex.h"

#include <stddef.h>
#include <string.h>

static uint32_t probe_cfg_crc(const p7501_probe_cfg_t *cfg) {
    return p7501_crc32_ieee((const uint8_t *)cfg, offsetof(p7501_probe_cfg_t, crc32));
}

static uint32_t recipe_hdr_crc(const p7501_recipe_hdr_t *hdr) {
    return p7501_crc32_ieee((const uint8_t *)hdr, offsetof(p7501_recipe_hdr_t, header_crc32));
}

void p7501_probe_cfg_resign(p7501_probe_cfg_t *cfg) {
    if (cfg == NULL) {
        return;
    }
    cfg->crc32 = probe_cfg_crc(cfg);
}

void p7501_probe_cfg_init(p7501_probe_cfg_t *cfg, const char *ssot_hash) {
    if (cfg == NULL) {
        return;
    }
    memset(cfg, 0, sizeof(*cfg));
    cfg->magic = P7501_PROBE_CFG_MAGIC;
    cfg->version = P7501_PROBE_CFG_VERSION;
    cfg->sequence = 1u;
    cfg->adc0_gain_ppm = P7501_ADC_GAIN_UNITY_PPM;
    cfg->adc0_offset_mv = 0;
    cfg->default_baud = 115200UL;
    if (ssot_hash != NULL) {
        for (size_t i = 0u; (i + 1u) < sizeof(cfg->ssot_hash) && (ssot_hash[i] != '\0'); ++i) {
            cfg->ssot_hash[i] = ssot_hash[i];
        }
    }
    p7501_probe_cfg_resign(cfg);
}

bool p7501_probe_cfg_verify(const p7501_probe_cfg_t *cfg) {
    if ((cfg == NULL) || (cfg->magic != P7501_PROBE_CFG_MAGIC) ||
        (cfg->version != P7501_PROBE_CFG_VERSION) || (cfg->sequence == 0u)) {
        return false;
    }
    if ((cfg->adc0_gain_ppm < P7501_ADC_GAIN_MIN_PPM) ||
        (cfg->adc0_gain_ppm > P7501_ADC_GAIN_MAX_PPM) ||
        (cfg->adc0_offset_mv < -P7501_ADC_OFFSET_MAX_ABS_MV) ||
        (cfg->adc0_offset_mv > P7501_ADC_OFFSET_MAX_ABS_MV)) {
        return false;
    }
    return cfg->crc32 == probe_cfg_crc(cfg);
}

int p7501_probe_cfg_select_ab(
    const p7501_probe_cfg_t *bank_a,
    const p7501_probe_cfg_t *bank_b,
    p7501_probe_cfg_t *out_active
) {
    const bool a_ok = p7501_probe_cfg_verify(bank_a);
    const bool b_ok = p7501_probe_cfg_verify(bank_b);
    if (a_ok && b_ok) {
        const bool b_newer = (bank_b->sequence >= bank_a->sequence);
        if (out_active != NULL) {
            *out_active = b_newer ? *bank_b : *bank_a;
        }
        return b_newer ? 1 : 0;
    }
    if (a_ok) {
        if (out_active != NULL) {
            *out_active = *bank_a;
        }
        return 0;
    }
    if (b_ok) {
        if (out_active != NULL) {
            *out_active = *bank_b;
        }
        return 1;
    }
    return -1;
}

uint32_t p7501_probe_cfg_apply_adc_cal_mv(
    uint32_t raw_mv,
    const p7501_probe_cfg_t *cfg
) {
    if (!p7501_probe_cfg_verify(cfg)) {
        return raw_mv;
    }
    const uint64_t scaled = (((uint64_t)raw_mv * (uint64_t)cfg->adc0_gain_ppm) + 500000ULL) / 1000000ULL;
    const int64_t adjusted = (int64_t)scaled + (int64_t)cfg->adc0_offset_mv;
    if (adjusted <= 0) {
        return 0u;
    }
    if (adjusted > 5000) {
        return 5000u;
    }
    return (uint32_t)adjusted;
}

void p7501_recipe_hdr_init(
    p7501_recipe_hdr_t *hdr,
    const char *recipe_id,
    const char *target_model,
    const uint8_t *payload,
    size_t payload_len
) {
    if (hdr == NULL) {
        return;
    }
    memset(hdr, 0, sizeof(*hdr));
    hdr->magic = P7501_RECIPE_MAGIC;
    hdr->version = P7501_RECIPE_VERSION;
    hdr->payload_bytes = (uint32_t)payload_len;
    hdr->payload_crc32 = ((payload != NULL) && (payload_len > 0u))
        ? p7501_crc32_ieee(payload, payload_len)
        : 0u;
    if (recipe_id != NULL) {
        for (size_t i = 0u; (i + 1u) < sizeof(hdr->recipe_id) && (recipe_id[i] != '\0'); ++i) {
            hdr->recipe_id[i] = recipe_id[i];
        }
    }
    if (target_model != NULL) {
        for (size_t i = 0u; (i + 1u) < sizeof(hdr->target_model) && (target_model[i] != '\0'); ++i) {
            hdr->target_model[i] = target_model[i];
        }
    }
    hdr->header_crc32 = recipe_hdr_crc(hdr);
}

bool p7501_recipe_hdr_verify(
    const p7501_recipe_hdr_t *hdr,
    const uint8_t *payload,
    size_t payload_len
) {
    if ((hdr == NULL) || (hdr->magic != P7501_RECIPE_MAGIC) ||
        (hdr->version != P7501_RECIPE_VERSION) ||
        (hdr->payload_bytes > (P7501_QSPI_P3_RECIPE_SIZE - sizeof(*hdr)))) {
        return false;
    }
    if (hdr->header_crc32 != recipe_hdr_crc(hdr)) {
        return false;
    }
    if (payload != NULL) {
        if (payload_len < hdr->payload_bytes) {
            return false;
        }
        if (p7501_crc32_ieee(payload, hdr->payload_bytes) != hdr->payload_crc32) {
            return false;
        }
    }
    return true;
}
