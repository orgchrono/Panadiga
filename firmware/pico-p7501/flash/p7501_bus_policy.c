/**
 * @file p7501_bus_policy.c
 * @brief Single Authoritative AM34 Bus & NOR Flash Access Policy Gate (SSOT-driven).
 *
 * Enforces all physical and forensic silicon invariants across CLI, GDB RSP,
 * PANARSP, DIAGWATCH, JEDEC stubs, and target bus HAL. UART TX payload and
 * buffered `BRIDGE-POLICY` framing live in `flash/p7501_bus_tx_guard.c`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_bus_policy.h"
#include "flash/p7501_flash.h"
#include "common/p7501_hex.h"
#include "hal/p7501_wireless_codec.h"

#include <string.h>

bool p7501_bus_span_wraps_u32(uint32_t addr, size_t len) {
    const uint64_t span = (len == 0u) ? 1ULL : (uint64_t)len;
    return span > (0x100000000ULL - (uint64_t)addr);
}

static bool range_overlaps_u64(uint32_t addr, size_t len, uint64_t lo, uint64_t hi_excl) {
    if (hi_excl <= lo) {
        return false;
    }
    const uint64_t raw_span = (len == 0u) ? 1ULL : (uint64_t)len;
    const uint64_t span = (raw_span > 0x100000000ULL) ? 0x100000000ULL : raw_span;
    const uint64_t s64 = (uint64_t)addr;
    const uint64_t e64 = s64 + span - 1ULL;
    if ((s64 <= (hi_excl - 1ULL)) && (e64 >= lo)) {
        return true;
    }
    if (e64 >= 0x100000000ULL) {
        const uint64_t wrap_e64 = e64 - 0x100000000ULL;
        return wrap_e64 >= lo;
    }
    return false;
}

static bool flash_range_overlaps_off_span(
    uint32_t addr,
    size_t len,
    uint32_t off_start,
    uint32_t off_end
) {
    if ((len == 0u) || (off_end <= off_start)) {
        return false;
    }
    const uint64_t c_lo = (uint64_t)P7501_SSOT_CS0_CACHED_BASE + (uint64_t)off_start;
    const uint64_t c_hi = (uint64_t)P7501_SSOT_CS0_CACHED_BASE + (uint64_t)off_end;
    const uint64_t h_lo = (uint64_t)P7501_SSOT_CS0_UNCACHED_BASE + (uint64_t)off_start;
    const uint64_t h_hi = (uint64_t)P7501_SSOT_CS0_UNCACHED_BASE + (uint64_t)off_end;
    return range_overlaps_u64(addr, len, c_lo, c_hi) ||
           range_overlaps_u64(addr, len, h_lo, h_hi);
}

bool p7501_bus_is_lethal_mmio_range(uint32_t addr, size_t len) {
    const uint64_t lo1 = (uint64_t)P7501_SSOT_AM34_LETHAL_REG_ADDR;
    const uint64_t hi1 = lo1 + (uint64_t)P7501_SSOT_AM34_LETHAL_REG_LEN;
    const uint64_t lo2 = (uint64_t)P7501_SSOT_AM34_LETHAL_REG2_ADDR;
    const uint64_t hi2 = lo2 + (uint64_t)P7501_SSOT_AM34_LETHAL_REG2_LEN;
    return range_overlaps_u64(addr, len, lo1, hi1) ||
           range_overlaps_u64(addr, len, lo2, hi2);
}

bool p7501_bus_touches_cs0_aperture(uint32_t addr, size_t len) {
    return range_overlaps_u64(
        addr,
        len,
        (uint64_t)P7501_SSOT_CS0_CACHED_BASE,
        (uint64_t)P7501_SSOT_CS0_APERTURE_END
    );
}

bool p7501_bus_is_cs0_alias_mirror(uint32_t flash_size_bytes, uint32_t addr, size_t len) {
    if (p7501_bus_span_wraps_u32(addr, len)) {
        return true;
    }
    if (!p7501_bus_touches_cs0_aperture(addr, len)) {
        return false;
    }
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    const uint64_t span = (len == 0u) ? 1ULL : (uint64_t)len;
    const uint64_t s64 = (uint64_t)addr;
    const uint64_t e64 = s64 + span;
    const uint64_t c_lo = (uint64_t)P7501_SSOT_CS0_CACHED_BASE;
    const uint64_t c_hi = c_lo + (uint64_t)eff_size;
    const uint64_t h_lo = (uint64_t)P7501_SSOT_CS0_UNCACHED_BASE;
    const uint64_t h_hi = h_lo + (uint64_t)eff_size;
    const bool in_cached = (s64 >= c_lo) && (e64 <= c_hi);
    const bool in_uncached = (s64 >= h_lo) && (e64 <= h_hi);
    return !(in_cached || in_uncached);
}

const char *p7501_bus_deny_name(p7501_bus_deny_t reason) {
    switch (reason) {
        case P7501_BUS_ALLOW:             return "ALLOW";
        case P7501_BUS_DENY_LETHAL_MMIO:  return "LETHAL_MMIO_0xC000100C";
        case P7501_BUS_DENY_WIRELESS_F14: return "F14_WIRELESS_LOCK";
        case P7501_BUS_DENY_CS0_MIRROR:   return "CS0_ALIAS_MIRROR";
        case P7501_BUS_DENY_IPL_LAYER1:   return "IPL_LAYER1_RO";
        case P7501_BUS_DENY_FULL_RO:      return "FULL_NOR_RO";
        case P7501_BUS_DENY_PROTECTED_RO: return "PROTECTED_XIP_RO";
        case P7501_BUS_DENY_UPPER_LOCKED: return "UPPER_NOR_LOCKED";
        default:                          return "E05_DENIED";
    }
}

bool p7501_flash_is_readonly_nor_range(
    uint32_t flash_size_bytes,
    uint32_t start_addr,
    uint32_t total_len
) {
    if ((total_len == 0U) || p7501_bus_span_wraps_u32(start_addr, (size_t)total_len)) {
        return false;
    }
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    const uint64_t s64 = (uint64_t)start_addr;
    const uint64_t e64 = s64 + (uint64_t)total_len;

    const uint64_t cached_lo = (uint64_t)P7501_SSOT_CS0_CACHED_BASE;
    const uint64_t cached_hi = cached_lo + (uint64_t)eff_size;
    const uint64_t hw_lo = (uint64_t)P7501_SSOT_CS0_UNCACHED_BASE;
    const uint64_t hw_hi = hw_lo + (uint64_t)eff_size;

    return ((s64 >= cached_lo) && (e64 <= cached_hi)) ||
           ((s64 >= hw_lo) && (e64 <= hw_hi));
}

bool p7501_flash_is_protected_ipl_range_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len
) {
    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(flash_size_bytes);
    if ((geo.nvram_size == 0UL) || (geo.nvram_base < P7501_SSOT_CS0_CACHED_BASE)) {
        return flash_range_overlaps_off_span(addr, len, 0UL, geo.bytes);
    }
    return flash_range_overlaps_off_span(addr, len, 0UL, geo.nvram_base - P7501_SSOT_CS0_CACHED_BASE);
}

bool p7501_flash_is_protected_ipl_range(uint32_t addr, size_t len) {
    return p7501_flash_is_protected_ipl_range_for_flash(
        p7501_flash_get_active_size(), addr, len
    );
}

bool p7501_flash_is_critical_boot_ipl_range(uint32_t addr, size_t len) {
    return flash_range_overlaps_off_span(addr, len, 0UL, P7501_SSOT_IPL_LAYER1_SIZE);
}

bool p7501_flash_is_upper_nor_unlocked(void) {
    return p7501_session_default_ctx()->upper_nor_unlocked;
}

void p7501_flash_set_upper_nor_unlocked(bool unlocked) {
    p7501_session_ctx_t *sess = p7501_session_default_ctx();
    if (unlocked && !p7501_wireless_nor_write_allowed(p7501_wireless_global_state())) {
        sess->upper_nor_unlocked = false;
        return;
    }
    if (unlocked && sess->profile_selected) {
        const p7501_ssot_target_profile_t *prof = p7501_flash_get_target_profile();
        if (!prof->upper_nor_unlock_allowed || prof->full_nor_readonly || !prof->writable_nvram) {
            sess->upper_nor_unlocked = false;
            return;
        }
    }
    sess->upper_nor_unlocked = unlocked;
}

void p7501_flash_apply_policy_req(p7501_policy_req_t req) {
    if (req == P7501_POLICY_REQ_UNLOCK) {
        p7501_flash_set_upper_nor_unlocked(true);
    } else if (req == P7501_POLICY_REQ_LOCK) {
        p7501_flash_set_upper_nor_unlocked(false);
    }
}

p7501_bus_deny_t p7501_bus_check_access(
    const p7501_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool is_write,
    bool is_erase,
    bool upper_nor_unlocked
) {
    if (p7501_bus_is_lethal_mmio_range(addr, len)) {
        return P7501_BUS_DENY_LETHAL_MMIO;
    }
    if (p7501_bus_span_wraps_u32(addr, len)) {
        return P7501_BUS_DENY_CS0_MIRROR;
    }
    if (!is_write && !is_erase) {
        return P7501_BUS_ALLOW;
    }

    const p7501_session_ctx_t *eff = (ctx != NULL) ? ctx : p7501_session_default_ctx();
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);

    if (!p7501_wireless_nor_write_allowed(p7501_wireless_global_state())) {
        return P7501_BUS_DENY_WIRELESS_F14;
    }

    uint32_t check_addr = addr;
    size_t check_len = (len == 0u) ? 1u : len;

    if (is_erase) {
        if (!p7501_flash_is_readonly_nor_range(eff_size, addr, 1UL)) {
            return P7501_BUS_DENY_CS0_MIRROR;
        }
        uint32_t sec_base = 0UL;
        uint32_t sec_size = 0UL;
        if (!p7501_flash_get_sector_span_at(eff_size, addr, &sec_base, &sec_size) || (sec_size == 0UL)) {
            return P7501_BUS_DENY_CS0_MIRROR;
        }
        const uint64_t req_end = (uint64_t)addr + (uint64_t)check_len;
        const uint64_t sec_end = (uint64_t)sec_base + (uint64_t)sec_size;
        const uint64_t eff_end = (req_end > sec_end) ? req_end : sec_end;
        if ((eff_end > 0x100000000ULL) || ((eff_end - (uint64_t)sec_base) > (uint64_t)eff_size)) {
            return P7501_BUS_DENY_CS0_MIRROR;
        }
        check_addr = sec_base;
        check_len = (size_t)(eff_end - (uint64_t)sec_base);
    }

    if (p7501_bus_is_cs0_alias_mirror(eff_size, check_addr, check_len)) {
        return P7501_BUS_DENY_CS0_MIRROR;
    }
    if (p7501_flash_is_critical_boot_ipl_range(check_addr, check_len)) {
        return P7501_BUS_DENY_IPL_LAYER1;
    }

    if (eff->profile_selected && ((size_t)eff->active_profile < P7501_TARGET_PROFILE_COUNT)) {
        const p7501_ssot_target_profile_t *prof = &k_p7501_ssot_target_profiles[eff->active_profile];
        const uint32_t prof_span = (prof->total_bytes > eff_size) ? prof->total_bytes : eff_size;
        if (prof->full_nor_readonly || !prof->writable_nvram) {
            if (flash_range_overlaps_off_span(check_addr, check_len, 0UL, prof_span)) {
                return P7501_BUS_DENY_FULL_RO;
            }
        }
        if ((prof->protected_ro_end > prof->protected_ro_start) &&
            flash_range_overlaps_off_span(
                check_addr, check_len, prof->protected_ro_start, prof->protected_ro_end
            )) {
            return P7501_BUS_DENY_PROTECTED_RO;
        }
        if (!prof->upper_nor_unlock_allowed) {
            upper_nor_unlocked = false;
        }
    }

    const p7501_ssot_flash_chip_t *chip = p7501_flash_find_chip_for_session(eff, eff_size);
    if (!chip->writable && flash_range_overlaps_off_span(check_addr, check_len, 0UL, eff_size)) {
        return P7501_BUS_DENY_FULL_RO;
    }

    if (!upper_nor_unlocked &&
        p7501_flash_is_protected_ipl_range_for_flash(eff_size, check_addr, check_len)) {
        return P7501_BUS_DENY_UPPER_LOCKED;
    }
    return P7501_BUS_ALLOW;
}

bool p7501_flash_is_write_blocked_in_session(
    const p7501_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked
) {
    return p7501_bus_check_access(
        ctx, flash_size_bytes, addr, len, true, false, upper_nor_unlocked
    ) != P7501_BUS_ALLOW;
}

bool p7501_flash_is_write_blocked_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool upper_nor_unlocked
) {
    return p7501_flash_is_write_blocked_in_session(
        p7501_session_default_ctx(), flash_size_bytes, addr, len, upper_nor_unlocked
    );
}

bool p7501_flash_is_write_blocked_with_policy(uint32_t addr, size_t len, bool upper_nor_unlocked) {
    return p7501_flash_is_write_blocked_for_flash(
        p7501_flash_get_active_size(), addr, len, upper_nor_unlocked
    );
}

bool p7501_flash_is_write_blocked(uint32_t addr, size_t len) {
    return p7501_flash_is_write_blocked_with_policy(
        addr, len, p7501_flash_is_upper_nor_unlocked()
    );
}

bool p7501_flash_is_erasable_nor_addr_for_flash(
    uint32_t flash_size_bytes,
    uint32_t addr,
    bool upper_nor_unlocked
) {
    return p7501_bus_check_access(
        p7501_session_default_ctx(), flash_size_bytes, addr, 1u, true, true, upper_nor_unlocked
    ) == P7501_BUS_ALLOW;
}

bool p7501_flash_is_erasable_nor_addr(uint32_t addr, bool upper_nor_unlocked) {
    return p7501_flash_is_erasable_nor_addr_for_flash(
        p7501_flash_get_active_size(), addr, upper_nor_unlocked
    );
}

int p7501_flash_eval_policy_token(const char *tok) {
    if (tok == NULL) {
        return 2;
    }
    if (p7501_ascii_ieq(tok, "NORLOCK")) {
        return 0;
    }
    if (p7501_ascii_iprefix(tok, "NORUNLOCK")) {
        return ((tok[9] == ':') && (strcmp(tok + 10, "CONFIRM_UPPER_NOR") == 0)) ? 1 : -1;
    }
    return 2;
}

bool p7501_flash_consume_policy_tok(const char *tok, int *io_override, bool *out_invalid) {
    *out_invalid = false;
    const int ptok = p7501_flash_eval_policy_token(tok);
    if (ptok == -1) {
        *out_invalid = true;
        return true;
    }
    if ((ptok == 0) || (ptok == 1)) {
        if (io_override != NULL) {
            *io_override = ptok;
        }
        return true;
    }
    return false;
}

bool p7501_flash_effective_policy(int policy_override) {
    p7501_session_ctx_t *sess = p7501_session_default_ctx();
    if (policy_override == 1) {
        if (!p7501_wireless_nor_write_allowed(p7501_wireless_global_state())) {
            return false;
        }
        if (sess->profile_selected) {
            const p7501_ssot_target_profile_t *prof = p7501_flash_get_target_profile();
            if (!prof->upper_nor_unlock_allowed || prof->full_nor_readonly || !prof->writable_nvram) {
                return false;
            }
        }
        return true;
    }
    if (policy_override == 0) {
        return false;
    }
    return sess->upper_nor_unlocked;
}
