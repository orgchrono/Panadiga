/**
 * @file p7501_bus_policy.h
 * @brief Single Authoritative AM34 Bus & NOR Flash Access Policy Gate (SSOT-driven).
 *
 * Enforces all physical and forensic silicon invariants across CLI, GDB RSP,
 * PANARSP, DIAGWATCH, JEDEC stubs, and target bus HAL:
 *  - Lethal AM34 MMIO (`0xC000100C..0xC000100F`): blocked for BOTH read & write.
 *  - CS0 alias mirrors (`[0x80000000, 0x88000000)` outside canonical primary window): blocked.
 *  - Layer-1 Boot IPL (`SA0..SA10`, first 256 KiB): permanently read-only (`E05`).
 *  - Full-RO profiles (`DMR-EZ485V`, `DMR-ES10`) & `MAIN_XIP` (`DMR-ES35V`): permanently RO.
 *  - Full physical sector span expansion on Sector Erase (`SA0..SA141` geometry).
 *  - F14 wireless write-lockout (`BLE_NUS` / `WIFI_TCP` require USB CDC `BLE_NOR_AUTH`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_BUS_POLICY_H
#define P7501_BUS_POLICY_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct p7501_session_ctx p7501_session_ctx_t;

typedef enum p7501_bus_deny {
    P7501_BUS_ALLOW = 0,
    P7501_BUS_DENY_LETHAL_MMIO = 1,
    P7501_BUS_DENY_WIRELESS_F14 = 2,
    P7501_BUS_DENY_CS0_MIRROR = 3,
    P7501_BUS_DENY_IPL_LAYER1 = 4,
    P7501_BUS_DENY_FULL_RO = 5,
    P7501_BUS_DENY_PROTECTED_RO = 6,
    P7501_BUS_DENY_UPPER_LOCKED = 7
} p7501_bus_deny_t;

/** True when `[addr, addr + max(len,1))` wraps past 32-bit address space `0xFFFFFFFF`. */
bool p7501_bus_span_wraps_u32(uint32_t addr, size_t len);

/** True when `[addr, addr + max(len,1))` overlaps AM34 lethal MMIO `0xC000100C..0xC000100F`. */
bool p7501_bus_is_lethal_mmio_range(uint32_t addr, size_t len);

/** True when `[addr, addr + max(len,1))` overlaps full CS0 aperture `[0x80000000, 0x88000000)`. */
bool p7501_bus_touches_cs0_aperture(uint32_t addr, size_t len);

/** True when `[addr, addr + max(len,1))` hits CS0 outside primary cached/uncached NOR capacity. */
bool p7501_bus_is_cs0_alias_mirror(uint32_t flash_size_bytes, uint32_t addr, size_t len);

/**
 * Single authoritative AM34 bus & NOR Flash access gate.
 *
 * When `is_erase == true`, expands `addr` to its full physical sector `[sec_base, sec_base + sec_size)`
 * before checking IPL, `MAIN_XIP`, `full_nor_readonly`, CS0 mirrors, and Upper NOR policy.
 */
p7501_bus_deny_t p7501_bus_check_access(
    const p7501_session_ctx_t *ctx,
    uint32_t flash_size_bytes,
    uint32_t addr,
    size_t len,
    bool is_write,
    bool is_erase,
    bool upper_nor_unlocked
);

/** Human-readable static denial tag for CLI / audit logs. */
const char *p7501_bus_deny_name(p7501_bus_deny_t reason);

/**
 * @brief Pure safety gate for interactive/bridge UART TX payloads (`UARTSEND`, `UARTTXHEX`, `WAITTX`, `BRIDGE`).
 *
 * Blocks any lethal AM34 MMIO reference (`0xC000100C..0xC000100F`, `0xDC0F0000..0xDC0F000F`),
 * GDB RSP write/erase frames or ASCII write commands targeting CS0 alias mirrors, critical IPL
 * `SA0..SA10`, F14 wireless lock, or locked Upper NOR, while allowing read-only monitor probes
 * (`MD 80000000`, `$m80000000,4`) and writable NVRAM (`MW 807F0000`).
 */
bool p7501_bus_check_uart_tx_payload(
    const uint8_t *payload,
    size_t len,
    bool is_bridge_stream,
    p7501_bus_deny_t *out_reason
);

#define P7501_BRIDGE_LINE_CAP 256u
#define P7501_BRIDGE_RSP_CAP  512u

typedef enum p7501_bridge_filter_action {
    P7501_BRIDGE_ACT_HOLD = 0,
    P7501_BRIDGE_ACT_FORWARD = 1,
    P7501_BRIDGE_ACT_PASS_CTL = 2,
    P7501_BRIDGE_ACT_BLOCKED = 3
} p7501_bridge_filter_action_t;

typedef struct p7501_bridge_tx_filter {
    uint8_t line_buf[P7501_BRIDGE_LINE_CAP];
    size_t line_len;
    bool line_overflow;
    uint8_t rsp_buf[P7501_BRIDGE_RSP_CAP];
    size_t rsp_len;
    bool in_rsp;
    uint8_t rsp_hash_tail;
    bool rsp_overflow;
} p7501_bridge_tx_filter_t;

/** Reset fail-closed buffered `BRIDGE-POLICY` filter state. */
void p7501_bridge_tx_filter_init(p7501_bridge_tx_filter_t *f);

/**
 * @brief Feed one host USB TX byte into fail-closed buffered `BRIDGE-POLICY` filter.
 *
 * Holds text lines until `\r`/`\n` and GDB RSP frames until `#xx` or `\r`/`\n`.
 * On `P7501_BRIDGE_ACT_FORWARD`, `out_buf[0..*out_len)` contains the validated bytes to transmit.
 */
p7501_bridge_filter_action_t p7501_bridge_tx_filter_feed(
    p7501_bridge_tx_filter_t *f,
    uint8_t byte,
    uint8_t *out_buf,
    size_t out_cap,
    size_t *out_len,
    p7501_bus_deny_t *out_reason
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_BUS_POLICY_H */
