/**
 * @file aiof_video.c
 * @brief AM34 `AIOF` TBC & Composite/S-Video Standard Selector (`NTSC` default, `PAL-M`, `PAL`).
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

openos_tv_system_t openos_tv_from_nfrom_id0117(uint32_t raw_id0117) {
    return ((raw_id0117 & 0xFFu) == AM34_OPENOS_ID0117_NTSC) ? OPENOS_TV_NTSC : OPENOS_TV_PAL;
}

bool openos_aiof_select_tv_system(openos_tv_system_t sys) {
    uint16_t vmodesel = AM34_OPENOS_VMODESEL_NTSC;
    if (sys == OPENOS_TV_PAL_M) {
        vmodesel = AM34_OPENOS_VMODESEL_PAL_M;
    } else if (sys == OPENOS_TV_PAL) {
        vmodesel = AM34_OPENOS_VMODESEL_PAL;
    }

    if (!am34_mmio_write16(AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_V_OFF,
                           AM34_OPENOS_AIOF_V_ENABLE)) {
        return false;
    }
    return am34_mmio_write16(AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_VMODESEL_OFF, vmodesel);
}

bool openos_aiof_apply_tbc_profile(openos_tv_system_t sys, bool nonstandard_sync, bool mvagc_line_tbc) {
    if (!openos_aiof_select_tv_system(sys)) {
        return false;
    }
    const uint16_t fs0 = nonstandard_sync ? AM34_OPENOS_AIOF_FS0_NONSTD : AM34_OPENOS_AIOF_FS0_STD;
    const uint16_t yc3 = mvagc_line_tbc ? AM34_OPENOS_AIOF_YC3_MVAGC_TBC : 0x0000U;
    const uint16_t cont = (sys == OPENOS_TV_NTSC) ? AM34_OPENOS_AIOF_CONT_75IRE : AM34_OPENOS_AIOF_CONT_0IRE;

    if (!am34_mmio_write16(AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_FS0_OFF, fs0)) {
        return false;
    }
    if (!am34_mmio_write16(AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_YC3_OFF, yc3)) {
        return false;
    }
    return am34_mmio_write16(AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_CONT_OFF, cont);
}
