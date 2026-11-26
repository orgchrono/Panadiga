#![deny(unsafe_code)]

//! C11 header (`digaos_ssot.h`) peripheral definitions (AIOF, AVDD, IIC, SDCD, DAVIS, MEIHFS) and struct declarations.

use std::fmt::Write as _;

use crate::constants::{
    AIOF_CONT_OFF, AIOF_CONT_SETUP_0IRE_LIGHTER, AIOF_CONT_SETUP_75IRE_DARKER,
    AIOF_FS0_NON_STANDARD, AIOF_FS0_OFF, AIOF_FS0_STANDARD, AIOF_V_CLOCK_START, AIOF_V_OFF,
    AIOF_VMODESEL_NTSC, AIOF_VMODESEL_OFF, AIOF_VMODESEL_PAL, AIOF_VMODESEL_PAL_M,
    AIOF_YC3_MVAGC_LINE_TBC_BIT, AIOF_YC3_OFF, AM34_AIOF_BASE, AM34_AVDD_BASE, AM34_IIC0_BASE,
    AM34_IIC1_BASE, AM34_SDCD_BUS_CFG_VA, AM34_SDCD_MMIO_BASE, AVDD_DACOUT1_OFF, AVDD_NPALCTL0_OFF,
    AVDD_REC656IN_OFF, AVDD_VDECCTL1_OFF, AVDD_VOUTMODE0_OFF, IC3001_I2C_ADDR_8BIT_WRITE,
    LBA_JOURNAL_CATALOG_OFFSET, LBA_MIRROR_MBR_SECTOR, LBA_PARTITION_1_SECTOR,
    LBA_SECTOR_SIZE_BYTES,
};
use crate::hdd::capacity_patch::specs::DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB;
use crate::re::davis_probe::{
    DAVIS_CH0_CONTROL_REG_VA, DAVIS_CH1_CONTROL_REG_VA, DAVIS_CH1_RESET_REG_VA,
};

use super::DigaOsConfigSpec;
use super::header_gen::{push_def_u8, push_def_u16, push_def_u32};

const DIGAOS_DAVIS_CH0_BASE_ADDR: u32 = 0x8900_0400;
const DIGAOS_MIN_DISK_LBA_5GB: u32 = 0x0093_0E20;
const DIGAOS_SATA_BRIDGE_WAIT_MS: u32 = 1_000;
const DIGAOS_XTAL_NTSC_M_HZ: u32 = 3_579_545;
const DIGAOS_XTAL_PAL_M_HZ: u32 = 3_575_611;
const DIGAOS_P7501_NSW5V_UPSTREAM_MV: u32 = 5_150;
const DIGAOS_P7501_NSW5V_MEASURED_MV: u32 = 3_290;
const DIGAOS_AIOF_REC656_OFF: u32 = 0x0004;
const DIGAOS_AIOF_REC656_NTSC: u16 = 0x0C00;
const DIGAOS_AIOF_COMMON_OFF: u32 = 0x0008;
const DIGAOS_AIOF_COMMON_525: u16 = 0x3C50;
const DIGAOS_AIOF_SYNC_OFF: u32 = 0x000C;
const DIGAOS_AIOF_SYNC_525: u16 = 0x057C;
const DIGAOS_AVDD_CLK_OFF: u32 = 0x0402;
const DIGAOS_AVDD_CLK_480I: u16 = 0x4FB3;
const DIGAOS_AVDD_ACTIVE_SIZE_480I: u32 = 0x01E0_02D0;
const DIGAOS_AVDD_SYNC_480I: u32 = 0x2800_0242;
const DIGAOS_AVDD_CLK_576I: u16 = 0x4BB1;
const DIGAOS_AVDD_ACTIVE_SIZE_576I: u32 = 0x0240_02D0;
const DIGAOS_AVDD_SYNC_576I: u32 = 0x2800_0642;
const DIGAOS_AVDD_DACOUT1_UNMUTED: u16 = 0x0DDF;
const DIGAOS_AVDD_DACOUT1_MUTED: u16 = 0x8D00;
const DIGAOS_AVDD_DACOUT1_MUTE_BIT: u16 = 0x8000;
const DIGAOS_AVDD_NPALCTL0_HI_KEEP: u32 = 0x0005_0000;
const DIGAOS_MAIN3410_CONFIG1_RAM: u32 = 0x5318_4B44;
const DIGAOS_MAIN3410_CFG_PON_REBOOT_VA: u32 = 0x5318_4C04;
const DIGAOS_MAIN3410_CFG_U88_CODE_VA: u32 = 0x5318_59A8;
const DIGAOS_MAIN3410_CFG_TV_SYS_VA: u32 = 0x5318_5E30;
const DIGAOS_MAIN3410_AVOUT_TV_SYS_VA: u32 = 0x515C_D08C;
const DIGAOS_MAIN3410_SYS_POW_STATE_VA: u32 = 0x515C_D4E0;
const DIGAOS_MAIN3410_MARKET_CODE_VA: u32 = 0x5318_6DBC;
const DIGAOS_PROG061_AUTOBOOT_STAMP_VA: u32 = 0x5043_95FC;
const DIGAOS_WAKEUP_XRESET_INVALID: u8 = 0x70;
const DIGAOS_STAGE15_NOR_BYTES: u32 = 0x0004_0000;

