/**
 * @file picofw_mn103_disasm.c
 * @brief Pure Matsushita MN103S prologue, CFG edge scanner & compact table-driven disassembler.
 *
 * (Paired two-way encoder lives in `diag/picofw_mn103_asm.c`.)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_math_gps.h"

#include "common/picofw_hex.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static const char *const k_reg_names[9] = {
    "D0", "D1", "D2", "D3", "A0", "A1", "A2", "A3", "SP"
};

static uint16_t read_u16_le(const uint8_t *p) {
    return (uint16_t)p[0] | (uint16_t)((uint16_t)p[1] << 8u);
}

bool picofw_mn103_detect_prologue(const uint8_t *buf, size_t len, size_t off, const char **out_kind) {
    if ((buf == NULL) || (off >= len)) {
        return false;
    }
    const size_t rem = len - off;
    if ((rem >= 2u) && (buf[off] == 0xCFu) && (buf[off + 1u] != 0x00u)) {
        if (out_kind != NULL) { *out_kind = "MOVM_SP"; }
        return true;
    }
    if ((rem >= 3u) && (buf[off] == 0xF8u) && (buf[off + 1u] == 0xFEu) && ((buf[off + 2u] & 0x80u) != 0u)) {
        if (out_kind != NULL) { *out_kind = "ADD_IMM8_SP"; }
        return true;
    }
    if ((rem >= 4u) && (buf[off] == 0xFAu) && (buf[off + 1u] == 0xFEu) && ((buf[off + 3u] & 0x80u) != 0u)) {
        if (out_kind != NULL) { *out_kind = "ADD_IMM16_SP"; }
        return true;
    }
    if ((rem >= 6u) && (buf[off] == 0xFCu) && (buf[off + 1u] == 0xFEu) && ((buf[off + 5u] & 0x80u) != 0u)) {
        if (out_kind != NULL) { *out_kind = "ADD_IMM32_SP"; }
        return true;
    }
    return false;
}

static bool is_valid_mn103_code_va(uint32_t va) {
    const uint32_t seg = va >> 24u;
    return (seg == 0x44u) || (seg == 0x50u) || (seg == 0x57u) ||
           (seg == 0x80u) || (seg == 0x84u) || (seg == 0x90u);
}

static bool is_am34_mmio_va(uint32_t va) {
    const uint32_t seg = va >> 24u;
    const uint32_t hi16 = va >> 16u;
    return (seg == 0x88u) || (seg == 0x89u) || (hi16 == 0xC000u) ||
           (hi16 == 0xD400u) || (hi16 == 0xD840u) || (hi16 == 0xDC01u);
}

static void record_edge(
    picofw_mn103_edge_t *out_edges,
    size_t max_edges,
    size_t idx,
    picofw_mn103_edge_kind_t kind,
    uint32_t site,
    uint32_t target,
    const char *label
) {
    if ((out_edges != NULL) && (idx < max_edges)) {
        out_edges[idx] = (picofw_mn103_edge_t){ kind, site, target, label };
    }
}

size_t picofw_mn103_scan_edges(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    picofw_mn103_edge_t *out_edges,
    size_t max_edges
) {
    if ((buf == NULL) || (len < 2u)) {
        return 0u;
    }
    size_t found = 0u;
    for (size_t off = 0u; off < len; ) {
        const char *pkind = NULL;
        const uint32_t site_va = base_va + (uint32_t)off;
        if (picofw_mn103_detect_prologue(buf, len, off, &pkind)) {
            record_edge(out_edges, max_edges, found++, PICOFW_EDGE_PROLOGUE, site_va, site_va, pkind);
            off += 2u;
            continue;
        }
        if ((buf[off] == 0xCDu) && ((off + 5u) <= len)) {
            const uint32_t target = (uint32_t)((int32_t)site_va + (int32_t)(int16_t)read_u16_le(&buf[off + 1u]));
            if (is_valid_mn103_code_va(target)) {
                record_edge(out_edges, max_edges, found++, PICOFW_EDGE_CALL, site_va, target, "CALL16");
                off += 5u;
                continue;
            }
        } else if ((buf[off] == 0xDDu) && ((off + 7u) <= len)) {
            const uint32_t target = (uint32_t)((int32_t)site_va + (int32_t)picofw_read_u32_le(&buf[off + 1u]));
            if (is_valid_mn103_code_va(target)) {
                record_edge(out_edges, max_edges, found++, PICOFW_EDGE_CALL, site_va, target, "CALL32");
                off += 7u;
                continue;
            }
        } else if ((buf[off] == 0xFCu) && ((off + 6u) <= len)) {
            const uint32_t imm = picofw_read_u32_le(&buf[off + 2u]);
            if (is_am34_mmio_va(imm)) {
                record_edge(out_edges, max_edges, found++, PICOFW_EDGE_MMIO, site_va, imm, "FC_MMIO");
                off += 6u;
                continue;
            }
        }
        off += 1u;
    }
    return found;
}

static bool decode_prefixed(const uint8_t *buf, size_t len, picofw_mn103_insn_info_t *out) {
    const uint8_t op = buf[0];
    const uint8_t plen = (op <= 0xF5u) ? 2u
        : (((op == 0xF8u) || (op == 0xF9u)) ? 3u
        : (((op == 0xFAu) || (op == 0xFBu)) ? 4u : ((op <= 0xFEu) ? 6u : 0u)));
    if ((plen == 0u) || (len < plen)) {
        return false;
    }
    out->len = plen;
    out->valid = true;
    const uint8_t sub = buf[1];
    if ((op == 0xF0u) && ((sub == 0xFCu) || (sub == 0xFDu))) {
        out->is_ret = true;
        (void)snprintf(out->mnemonic, sizeof(out->mnemonic), (sub == 0xFCu) ? "RETS" : "RTI");
        return true;
    }
    if ((op == 0xF8u) && (sub == 0xFEu)) {
        out->imm_kind = PICOFW_IMM_U8;
        out->imm_off = 2u;
        out->imm_val = buf[2];
        out->reg_idx = 8u;
        (void)snprintf(out->mnemonic, sizeof(out->mnemonic), "ADD 0x%02X,SP", (unsigned)buf[2]);
        return true;
    }
    if ((op == 0xFCu) && (((sub >= 0xCCu) && (sub <= 0xDFu)) || (sub == 0xFEu) || (sub == 0x81u))) {
        out->imm_kind = PICOFW_IMM_U32;
        out->imm_off = 2u;
        out->imm_val = picofw_read_u32_le(&buf[2]);
        out->reg_idx = (sub == 0xFEu) ? 8u : (uint8_t)(((sub >= 0xDCu) ? 4u : 0u) + (sub & 3u));
        if (sub == 0x81u) {
            (void)snprintf(out->mnemonic, sizeof(out->mnemonic), "STORE D0,0x%08" PRIX32, out->imm_val);
        } else {
            (void)snprintf(out->mnemonic, sizeof(out->mnemonic), "%s 0x%08" PRIX32 ",%s",
                           (sub == 0xFEu) ? "ADD" : "MOV", out->imm_val, k_reg_names[out->reg_idx]);
        }
        return true;
    }
    (void)snprintf(out->mnemonic, sizeof(out->mnemonic), "OP_%02X_%02X", (unsigned)op, (unsigned)sub);
    return true;
}

bool picofw_mn103_decode_insn(
    const uint8_t *buf,
    size_t len,
    uint32_t pc_va,
    picofw_mn103_insn_info_t *out
) {
    if ((buf == NULL) || (len == 0u) || (out == NULL)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    const uint8_t op = buf[0];
    if (op >= 0xF0u) {
        return decode_prefixed(buf, len, out);
    }
    const uint8_t n = op & 3u;
    const uint8_t m = (op >> 2u) & 3u;
    if ((op >= 0x20u) && (op <= 0x2Fu)) {
        const bool is_16 = (op >= 0x24u && op <= 0x27u) || (op >= 0x2Cu);
        const uint8_t ilen = is_16 ? 3u : 2u;
        if (len < ilen) { return false; }
        const uint8_t r = (uint8_t)(((op < 0x28u) ? 4u : 0u) + n);
        out->len = ilen;
        out->valid = true;
        out->imm_off = 1u;
        out->reg_idx = r;
        out->imm_kind = is_16 ? PICOFW_IMM_U16 : PICOFW_IMM_U8;
        out->imm_val = is_16 ? (uint32_t)read_u16_le(&buf[1]) : (uint32_t)buf[1];
        (void)snprintf(out->mnemonic, sizeof(out->mnemonic),
                       (op & 0x0Cu) ? "MOV 0x%X,%s" : "ADD 0x%X,%s",
                       (unsigned)out->imm_val, k_reg_names[r]);
        return true;
    }
    if ((op >= 0x80u) && (op <= 0x8Fu) && (m == n)) {
        if (len < 2u) { return false; }
        out->len = 2u; out->valid = true; out->imm_kind = PICOFW_IMM_U8;
        out->imm_off = 1u; out->imm_val = buf[1]; out->reg_idx = n;
        (void)snprintf(out->mnemonic, sizeof(out->mnemonic), "MOV 0x%02X,D%u", (unsigned)buf[1], (unsigned)n);
        return true;
    }
    if (((op >= 0xC0u) && (op <= 0xCAu)) || (op == 0xCCu) || (op == 0xCDu) ||
        (op == 0xDCu) || (op == 0xDDu)) {
        const uint8_t ilen = (op <= 0xCAu) ? 2u : ((op == 0xCCu) ? 3u : ((op == 0xCDu || op == 0xDCu) ? 5u : 7u));
        if (len < ilen) { return false; }
        out->len = ilen; out->valid = true; out->is_rel_branch = true;
        const int32_t d = (ilen == 2u) ? (int32_t)(int8_t)buf[1]
            : ((ilen <= 3u || op == 0xCDu) ? (int32_t)(int16_t)read_u16_le(&buf[1])
            : (int32_t)picofw_read_u32_le(&buf[1]));
        const uint32_t dst = (uint32_t)((int32_t)pc_va + d);
        (void)snprintf(out->mnemonic, sizeof(out->mnemonic), "%s 0x%08" PRIX32,
                       (op == 0xCDu || op == 0xDDu) ? "CALL" : ((op == 0xCCu || op == 0xDCu) ? "JMP" : "BRA"), dst);
        return true;
    }
    if ((op == 0xCEu) || (op == 0xCFu) || (op == 0xDEu) || (op == 0xDFu)) {
        const uint8_t ilen = (op >= 0xDEu) ? 3u : 2u;
        if (len < ilen) { return false; }
        out->len = ilen; out->valid = true; out->is_ret = (op >= 0xDEu);
        (void)snprintf(out->mnemonic, sizeof(out->mnemonic),
                       (op == 0xDEu) ? "RETF" : ((op == 0xDFu) ? "RET" : ((op == 0xCFu) ? "MOVM_PUSH" : "MOVM_POP")));
        return true;
    }
    uint8_t clen = ((op <= 0x0Fu && (op & 3u) != 0u) || (op >= 0x30u && op <= 0x3Bu)) ? 3u : 1u;
    if ((((op & 0xF0u) == 0x40u) && ((op & 0x02u) != 0u)) ||
        (((op & 0xF0u) == 0x50u) && ((op & 0x08u) != 0u)) ||
        ((op >= 0x90u) && (op <= 0xBFu) && (m == n))) {
        clen = 2u;
    }
    if ((op >= 0xD0u) && (op <= 0xDAu)) { out->is_rel_branch = true; }
    if (len < clen) { return false; }
    out->len = clen; out->valid = true;
    (void)snprintf(out->mnemonic, sizeof(out->mnemonic),
                   (op == 0xCBu) ? "NOP" : ((op <= 0x0Cu && (op & 3u) == 0u) ? "CLR D%u" : "OP_%02X"),
                   (op <= 0x0Cu && (op & 3u) == 0u) ? (unsigned)m : (unsigned)op);
    return true;
}

bool picofw_mn103_cover_span(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    size_t min_bytes,
    size_t *out_cover,
    bool *out_reloc_ok
) {
    if ((buf == NULL) || (min_bytes == 0u) || (out_cover == NULL) || (out_reloc_ok == NULL)) {
        return false;
    }
    size_t pos = 0u;
    bool reloc = true;
    while ((pos < min_bytes) && (pos < len)) {
        picofw_mn103_insn_info_t insn;
        if (!picofw_mn103_decode_insn(&buf[pos], len - pos, base_va + (uint32_t)pos, &insn) || !insn.valid) {
            return false;
        }
        if (insn.is_rel_branch || insn.is_ret) {
            reloc = false;
        }
        pos += (size_t)insn.len;
    }
    if (pos < min_bytes) {
        return false;
    }
    *out_cover = pos;
    *out_reloc_ok = reloc;
    return true;
}
