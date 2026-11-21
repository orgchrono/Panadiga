/**
 * @file p7501_pana_jedec_stubs.c
 * @brief MN103S SRAM 6-cycle Erase & 4-cycle Program Spansion JEDEC stub builders + emitter primitives.
 *
 * Split from monolithic `p7501_pana_jedec.c` (`p7501_pana_jedec_id.c` owns Autoselect/CFI & validators).
 * Enforces `p7501_bus_check_access()` defense-in-depth inside `build_jedec_checked()`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/p7501_pana_jedec.h"
#include "uart/p7501_pana_jedec_internal.h"
#include "uart/p7501_pana_rsp_internal.h"
#include "common/p7501_hex.h"
#include "flash/p7501_flash.h"

#include <string.h>

void p7501_j_u8(p7501_jedec_buf_t *jb, uint8_t v) {
    if (jb->pos < jb->cap) {
        jb->out[jb->pos] = v;
    }
    jb->pos += 1u;
}

void p7501_j_arr(p7501_jedec_buf_t *jb, const uint8_t *src, size_t n) {
    if ((jb->pos + n) <= jb->cap) {
        memcpy(&jb->out[jb->pos], src, n);
    }
    jb->pos += n;
}

size_t p7501_j_imm32(p7501_jedec_buf_t *jb, uint8_t sub_op, uint32_t imm) {
    const size_t lit_off = jb->pos + 2u;
    uint8_t le[4];
    p7501_rsp_encode_u32_le(imm, le);
    p7501_j_u8(jb, 0xFCu);
    p7501_j_u8(jb, sub_op);
    p7501_j_arr(jb, le, 4u);
    return lit_off;
}

void p7501_j_patch_s8(p7501_jedec_buf_t *jb, size_t br_off, size_t target_off) {
    if ((br_off + 1u) < jb->cap) {
        jb->out[br_off + 1u] = (uint8_t)((int8_t)((ptrdiff_t)target_off - (ptrdiff_t)br_off));
    }
}

void p7501_j_patch_s16(p7501_jedec_buf_t *jb, size_t insn_off, size_t target_off) {
    if ((insn_off + 2u) < jb->cap) {
        const int16_t d = (int16_t)((ptrdiff_t)target_off - (ptrdiff_t)insn_off);
        jb->out[insn_off + 1u] = (uint8_t)(d & 0xFF);
        jb->out[insn_off + 2u] = (uint8_t)((d >> 8) & 0xFF);
    }
}

void p7501_j_patch_u32(p7501_jedec_buf_t *jb, size_t lit_off, uint32_t val) {
    if ((lit_off + 4u) <= jb->cap) {
        p7501_rsp_encode_u32_le(val, &jb->out[lit_off]);
    }
}

void p7501_j_cmd_a0(p7501_jedec_buf_t *jb, uint32_t bus_va, uint8_t cmd_byte) {
    (void)p7501_j_imm32(jb, 0xDCu, bus_va);
    const uint8_t wr[5] = { 0x80u, cmd_byte, 0x14u, 0xF0u, 0x70u }; /* mov cmd,d0; extbu d0; movhu d0,(a0) */
    p7501_j_arr(jb, wr, sizeof(wr));
}

