/**
 * @file picofw_pana_rsp_guard.c
 * @brief Adversarial E05, Lethal MMIO (`0xC000100C`), and F14 Wireless guards for `PANARSP RAW:/HEX:`.
 *
 * Extracted from `picofw_pana_rsp_parse.c` so raw byte/text security inspection
 * lives in one dedicated, host-testable gate backed by `picofw_bus_check_access()`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_pana_rsp_internal.h"
#include "flash/picofw_flash.h"
#include "common/picofw_hex.h"
#include "hal/picofw_wireless_codec.h"

static bool fail_e05(bool *out_e05) {
    if (out_e05 != NULL) {
        *out_e05 = true;
    }
    return false;
}

static bool check_rsp_mem_op_bytes(
    const uint8_t *buf,
    size_t len,
    size_t op_idx,
    bool is_write,
    bool eff_unlocked,
    bool strict_syntax,
    bool *out_e05
) {
    uint32_t addr = 0u;
    const size_t addr_pos = op_idx + 1u;
    const size_t addr_chars = picofw_scan_hex_u32((const char *)buf, len, addr_pos, &addr);
    if (addr_chars == 0u) {
        /* Fail closed if hex digits follow (e.g. >8-digit overflow attack `M080000000,4:...`). */
        if ((addr_pos < len) && (picofw_hex_char_to_nibble((char)buf[addr_pos]) >= 0)) {
            return fail_e05(out_e05);
        }
        return !strict_syntax;
    }
    const size_t comma_idx = addr_pos + addr_chars;
    if ((comma_idx >= len) || (buf[comma_idx] != (uint8_t)',')) {
        return !strict_syntax;
    }
    uint32_t op_len = 0u;
    const size_t len_pos = comma_idx + 1u;
    const size_t len_chars = picofw_scan_hex_u32((const char *)buf, len, len_pos, &op_len);
    const size_t colon_idx = len_pos + len_chars;
    if (len_chars == 0u) {
        if ((len_pos < len) && (picofw_hex_char_to_nibble((char)buf[len_pos]) >= 0)) {
            return fail_e05(out_e05);
        }
        return !strict_syntax;
    }
    if ((op_len == 0u) ||
        (is_write && strict_syntax && ((colon_idx >= len) || (buf[colon_idx] != (uint8_t)':')))) {
        return !strict_syntax;
    }
    const picofw_bus_deny_t deny = picofw_bus_check_access(
        picofw_session_default_ctx(),
        picofw_flash_get_active_size(),
        addr,
        (size_t)op_len,
        is_write,
        false,
        eff_unlocked
    );
    if (deny != PICOFW_BUS_ALLOW) {
        return fail_e05(out_e05);
    }
    return true;
}

