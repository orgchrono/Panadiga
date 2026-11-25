/**
 * @file picofw_bus_tx_guard.c
 * @brief Fail-closed UART TX payload & buffered `BRIDGE-POLICY` filter (`A01` / `S04`).
 *
 * Extracted from `picofw_bus_policy.c` to enforce modular boundaries (<350 LOC),
 * block 9-digit hex MMIO bypasses, measure real `vFlashWrite:` byte spans, handle
 * compact `PANAMON1>` verbs, and buffer lines/RSP packets before SC0 emission.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_bus_policy.h"
#include "flash/picofw_flash.h"
#include "common/picofw_hex.h"

#include <string.h>

static char bus_ascii_upper(uint8_t c) {
    return (c >= (uint8_t)'a' && c <= (uint8_t)'z') ? (char)(c - ('a' - 'A')) : (char)c;
}

static bool bus_is_delim_char(uint8_t c) {
    return (c <= 0x20u) || (c == (uint8_t)',') || (c == (uint8_t)';') || (c == (uint8_t)'=') ||
           (c == (uint8_t)':') || (c == (uint8_t)'(') || (c == (uint8_t)')') ||
           (c == (uint8_t)'[') || (c == (uint8_t)']') || (c == (uint8_t)'<') ||
           (c == (uint8_t)'>') || (c == (uint8_t)'#') || (c == (uint8_t)'"');
}

static size_t bus_parse_hex_u32_slice(
    const uint8_t *buf,
    size_t len,
    size_t start,
    uint32_t *out_val
) {
    size_t pos = start;
    uint32_t val = 0u;
    size_t total_digits = 0u;
    while (pos < len) {
        const int nib = picofw_hex_char_to_nibble((char)buf[pos]);
        if (nib < 0) {
            break;
        }
        if (total_digits >= 8u) {
            return 0u;
        }
        ++total_digits;
        val = (val << 4) | (uint32_t)nib;
        ++pos;
    }
    if (total_digits == 0u) {
        return 0u;
    }
    *out_val = val;
    return pos - start;
}

static bool bus_payload_matches_ci_at(
    const uint8_t *buf,
    size_t len,
    size_t pos,
    const char *lit
) {
    for (size_t i = 0u; lit[i] != '\0'; ++i) {
        if ((pos + i) >= len) {
            return false;
        }
        if (bus_ascii_upper(buf[pos + i]) != bus_ascii_upper((uint8_t)lit[i])) {
            return false;
        }
    }
    return true;
}

static size_t count_vflash_unescaped_bytes(const uint8_t *payload, size_t len, size_t start) {
    size_t count = 0u;
    for (size_t d = start; (d < len) && (payload[d] != (uint8_t)'#'); ++d) {
        if (payload[d] == (uint8_t)0x7Du) {
            if (((d + 1u) < len) && (payload[d + 1u] != (uint8_t)'#')) {
                ++d;
            }
        }
        ++count;
    }
    return count;
}

static bool bus_check_rsp_packets(
    const uint8_t *payload,
    size_t len,
    picofw_bus_deny_t *out_reason
) {
    const picofw_session_ctx_t *sess = picofw_session_default_ctx();
    const uint32_t flash_bytes = picofw_flash_get_active_size();
    const bool upper_ok = picofw_flash_is_upper_nor_unlocked();

    for (size_t i = 0u; i < len; ++i) {
        if ((payload[i] != (uint8_t)'$') || ((i + 1u) >= len)) {
            continue;
        }
        size_t pos = i + 1u;
        bool is_read = false;
        bool is_erase = false;
        bool is_vflash_write = false;
        if (payload[pos] == (uint8_t)'m') {
            is_read = true;
            pos += 1u;
        } else if ((payload[pos] == (uint8_t)'M') || (payload[pos] == (uint8_t)'X')) {
            pos += 1u;
        } else if (bus_payload_matches_ci_at(payload, len, pos, "vFlashErase:")) {
            is_erase = true;
            pos += strlen("vFlashErase:");
        } else if (bus_payload_matches_ci_at(payload, len, pos, "vFlashWrite:")) {
            is_vflash_write = true;
            pos += strlen("vFlashWrite:");
        } else {
            continue;
        }

        uint32_t addr = 0u;
        const size_t consumed = bus_parse_hex_u32_slice(payload, len, pos, &addr);
        if (consumed == 0u) {
            /* Fail closed for BOTH read and write RSP packets (blocks 9+ digit `$m` smuggling). */
            if (out_reason != NULL) {
                *out_reason = PICOFW_BUS_DENY_PROTECTED_RO;
            }
            return false;
        }
        pos += consumed;

        uint32_t span = 4u;
        if (is_vflash_write) {
            if ((pos < len) && (payload[pos] == (uint8_t)':')) {
                const size_t nbytes = count_vflash_unescaped_bytes(payload, len, pos + 1u);
                span = (nbytes > 0u) ? (uint32_t)nbytes : 1u;
            } else {
                if (out_reason != NULL) {
                    *out_reason = PICOFW_BUS_DENY_PROTECTED_RO;
                }
                return false;
            }
        } else if ((pos < len) && (payload[pos] == (uint8_t)',')) {
            uint32_t parsed_len = 0u;
            const size_t len_chars = bus_parse_hex_u32_slice(payload, len, pos + 1u, &parsed_len);
            if ((len_chars == 0u) || (parsed_len == 0u)) {
                if (out_reason != NULL) {
                    *out_reason = PICOFW_BUS_DENY_PROTECTED_RO;
                }
                return false;
            }
            span = parsed_len;
        }

        if (picofw_bus_span_wraps_u32(addr, (size_t)span)) {
            if (out_reason != NULL) {
                *out_reason = PICOFW_BUS_DENY_CS0_MIRROR;
            }
            return false;
        }

        const picofw_bus_deny_t deny = is_read
            ? (picofw_bus_is_lethal_mmio_range(addr, (size_t)span)
                   ? PICOFW_BUS_DENY_LETHAL_MMIO
                   : PICOFW_BUS_ALLOW)
            : picofw_bus_check_access(sess, flash_bytes, addr, (size_t)span, true, is_erase, upper_ok);
        if (deny != PICOFW_BUS_ALLOW) {
            if (out_reason != NULL) {
                *out_reason = deny;
            }
            return false;
        }
    }
    return true;
}

