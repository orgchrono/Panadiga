/**
 * @file p7501_mn103_asm.c
 * @brief Pure host-testable deterministic two-way Matsushita MN103S assembler (`ASM`).
 *
 * Inverse of `p7501_mn103_decode_insn` (`diag/p7501_mn103_disasm.c`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_math_gps.h"

#include "common/p7501_hex.h"

#include <stdio.h>
#include <string.h>

static void asm_u16_le(uint8_t *dst, uint16_t v) {
    dst[0] = (uint8_t)(v & 0xFFu);
    dst[1] = (uint8_t)((v >> 8u) & 0xFFu);
}

static bool parse_i64_auto(const char *s, int64_t *out) {
    if ((s == NULL) || (*s == '\0') || (out == NULL)) {
        return false;
    }
    bool neg = false;
    if (*s == '-') {
        neg = true;
        ++s;
    }
    uint32_t u = 0u;
    if (!p7501_parse_u32_auto(s, &u)) {
        return false;
    }
    *out = neg ? -(int64_t)u : (int64_t)u;
    return true;
}

static bool parse_reg(const char *s, uint8_t *out_reg) {
    static const char *const k_regs[9] = {
        "D0", "D1", "D2", "D3", "A0", "A1", "A2", "A3", "SP"
    };
    if ((s == NULL) || (out_reg == NULL)) {
        return false;
    }
    for (size_t i = 0u; i < 9u; ++i) {
        if (p7501_ascii_ieq(s, k_regs[i])) {
            *out_reg = (uint8_t)i;
            return true;
        }
    }
    return false;
}

static bool parse_mov(
    const char *op,
    const char *t1,
    const char *t2,
    uint8_t out[8],
    uint8_t *out_len
) {
    int64_t imm = 0;
    uint8_t r = 0u;
    if (!parse_i64_auto(t1, &imm) || !parse_reg(t2, &r) || (r >= 8u)) {
        return false;
    }
    const bool is_a = (r >= 4u);
    const uint8_t idx = (uint8_t)(r & 3u);
    if (!is_a && (p7501_ascii_ieq(op, "MOV8") ||
        (p7501_ascii_ieq(op, "MOV") && (imm >= -128) && (imm <= 127)))) {
        out[0] = (uint8_t)(0x80u | (idx << 2u) | idx);
        out[1] = (uint8_t)(int8_t)imm;
        *out_len = 2u;
        return true;
    }
    if (p7501_ascii_ieq(op, "MOV16") ||
        (p7501_ascii_ieq(op, "MOV") &&
         (is_a ? ((imm >= 0) && (imm <= 65535)) : ((imm >= -32768) && (imm <= 32767))))) {
        out[0] = (uint8_t)((is_a ? 0x24u : 0x2Cu) | idx);
        asm_u16_le(&out[1], (uint16_t)(int16_t)imm);
        *out_len = 3u;
        return true;
    }
    out[0] = 0xFCu;
    out[1] = (uint8_t)((is_a ? 0xDCu : 0xCCu) | idx);
    p7501_write_u32_le(&out[2], (uint32_t)imm);
    *out_len = 6u;
    return true;
}

bool p7501_mn103_assemble_insn(
    const char *text,
    uint32_t pc_va,
    uint8_t out_bytes[8],
    uint8_t *out_len
) {
    if ((text == NULL) || (out_bytes == NULL) || (out_len == NULL)) {
        return false;
    }
    char buf[96];
    size_t bi = 0u;
    while ((text[bi] != '\0') && ((bi + 1u) < sizeof(buf))) {
        buf[bi] = (text[bi] == ',') ? ' ' : text[bi];
        ++bi;
    }
    buf[bi] = '\0';

    char *toks[4] = { NULL, NULL, NULL, NULL };
    const size_t n = p7501_split_ws_tokens(buf, toks, 4u);
    if (n == 0u) {
        return false;
    }
    const char *const op = toks[0];
    if (p7501_ascii_ieq(op, "NOP") && (n == 1u)) {
        out_bytes[0] = 0xCBu;
        *out_len = 1u;
        return true;
    }
    if ((p7501_ascii_ieq(op, "RETS") || p7501_ascii_ieq(op, "RTI")) && (n == 1u)) {
        out_bytes[0] = 0xF0u;
        out_bytes[1] = p7501_ascii_ieq(op, "RETS") ? 0xFCu : 0xFDu;
        *out_len = 2u;
        return true;
    }
    if ((p7501_ascii_ieq(op, "RET") || p7501_ascii_ieq(op, "RETF")) && (n == 1u)) {
        out_bytes[0] = p7501_ascii_ieq(op, "RET") ? 0xDFu : 0xDEu;
        out_bytes[1] = 0x00u;
        out_bytes[2] = 0x00u;
        *out_len = 3u;
        return true;
    }
    if (p7501_ascii_ieq(op, "CLR") && (n == 2u)) {
        uint8_t r = 0u;
        if (!parse_reg(toks[1], &r) || (r >= 4u)) {
            return false;
        }
        out_bytes[0] = (uint8_t)(r << 2u);
        *out_len = 1u;
        return true;
    }
    if ((n == 3u) && (p7501_ascii_ieq(op, "MOV") || p7501_ascii_ieq(op, "MOV8") ||
                     p7501_ascii_ieq(op, "MOV16") || p7501_ascii_ieq(op, "MOV32"))) {
        return parse_mov(op, toks[1], toks[2], out_bytes, out_len);
    }
    if ((n == 3u) && p7501_ascii_ieq(op, "STORE") && p7501_ascii_ieq(toks[1], "D0")) {
        int64_t addr = 0;
        if (!parse_i64_auto(toks[2], &addr)) {
            return false;
        }
        out_bytes[0] = 0xFCu;
        out_bytes[1] = 0x81u;
        p7501_write_u32_le(&out_bytes[2], (uint32_t)addr);
        *out_len = 6u;
        return true;
    }
    if ((n == 3u) && p7501_ascii_ieq(op, "ADD") && p7501_ascii_ieq(toks[2], "SP")) {
        int64_t imm = 0;
        if (!parse_i64_auto(toks[1], &imm)) {
            return false;
        }
        if ((imm >= -128) && (imm <= 127)) {
            out_bytes[0] = 0xF8u;
            out_bytes[1] = 0xFEu;
            out_bytes[2] = (uint8_t)(int8_t)imm;
            *out_len = 3u;
            return true;
        }
        out_bytes[0] = 0xFCu;
        out_bytes[1] = 0xFEu;
        p7501_write_u32_le(&out_bytes[2], (uint32_t)imm);
        *out_len = 6u;
        return true;
    }
    if ((n == 2u) && (p7501_ascii_ieq(op, "MOVM_PUSH") || p7501_ascii_ieq(op, "MOVM_POP"))) {
        uint32_t mask = 0u;
        if (!p7501_parse_u32_auto(toks[1], &mask) || (mask > 0xFFu)) {
            return false;
        }
        out_bytes[0] = p7501_ascii_ieq(op, "MOVM_PUSH") ? 0xCFu : 0xCEu;
        out_bytes[1] = (uint8_t)mask;
        *out_len = 2u;
        return true;
    }
    if ((n == 2u) && (p7501_ascii_ieq(op, "BRA") || p7501_ascii_ieq(op, "JMP") ||
                     p7501_ascii_ieq(op, "JMP16") || p7501_ascii_ieq(op, "JMP32") ||
                     p7501_ascii_ieq(op, "CALL") || p7501_ascii_ieq(op, "CALL16") ||
                     p7501_ascii_ieq(op, "CALL32"))) {
        uint32_t target = 0u;
        if (!p7501_parse_u32_auto(toks[1], &target)) {
            return false;
        }
        const int32_t disp = (int32_t)(target - pc_va);
        if (p7501_ascii_ieq(op, "BRA")) {
            if ((disp < -128) || (disp > 127)) {
                return false;
            }
            out_bytes[0] = 0xCAu;
            out_bytes[1] = (uint8_t)(int8_t)disp;
            *out_len = 2u;
            return true;
        }
        const bool is_call = (op[0] == 'C' || op[0] == 'c');
        const bool force_32 = (strlen(op) >= 5u) && (op[strlen(op) - 2u] == '3');
        const bool force_16 = (strlen(op) >= 5u) && (op[strlen(op) - 2u] == '1');
        if (!force_32 && ((disp >= -32768) && (disp <= 32767))) {
            out_bytes[0] = is_call ? 0xCDu : 0xCCu;
            asm_u16_le(&out_bytes[1], (uint16_t)(int16_t)disp);
            if (is_call) {
                out_bytes[3] = 0x00u;
                out_bytes[4] = 0x00u;
                *out_len = 5u;
            } else {
                *out_len = 3u;
            }
            return true;
        }
        if (force_16) {
            return false;
        }
        out_bytes[0] = is_call ? 0xDDu : 0xDCu;
        p7501_write_u32_le(&out_bytes[1], (uint32_t)disp);
        if (is_call) {
            out_bytes[5] = 0x00u;
            out_bytes[6] = 0x00u;
            *out_len = 7u;
        } else {
            *out_len = 5u;
        }
        return true;
    }
    return false;
}