static bool check_panamon_line_op(
    const uint8_t *buf,
    size_t len,
    size_t op_idx,
    bool eff_unlocked,
    bool *out_e05
) {
    const uint8_t op = buf[op_idx];
    const bool is_write = (op == (uint8_t)'W') || (op == (uint8_t)'w');
    const bool is_erase = (op == (uint8_t)'E') || (op == (uint8_t)'e');
    const bool is_read = (op == (uint8_t)'R') || (op == (uint8_t)'r');
    const bool is_jump = (op == (uint8_t)'J') || (op == (uint8_t)'j');
    const bool is_format = (op == (uint8_t)'F') || (op == (uint8_t)'f');
    const bool f14_ok = picofw_wireless_nor_write_allowed(picofw_wireless_global_state());

    if (is_format) {
        if (!eff_unlocked || !f14_ok) {
            return fail_e05(out_e05);
        }
        return true;
    }
    if (!is_write && !is_erase && !is_read && !is_jump) {
        return true;
    }

    size_t p = op_idx + 1u;
    while ((p < len) && ((buf[p] == (uint8_t)' ') || (buf[p] == (uint8_t)'\t'))) {
        ++p;
    }
    if ((p >= len) || (buf[p] == (uint8_t)'\r') || (buf[p] == (uint8_t)'\n') || (buf[p] == (uint8_t)';')) {
        /* D1: In PANARSP RAW:/HEX:, W/E/J without an explicit hex address are strictly rejected (E05)
         * even when Upper NOR is unlocked, preventing multi-frame or unvalidated IPL SA0..SA10 erases. */
        if (is_write || is_erase || is_jump) {
            return fail_e05(out_e05);
        }
        return true;
    }
    uint32_t addr = 0u;
    const size_t a_len = picofw_scan_hex_u32((const char *)buf, len, p, &addr);
    if (a_len == 0u) {
        if ((picofw_hex_char_to_nibble((char)buf[p]) >= 0) || is_write || is_erase || is_jump) {
            return fail_e05(out_e05);
        }
        return true;
    }

    if (is_jump) {
        if (!f14_ok || picofw_bus_is_lethal_mmio_range(addr, 4u)) {
            return fail_e05(out_e05);
        }
        return true;
    }

    uint32_t span = 1u;
    const size_t after = p + a_len;
    if (is_read && (after < len) && (buf[after] == (uint8_t)',')) {
        uint32_t rlen = 0u;
        const size_t rchars = picofw_scan_hex_u32((const char *)buf, len, after + 1u, &rlen);
        if (rchars == 0u) {
            return fail_e05(out_e05);
        }
        if (rlen > 0u) {
            span = rlen;
        }
    }
    const picofw_bus_deny_t deny = picofw_bus_check_access(
        picofw_session_default_ctx(),
        picofw_flash_get_active_size(),
        addr,
        (size_t)span,
        is_write || is_erase,
        is_erase,
        eff_unlocked
    );
    if (deny != PICOFW_BUS_ALLOW) {
        return fail_e05(out_e05);
    }
    return true;
}

bool picofw_rsp_validate_raw_or_hex_guard(
    const uint8_t *buf,
    size_t len,
    bool eff_unlocked,
    bool is_raw_text,
    bool *out_e05
) {
    if ((buf == NULL) || (len == 0u)) {
        return true;
    }
    if (is_raw_text) {
        size_t start = 0u;
        while ((start < len) && ((buf[start] == (uint8_t)' ') || (buf[start] == (uint8_t)'\t'))) {
            ++start;
        }
        if ((start < len) && (buf[start] == (uint8_t)'$')) {
            ++start;
        }
        if (start < len) {
            const uint8_t op = buf[start];
            if ((op == (uint8_t)'M') || (op == (uint8_t)'X')) {
                if (!check_rsp_mem_op_bytes(buf, len, start, true, eff_unlocked, true, out_e05)) {
                    return false;
                }
            } else if ((op == (uint8_t)'m') || (op == (uint8_t)'x')) {
                if (!check_rsp_mem_op_bytes(buf, len, start, false, eff_unlocked, true, out_e05)) {
                    return false;
                }
            }
        }
    }
    bool at_line_start = true;
    for (size_t i = 0u; i < len; ++i) {
        const uint8_t op = buf[i];
        if ((op == (uint8_t)'\r') || (op == (uint8_t)'\n') || (op == (uint8_t)';')) {
            at_line_start = true;
            continue;
        }
        if (at_line_start && ((op == (uint8_t)' ') || (op == (uint8_t)'\t'))) {
            continue;
        }
        const bool after_dollar = (i > 0u) && (buf[i - 1u] == (uint8_t)'$');
        const bool at_cmd_start = at_line_start || after_dollar;
        at_line_start = false;
        if (!at_cmd_start) {
            continue;
        }
        if ((op == (uint8_t)'M') || (op == (uint8_t)'X')) {
            if (!check_rsp_mem_op_bytes(buf, len, i, true, eff_unlocked, after_dollar, out_e05)) {
                return false;
            }
        } else if ((op == (uint8_t)'m') || (op == (uint8_t)'x')) {
            if (!check_rsp_mem_op_bytes(buf, len, i, false, eff_unlocked, after_dollar, out_e05)) {
                return false;
            }
        } else {
            if (!check_panamon_line_op(buf, len, i, eff_unlocked, out_e05)) {
                return false;
            }
        }
    }
    return true;
}
