/**
 * @file picofw_pana_jedec_internal.h
 * @brief Internal shared MN103S stub emitter primitives for JEDEC erase/prog & autoselect/CFI builders.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PANA_JEDEC_INTERNAL_H
#define PICOFW_PANA_JEDEC_INTERNAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct picofw_jedec_buf {
    uint8_t *out;
    size_t pos;
    size_t cap;
} picofw_jedec_buf_t;

void picofw_j_u8(picofw_jedec_buf_t *jb, uint8_t v);
void picofw_j_arr(picofw_jedec_buf_t *jb, const uint8_t *src, size_t n);
size_t picofw_j_imm32(picofw_jedec_buf_t *jb, uint8_t sub_op, uint32_t imm);
void picofw_j_patch_s8(picofw_jedec_buf_t *jb, size_t br_off, size_t target_off);
void picofw_j_patch_s16(picofw_jedec_buf_t *jb, size_t insn_off, size_t target_off);
void picofw_j_patch_u32(picofw_jedec_buf_t *jb, size_t lit_off, uint32_t val);
void picofw_j_cmd_a0(picofw_jedec_buf_t *jb, uint32_t bus_va, uint8_t cmd_byte);
size_t picofw_j_emit_status_msg(picofw_jedec_buf_t *jb, size_t *s_patch, size_t *e_patch);
void picofw_j_br(picofw_jedec_buf_t *jb, uint8_t op, size_t target_off);
bool picofw_j_append_span_putc_msgs(
    picofw_jedec_buf_t *jb,
    uint32_t load_addr,
    const char hdr8[8],
    size_t c_hdr,
    size_t hdr_s,
    size_t hdr_e,
    size_t c_done,
    size_t done_s,
    size_t done_e,
    size_t *out_putc_off
);
size_t picofw_j_build_stub_common(
    bool is_erase,
    uint32_t norm_addr,
    uint32_t sector_step,
    uint32_t sector_count,
    const uint8_t *data,
    size_t len,
    uint32_t load_addr,
    uint8_t *out_buf,
    size_t out_cap
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PANA_JEDEC_INTERNAL_H */
