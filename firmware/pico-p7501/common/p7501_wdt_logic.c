/**
 * @file p7501_wdt_logic.c
 * @brief Pure ISO C11 watchdog scratch word encoding, window clamping, and CDC abort helpers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_wdt_logic.h"

#include <stddef.h>

uint32_t p7501_wdt_pack_scratch0(uint8_t cmd_id, uint8_t flags, uint16_t recoveries) {
    return (((uint32_t)cmd_id) << 24u) |
           (((uint32_t)flags) << 16u) |
           ((uint32_t)recoveries);
}

bool p7501_wdt_unpack_scratch0(
    uint32_t word,
    uint8_t *out_cmd_id,
    uint8_t *out_flags,
    uint16_t *out_recoveries
) {
    if ((out_cmd_id == NULL) || (out_flags == NULL) || (out_recoveries == NULL)) {
        return false;
    }
    *out_cmd_id = (uint8_t)((word >> 24u) & 0xFFu);
    *out_flags = (uint8_t)((word >> 16u) & 0xFFu);
    *out_recoveries = (uint16_t)(word & 0xFFFFu);
    return true;
}

uint32_t p7501_wdt_clamp_window_ms(uint32_t requested_ms, uint32_t default_ms, uint32_t min_ms) {
    uint32_t eff_min = (min_ms == 0u) ? 1u : min_ms;
    if (eff_min > P7501_WDT_MAX_CMD_WINDOW_MS) {
        eff_min = P7501_WDT_MAX_CMD_WINDOW_MS;
    }
    uint32_t val = (requested_ms == 0u) ? default_ms : requested_ms;
    if (val < eff_min) {
        return eff_min;
    }
    if (val > P7501_WDT_MAX_CMD_WINDOW_MS) {
        return P7501_WDT_MAX_CMD_WINDOW_MS;
    }
    return val;
}

bool p7501_wdt_is_abort_char(int ch) {
    return ch == P7501_CDC_ABORT_CTRL_C;
}
