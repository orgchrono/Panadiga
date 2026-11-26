/**
 * @file avdd_iic_video.c
 * @brief AM34 `AVDD` Video DAC + `IIC0`/`IIC1` `IC3001` (`BH7629KS2` vs `C1AB00002379`) Switch Driver for `DigaOS`.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/digaos_drivers.h"

static bool digaos_iic_base_safe(uint32_t iic_base) {
    if ((iic_base != AM34_DIGAOS_IIC0_BASE) && (iic_base != AM34_DIGAOS_IIC1_BASE)) {
        return false;
    }
    return !am34_mmio_is_forbidden(iic_base) && am34_mmio_write_allowed(iic_base);
}

static bool digaos_avdd_write16(uint32_t reg_off, uint16_t val) {
    return am34_mmio_write16(AM34_DIGAOS_AVDD_BASE + reg_off, val);
}

uint8_t digaos_ic3001_addr8(digaos_board_video_ic_t ic, bool adr_high) {
    if (ic == DIGAOS_VIDEO_IC_C1AB2379_EC) {
        return AM34_DIGAOS_IC3001_EC_C1AB2379_ADDR8;
    }
    return adr_high ? AM34_DIGAOS_IC3001_BR_BH7629_ADDR8_H : AM34_DIGAOS_IC3001_BR_BH7629_ADDR8_L;
}

bool digaos_iic_probe_ic3001(digaos_board_video_ic_t ic, uint8_t *out_addr8) {
    if ((out_addr8 == NULL) || !digaos_iic_base_safe(AM34_DIGAOS_IIC1_BASE)) {
        return false;
    }
    uint8_t candidates[2];
    size_t count = 0u;
    if (ic == DIGAOS_VIDEO_IC_C1AB2379_EC) {
        candidates[0] = AM34_DIGAOS_IC3001_EC_C1AB2379_ADDR8;
        count = 1u;
    } else {
        candidates[0] = AM34_DIGAOS_IC3001_BR_BH7629_ADDR8_L;
        candidates[1] = AM34_DIGAOS_IC3001_BR_BH7629_ADDR8_H;
        count = 2u;
    }

    for (size_t i = 0u; i < count; ++i) {
        const uint8_t addr8 = candidates[i];
        const uint8_t addr_write = (uint8_t)(addr8 & 0xFEu);
        uint32_t stat = AM34_DIGAOS_IIC_STAT_NACK_BIT;
        if (!am34_mmio_write32(AM34_DIGAOS_IIC1_BASE + AM34_DIGAOS_IIC_CTRL_OFF, 0x00000001UL) ||
            !am34_mmio_write8(AM34_DIGAOS_IIC1_BASE + AM34_DIGAOS_IIC_TX_OFF, addr_write) ||
            !am34_mmio_read32(AM34_DIGAOS_IIC1_BASE + AM34_DIGAOS_IIC_STAT_OFF, &stat)) {
            return false;
        }
        if ((stat & AM34_DIGAOS_IIC_STAT_NACK_BIT) == 0UL) {
            *out_addr8 = addr8;
            return true;
        }
    }
    return false;
}

bool digaos_iic_write_stream_checked(
    uint32_t iic_base,
    uint8_t slave_addr8,
    const uint8_t *buf,
    size_t len
) {
    if ((buf == NULL) || (len == 0u) || !digaos_iic_base_safe(iic_base)) {
        return false;
    }
    const uint8_t addr_write = (uint8_t)(slave_addr8 & 0xFEu);
    if (!am34_mmio_write32(iic_base + AM34_DIGAOS_IIC_CTRL_OFF, 0x00000001UL) ||
        !am34_mmio_write8(iic_base + AM34_DIGAOS_IIC_TX_OFF, addr_write)) {
        return false;
    }
    for (size_t i = 0u; i < len; ++i) {
        if (!am34_mmio_write8(iic_base + AM34_DIGAOS_IIC_TX_OFF, buf[i])) {
            return false;
        }
    }
    return true;
}

bool digaos_iic_write_reg_checked(uint32_t iic_base, uint8_t slave_addr8, uint8_t reg, uint8_t val) {
    const uint8_t pair[2] = { reg, val };
    return digaos_iic_write_stream_checked(iic_base, slave_addr8, pair, sizeof(pair));
}

bool digaos_iic5_init_m6930lb_av(void) {
    static const uint8_t k_bh7629_m6930lb_6b[6] = {
        0x10u, 0x08u, 0x07u, 0x63u, 0xC0u, 0x30u
    };
    static const uint8_t k_audio_m6930lb_4b[4] = {
        0x03u, 0x80u, 0x00u, 0x22u
    };
    if (!digaos_iic_write_stream_checked(
            AM34_DIGAOS_IIC0_BASE,
            AM34_DIGAOS_IC3001_BR_BH7629_ADDR8_L,
            k_bh7629_m6930lb_6b,
            sizeof(k_bh7629_m6930lb_6b))) {
        return false;
    }
    return digaos_iic_write_stream_checked(
        AM34_DIGAOS_IIC0_BASE,
        0x84u,
        k_audio_m6930lb_4b,
        sizeof(k_audio_m6930lb_4b)
    );
}

static uint16_t digaos_avdd_npal_bits(digaos_tv_system_t sys) {
    if (sys == DIGAOS_TV_PAL_M) {
        return 0x0001U;
    }
    if ((sys == DIGAOS_TV_PAL) || (sys == DIGAOS_TV_SECAM)) {
        return 0x0002U;
    }
    return 0x0000U;
}

bool digaos_avdd_configure_output(digaos_tv_system_t sys, bool unmute_dac) {
    const uint16_t npal = digaos_avdd_npal_bits(sys);
    const uint16_t voutmode0 = unmute_dac ? AM34_DIGAOS_AVDD_VOUTMODE0_NORMAL
                                          : AM34_DIGAOS_AVDD_VOUTMODE0_MUTE;
    const uint16_t dacout1 = unmute_dac ? AM34_DIGAOS_AVDD_DACOUT1_ENABLE
                                        : AM34_DIGAOS_AVDD_DACOUT1_MUTE;

    if (!digaos_avdd_write16(AM34_DIGAOS_AVDD_NPALCTL0_OFF, npal)) {
        return false;
    }
    if (!digaos_avdd_write16(AM34_DIGAOS_AVDD_VOUTMODE0_OFF, voutmode0)) {
        return false;
    }
    return digaos_avdd_write16(AM34_DIGAOS_AVDD_DACOUT1_OFF, dacout1);
}

bool digaos_avdd_configure_geometry(digaos_tv_system_t sys, bool unmute_dac) {
    const bool is_625 = (sys == DIGAOS_TV_PAL) || (sys == DIGAOS_TV_SECAM);
    const uint16_t clk = is_625 ? AM34_DIGAOS_AVDD_CLK_576I : AM34_DIGAOS_AVDD_CLK_480I;
    const uint32_t active_size =
        is_625 ? AM34_DIGAOS_AVDD_ACTIVE_SIZE_576I : AM34_DIGAOS_AVDD_ACTIVE_SIZE_480I;
    const uint32_t sync = is_625 ? AM34_DIGAOS_AVDD_SYNC_576I : AM34_DIGAOS_AVDD_SYNC_480I;
    const uint32_t npal_lo = (uint32_t)digaos_avdd_npal_bits(sys);

    if (!digaos_avdd_write16(AM34_DIGAOS_AVDD_CLK_OFF, clk) ||
        !am34_mmio_write32(AM34_DIGAOS_AVDD_BASE + 0x0408UL, active_size) ||
        !am34_mmio_write32(AM34_DIGAOS_AVDD_BASE + 0x040CUL, sync) ||
        !am34_mmio_write32(
            AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_NPALCTL0_OFF,
            AM34_DIGAOS_AVDD_NPALCTL0_HI_KEEP | npal_lo
        )) {
        return false;
    }
    return digaos_avdd_configure_output(sys, unmute_dac);
}

bool digaos_avdd_configure_480i_m6930lb(digaos_tv_system_t sys, bool unmute_dac) {
    return digaos_avdd_configure_geometry(sys, unmute_dac);
}

bool digaos_avdd_set_colorbar(bool enable) {
    const uint16_t voutmode0 = enable ? AM34_DIGAOS_AVDD_VOUTMODE0_COLORBAR
                                      : AM34_DIGAOS_AVDD_VOUTMODE0_NORMAL;
    if (!digaos_avdd_write16(AM34_DIGAOS_AVDD_VOUTMODE0_OFF, voutmode0)) {
        return false;
    }
    if (enable) {
        return digaos_avdd_write16(AM34_DIGAOS_AVDD_DACOUT1_OFF, AM34_DIGAOS_AVDD_DACOUT1_ENABLE);
    }
    return true;
}

bool digaos_video_bringup_board(
    digaos_board_video_ic_t ic,
    digaos_tv_system_t sys,
    bool nonstandard_sync,
    bool mvagc_line_tbc
) {
    if (!digaos_aiof_apply_tbc_profile(sys, nonstandard_sync, mvagc_line_tbc)) {
        return false;
    }
    if (ic == DIGAOS_VIDEO_IC_BH7629KS2_GPIO_M6930LB) {
        return digaos_avdd_configure_geometry(sys, true);
    }
    if (!digaos_avdd_configure_geometry(sys, true)) {
        return false;
    }
    if (ic == DIGAOS_VIDEO_IC_BH7629KS2_BR_NA) {
        return digaos_iic5_init_m6930lb_av();
    }
    const uint8_t slave_addr8 = digaos_ic3001_addr8(ic, false);
    const uint8_t mode_val = ((sys == DIGAOS_TV_PAL) || (sys == DIGAOS_TV_SECAM)) ? 0x01u : 0x00u;
    if (!digaos_iic_write_reg_checked(AM34_DIGAOS_IIC1_BASE, slave_addr8, 0x00u, mode_val)) {
        return false;
    }
    return digaos_iic_write_reg_checked(AM34_DIGAOS_IIC1_BASE, slave_addr8, 0x01u, 0x01u);
}

bool digaos_video_auto_bringup(digaos_tv_system_t sys, digaos_board_video_ic_t *out_ic) {
    uint8_t detected_addr8 = 0u;
    digaos_board_video_ic_t ic = DIGAOS_VIDEO_IC_BH7629KS2_GPIO_M6930LB;
    if (digaos_iic_probe_ic3001(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, &detected_addr8)) {
        ic = DIGAOS_VIDEO_IC_BH7629KS2_BR_NA;
    } else if (digaos_iic_probe_ic3001(DIGAOS_VIDEO_IC_C1AB2379_EC, &detected_addr8)) {
        ic = DIGAOS_VIDEO_IC_C1AB2379_EC;
    }
    if (out_ic != NULL) {
        *out_ic = ic;
    }
    return digaos_video_bringup_board(ic, sys, false, true);
}

bool digaos_video_inspect_status(digaos_video_status_t *out) {
    if (out == NULL) {
        return false;
    }
    if (!am34_mmio_read16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_VMODESEL_OFF, &out->aiof_vmodesel) ||
        !am34_mmio_read16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_CONT_OFF, &out->aiof_cont) ||
        !am34_mmio_read16(AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_CLK_OFF, &out->avdd_clk) ||
        !am34_mmio_read16(AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_DACOUT1_OFF, &out->avdd_dacout1) ||
        !am34_mmio_read16(AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_VOUTMODE0_OFF, &out->avdd_voutmode0) ||
        !am34_mmio_read32(AM34_DIGAOS_AVDD_BASE + 0x0408UL, &out->avdd_active_size) ||
        !am34_mmio_read32(AM34_DIGAOS_AVDD_BASE + 0x040CUL, &out->avdd_sync)) {
        return false;
    }
    out->dac_unmuted = ((out->avdd_dacout1 & AM34_DIGAOS_AVDD_DACOUT1_MUTE_BIT) == 0U) &&
                       (out->avdd_dacout1 != 0U);
    return true;
}
