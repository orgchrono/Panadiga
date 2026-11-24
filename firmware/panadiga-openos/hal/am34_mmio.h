/**
 * @file am34_mmio.h
 * @brief Matsushita AM34 / MN2DS0009 MMIO Aperture & Silicon Safety Interlocks for Panadiga OpenOS.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Enforces runtime + compile-time barriers sourced from Rust SSOT (`generated/openos_ssot.h`):
 *  - Lethal MMIO register (`0xC000100C..0xC000100F`)
 *  - SYSCON lockup aperture (`0xDC0F0000..0xDC0F0FFF`)
 *  - Permanently Read-Only Layer-1 IPL Flash (`SA0..SA10`, `256 KiB` cached & uncached)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PANADIGA_OPENOS_AM34_MMIO_H
#define PANADIGA_OPENOS_AM34_MMIO_H

#include "generated/openos_ssot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Forbidden & Protected Regions (SSOT) */
#define AM34_OPENOS_LETHAL_MMIO_START       OPENOS_SSOT_LETHAL_MMIO_START
#define AM34_OPENOS_LETHAL_MMIO_END         OPENOS_SSOT_LETHAL_MMIO_END
#define AM34_OPENOS_SYSCON_DANGER_START     OPENOS_SSOT_SYSCON_DANGER_START
#define AM34_OPENOS_SYSCON_DANGER_END       OPENOS_SSOT_SYSCON_DANGER_END
#define AM34_OPENOS_IPL_CS0_BASE            OPENOS_SSOT_IPL_CS0_BASE
#define AM34_OPENOS_IPL_CS0_END             OPENOS_SSOT_IPL_CS0_END
#define AM34_OPENOS_IPL_CS1_BASE            OPENOS_SSOT_IPL_CS1_BASE
#define AM34_OPENOS_IPL_CS1_END             OPENOS_SSOT_IPL_CS1_END

/* SDRAM Boot Slots */
#define AM34_OPENOS_PROG_RAM_BASE            OPENOS_SSOT_PROG_RAM_BASE
#define AM34_OPENOS_MAIN_RAM_BASE            OPENOS_SSOT_MAIN_RAM_BASE

/* Verified AM34 Peripheral Apertures (SSOT) */
#define AM34_OPENOS_SC0_CTR                 OPENOS_SSOT_SC0_CTR
#define AM34_OPENOS_SC0_TXB                 OPENOS_SSOT_SC0_TXB
#define AM34_OPENOS_SC0_RXB                 OPENOS_SSOT_SC0_RXB
#define AM34_OPENOS_SC0_STR                 OPENOS_SSOT_SC0_STR
#define AM34_OPENOS_SC0_TX_BUSY_MASK        OPENOS_SSOT_SC0_TX_BUSY_MASK
#define AM34_OPENOS_SC0_RX_READY_MASK       OPENOS_SSOT_SC0_RX_READY_MASK
#define AM34_OPENOS_SC0_SPIN_LIMIT          OPENOS_SSOT_SC0_SPIN_LIMIT

#define AM34_OPENOS_TBUS_BASE               OPENOS_SSOT_TBUS_BASE
#define AM34_OPENOS_TBUS_MBOX               OPENOS_SSOT_TBUS_MBOX

#define AM34_OPENOS_AIOF_BASE               OPENOS_SSOT_AIOF_BASE
#define AM34_OPENOS_AIOF_VMODESEL_OFF       OPENOS_SSOT_AIOF_VMODESEL_OFF
#define AM34_OPENOS_AIOF_FS0_OFF            OPENOS_SSOT_AIOF_FS0_OFF
#define AM34_OPENOS_AIOF_FS0_STD            OPENOS_SSOT_AIOF_FS0_STD
#define AM34_OPENOS_AIOF_FS0_NONSTD         OPENOS_SSOT_AIOF_FS0_NONSTD
#define AM34_OPENOS_AIOF_YC3_OFF            OPENOS_SSOT_AIOF_YC3_OFF
#define AM34_OPENOS_AIOF_YC3_MVAGC_TBC      OPENOS_SSOT_AIOF_YC3_MVAGC
#define AM34_OPENOS_AIOF_V_OFF              OPENOS_SSOT_AIOF_V_OFF
#define AM34_OPENOS_AIOF_V_ENABLE           OPENOS_SSOT_AIOF_V_ENABLE
#define AM34_OPENOS_AIOF_CONT_OFF           OPENOS_SSOT_AIOF_CONT_OFF
#define AM34_OPENOS_AIOF_CONT_0IRE          OPENOS_SSOT_AIOF_CONT_0IRE
#define AM34_OPENOS_AIOF_CONT_75IRE         OPENOS_SSOT_AIOF_CONT_75IRE
#define AM34_OPENOS_VMODESEL_NTSC           OPENOS_SSOT_VMODESEL_NTSC
#define AM34_OPENOS_VMODESEL_PAL_M          OPENOS_SSOT_VMODESEL_PAL_M
#define AM34_OPENOS_VMODESEL_PAL            OPENOS_SSOT_VMODESEL_PAL