size_t p7501_j_emit_status_msg(p7501_jedec_buf_t *jb, size_t *s_patch, size_t *e_patch) {
    *s_patch = p7501_j_imm32(jb, 0xDCu, 0u);
    *e_patch = p7501_j_imm32(jb, 0xDDu, 0u);
    const size_t c_off = jb->pos;
    static const uint8_t k_call[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    p7501_j_arr(jb, k_call, sizeof(k_call));
    return c_off;
}

void p7501_j_br(p7501_jedec_buf_t *jb, uint8_t op, size_t target_off) {
    const size_t br_off = jb->pos;
    p7501_j_u8(jb, op);
    p7501_j_u8(jb, 0u);
    p7501_j_patch_s8(jb, br_off, target_off);
}

static size_t build_jedec_checked(
    uint32_t flash_size_bytes,
    bool is_erase,
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    const uint32_t span = is_erase ? 1UL : (uint32_t)len;
    if ((out_buf == NULL) || (load_addr > 0xFFF00000UL) ||
        (!is_erase && ((data == NULL) || (len == 0u) || (len > P7501_JEDEC_MAX_DATA_LEN))) ||
        !p7501_flash_is_readonly_nor_range(eff_size, target_addr, span)) {
        return 0u;
    }
    /* Defense-in-depth: enforce authoritative AM34 bus & 3-layer NOR write policy. */
    if (p7501_bus_check_access(
            p7501_session_default_ctx(),
            eff_size,
            target_addr,
            (size_t)span,
            true,
            is_erase,
            p7501_flash_is_upper_nor_unlocked()
        ) != P7501_BUS_ALLOW) {
        return 0u;
    }
    return p7501_j_build_stub_common(
        is_erase,
        p7501_flash_normalize_uncached_addr(target_addr),
        0u,
        1u,
        data,
        len,
        load_addr,
        out_buf,
        out_cap
    );
}

size_t p7501_jedec_build_erase_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t target_addr,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return build_jedec_checked(
        flash_size_bytes, true, target_addr, NULL, 0u, load_addr, out_buf, out_cap
    );
}

size_t p7501_jedec_build_erase_stub(
    uint32_t target_addr,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return p7501_jedec_build_erase_stub_for_flash(
        p7501_flash_get_active_size(), target_addr, load_addr, out_buf, out_cap
    );
}

size_t p7501_jedec_build_erase_range_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t start_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    const uint32_t eff_size = p7501_flash_normalize_size(flash_size_bytes);
    if ((out_buf == NULL) || (load_addr > 0xFFF00000UL) ||
        (sector_count == 0u) || (sector_count > 32u)) {
        return 0u;
    }
    if ((sector_count > 1u) && (sector_step < 4096u)) {
        return 0u;
    }
    const uint64_t span = (uint64_t)sector_step * (uint64_t)sector_count;
    if (span > 0x01000000ULL) {
        return 0u;
    }
    uint32_t first_norm = 0u;
    for (uint32_t s = 0u; s < sector_count; ++s) {
        const uint64_t addr64 = (uint64_t)start_addr + ((uint64_t)s * (uint64_t)sector_step);
        if (addr64 > 0xFFFFFFFFULL) {
            return 0u;
        }
        uint32_t sec_norm = 0u;
        if (!p7501_jedec_validate_erase_addr_for_flash(eff_size, (uint32_t)addr64, false, &sec_norm)) {
            return 0u;
        }
        if (s == 0u) {
            first_norm = sec_norm;
        } else if (((uint64_t)first_norm + ((uint64_t)s * (uint64_t)sector_step)) != (uint64_t)sec_norm) {
            return 0u;
        }
    }
    if (((uint64_t)first_norm + span) > 0xFFFFFFFFULL) {
        return 0u;
    }
    return p7501_j_build_stub_common(
        true,
        first_norm,
        sector_step,
        sector_count,
        NULL,
        0u,
        load_addr,
        out_buf,
        out_cap
    );
}

size_t p7501_jedec_build_erase_range_stub(
    uint32_t start_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return p7501_jedec_build_erase_range_stub_for_flash(
        p7501_flash_get_active_size(),
        start_addr,
        sector_step,
        sector_count,
        load_addr,
        out_buf,
        out_cap
    );
}

size_t p7501_jedec_build_prog_stub_for_flash(
    uint32_t flash_size_bytes,
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return build_jedec_checked(
        flash_size_bytes, false, target_addr, data, len, load_addr, out_buf, out_cap
    );
}

size_t p7501_jedec_build_prog_stub(
    uint32_t target_addr,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
) {
    return p7501_jedec_build_prog_stub_for_flash(
        p7501_flash_get_active_size(), target_addr, data, len, load_addr, out_buf, out_cap
    );
}