static const char *const k_write_verbs[] = {
    "FAST_WRITE", "FAST_ERASE", "WRITEMEM", "MEMSET", "WRITE",
    "PATCH", "ERASE", "FLASH", "POKE", "MEMW", "PROG", "BURN",
    "MW", "MB", "MH", "WB", "WH", "WW", "PW", "PB", "PH", "PO", "EW", "SW",
    "W", "P", "E"
};

static bool bus_token_is_write_verb(const char *tok) {
    for (size_t i = 0u; i < (sizeof(k_write_verbs) / sizeof(k_write_verbs[0])); ++i) {
        if (picofw_ascii_ieq(tok, k_write_verbs[i])) {
            return true;
        }
    }
    return false;
}

static bool bus_token_parse_am34_addr(const char *tok, uint32_t *out_addr, bool *out_overflow) {
    if (out_overflow != NULL) {
        *out_overflow = false;
    }
    const char *p = tok;
    if (p[0] == '$') {
        ++p;
    } else if ((p[0] == '0') && ((p[1] == 'x') || (p[1] == 'X'))) {
        p += 2;
    }
    size_t total_digits = 0u;
    uint32_t addr = 0u;
    for (; *p != '\0'; ++p) {
        if (*p == '_') {
            continue;
        }
        const int nib = picofw_hex_char_to_nibble(*p);
        if (nib < 0) {
            return false;
        }
        if (total_digits >= 8u) {
            if (out_overflow != NULL) {
                *out_overflow = true;
            }
            return false;
        }
        ++total_digits;
        addr = (addr << 4) | (uint32_t)nib;
    }
    if (total_digits < 7u) {
        return false;
    }
    *out_addr = addr;
    return true;
}

