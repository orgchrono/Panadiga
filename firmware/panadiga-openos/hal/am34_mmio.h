/**
 * @file am34_mmio.h
 * @brief Matsushita AM34 / MN2DS0009 MMIO Aperture & Silicon Safety Interlocks for Panadiga OpenOS.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Enforces runtime + compile-time barriers against:
 *  - Lethal MMIO register (`0xC000100C..0xC000100F`)
 *  - SYSCON lockup aperture (`0xDC0F0000..0xDC0F0FFF`)
 *  - Permanently Read-Only Layer-1 IPL Flash (`SA0..SA10`, `256 KiB` cached & uncached)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PANADIGA_OPENOS_AM34_MMIO_H
#define PANADIGA_OPENOS_AM34_MMIO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Forbidden & Protected Regions */
#define AM34_OPENOS_LETHAL_MMIO_START   0xC000100CUL
#define AM34_OPENOS_LETHAL_MMIO_END     0xC0001010UL
#define AM34_OPENOS_SYSCON_DANGER_START 0xDC0F0000UL
#define AM34_OPENOS_SYSCON_DANGER_END   0xDC0F1000UL
#define AM34_OPENOS_IPL_CS0_BASE        0x80000000UL
#define AM34_OPENOS_IPL_CS0_END         0x80040000UL
#define AM34_OPENOS_IPL_CS1_BASE        0x84000000UL
#define AM34_OPENOS_IPL_CS1_END         0x84040000UL

/* SDRAM Boot Slots */
#define AM34_OPENOS_STAGE15_BASE        0x50400000UL
#define AM34_OPENOS_STAGE2_BASE         0x57000000UL

/* Verified AM34 Peripheral Apertures */
#define AM34_OPENOS_SC0_CTR             0xD4002000UL
#define AM34_OPENOS_SC0_TXB             0xD4002008UL
#define AM34_OPENOS_SC0_RXB             0xD4002009UL
#define AM34_OPENOS_SC0_STR             0xD400200CUL
#define AM34_OPENOS_SC0_TX_BUSY_MASK    0x00A0U
#define AM34_OPENOS_SC0_SPIN_LIMIT      0x00040000UL

#define AM34_OPENOS_TBUS_BASE           0xDC010000UL
#define AM34_OPENOS_TBUS_MBOX           0xDC010010UL

#define AM34_OPENOS_AIOF_BASE           0x88000000UL
#define AM34_OPENOS_AIOF_VMODESEL_OFF   0x0000UL
#define AM34_OPENOS_AIOF_FS0_OFF        0x002CUL
#define AM34_OPENOS_AIOF_FS0_STD        0x40F0U
#define AM34_OPENOS_AIOF_FS0_NONSTD     0xC0F0U
#define AM34_OPENOS_AIOF_YC3_OFF        0x0064UL
#define AM34_OPENOS_AIOF_YC3_MVAGC_TBC  0x0020U
#define AM34_OPENOS_AIOF_V_OFF          0x0066UL
#define AM34_OPENOS_AIOF_V_ENABLE       0x8001U
#define AM34_OPENOS_AIOF_CONT_OFF       0x00A4UL
#define AM34_OPENOS_AIOF_CONT_0IRE      0x4000U
#define AM34_OPENOS_AIOF_CONT_75IRE     0x8000U
#define AM34_OPENOS_VMODESEL_NTSC       0x8020U
#define AM34_OPENOS_VMODESEL_PAL_M      0x8021U
#define AM34_OPENOS_VMODESEL_PAL        0x8027U

#define AM34_OPENOS_AVDD_BASE           0x89800000UL
#define AM34_OPENOS_AVDD_DACOUT1_OFF    0x0404UL
#define AM34_OPENOS_AVDD_DACOUT1_MUTE   0x0000U
#define AM34_OPENOS_AVDD_DACOUT1_ENABLE 0x0007U
#define AM34_OPENOS_AVDD_VOUTMODE0_OFF  0x0484UL
#define AM34_OPENOS_AVDD_VOUTMODE0_MUTE 0x0000U
#define AM34_OPENOS_AVDD_VOUTMODE0_NORMAL 0x0001U
/* Prototype bit mask (bit 4 colorbar enable on VOUTMODE0) pending live AM34 capture. */
#define AM34_OPENOS_AVDD_VOUTMODE0_COLORBAR 0x0011U
#define AM34_OPENOS_AVDD_REC656IN_OFF   0x050CUL
#define AM34_OPENOS_AVDD_NPALCTL0_OFF   0x0624UL
#define AM34_OPENOS_AVDD_VDECCTL1_OFF   0x0858UL

#define AM34_OPENOS_IIC0_BASE           0xD8400000UL
#define AM34_OPENOS_IIC1_BASE           0xD8401000UL
#define AM34_OPENOS_IIC_CTRL_OFF        0x0000UL
#define AM34_OPENOS_IIC_TX_OFF          0x0004UL
#define AM34_OPENOS_IIC_STAT_OFF        0x0008UL
/* Prototype bit mask (bit 0 NACK flag on IIC_STAT) pending live AM34 capture. */
#define AM34_OPENOS_IIC_STAT_NACK_BIT   0x00000001UL
#define AM34_OPENOS_IC3001_BR_BH7629_ADDR8_L 0x90U
#define AM34_OPENOS_IC3001_BR_BH7629_ADDR8_H 0x92U
#define AM34_OPENOS_IC3001_EC_C1AB2379_ADDR8 0x94U
#define AM34_OPENOS_SDCD_BASE           0x89001800UL

#define AM34_OPENOS_SECTOR_BYTES        512UL
#define AM34_OPENOS_MEIHFS_MBR_LBA      0UL
#define AM34_OPENOS_MEIHFS_JOURNAL_LBA  16384UL
#define AM34_OPENOS_MEIHFS_MBR_MIRROR_LBA 57344UL
#define AM34_OPENOS_MEIHFS_PRIMARY_FW_LBA 0x00010000UL
#define AM34_OPENOS_MEIHFS_MIRROR_DELTA_LBA 0x00020000UL
#define AM34_OPENOS_MEIHFS_MIN_DISK_LBA 0x00930E20UL /* ~5 GB SV0511D floor */
#define AM34_OPENOS_MEIHFS_MAX_BLOCKS_2KB 0x3FFFFFFFUL /* 2 TiB dynamic unclamped cap */

#define AM34_OPENOS_CONFIG1_ID0117_PAGE 9U
#define AM34_OPENOS_CONFIG1_ID0117_OFF  0x00F0U
#define AM34_OPENOS_ID0117_NTSC         0x00U
#define AM34_OPENOS_ID0117_PAL          0x01U

#define AM34_OPENOS_DAVIS_CH0_BASE      0x89000400UL
#define AM34_OPENOS_DAVIS_CH0_RST       0x89000420UL
#define AM34_OPENOS_DAVIS_CH1_BASE      0x89000C00UL
#define AM34_OPENOS_DAVIS_CH1_RST       0x89000C20UL
#define OPENOS_SATA_BRIDGE_WAIT_MS      1000U

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
