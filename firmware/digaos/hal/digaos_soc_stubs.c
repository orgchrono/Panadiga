/**
 * @file digaos_soc_stubs.c
 * @brief Extensible Multi-SoC Fail-Closed HAL Implementation (`AM34` active, `AM33-2`/`UniPhier`/`SPARC` guarded).
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/digaos_soc_stubs.h"

#include <string.h>

const digaos_soc_target_t *digaos_soc_find_target(const char *soc_core_id) {
    if (soc_core_id == NULL) {
        return NULL;
    }
    for (size_t i = 0u; i < DIGAOS_SSOT_SOC_TARGET_COUNT; ++i) {
        if (strcmp(k_digaos_ssot_soc_targets[i].soc_core_id, soc_core_id) == 0) {
            return &k_digaos_ssot_soc_targets[i];
        }
    }
    return NULL;
}

bool digaos_soc_mmio_guard(
    const char *soc_core_id,
    uint32_t addr,
    bool is_write,
    const char **out_err
) {
    const digaos_soc_target_t *target = digaos_soc_find_target(soc_core_id);
    if (target == NULL) {
        if (out_err != NULL) {
            *out_err = "UNKNOWN_SOC_TARGET";
        }
        return false;
    }
    if ((target->logic_voltage_mv == 0u) || (target->logic_voltage_mv > 3300u)) {
        if (out_err != NULL) {
            *out_err = "VOLTAGE_INVARIANT_EXCEEDED";
        }
        return false;
    }
    if ((target->mmio_verified == 0u) || (target->is_primary_executable == 0u)) {
        if (out_err != NULL) {
            *out_err = "UNVERIFIED_MMIO";
        }
        return false;
    }
    if (am34_mmio_is_forbidden(addr)) {
        if (out_err != NULL) {
            *out_err = "LETHAL_MMIO_LOCKED";
        }
        return false;
    }
    if (is_write && !am34_mmio_write_allowed(addr)) {
        if (out_err != NULL) {
            *out_err = "IPL_RO_E05";
        }
        return false;
    }
    if (out_err != NULL) {
        *out_err = "OK";
    }
    return true;
}

bool digaos_soc_read32(
    const char *soc_core_id,
    uint32_t addr,
    uint32_t *out_val,
    const char **out_err
) {
    if (out_val == NULL) {
        if (out_err != NULL) {
            *out_err = "NULL_OUT_PTR";
        }
        return false;
    }
    if (!digaos_soc_mmio_guard(soc_core_id, addr, false, out_err)) {
        return false;
    }
    return am34_mmio_read32(addr, out_val);
}

bool digaos_soc_write32(
    const char *soc_core_id,
    uint32_t addr,
    uint32_t val,
    const char **out_err
) {
    if (!digaos_soc_mmio_guard(soc_core_id, addr, true, out_err)) {
        return false;
    }
    return am34_mmio_write32(addr, val);
}