static bool bus_try_split_compact_verb_addr(
    const char *tok,
    bool *out_is_write,
    uint32_t *out_addr,
    bool *out_overflow
) {
    const size_t tlen = strlen(tok);
    for (size_t i = 0u; i < (sizeof(k_write_verbs) / sizeof(k_write_verbs[0])); ++i) {
        const size_t vlen = strlen(k_write_verbs[i]);
        if ((tlen > vlen) && picofw_ascii_iprefix(tok, k_write_verbs[i])) {
            if (bus_token_parse_am34_addr(tok + vlen, out_addr, out_overflow) || *out_overflow) {
                *out_is_write = true;
                return true;
            }
        }
    }
    static const char *const k_read_prefixes[] = { "MD", "RD", "M", "R" };
    for (size_t i = 0u; i < (sizeof(k_read_prefixes) / sizeof(k_read_prefixes[0])); ++i) {
        const size_t plen = strlen(k_read_prefixes[i]);
        if ((tlen > plen) && picofw_ascii_iprefix(tok, k_read_prefixes[i])) {
            if (bus_token_parse_am34_addr(tok + plen, out_addr, out_overflow) || *out_overflow) {
                *out_is_write = false;
                return true;
            }
        }
    }
    return false;
}

bool picofw_bus_check_uart_tx_payload(
    const uint8_t *payload,
    size_t len,
    bool is_bridge_stream,
    picofw_bus_deny_t *out_reason
) {
    if (out_reason != NULL) {
        *out_reason = PICOFW_BUS_ALLOW;
    }
    if ((payload == NULL) || (len == 0u)) {
        return true;
    }
    (void)is_bridge_stream;

    if (!bus_check_rsp_packets(payload, len, out_reason)) {
        return false;
    }

    bool has_write_intent = false;
    uint32_t addrs[16] = {0};
    size_t addr_count = 0u;

    size_t pos = 0u;
    while (pos < len) {
        while ((pos < len) && bus_is_delim_char(payload[pos])) {
            ++pos;
        }
        if (pos >= len) {
            break;
        }
        char tok[48];
        size_t tlen = 0u;
        while ((pos < len) && !bus_is_delim_char(payload[pos])) {
            if ((tlen + 1u) < sizeof(tok)) {
                tok[tlen++] = (char)payload[pos];
            }
            ++pos;
        }
        tok[tlen] = '\0';

        if (bus_token_is_write_verb(tok)) {
            has_write_intent = true;
            continue;
        }

        uint32_t addr = 0u;
        bool compact_write = false;
        bool overflow = false;
        bool parsed = bus_token_parse_am34_addr(tok, &addr, &overflow);
        if (!parsed && !overflow && bus_try_split_compact_verb_addr(tok, &compact_write, &addr, &overflow)) {
            parsed = !overflow;
            if (compact_write) {
                has_write_intent = true;
            }
        }
        if (overflow) {
            if (out_reason != NULL) {
                *out_reason = PICOFW_BUS_DENY_CS0_MIRROR;
            }
            return false;
        }
        if (parsed) {
            if (picofw_bus_is_lethal_mmio_range(addr, 4u)) {
                if (out_reason != NULL) {
                    *out_reason = PICOFW_BUS_DENY_LETHAL_MMIO;
                }
                return false;
            }
            if (addr_count >= (sizeof(addrs) / sizeof(addrs[0]))) {
                if (out_reason != NULL) {
                    *out_reason = PICOFW_BUS_DENY_PROTECTED_RO;
                }
                return false;
            }
            addrs[addr_count++] = addr;
        }
    }

    if (has_write_intent) {
        const picofw_session_ctx_t *sess = picofw_session_default_ctx();
        const uint32_t flash_bytes = picofw_flash_get_active_size();
        const bool upper_ok = picofw_flash_is_upper_nor_unlocked();
        for (size_t i = 0u; i < addr_count; ++i) {
            const picofw_bus_deny_t deny = picofw_bus_check_access(
                sess, flash_bytes, addrs[i], 4u, true, false, upper_ok
            );
            if (deny != PICOFW_BUS_ALLOW) {
                if (out_reason != NULL) {
                    *out_reason = deny;
                }
                return false;
            }
        }
    }
    return true;
}

void picofw_bridge_tx_filter_init(picofw_bridge_tx_filter_t *f) {
    if (f != NULL) {
        memset(f, 0, sizeof(*f));
    }
}