#define AM34_OPENOS_AVDD_BASE               OPENOS_SSOT_AVDD_BASE
#define AM34_OPENOS_AVDD_DACOUT1_OFF        OPENOS_SSOT_AVDD_DACOUT1_OFF
#define AM34_OPENOS_AVDD_DACOUT1_MUTE       0x0000U
#define AM34_OPENOS_AVDD_DACOUT1_ENABLE     0x0007U
#define AM34_OPENOS_AVDD_VOUTMODE0_OFF      OPENOS_SSOT_AVDD_VOUTMODE0_OFF
#define AM34_OPENOS_AVDD_VOUTMODE0_MUTE     0x0000U
#define AM34_OPENOS_AVDD_VOUTMODE0_NORMAL   0x0001U
/* Prototype bit mask (bit 4 colorbar enable on VOUTMODE0) pending live AM34 capture. */
#define AM34_OPENOS_AVDD_VOUTMODE0_COLORBAR 0x0011U
#define AM34_OPENOS_AVDD_REC656IN_OFF       OPENOS_SSOT_AVDD_REC656IN_OFF
#define AM34_OPENOS_AVDD_NPALCTL0_OFF       OPENOS_SSOT_AVDD_NPALCTL0_OFF
#define AM34_OPENOS_AVDD_VDECCTL1_OFF       OPENOS_SSOT_AVDD_VDECCTL1_OFF

#define AM34_OPENOS_IIC0_BASE               OPENOS_SSOT_IIC0_BASE
#define AM34_OPENOS_IIC1_BASE               OPENOS_SSOT_IIC1_BASE
#define AM34_OPENOS_IIC_CTRL_OFF            0x0000UL
#define AM34_OPENOS_IIC_TX_OFF              0x0004UL
#define AM34_OPENOS_IIC_STAT_OFF            0x0008UL
/* Prototype bit mask (bit 0 NACK flag on IIC_STAT) pending live AM34 capture. */
#define AM34_OPENOS_IIC_STAT_NACK_BIT       0x00000001UL
#define AM34_OPENOS_IC3001_BR_BH7629_ADDR8_L 0x90U
#define AM34_OPENOS_IC3001_BR_BH7629_ADDR8_H 0x92U
#define AM34_OPENOS_IC3001_EC_C1AB2379_ADDR8 OPENOS_SSOT_IC3001_EC_ADDR8

#define AM34_OPENOS_SDCD_BASE               OPENOS_SSOT_SDCD_BASE
#define AM34_OPENOS_SDCD_BUS_CFG            OPENOS_SSOT_SDCD_BUS_CFG
#define AM34_OPENOS_SDCD_1BIT_MASK          0x00008000UL

#define AM34_OPENOS_SECTOR_BYTES            OPENOS_SSOT_SECTOR_BYTES
#define AM34_OPENOS_MEIHFS_MBR_LBA          OPENOS_SSOT_MEIHFS_MBR_LBA
#define AM34_OPENOS_MEIHFS_JOURNAL_LBA      OPENOS_SSOT_MEIHFS_JOURNAL_LBA
#define AM34_OPENOS_MEIHFS_MBR_MIRROR_LBA   OPENOS_SSOT_MEIHFS_MBR_MIRROR_LBA
#define AM34_OPENOS_MEIHFS_PRIMARY_FW_LBA   OPENOS_SSOT_MEIHFS_PRIMARY_FW_LBA
#define AM34_OPENOS_MEIHFS_MIRROR_DELTA_LBA OPENOS_SSOT_MEIHFS_MIRROR_DELTA_LBA
#define AM34_OPENOS_MEIHFS_MIN_DISK_LBA     OPENOS_SSOT_MEIHFS_MIN_DISK_LBA
#define AM34_OPENOS_MEIHFS_MAX_BLOCKS_2KB   OPENOS_SSOT_MEIHFS_MAX_BLOCKS_2KB

#define AM34_OPENOS_CONFIG1_ID0117_PAGE     9U
#define AM34_OPENOS_CONFIG1_ID0117_OFF      0x00F0U
#define AM34_OPENOS_ID0117_NTSC             0x00U
#define AM34_OPENOS_ID0117_PAL              0x01U

#define AM34_OPENOS_DAVIS_CH0_BASE          OPENOS_SSOT_DAVIS_CH0_BASE
#define AM34_OPENOS_DAVIS_CH0_RST           OPENOS_SSOT_DAVIS_CH0_RST
#define AM34_OPENOS_DAVIS_CH1_BASE          OPENOS_SSOT_DAVIS_CH1_BASE
#define AM34_OPENOS_DAVIS_CH1_RST           OPENOS_SSOT_DAVIS_CH1_RST
#define OPENOS_SATA_BRIDGE_WAIT_MS          OPENOS_SSOT_SATA_BRIDGE_WAIT_MS

_Static_assert(AM34_OPENOS_LETHAL_MMIO_START == 0xC000100CUL, "0xC000100C lethal lock guard required");
_Static_assert((AM34_OPENOS_IPL_CS1_END - AM34_OPENOS_IPL_CS1_BASE) == 0x00040000UL,
               "IPL Layer-1 guard must span full 256 KiB");
