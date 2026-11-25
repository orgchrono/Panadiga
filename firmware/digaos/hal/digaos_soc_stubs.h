/**
 * @file digaos_soc_stubs.h
 * @brief Extensible Multi-SoC Fail-Closed HAL Guards for `DigaOS`.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Only `mn10300_am34` (`MN2DS0009`) has verified executable MMIO adapters. Non-AM34 targets
 * (`AM33-2`, `UniPhier ARMv7/ARMv8`, `SPARC v8 DoMiNo`) compile cleanly from SSOT and fail closed
 * with `UNVERIFIED_MMIO` until verified on physical silicon.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PANADIGA_DIGAOS_SOC_STUBS_H
#define PANADIGA_DIGAOS_SOC_STUBS_H

#include "hal/am34_mmio.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Resolve SSOT `digaos_soc_target_t` entry by `soc_core_id` (case-sensitive). */
const digaos_soc_target_t *digaos_soc_find_target(const char *soc_core_id);

/** Return true iff `soc_core_id` is marked `mmio_verified` and passes MMIO/IPL barriers. */
bool digaos_soc_mmio_guard(
    const char *soc_core_id,
    uint32_t addr,
    bool is_write,
    const char **out_err
);

/** Fail-closed 32-bit MMIO read across SSOT SoC targets (`UNVERIFIED_MMIO` for stubs). */
bool digaos_soc_read32(
    const char *soc_core_id,
    uint32_t addr,
    uint32_t *out_val,
    const char **out_err
);

/** Fail-closed 32-bit MMIO write across SSOT SoC targets (`UNVERIFIED_MMIO` for stubs). */
bool digaos_soc_write32(
    const char *soc_core_id,
    uint32_t addr,
    uint32_t val,
    const char **out_err
);

#ifdef __cplusplus
}
#endif

#endif /* PANADIGA_DIGAOS_SOC_STUBS_H */