picofw_bridge_filter_action_t picofw_bridge_tx_filter_feed(
    picofw_bridge_tx_filter_t *f,
    uint8_t b,
    uint8_t *out_buf,
    size_t out_cap,
    size_t *out_len,
    picofw_bus_deny_t *out_reason
) {
    if (out_len != NULL) {
        *out_len = 0u;
    }
    if (out_reason != NULL) {
        *out_reason = PICOFW_BUS_ALLOW;
    }
    if ((f == NULL) || (out_buf == NULL) || (out_len == NULL) || (out_cap == 0u)) {
        return PICOFW_BRIDGE_ACT_BLOCKED;
    }

    if (f->in_rsp || (b == (uint8_t)'$')) {
        if (!f->in_rsp) {
            f->in_rsp = true;
            f->rsp_len = 0u;
            f->rsp_hash_tail = 0u;
            f->rsp_overflow = false;
        }
        if (f->rsp_len < sizeof(f->rsp_buf)) {
            f->rsp_buf[f->rsp_len++] = b;
        } else {
            f->rsp_overflow = true;
        }
        if (b == (uint8_t)'#') {
            f->rsp_hash_tail = 2u;
        } else if (f->rsp_hash_tail > 0u) {
            --f->rsp_hash_tail;
        }

        const bool rsp_done = (b == (uint8_t)'\r') || (b == (uint8_t)'\n') ||
                              ((f->rsp_len > 3u) &&
                               !f->rsp_overflow &&
                               (f->rsp_buf[f->rsp_len - 3u] == (uint8_t)'#') &&
                               (f->rsp_hash_tail == 0u));
        if (!rsp_done) {
            return PICOFW_BRIDGE_ACT_HOLD;
        }
        const bool ovf = f->rsp_overflow || (f->rsp_len > out_cap);
        const size_t flen = f->rsp_len;
        f->in_rsp = false;
        f->rsp_len = 0u;
        f->rsp_hash_tail = 0u;
        f->rsp_overflow = false;
        if (ovf) {
            if (out_reason != NULL) {
                *out_reason = PICOFW_BUS_DENY_PROTECTED_RO;
            }
            return PICOFW_BRIDGE_ACT_BLOCKED;
        }
        if (picofw_bus_check_uart_tx_payload(f->rsp_buf, flen, true, out_reason)) {
            memcpy(out_buf, f->rsp_buf, flen);
            *out_len = flen;
            return PICOFW_BRIDGE_ACT_FORWARD;
        }
        return PICOFW_BRIDGE_ACT_BLOCKED;
    }

    if ((f->line_len == 0u) && !f->line_overflow &&
        ((b == (uint8_t)'+') || (b == (uint8_t)'-') || (b == 0x03u))) {
        out_buf[0] = b;
        *out_len = 1u;
        return PICOFW_BRIDGE_ACT_PASS_CTL;
    }

    if ((b == (uint8_t)'\r') || (b == (uint8_t)'\n')) {
        const size_t llen = f->line_len;
        const bool ovf = f->line_overflow || ((llen + 1u) > out_cap);
        f->line_len = 0u;
        f->line_overflow = false;
        if (ovf) {
            if (out_reason != NULL) {
                *out_reason = PICOFW_BUS_DENY_PROTECTED_RO;
            }
            return PICOFW_BRIDGE_ACT_BLOCKED;
        }
        if (!picofw_bus_check_uart_tx_payload(f->line_buf, llen, true, out_reason)) {
            return PICOFW_BRIDGE_ACT_BLOCKED;
        }
        if (llen > 0u) {
            memcpy(out_buf, f->line_buf, llen);
        }
        out_buf[llen] = b;
        *out_len = llen + 1u;
        return PICOFW_BRIDGE_ACT_FORWARD;
    }

    if ((b == 0x08u) || (b == 0x7Fu)) {
        if ((f->line_len > 0u) && !f->line_overflow) {
            --f->line_len;
        }
        return PICOFW_BRIDGE_ACT_HOLD;
    }
    if (b == 0x15u) {
        f->line_len = 0u;
        f->line_overflow = false;
        return PICOFW_BRIDGE_ACT_HOLD;
    }
    if (f->line_len < (sizeof(f->line_buf) - 1u)) {
        f->line_buf[f->line_len++] = b;
    } else {
        f->line_overflow = true;
    }
    return PICOFW_BRIDGE_ACT_HOLD;
}