pub(super) fn append_peripherals_storage_and_tables(out: &mut String, cfg: &DigaOsConfigSpec) {
    let total_menu_items: usize = cfg.menus.iter().map(|m| m.items.len()).sum();

    push_def_u32(out, "DIGAOS_SSOT_AIOF_BASE", AM34_AIOF_BASE, true);
    push_def_u32(
        out,
        "DIGAOS_SSOT_AIOF_VMODESEL_OFF",
        AIOF_VMODESEL_OFF,
        true,
    );
    push_def_u32(out, "DIGAOS_SSOT_AIOF_FS0_OFF", AIOF_FS0_OFF, true);
    push_def_u16(out, "DIGAOS_SSOT_AIOF_FS0_STD", AIOF_FS0_STANDARD);
    push_def_u16(out, "DIGAOS_SSOT_AIOF_FS0_NONSTD", AIOF_FS0_NON_STANDARD);
    push_def_u32(out, "DIGAOS_SSOT_AIOF_YC3_OFF", AIOF_YC3_OFF, true);
    push_def_u16(
        out,
        "DIGAOS_SSOT_AIOF_YC3_MVAGC",
        AIOF_YC3_MVAGC_LINE_TBC_BIT,
    );
    push_def_u32(out, "DIGAOS_SSOT_AIOF_V_OFF", AIOF_V_OFF, true);
    push_def_u16(out, "DIGAOS_SSOT_AIOF_V_ENABLE", AIOF_V_CLOCK_START);
    push_def_u32(out, "DIGAOS_SSOT_AIOF_CONT_OFF", AIOF_CONT_OFF, true);
    push_def_u16(
        out,
        "DIGAOS_SSOT_AIOF_CONT_0IRE",
        AIOF_CONT_SETUP_0IRE_LIGHTER,
    );
    push_def_u16(
        out,
        "DIGAOS_SSOT_AIOF_CONT_75IRE",
        AIOF_CONT_SETUP_75IRE_DARKER,
    );
    push_def_u16(out, "DIGAOS_SSOT_VMODESEL_NTSC", AIOF_VMODESEL_NTSC);
    push_def_u16(out, "DIGAOS_SSOT_VMODESEL_PAL_M", AIOF_VMODESEL_PAL_M);
    push_def_u16(out, "DIGAOS_SSOT_VMODESEL_PAL", AIOF_VMODESEL_PAL);
    push_def_u32(
        out,
        "DIGAOS_SSOT_XTAL_NTSC_M_HZ",
        DIGAOS_XTAL_NTSC_M_HZ,
        false,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_XTAL_PAL_M_HZ",
        DIGAOS_XTAL_PAL_M_HZ,
        false,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_P7501_NSW5V_UPSTREAM_MV",
        DIGAOS_P7501_NSW5V_UPSTREAM_MV,
        false,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_P7501_NSW5V_MEASURED_MV",
        DIGAOS_P7501_NSW5V_MEASURED_MV,
        false,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_AIOF_REC656_OFF",
        DIGAOS_AIOF_REC656_OFF,
        true,
    );
    push_def_u16(out, "DIGAOS_SSOT_AIOF_REC656_NTSC", DIGAOS_AIOF_REC656_NTSC);
    push_def_u32(
        out,
        "DIGAOS_SSOT_AIOF_COMMON_OFF",
        DIGAOS_AIOF_COMMON_OFF,
        true,
    );
    push_def_u16(out, "DIGAOS_SSOT_AIOF_COMMON_525", DIGAOS_AIOF_COMMON_525);
    push_def_u32(out, "DIGAOS_SSOT_AIOF_SYNC_OFF", DIGAOS_AIOF_SYNC_OFF, true);
    push_def_u16(out, "DIGAOS_SSOT_AIOF_SYNC_525", DIGAOS_AIOF_SYNC_525);

    push_def_u32(out, "DIGAOS_SSOT_AVDD_BASE", AM34_AVDD_BASE, true);
    push_def_u32(out, "DIGAOS_SSOT_AVDD_DACOUT1_OFF", AVDD_DACOUT1_OFF, true);
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_VOUTMODE0_OFF",
        AVDD_VOUTMODE0_OFF,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_REC656IN_OFF",
        AVDD_REC656IN_OFF,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_NPALCTL0_OFF",
        AVDD_NPALCTL0_OFF,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_VDECCTL1_OFF",
        AVDD_VDECCTL1_OFF,
        true,
    );
    push_def_u32(out, "DIGAOS_SSOT_AVDD_CLK_OFF", DIGAOS_AVDD_CLK_OFF, true);
    push_def_u16(out, "DIGAOS_SSOT_AVDD_CLK_480I", DIGAOS_AVDD_CLK_480I);
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_ACTIVE_SIZE_480I",
        DIGAOS_AVDD_ACTIVE_SIZE_480I,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_SYNC_480I",
        DIGAOS_AVDD_SYNC_480I,
        true,
    );
    push_def_u16(out, "DIGAOS_SSOT_AVDD_CLK_576I", DIGAOS_AVDD_CLK_576I);
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_ACTIVE_SIZE_576I",
        DIGAOS_AVDD_ACTIVE_SIZE_576I,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_SYNC_576I",
        DIGAOS_AVDD_SYNC_576I,
        true,
    );
    push_def_u16(
        out,
        "DIGAOS_SSOT_AVDD_DACOUT1_UNMUTED",
        DIGAOS_AVDD_DACOUT1_UNMUTED,
    );
    push_def_u16(
        out,
        "DIGAOS_SSOT_AVDD_DACOUT1_MUTED",
        DIGAOS_AVDD_DACOUT1_MUTED,
    );
    push_def_u16(
        out,
        "DIGAOS_SSOT_AVDD_DACOUT1_MUTE_BIT",
        DIGAOS_AVDD_DACOUT1_MUTE_BIT,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_AVDD_NPALCTL0_HI_KEEP",
        DIGAOS_AVDD_NPALCTL0_HI_KEEP,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MAIN3410_CONFIG1_RAM",
        DIGAOS_MAIN3410_CONFIG1_RAM,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MAIN3410_CFG_PON_REBOOT_VA",
        DIGAOS_MAIN3410_CFG_PON_REBOOT_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MAIN3410_CFG_U88_CODE_VA",
        DIGAOS_MAIN3410_CFG_U88_CODE_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MAIN3410_CFG_TV_SYS_VA",
        DIGAOS_MAIN3410_CFG_TV_SYS_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MAIN3410_AVOUT_TV_SYS_VA",
        DIGAOS_MAIN3410_AVOUT_TV_SYS_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MAIN3410_SYS_POW_STATE_VA",
        DIGAOS_MAIN3410_SYS_POW_STATE_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MAIN3410_MARKET_CODE_VA",
        DIGAOS_MAIN3410_MARKET_CODE_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_PROG061_AUTOBOOT_STAMP_VA",
        DIGAOS_PROG061_AUTOBOOT_STAMP_VA,
        true,
    );
    push_def_u8(
        out,
        "DIGAOS_SSOT_WAKEUP_XRESET_INVALID",
        DIGAOS_WAKEUP_XRESET_INVALID,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_STAGE15_NOR_BYTES",
        DIGAOS_STAGE15_NOR_BYTES,
        true,
    );

    push_def_u32(out, "DIGAOS_SSOT_IIC0_BASE", AM34_IIC0_BASE, true);
    push_def_u32(out, "DIGAOS_SSOT_IIC1_BASE", AM34_IIC1_BASE, true);
    push_def_u8(
        out,
        "DIGAOS_SSOT_IC3001_EC_ADDR8",
        IC3001_I2C_ADDR_8BIT_WRITE,
    );
    push_def_u32(out, "DIGAOS_SSOT_SDCD_BASE", AM34_SDCD_MMIO_BASE, true);
    push_def_u32(out, "DIGAOS_SSOT_SDCD_BUS_CFG", AM34_SDCD_BUS_CFG_VA, true);

    push_def_u32(
        out,
        "DIGAOS_SSOT_DAVIS_CH0_BASE",
        DIGAOS_DAVIS_CH0_BASE_ADDR,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_DAVIS_CH0_RST",
        DAVIS_CH0_CONTROL_REG_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_DAVIS_CH1_BASE",
        DAVIS_CH1_CONTROL_REG_VA,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_DAVIS_CH1_RST",
        DAVIS_CH1_RESET_REG_VA,
        true,
    );
    let wait_ms = DIGAOS_SATA_BRIDGE_WAIT_MS;
    let _ = writeln!(
        out,
        "#define {:<38} {wait_ms}U",
        "DIGAOS_SSOT_SATA_BRIDGE_WAIT_MS"
    );

    push_def_u32(
        out,
        "DIGAOS_SSOT_SECTOR_BYTES",
        u32::try_from(LBA_SECTOR_SIZE_BYTES).expect("sector bytes u32"),
        false,
    );
    push_def_u32(out, "DIGAOS_SSOT_MEIHFS_MBR_LBA", 0, false);
    push_def_u32(
        out,
        "DIGAOS_SSOT_MEIHFS_JOURNAL_LBA",
        u32::try_from(LBA_JOURNAL_CATALOG_OFFSET / LBA_SECTOR_SIZE_BYTES as u64)
            .expect("journal lba u32"),
        false,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MEIHFS_MBR_MIRROR_LBA",
        u32::try_from(LBA_MIRROR_MBR_SECTOR).expect("mirror lba u32"),
        false,
    );
    let pri_fw = u32::try_from(LBA_PARTITION_1_SECTOR).expect("pri fw lba u32");
    push_def_u32(out, "DIGAOS_SSOT_MEIHFS_PRIMARY_FW_LBA", pri_fw, true);
    push_def_u32(out, "DIGAOS_SSOT_MEIHFS_MIRROR_DELTA_LBA", pri_fw * 2, true);
    push_def_u32(
        out,
        "DIGAOS_SSOT_MEIHFS_MIN_DISK_LBA",
        DIGAOS_MIN_DISK_LBA_5GB,
        true,
    );
    push_def_u32(
        out,
        "DIGAOS_SSOT_MEIHFS_MAX_BLOCKS_2KB",
        DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB,
        true,
    );

    let _ = writeln!(
        out,
        "\n#define DIGAOS_SSOT_MENU_ITEM_COUNT     {}u\n\
         #define DIGAOS_SSOT_MENU_COUNT          {}u\n\
         #define DIGAOS_SSOT_THEME_COUNT         {}u\n\
         #define DIGAOS_SSOT_POWER_PROFILE_COUNT {}u\n\
         #define DIGAOS_SSOT_SOC_TARGET_COUNT    {}u\n\n\
         typedef struct digaos_menu_item_s {{\n\
         \x20   const char *id;\n\
         \x20   const char *label_key;\n\
         \x20   const char *command;\n\
         \x20   uint8_t kind;\n\
         \x20   uint8_t min_safety_tier;\n\
         \x20   uint8_t requires_unlock;\n\
         }} digaos_menu_item_t;\n\n\
         typedef struct digaos_menu_s {{\n\
         \x20   const char *id;\n\
         \x20   const char *title_key;\n\
         \x20   uint16_t item_start;\n\
         \x20   uint16_t item_count;\n\
         }} digaos_menu_t;\n\n\
         typedef struct digaos_theme_s {{\n\
         \x20   const char *id;\n\
         \x20   const char *name_key;\n\
         \x20   uint16_t bg_rgb565;\n\
         \x20   uint16_t fg_rgb565;\n\
         \x20   uint16_t accent_rgb565;\n\
         \x20   uint16_t warn_rgb565;\n\
         \x20   uint16_t danger_rgb565;\n\
         \x20   uint8_t osd_font_mode;\n\
         \x20   uint8_t overscan_px;\n\
         \x20   uint8_t contrast_level;\n\
         }} digaos_theme_t;\n\n\
         typedef struct digaos_power_profile_s {{\n\
         \x20   const char *id;\n\
         \x20   const char *name_key;\n\
         \x20   const char *soc_core_id;\n\
         \x20   uint8_t tbc_mode;\n\
         \x20   uint8_t aiof_fs0_mode;\n\
         \x20   uint8_t avdd_gain_preset;\n\
         \x20   uint8_t sata_udma_mode;\n\
         \x20   uint8_t dram_refresh_preset;\n\
         \x20   uint32_t uart_baud;\n\
         \x20   uint8_t unlock_required;\n\
         \x20   uint8_t lethal_mmio_guard;\n\
         \x20   uint8_t ipl_ro_guard;\n\
         }} digaos_power_profile_t;\n\n\
         typedef struct digaos_soc_target_s {{\n\
         \x20   const char *soc_core_id;\n\
         \x20   const char *soc_id;\n\
         \x20   const char *isa;\n\
         \x20   uint8_t is_big_endian;\n\
         \x20   uint8_t mmio_verified;\n\
         \x20   uint8_t is_primary_executable;\n\
         \x20   uint32_t logic_voltage_mv;\n\
         }} digaos_soc_target_t;\n\n\
         extern const digaos_menu_item_t k_digaos_ssot_menu_items[DIGAOS_SSOT_MENU_ITEM_COUNT];\n\
         extern const digaos_menu_t k_digaos_ssot_menus[DIGAOS_SSOT_MENU_COUNT];\n\
         extern const digaos_theme_t k_digaos_ssot_themes[DIGAOS_SSOT_THEME_COUNT];\n\
         extern const digaos_power_profile_t k_digaos_ssot_power_profiles[DIGAOS_SSOT_POWER_PROFILE_COUNT];\n\
         extern const digaos_soc_target_t k_digaos_ssot_soc_targets[DIGAOS_SSOT_SOC_TARGET_COUNT];\n",
        total_menu_items,
        cfg.menus.len(),
        cfg.themes.len(),
        cfg.power_profiles.len(),
        cfg.soc_targets.len(),
    );
}