_Static_assert(AM34_OPENOS_AIOF_VMODESEL_OFF == 0x0000UL, "AIOF_VMODESEL offset must match SSOT +0x00");
_Static_assert(AM34_OPENOS_AVDD_DACOUT1_OFF == 0x0404UL, "AVDD_DACOUT1 offset must match SSOT +0x0404");

typedef enum openos_tv_system {
    OPENOS_TV_NTSC = 0,
    OPENOS_TV_PAL_M = 1,
    OPENOS_TV_PAL = 2
} openos_tv_system_t;

typedef enum openos_board_video_ic {
    OPENOS_VIDEO_IC_BH7629KS2_BR_NA = 0,
    OPENOS_VIDEO_IC_C1AB2379_EC = 1
} openos_board_video_ic_t;

static inline bool am34_mmio_is_ipl_addr(uint32_t addr) {
    return ((addr >= AM34_OPENOS_IPL_CS0_BASE) && (addr < AM34_OPENOS_IPL_CS0_END)) ||
           ((addr >= AM34_OPENOS_IPL_CS1_BASE) && (addr < AM34_OPENOS_IPL_CS1_END));
}

/**
 * Returns `true` for ANY forbidden read/write address (`0xC000100C..100F`, `0xDC0F0000..1000`).
 * Writes to IPL `SA0..SA10` are additionally blocked via `am34_mmio_write_allowed`.
 */
static inline bool am34_mmio_is_forbidden(uint32_t addr) {
    if ((addr >= AM34_OPENOS_LETHAL_MMIO_START) && (addr < AM34_OPENOS_LETHAL_MMIO_END)) {
        return true;
    }
    if ((addr >= AM34_OPENOS_SYSCON_DANGER_START) && (addr < AM34_OPENOS_SYSCON_DANGER_END)) {
        return true;
    }
    return false;
}

static inline bool am34_mmio_write_allowed(uint32_t addr) {
    return !am34_mmio_is_forbidden(addr) && !am34_mmio_is_ipl_addr(addr);
}

#if defined(PANADIGA_OPENOS_HOST_TEST) && (PANADIGA_OPENOS_HOST_TEST != 0)
uint32_t openos_host_mmio_read32(uint32_t addr);
uint16_t openos_host_mmio_read16(uint32_t addr);
uint8_t openos_host_mmio_read8(uint32_t addr);
void openos_host_mmio_write32(uint32_t addr, uint32_t val);
void openos_host_mmio_write16(uint32_t addr, uint16_t val);
void openos_host_mmio_write8(uint32_t addr, uint8_t val);
void openos_host_sleep_ms(uint32_t ms);
#else
static inline uint32_t openos_host_mmio_read32(uint32_t addr) {
    return *(volatile const uint32_t *)(uintptr_t)addr;
}
static inline uint16_t openos_host_mmio_read16(uint32_t addr) {
    return *(volatile const uint16_t *)(uintptr_t)addr;
}
static inline uint8_t openos_host_mmio_read8(uint32_t addr) {
    return *(volatile const uint8_t *)(uintptr_t)addr;
}
static inline void openos_host_mmio_write32(uint32_t addr, uint32_t val) {
    *(volatile uint32_t *)(uintptr_t)addr = val;
}
static inline void openos_host_mmio_write16(uint32_t addr, uint16_t val) {
    *(volatile uint16_t *)(uintptr_t)addr = val;
}
static inline void openos_host_mmio_write8(uint32_t addr, uint8_t val) {
    *(volatile uint8_t *)(uintptr_t)addr = val;
}
void openos_host_sleep_ms(uint32_t ms);
#endif

static inline bool am34_mmio_read32(uint32_t addr, uint32_t *out_val) {
    if ((out_val == NULL) || am34_mmio_is_forbidden(addr)) {
        return false;
    }
    *out_val = openos_host_mmio_read32(addr);
    return true;
}

static inline bool am34_mmio_read16(uint32_t addr, uint16_t *out_val) {
    if ((out_val == NULL) || am34_mmio_is_forbidden(addr)) {
        return false;
    }
    *out_val = openos_host_mmio_read16(addr);
    return true;
}

static inline bool am34_mmio_read8(uint32_t addr, uint8_t *out_val) {
    if ((out_val == NULL) || am34_mmio_is_forbidden(addr)) {
        return false;
    }
    *out_val = openos_host_mmio_read8(addr);
    return true;
}

static inline bool am34_mmio_write32(uint32_t addr, uint32_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        return false;
    }
    openos_host_mmio_write32(addr, val);
    return true;
}

static inline bool am34_mmio_write16(uint32_t addr, uint16_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        return false;
    }
    openos_host_mmio_write16(addr, val);
    return true;
}

static inline bool am34_mmio_write8(uint32_t addr, uint8_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        return false;
    }
    openos_host_mmio_write8(addr, val);
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* PANADIGA_OPENOS_AM34_MMIO_H */
