/**
 * @file avdd_iic_video.c
 * @brief AM34 `AVDD` Video DAC + `IIC0`/`IIC1` `IC3001` (`BH7629KS2` vs `C1AB00002379`) Switch Driver.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

static bool openos_iic_base_safe(uint32_t iic_base) {
    if ((iic_base != AM34_OPENOS_IIC0_BASE) && (iic_base != AM34_OPENOS_IIC1_BASE)) {
        return false;
    }
    return !am34_mmio_is_forbidden(iic_base) && am34_mmio_write_allowed(iic_base);
}

static bool openos_avdd_write16(uint32_t reg_off, uint16_t val) {
    return am34_mmio_write16(AM34_OPENOS_AVDD_BASE + reg_off, val);
}

uint8_t openos_ic3001_addr8(openos_board_video_ic_t ic, bool adr_high) {
    if (ic == OPENOS_VIDEO_IC_C1AB2379_EC) {
        return AM34_OPENOS_IC3001_EC_C1AB2379_ADDR8;
    }
    return adr_high ? AM34_OPENOS_IC3001_BR_BH7629_ADDR8_H : AM34_OPENOS_IC3001_BR_BH7629_ADDR8_L;
}

bool openos_iic_probe_ic3001(openos_board_video_ic_t ic, uint8_t *out_addr8) {
    if ((out_addr8 == NULL) || !openos_iic_base_safe(AM34_OPENOS_IIC1_BASE)) {
        return false;
    }
    uint8_t candidates[2];
    size_t count = 0u;
    if (ic == OPENOS_VIDEO_IC_C1AB2379_EC) {
        candidates[0] = AM34_OPENOS_IC3001_EC_C1AB2379_ADDR8;
        count = 1u;
    } else {
        candidates[0] = AM34_OPENOS_IC3001_BR_BH7629_ADDR8_L;
        candidates[1] = AM34_OPENOS_IC3001_BR_BH7629_ADDR8_H;
        count = 2u;
    }

    for (size_t i = 0u; i < count; ++i) {
        const uint8_t addr8 = candidates[i];
        const uint8_t addr_write = (uint8_t)(addr8 & 0xFEu);
        uint32_t stat = AM34_OPENOS_IIC_STAT_NACK_BIT;
        if (!am34_mmio_write32(AM34_OPENOS_IIC1_BASE + AM34_OPENOS_IIC_CTRL_OFF, 0x00000001UL) ||
            !am34_mmio_write8(AM34_OPENOS_IIC1_BASE + AM34_OPENOS_IIC_TX_OFF, addr_write) ||
            !am34_mmio_read32(AM34_OPENOS_IIC1_BASE + AM34_OPENOS_IIC_STAT_OFF, &stat)) {
            return false;
        }
        if ((stat & AM34_OPENOS_IIC_STAT_NACK_BIT) == 0UL) {
            *out_addr8 = addr8;
            return true;
        }
    }
    return false;
}

bool openos_iic_write_reg_checked(uint32_t iic_base, uint8_t slave_addr8, uint8_t reg, uint8_t val) {
    if (!openos_iic_base_safe(iic_base)) {
        return false;
    }
    const uint8_t addr_write = (uint8_t)(slave_addr8 & 0xFEu);
    if (!am34_mmio_write32(iic_base + AM34_OPENOS_IIC_CTRL_OFF, 0x00000001UL)) {
        return false;
    }
    if (!am34_mmio_write8(iic_base + AM34_OPENOS_IIC_TX_OFF, addr_write)) {
        return false;
    }
    if (!am34_mmio_write8(iic_base + AM34_OPENOS_IIC_TX_OFF, reg)) {
        return false;
    }
    return am34_mmio_write8(iic_base + AM34_OPENOS_IIC_TX_OFF, val);
}

bool openos_avdd_configure_output(openos_tv_system_t sys, bool unmute_dac) {
    uint16_t npal = 0x0000U;
    if (sys == OPENOS_TV_PAL_M) {
        npal = 0x0001U;
    } else if (sys == OPENOS_TV_PAL) {
        npal = 0x0002U;
    }
    const uint16_t voutmode0 = unmute_dac ? AM34_OPENOS_AVDD_VOUTMODE0_NORMAL
                                          : AM34_OPENOS_AVDD_VOUTMODE0_MUTE;
    const uint16_t dacout1 = unmute_dac ? AM34_OPENOS_AVDD_DACOUT1_ENABLE
                                        : AM34_OPENOS_AVDD_DACOUT1_MUTE;

    if (!openos_avdd_write16(AM34_OPENOS_AVDD_NPALCTL0_OFF, npal)) {
        return false;
    }
    if (!openos_avdd_write16(AM34_OPENOS_AVDD_VOUTMODE0_OFF, voutmode0)) {
        return false;
    }
    return openos_avdd_write16(AM34_OPENOS_AVDD_DACOUT1_OFF, dacout1);
}

bool openos_avdd_set_colorbar(bool enable) {
    const uint16_t voutmode0 = enable ? AM34_OPENOS_AVDD_VOUTMODE0_COLORBAR
                                      : AM34_OPENOS_AVDD_VOUTMODE0_NORMAL;
    if (!openos_avdd_write16(AM34_OPENOS_AVDD_VOUTMODE0_OFF, voutmode0)) {
        return false;
    }
    if (enable) {
        return openos_avdd_write16(AM34_OPENOS_AVDD_DACOUT1_OFF, AM34_OPENOS_AVDD_DACOUT1_ENABLE);
    }
    return true;
}

bool openos_video_bringup_board(
    openos_board_video_ic_t ic,
    openos_tv_system_t sys,
    bool nonstandard_sync,
    bool mvagc_line_tbc
) {
    if (!openos_aiof_apply_tbc_profile(sys, nonstandard_sync, mvagc_line_tbc)) {
        return false;
    }
    if (!openos_avdd_configure_output(sys, true)) {
        return false;
    }
    const uint8_t slave_addr8 = openos_ic3001_addr8(ic, false);
    const uint8_t mode_val = (sys == OPENOS_TV_PAL) ? 0x01u : 0x00u;
    if (!openos_iic_write_reg_checked(AM34_OPENOS_IIC1_BASE, slave_addr8, 0x00u, mode_val)) {
        return false;
    }
    return openos_iic_write_reg_checked(AM34_OPENOS_IIC1_BASE, slave_addr8, 0x01u, 0x01u);
}
