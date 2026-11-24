/**
 * @file tbus_ic7501.c
 * @brief `IC7501` Front-Panel Timer Supervisor `TBUS` Keepalive & VFD Driver.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

#include <string.h>

#define IC7501_CMD_WDT_255S     0x8Au
#define IC7501_CMD_VFD_TEXT     0xA3u
#define IC7501_CHECKSUM_BIAS    0x5Bu
#define IC7501_TBUS_STROBE_BIT  0x00000010UL

static uint32_t load_le32(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8u) |
           ((uint32_t)p[2] << 16u) | ((uint32_t)p[3] << 24u);
}

uint8_t openos_tbus_checksum15(const uint8_t body15[15]) {
    uint32_t sum = 0u;
    for (size_t i = 0u; i < 15u; ++i) {
        sum += (uint32_t)body15[i];
    }
    return (uint8_t)((sum - IC7501_CHECKSUM_BIAS) & 0xFFu);
}

void openos_tbus_build_wdt_255s_frame(uint8_t out16[16]) {
    if (out16 == NULL) {
        return;
    }
    memset(out16, 0, 16u);
    out16[0] = IC7501_CMD_WDT_255S;
    out16[1] = 0x02u;
    out16[3] = 0xFFu;
    out16[15] = openos_tbus_checksum15(out16);
}

void openos_tbus_build_vfd_frame(const char *ascii7, uint8_t out16[16]) {
    if (out16 == NULL) {
        return;
    }
    memset(out16, 0, 16u);
    out16[0] = IC7501_CMD_VFD_TEXT;
    memset(&out16[3], ' ', 7u);
    if (ascii7 != NULL) {
        for (size_t i = 0u; (i < 7u) && (ascii7[i] != '\0'); ++i) {
            const uint8_t ch = (uint8_t)ascii7[i];
            out16[3u + i] = ((ch >= 0x20u) && (ch <= 0x7Eu)) ? ch : (uint8_t)' ';
        }
    }
    out16[15] = openos_tbus_checksum15(out16);
}

bool openos_tbus_opcode_safe_for_br_ic7501(uint8_t opcode) {
    switch (opcode) {
        case 0x85u:
        case 0x89u:
        case 0x9Bu:
        case 0xA9u:
        case 0xAAu:
        case 0xC9u:
        case 0xDAu:
        case 0xE3u:
        case 0xE4u:
        case 0xECu:
        case 0xF0u:
        case 0xF9u:
        case 0xFAu:
            return false;
        default:
            return true;
    }
}

bool openos_tbus_send_frame_checked(const uint8_t frame16[16]) {
    if (frame16 == NULL) {
        return false;
    }
    if (!openos_tbus_opcode_safe_for_br_ic7501(frame16[0])) {
        return false;
    }
    if (frame16[15] != openos_tbus_checksum15(frame16)) {
        return false;
    }
    for (uint32_t w = 0u; w < 4u; ++w) {
        if (!am34_mmio_write32(AM34_OPENOS_TBUS_MBOX + (w * 4u), load_le32(&frame16[w * 4u]))) {
            return false;
        }
    }
    return am34_mmio_write32(AM34_OPENOS_TBUS_BASE, IC7501_TBUS_STROBE_BIT);
}

bool openos_tbus_send_vfd_text(const char *ascii7) {
    uint8_t frame[16];
    openos_tbus_build_vfd_frame(ascii7, frame);
    return openos_tbus_send_frame_checked(frame);
}

bool openos_tbus_send_keepalive_255s(void) {
    uint8_t frame[16];
    openos_tbus_build_wdt_255s_frame(frame);
    return openos_tbus_send_frame_checked(frame);
}
