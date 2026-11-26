/**
 * @file aiof_video.c
 * @brief AM34 `AIOF` TBC & Composite/S-Video Standard Selector (`NTSC`, `PAL-M`, `PAL`, `SECAM`) for `DigaOS`.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Absorbs `probe_tbc_aiof_baseline` and `DIGAOS_TV_SECAM` support from retired `blackbox-fw`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/digaos_drivers.h"

digaos_tv_system_t digaos_tv_from_nfrom_id0117(uint32_t raw_id0117) {
    return ((raw_id0117 & 0xFFu) == AM34_DIGAOS_ID0117_NTSC) ? DIGAOS_TV_NTSC : DIGAOS_TV_PAL;
}

digaos_tv_system_t digaos_tv_from_main3410_cfg0117(uint8_t cfg_byte) {
    return (cfg_byte == AM34_DIGAOS_MAIN3410_CFG_ID0117_NTSC) ? DIGAOS_TV_NTSC : DIGAOS_TV_PAL;
}

bool digaos_tv_from_avout_tv_system(uint32_t raw_avout, digaos_tv_system_t *out_sys) {
    if ((out_sys == NULL) || (raw_avout > 1UL)) {
        return false;
    }
    *out_sys = (raw_avout == 0UL) ? DIGAOS_TV_NTSC : DIGAOS_TV_PAL;
    return true;
}

bool digaos_aiof_select_tv_system(digaos_tv_system_t sys) {
    uint16_t vmodesel = AM34_DIGAOS_VMODESEL_NTSC;
    uint16_t fs0 = AM34_DIGAOS_AIOF_FS0_STD;
    if (sys == DIGAOS_TV_PAL_M) {
        vmodesel = AM34_DIGAOS_VMODESEL_PAL_M;
    } else if ((sys == DIGAOS_TV_PAL) || (sys == DIGAOS_TV_SECAM)) {
        vmodesel = AM34_DIGAOS_VMODESEL_PAL;
        fs0 = AM34_DIGAOS_AIOF_FS0_NONSTD;
    }

    if (!am34_mmio_write16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_V_OFF,
                           AM34_DIGAOS_AIOF_V_ENABLE)) {
        return false;
    }
    if (!am34_mmio_write16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_FS0_OFF, fs0)) {
        return false;
    }
    return am34_mmio_write16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_VMODESEL_OFF, vmodesel);
}

bool probe_tbc_aiof_baseline(digaos_tv_system_t std_mode) {
    uint32_t aiof_id = 0u;
    if (!am34_mmio_read32(AM34_DIGAOS_AIOF_BASE, &aiof_id)) {
        return false;
    }
    switch (std_mode) {
        case DIGAOS_TV_PAL_M:
            digaos_uart0_puts("[TBC] PAL-M baseline stub armed (AIOF safe probe)\n");
            break;
        case DIGAOS_TV_PAL:
            digaos_uart0_puts("[TBC] PAL baseline stub armed\n");
            break;
        case DIGAOS_TV_SECAM:
            digaos_uart0_puts("[TBC] SECAM experimental baseline armed\n");
            break;
        case DIGAOS_TV_NTSC:
        default:
            digaos_uart0_puts("[TBC] NTSC baseline stub armed\n");
            break;
    }
    return true;
}

bool digaos_aiof_apply_tbc_profile(digaos_tv_system_t sys, bool nonstandard_sync, bool mvagc_line_tbc) {
    if (!digaos_aiof_select_tv_system(sys)) {
        return false;
    }
    const uint16_t fs0 =
        (nonstandard_sync || (sys == DIGAOS_TV_PAL) || (sys == DIGAOS_TV_SECAM))
            ? AM34_DIGAOS_AIOF_FS0_NONSTD
            : AM34_DIGAOS_AIOF_FS0_STD;
    const uint16_t yc3 = mvagc_line_tbc ? AM34_DIGAOS_AIOF_YC3_MVAGC_TBC : 0x0000U;
    const uint16_t cont = (sys == DIGAOS_TV_NTSC) ? AM34_DIGAOS_AIOF_CONT_75IRE : AM34_DIGAOS_AIOF_CONT_0IRE;

    if (!am34_mmio_write16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_FS0_OFF, fs0)) {
        return false;
    }
    if (!am34_mmio_write16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_YC3_OFF, yc3)) {
        return false;
    }
    return am34_mmio_write16(AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_CONT_OFF, cont);
}
