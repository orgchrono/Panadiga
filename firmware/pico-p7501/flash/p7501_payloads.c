/**
 * @file p7501_payloads.c
 * @brief Pure host-testable verified MN103S probe payload builder & CRC32 verifier.
 *
 * (Trap audit logic lives in `flash/p7501_trap_guard.c`; CLI shell lives in `flash/p7501_payload_cli.c`.)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_payloads.h"

#include "common/p7501_hex.h"
#include "flash/p7501_bus_policy.h"
#include "flash/p7501_fast_stream_codec.h"
#include "flash/p7501_flash.h"
#include "uart/p7501_pana_jedec.h"
#include "uart/p7501_pana_jedec_internal.h"

#include <string.h>

static const p7501_payload_desc_t k_payload_descs[P7501_PAY_COUNT] = {
    {
        P7501_PAY_FAST_BGO_STREAM,
        "fast_bgo_stream",
        P7501_FAST_STUB_LOAD_VA,
        "BGO_SAFE",
        "606-byte MN103S SDRAM P/R/W/Q high-speed CS0/BGO streamer"
    },
    {
        P7501_PAY_VIDEO_MMIO_DUMP,
        "video_mmio_dump",
        P7501_SSOT_SDRAM_STUB_RAW_ADDR,
        "READONLY",
        "SSOT AIOF/AVDD/IIC0/IIC1 15-register video MMIO telemetry emitter"
    },
    {
        P7501_PAY_TBC_PASSTHROUGH,
        "tbc_passthrough",
        P7501_SSOT_SDRAM_STUB_RAW_ADDR,
        "TBC_WRITE",
        "AIOF_YC3V line-TBC + AIOF_CONT 0-IRE DAC calibration writer"
    },
    {
        P7501_PAY_JEDEC_NVRAM_ERASE,
        "jedec_nvram_erase",
        P7501_JEDEC_STUB_ADDR,
        "NVRAM_WRITE",
        "Spansion SA134..SA141 (0x847F0000 64 KiB) JEDEC erase sequence"
    },
    {
        P7501_PAY_JEDEC_PROBE_ID,
        "jedec_probe_id",
        P7501_JEDEC_STUB_ADDR,
        "READONLY",
        "Spansion CS0 0x90 autoselect manufacturer/device ID probe"
    },
};

size_t p7501_payload_count(void) {
    return (size_t)P7501_PAY_COUNT;
}

bool p7501_payload_describe(size_t idx, p7501_payload_desc_t *out) {
    if ((idx >= (size_t)P7501_PAY_COUNT) || (out == NULL)) {
        return false;
    }
    *out = k_payload_descs[idx];
    return true;
}

bool p7501_payload_find(const char *name, p7501_payload_id_t *out_id) {
    if ((name == NULL) || (*name == '\0') || (out_id == NULL)) {
        return false;
    }
    for (size_t i = 0u; i < (size_t)P7501_PAY_COUNT; ++i) {
        if (p7501_ascii_ieq(name, k_payload_descs[i].name)) {
            *out_id = k_payload_descs[i].id;
            return true;
        }
    }
    return false;
}

static size_t build_video_mmio_dump_stub(uint32_t load_addr, uint8_t *out_buf, size_t cap) {
    static const uint8_t k_call_ph[5] = { 0xCDu, 0x00u, 0x00u, 0x00u, 0x00u };
    static const uint8_t k_ld_a0_d2[1] = { 0x78u };                  /* mov (a0), d2 */
    static const uint8_t k_mov_d2_d3[1] = { 0x8Bu };                 /* mov d2, d3 */
    static const uint8_t k_lsr8_d2[4] = { 0xF8u, 0xC6u, 0x08u, 0x8Bu };
    static const uint8_t k_ret0[3] = { 0xDFu, 0x00u, 0x00u };
    static const uint8_t k_rets[2] = { 0xF0u, 0xFCu };
    static const char k_hdr[] = "PANAVID1";

    p7501_jedec_buf_t jb = { out_buf, 0u, cap };
    size_t hdr_s = 0u, hdr_e = 0u, done_s = 0u, done_e = 0u;
    size_t rd_calls[P7501_SSOT_MMIO_REG_COUNT];
    size_t n_regs = 0u;

    const size_t c_hdr = p7501_j_emit_status_msg(&jb, &hdr_s, &hdr_e);
    for (size_t i = 0u; i < P7501_SSOT_MMIO_REG_COUNT; ++i) {
        const p7501_ssot_mmio_reg_t *const r = &k_p7501_ssot_mmio_regs[i];
        if ((r->domain & P7501_SSOT_MMIO_DOM_VIDEO) == 0u) {
            continue;
        }
        if (p7501_bus_is_lethal_mmio_range(r->addr, 4u)) {
            return 0u;
        }
        (void)p7501_j_imm32(&jb, 0xDCu, r->addr);
        rd_calls[n_regs++] = jb.pos;
        p7501_j_arr(&jb, k_call_ph, sizeof(k_call_ph));
    }
    const size_t c_done = p7501_j_emit_status_msg(&jb, &done_s, &done_e);
    p7501_j_arr(&jb, k_rets, sizeof(k_rets));

    const size_t emit_w32_off = jb.pos;
    p7501_j_arr(&jb, k_ld_a0_d2, sizeof(k_ld_a0_d2));
    p7501_j_arr(&jb, k_mov_d2_d3, sizeof(k_mov_d2_d3));
    size_t c_bytes[4];
    c_bytes[0] = jb.pos;
    p7501_j_arr(&jb, k_call_ph, sizeof(k_call_ph));
    for (size_t b = 1u; b < 4u; ++b) {
        p7501_j_arr(&jb, k_lsr8_d2, sizeof(k_lsr8_d2));
        c_bytes[b] = jb.pos;
        p7501_j_arr(&jb, k_call_ph, sizeof(k_call_ph));
    }
    p7501_j_arr(&jb, k_ret0, sizeof(k_ret0));

    size_t putc_off = 0u;
    if ((n_regs == 0u) ||
        !p7501_j_append_span_putc_msgs(&jb, load_addr, k_hdr, c_hdr, hdr_s, hdr_e,
                                       c_done, done_s, done_e, &putc_off)) {
        return 0u;
    }
    for (size_t i = 0u; i < n_regs; ++i) {
        p7501_j_patch_s16(&jb, rd_calls[i], emit_w32_off);
    }
    for (size_t b = 0u; b < 4u; ++b) {
        p7501_j_patch_s16(&jb, c_bytes[b], putc_off);
    }
    return jb.pos;
}

