/**
 * @file p7501_flash_geo.c
 * @brief SSOT-driven C11 Spansion NOR Flash, EEMUL NVRAM & BGO/[nfrom] geometry helpers.
 *
 * Host-testable with zero Pico SDK hardware dependencies (`C11_HOST_SOURCES`).
 * Consumes `k_p7501_ssot_flash_chips[]` and `k_p7501_ssot_target_profiles[]`
 * emitted by `crates/panadiga-core::ssot::pico_header`.
 * Security policy enforcement lives in `flash/p7501_bus_policy.c`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_flash.h"
#include "common/p7501_hex.h"
#include <string.h>

static p7501_session_ctx_t s_default_session = {
    .active_flash_size = P7501_FLASH_SIZE_8MB,
    .active_profile = P7501_TARGET_PROFILE_DMR_EH55,
    .profile_selected = false,
    .upper_nor_unlocked = false,
    .jedec_verified = false,
    .jedec_mfg = 0u,
    .jedec_dev = { 0u, 0u, 0u },
};

p7501_session_ctx_t *p7501_session_default_ctx(void) {
    return &s_default_session;
}

void p7501_session_ctx_reset(p7501_session_ctx_t *ctx) {
    if (ctx == NULL) {
        return;
    }
    ctx->active_flash_size = P7501_FLASH_SIZE_8MB;
    ctx->active_profile = P7501_TARGET_PROFILE_DMR_EH55;
    ctx->profile_selected = false;
    ctx->upper_nor_unlocked = false;
    ctx->jedec_verified = false;
    ctx->jedec_mfg = 0u;
    ctx->jedec_dev[0] = 0u;
    ctx->jedec_dev[1] = 0u;
    ctx->jedec_dev[2] = 0u;
}

static bool is_supported_flash_size(uint32_t flash_size_bytes) {
    return (flash_size_bytes == P7501_FLASH_SIZE_1MB) ||
           (flash_size_bytes == P7501_FLASH_SIZE_2MB) ||
           (flash_size_bytes == P7501_FLASH_SIZE_4MB) ||
           (flash_size_bytes == P7501_FLASH_SIZE_8MB) ||
           (flash_size_bytes == P7501_FLASH_SIZE_16MB);
}

uint32_t p7501_flash_normalize_size(uint32_t flash_size_bytes) {
    return is_supported_flash_size(flash_size_bytes)
        ? flash_size_bytes
        : P7501_FLASH_SIZE_8MB;
}

const p7501_ssot_flash_chip_t *p7501_flash_find_chip_for_session(
    const p7501_session_ctx_t *eff,
    uint32_t eff_size
) {
    if ((eff != NULL) && eff->profile_selected &&
        ((size_t)eff->active_profile < P7501_TARGET_PROFILE_COUNT)) {
        const p7501_ssot_target_profile_t *prof =
            &k_p7501_ssot_target_profiles[eff->active_profile];
        if ((prof->total_bytes == eff_size) && ((size_t)prof->flash_id < P7501_FLASH_ID_COUNT)) {
            return &k_p7501_ssot_flash_chips[prof->flash_id];
        }
    }
    for (size_t i = 0u; i < P7501_FLASH_ID_COUNT; ++i) {
        if (k_p7501_ssot_flash_chips[i].total_bytes == eff_size) {
            return &k_p7501_ssot_flash_chips[i];
        }
    }
    return &k_p7501_ssot_flash_chips[P7501_FLASH_ID_S29JL064H_8MB];
}

const p7501_ssot_flash_chip_t *p7501_flash_find_chip_by_size(uint32_t flash_size_bytes) {
    const uint32_t eff = p7501_flash_normalize_size(flash_size_bytes);
    return p7501_flash_find_chip_for_session(&s_default_session, eff);
}

const p7501_ssot_flash_chip_t *p7501_flash_find_chip_by_jedec(
    uint16_t mfg,
    const uint16_t dev[3],
    uint8_t dev_cycles
) {
    if (dev == NULL) {
        return NULL;
    }
    for (size_t i = 0u; i < P7501_FLASH_ID_COUNT; ++i) {
        const p7501_ssot_flash_chip_t *c = &k_p7501_ssot_flash_chips[i];
        if ((c->jedec_mfg != mfg) || (c->jedec_cycles == 0u) || (dev_cycles < c->jedec_cycles)) {
            continue;
        }
        bool all_ok = true;
        for (uint8_t k = 0u; (k < c->jedec_cycles) && (k < 3u); ++k) {
            if (c->jedec_dev[k] != dev[k]) {
                all_ok = false;
                break;
            }
        }
        if (all_ok) {
            return c;
        }
    }
    return NULL;
}

uint32_t p7501_flash_get_active_size(void) {
    return s_default_session.active_flash_size;
}

bool p7501_flash_set_active_size(uint32_t flash_size_bytes) {
    if (!is_supported_flash_size(flash_size_bytes)) {
        return false;
    }
    s_default_session.active_flash_size = flash_size_bytes;
    return true;
}

const p7501_ssot_target_profile_t *p7501_flash_get_target_profile(void) {
    const size_t idx = (size_t)s_default_session.active_profile;
    return (idx < P7501_TARGET_PROFILE_COUNT)
        ? &k_p7501_ssot_target_profiles[idx]
        : &k_p7501_ssot_target_profiles[P7501_TARGET_PROFILE_DMR_EH55];
}

bool p7501_flash_set_target_profile_by_id(p7501_ssot_target_profile_id_t profile_id) {
    if ((size_t)profile_id >= P7501_TARGET_PROFILE_COUNT) {
        return false;
    }
    const p7501_ssot_target_profile_t *prof = &k_p7501_ssot_target_profiles[profile_id];
    s_default_session.active_profile = profile_id;
    s_default_session.profile_selected = true;
    s_default_session.active_flash_size = prof->total_bytes;
    if (!prof->upper_nor_unlock_allowed || prof->full_nor_readonly || !prof->writable_nvram) {
        s_default_session.upper_nor_unlocked = false;
    }
    return true;
}

bool p7501_flash_set_target_profile_name(const char *model_name) {
    if ((model_name == NULL) || (*model_name == '\0')) {
        return false;
    }
    for (size_t i = 0u; i < P7501_TARGET_PROFILE_COUNT; ++i) {
        if (p7501_ascii_ieq(model_name, k_p7501_ssot_target_profiles[i].model_name)) {
            return p7501_flash_set_target_profile_by_id(
                k_p7501_ssot_target_profiles[i].profile_id
            );
        }
    }
    return false;
}

p7501_flash_geometry_t p7501_flash_get_geometry(uint32_t flash_size_bytes) {
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    const p7501_ssot_flash_chip_t *chip = p7501_flash_find_chip_by_size(eff_size);
    const p7501_ssot_target_profile_t *prof = p7501_flash_get_target_profile();
    const bool use_prof = s_default_session.profile_selected && (prof->total_bytes == eff_size);

    const uint32_t nvram_size = use_prof ? prof->eemul_size : chip->nvram_size;
    const uint32_t nvram_offset = use_prof ? prof->eemul_offset : chip->nvram_offset;

    p7501_flash_geometry_t geo;
    memset(&geo, 0, sizeof(geo));
    geo.bytes = eff_size;
    geo.mb = eff_size / (1024UL * 1024UL);
    geo.nvram_size = nvram_size;
    geo.hw_uncached_base = P7501_SSOT_CS0_UNCACHED_BASE;

    if (nvram_size > 0UL) {
        geo.nvram_base = P7501_SSOT_CS0_CACHED_BASE + nvram_offset;
        geo.nvram_end = geo.nvram_base + nvram_size;
        geo.hw_nvram_base = P7501_SSOT_CS0_UNCACHED_BASE + nvram_offset;
        geo.hw_nvram_end = geo.hw_nvram_base + nvram_size;
        geo.base_sector = use_prof ? prof->eemul_base_sa : chip->nvram_base_sa;
        geo.sector_count = use_prof ? prof->eemul_sector_count : chip->nvram_sector_count;
    }

    if (use_prof) {
        geo.bgo_sector_count = prof->bgo_sector_count;
        if (prof->bgo_size > 0UL) {
            geo.bgo_hw_base = P7501_SSOT_CS0_UNCACHED_BASE + prof->bgo_offset;
            geo.bgo_hw_end = geo.bgo_hw_base + prof->bgo_size;
        }
    } else if ((eff_size == P7501_FLASH_SIZE_8MB) || (eff_size == P7501_FLASH_SIZE_16MB)) {
        geo.bgo_hw_base = P7501_SSOT_CS0_UNCACHED_BASE + P7501_BGO_PARTITION_OFFSET;
        geo.bgo_hw_end = geo.bgo_hw_base + P7501_BGO_PARTITION_SIZE;
        geo.bgo_sector_count = P7501_BGO_SECTOR_COUNT;
    } else if (eff_size == P7501_FLASH_SIZE_4MB) {
        /* 4 MiB non-overlapping BGO: 0x84100000..0x8423FFFF (< NVRAM 0x843F0000) */
        geo.bgo_hw_base = P7501_SSOT_CS0_UNCACHED_BASE + 0x00100000UL;
        geo.bgo_hw_end = P7501_SSOT_CS0_UNCACHED_BASE + 0x00240000UL;
        geo.bgo_sector_count = 20u;
    }
    return geo;
}

