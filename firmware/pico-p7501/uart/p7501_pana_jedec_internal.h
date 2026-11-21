/**
 * @file p7501_pana_jedec_internal.h
 * @brief Internal shared MN103S stub emitter primitives for JEDEC erase/prog & autoselect/CFI builders.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PANA_JEDEC_INTERNAL_H
#define P7501_PANA_JEDEC_INTERNAL_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct p7501_jedec_buf {
    uint8_t *out;
    size_t pos;
    size_t cap;
} p7501_jedec_buf_t;

void p7501_j_u8(p7501_jedec_buf_t *jb, uint8_t v);
void p7501_j_arr(p7501_jedec_buf_t *jb, const uint8_t *src, size_t n);
size_t p7501_j_imm32(p7501_jedec_buf_t *jb, uint8_t sub_op, uint32_t imm);
void p7501_j_patch_s8(p7501_jedec_buf_t *jb, size_t br_off, size_t target_off);
void p7501_j_patch_s16(p7501_jedec_buf_t *jb, size_t insn_off, size_t target_off);
void p7501_j_patch_u32(p7501_jedec_buf_t *jb, size_t lit_off, uint32_t val);
void p7501_j_cmd_a0(p7501_jedec_buf_t *jb, uint32_t bus_va, uint8_t cmd_byte);
size_t p7501_j_emit_status_msg(p7501_jedec_buf_t *jb, size_t *s_patch, size_t *e_patch);
void p7501_j_br(p7501_jedec_buf_t *jb, uint8_t op, size_t target_off);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_JEDEC_INTERNAL_H */