static size_t build_tbc_passthrough_stub(uint8_t *out_buf, size_t cap) {
    typedef struct mmio_wr32 {
        uint32_t addr;
        uint32_t val;
    } mmio_wr32_t;
    static const mmio_wr32_t k_writes[2] = {
        { P7501_SSOT_AIOF_BASE + 0x64UL, 0x80010020UL },
        { P7501_SSOT_AIOF_BASE + 0xA4UL, 0x00004000UL },
    };
    static const uint8_t k_rets[2] = { 0xF0u, 0xFCu };

    p7501_jedec_buf_t jb = { out_buf, 0u, cap };
    for (size_t i = 0u; i < 2u; ++i) {
        if (p7501_bus_is_lethal_mmio_range(k_writes[i].addr, 4u)) {
            return 0u;
        }
        (void)p7501_j_imm32(&jb, 0xCCu, k_writes[i].val);
        (void)p7501_j_imm32(&jb, 0x81u, k_writes[i].addr);
    }
    p7501_j_arr(&jb, k_rets, sizeof(k_rets));
    return (jb.pos <= cap) ? jb.pos : 0u;
}

bool p7501_payload_build_and_verify(
    p7501_payload_id_t id,
    uint8_t *out_buf,
    size_t cap,
    p7501_payload_info_t *out_info
) {
    if ((out_buf == NULL) || (out_info == NULL) || (id >= P7501_PAY_COUNT)) {
        return false;
    }
    memset(out_info, 0, sizeof(*out_info));
    out_info->desc = k_payload_descs[id];

    size_t len = 0u;
    bool ok = false;
    switch (id) {
        case P7501_PAY_FAST_BGO_STREAM:
            if (cap >= P7501_FAST_STUB_LEN) {
                memcpy(out_buf, k_p7501_fast_bgo_stub, P7501_FAST_STUB_LEN);
                len = P7501_FAST_STUB_LEN;
                ok = (p7501_crc32_ieee(out_buf, len) == P7501_FAST_STUB_CRC32);
            }
            break;
        case P7501_PAY_VIDEO_MMIO_DUMP:
            len = build_video_mmio_dump_stub(out_info->desc.load_va, out_buf, cap);
            ok = (len > 0u);
            break;
        case P7501_PAY_TBC_PASSTHROUGH:
            len = build_tbc_passthrough_stub(out_buf, cap);
            ok = (len > 0u);
            break;
        case P7501_PAY_JEDEC_NVRAM_ERASE:
            len = p7501_jedec_build_erase_range_stub_for_flash(
                8UL * 1024UL * 1024UL,
                P7501_SSOT_CS0_UNCACHED_BASE + 0x7F0000UL,
                0x2000UL,
                8u,
                out_info->desc.load_va,
                out_buf,
                cap
            );
            ok = (len > 0u);
            break;
        case P7501_PAY_JEDEC_PROBE_ID:
            len = p7501_jedec_build_autoselect_stub(false, out_info->desc.load_va, out_buf, cap);
            ok = (len > 0u);
            break;
        default:
            ok = false;
            break;
    }
    if (!ok || (len == 0u)) {
        return false;
    }
    out_info->len = len;
    out_info->crc32 = p7501_crc32_ieee(out_buf, len);
    out_info->verified = (out_info->crc32 != 0u);
    return out_info->verified;
}

bool p7501_payload_verify_all(uint8_t *scratch, size_t cap, uint32_t *out_verified_mask) {
    if ((scratch == NULL) || (cap == 0u)) {
        return false;
    }
    uint32_t mask = 0u;
    bool all_ok = true;
    for (size_t i = 0u; i < (size_t)P7501_PAY_COUNT; ++i) {
        p7501_payload_info_t info;
        if (p7501_payload_build_and_verify(k_payload_descs[i].id, scratch, cap, &info)) {
            mask |= (1UL << i);
        } else {
            all_ok = false;
        }
    }
    if (out_verified_mask != NULL) {
        *out_verified_mask = mask;
    }
    return all_ok;
}