uint32_t p7501_flash_normalize_uncached_addr(uint32_t addr) {
    if ((addr >= P7501_SSOT_CS0_CACHED_BASE) &&
        (addr < (P7501_SSOT_CS0_CACHED_BASE + P7501_FLASH_SIZE_16MB))) {
        return P7501_SSOT_CS0_UNCACHED_BASE + (addr - P7501_SSOT_CS0_CACHED_BASE);
    }
    return addr;
}

bool p7501_flash_get_sector_span_at(
    uint32_t flash_size_bytes,
    uint32_t addr,
    uint32_t *out_base_addr,
    uint32_t *out_size
) {
    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(flash_size_bytes);
    uint32_t base_win = 0UL;
    uint32_t offset = 0UL;
    if ((addr >= P7501_SSOT_CS0_CACHED_BASE) &&
        (addr < (P7501_SSOT_CS0_CACHED_BASE + geo.bytes))) {
        base_win = P7501_SSOT_CS0_CACHED_BASE;
        offset = addr - P7501_SSOT_CS0_CACHED_BASE;
    } else if ((addr >= geo.hw_uncached_base) && (addr < (geo.hw_uncached_base + geo.bytes))) {
        base_win = geo.hw_uncached_base;
        offset = addr - geo.hw_uncached_base;
    } else if (addr < geo.bytes) {
        base_win = 0UL;
        offset = addr;
    } else {
        return false;
    }

    const p7501_ssot_flash_chip_t *chip = p7501_flash_find_chip_by_size(geo.bytes);
    uint32_t cursor = 0UL;
    for (uint8_t b = 0u; (b < chip->bank_count) && (b < 4u); ++b) {
        const uint32_t ssz = chip->banks[b].sector_size;
        const uint32_t bank_span = (uint32_t)chip->banks[b].count * ssz;
        if ((ssz > 0UL) && (offset >= cursor) && (offset < (cursor + bank_span))) {
            const uint32_t idx_in_bank = (offset - cursor) / ssz;
            if (out_base_addr != NULL) {
                *out_base_addr = base_win + cursor + (idx_in_bank * ssz);
            }
            if (out_size != NULL) {
                *out_size = ssz;
            }
            return true;
        }
        cursor += bank_span;
    }
    return false;
}

uint32_t p7501_flash_get_sector_size_at(uint32_t flash_size_bytes, uint32_t addr) {
    uint32_t sec_size = 0UL;
    return p7501_flash_get_sector_span_at(flash_size_bytes, addr, NULL, &sec_size)
        ? sec_size
        : 0UL;
}

bool p7501_flash_is_nvram_microsector_addr_for_flash(uint32_t flash_size_bytes, uint32_t addr) {
    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(flash_size_bytes);
    if (geo.nvram_size == 0UL) {
        return false;
    }
    return ((addr >= geo.nvram_base) && (addr < geo.nvram_end)) ||
           ((addr >= geo.hw_nvram_base) && (addr < geo.hw_nvram_end));
}

bool p7501_flash_is_nvram_microsector_addr(uint32_t addr) {
    return p7501_flash_is_nvram_microsector_addr_for_flash(
        s_default_session.active_flash_size, addr
    );
}
