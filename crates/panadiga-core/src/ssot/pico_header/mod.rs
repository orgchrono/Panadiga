//! Deterministic C11 SSOT header & `.rodata` table generator for `PicoFW` (`generated/picofw_ssot.h`).

mod header_gen;
mod header_sections;
mod tables_gen;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub use header_gen::generate_c11_ssot_header;
pub(super) use header_gen::resolve_eh55_verified_nor_context;
pub use tables_gen::generate_c11_ssot_tables;

/// Single-byte `_dev_SERIAL` RSP entry ACK/NAK strobe confirmed empirically in `3410PQ`.
pub const DEV_SERIAL_NAK_BYTE: u8 = 0x15;

/// Verified P7501 (`B6B-ZR-3R4`) RP2040/RP2350 GPIO pin assignments.
pub const P7501_GPIO_RX_GP2: u8 = 2;
pub const P7501_GPIO_NSW5V_HIZ_GP3: u8 = 3;
pub const P7501_GPIO_TX_GP4: u8 = 4;
pub const P7501_GPIO_XRESET_OD_GP5: u8 = 5;
pub const P7501_GPIO_SW_UART_GP26: u8 = 26;

/// Canonical integer millivolt electrical thresholds (zero float drift in C11 core).
pub const P7501_VDD_MIN_MV: u32 = 2500;
pub const P7501_VDD_WAKE_MIN_MV: u32 = 2000;
pub const P7501_VDD_PULLDOWN_MIN_MV: u32 = 1500;
pub const P7501_VDD_SAFE_MIN_MV: u32 = 3000;
pub const P7501_VDD_NOMINAL_MV: u32 = 3300;
pub const P7501_VDD_FLOATING_CEIL_MV: u32 = 3450;

/// Canonical Pico hardware watchdog timeout (ms).
pub const P7501_WDT_TIMEOUT_MS_SSOT: u32 = 4000;
/// Canonical CLI command line maximum capacity (bytes, including NUL).
pub const P7501_CLI_LINE_MAX_SSOT: u32 = 512;
/// Canonical CLI `FAST_WRITE` max raw binary bytes per single-line command (`<= 128 B` fits `P7501_CLI_LINE_MAX_SSOT`).
pub const P7501_MAX_FAST_WRITE_BYTES_SSOT: u32 = 128;
/// Canonical fast binary stub (`0x9043A000`) NOR page window size (bytes).
pub const P7501_FAST_STUB_PAGE_BYTES_SSOT: u32 = 256;
/// Canonical default JIG SW_UART LOW hold duration (ms).
pub const P7501_JIG_DEFAULT_HOLD_MS_SSOT: u32 = 2500;
/// Canonical GDB RSP sector write-buffer cache size (bytes, bounded for RP2040/RP2350 WDT safety).
pub const P7501_GDB_SECTOR_CACHE_SIZE_SSOT: u32 = 512;
/// Canonical lethal AM34 MMIO register span (bytes: `0xC000100C..0xC000100F`).
pub const P7501_AM34_LETHAL_REG_LEN_SSOT: u32 = 4;
/// Secondary AM34 lethal MMIO trap (`0xDC0F0000..0xDC0F000F`; preserves safe `0xDC0F0080` DAVIS stub).
pub const P7501_AM34_LETHAL_REG2_ADDR_SSOT: u32 = 0xDC0F_0000;
pub const P7501_AM34_LETHAL_REG2_LEN_SSOT: u32 = 0x10;

/// Verified `IC3001` (`BH7629KS2` BR/NA vs `C1AB00002379` EC) 7-bit I2C addresses.
pub const P7501_IC3001_BR_NA_ADDR7_L_SSOT: u8 = 0x48;
pub const P7501_IC3001_BR_NA_ADDR7_H_SSOT: u8 = 0x49;

/// Domain flags for `k_p7501_ssot_mmio_regs[]`.
pub const P7501_MMIO_DOM_CORE: u8 = 1;
pub const P7501_MMIO_DOM_VIDEO: u8 = 2;
pub const P7501_MMIO_DOM_STORAGE: u8 = 4;

/// Binary signature locator table exported to Pico C11 (`k_p7501_ssot_sigs[]`).
pub(super) const PICO_SSOT_SIGNATURES: &[(&str, &[u8])] = &[
    ("EXTRHEAD", b"EXTRHEAD"),
    ("EXTRFOOT", b"EXTRFOOT"),
    ("BGO_P1", &[0x01, 0xF1]),
    ("EEMUL_1284", b"1284"),
    ("MEIHFS_SH", b"SH"),
    ("AVIN_IIC_94", &[0x2D, 0x94, 0x00, 0xDD]),
    ("LOADER_061", b"Loader ver0.61"),
    ("MAIN_PIE", b"PiE-OS"),
];

pub(super) const P7501_TRAP_R: u8 = 1;
pub(super) const P7501_TRAP_W: u8 = 2;
pub(super) const P7501_TRAP_E: u8 = 4;
pub(super) const P7501_TRAP_SEV_WARN: u8 = 1;
pub(super) const P7501_TRAP_SEV_DENY: u8 = 2;

pub(super) const fn pico_trap_specs() -> [(&'static str, u32, u32, u8, u8, &'static str); 7] {
    use crate::constants::{
        AM34_FLASH_BASE, AM34_FLASH_BGO_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE,
        AM34_FLASH_UNCACHED_BASE, AM34_LETHAL_MMIO_ADDR,
    };
    let cs0_mirror_len = 0x0400_0000 - AM34_FLASH_SIZE;
    [
        (
            "LETHAL_MMIO_C000100C",
            AM34_LETHAL_MMIO_ADDR,
            P7501_AM34_LETHAL_REG_LEN_SSOT,
            P7501_TRAP_R | P7501_TRAP_W | P7501_TRAP_E,
            P7501_TRAP_SEV_DENY,
            "AM34 WDT/bus hardware hang on read/write/erase",
        ),
        (
            "LETHAL_MMIO_DC0F0000",
            P7501_AM34_LETHAL_REG2_ADDR_SSOT,
            P7501_AM34_LETHAL_REG2_LEN_SSOT,
            P7501_TRAP_R | P7501_TRAP_W | P7501_TRAP_E,
            P7501_TRAP_SEV_DENY,
            "AM34 SysCtrl/DAVIS bus-stall trap range",
        ),
        (
            "IPL_L1_CACHED_RO",
            AM34_FLASH_BASE,
            AM34_FLASH_CRITICAL_IPL_SIZE,
            P7501_TRAP_W | P7501_TRAP_E,
            P7501_TRAP_SEV_DENY,
            "SA0..SA10 critical boot IPL permanently read-only (E05)",
        ),
        (
            "IPL_L1_UNCACHED_RO",
            AM34_FLASH_UNCACHED_BASE,
            AM34_FLASH_CRITICAL_IPL_SIZE,
            P7501_TRAP_W | P7501_TRAP_E,
            P7501_TRAP_SEV_DENY,
            "SA0..SA10 uncached boot IPL permanently read-only (E05)",
        ),
        (
            "CS0_MIRROR_CACHED",
            AM34_FLASH_BASE + AM34_FLASH_SIZE,
            cs0_mirror_len,
            P7501_TRAP_W | P7501_TRAP_E,
            P7501_TRAP_SEV_DENY,
            "8MB flash CS0 cached alias mirror (blocks phantom wrap writes)",
        ),
        (
            "CS0_MIRROR_UNCACHED",
            AM34_FLASH_UNCACHED_BASE + AM34_FLASH_SIZE,
            cs0_mirror_len,
            P7501_TRAP_W | P7501_TRAP_E,
            P7501_TRAP_SEV_DENY,
            "8MB flash CS0 uncached alias mirror (blocks phantom wrap writes)",
        ),
        (
            "BGO_4MB_ALIAS_TRAP",
            AM34_FLASH_BGO_BASE,
            0x0001_0000,
            P7501_TRAP_W | P7501_TRAP_E,
            P7501_TRAP_SEV_WARN,
            "BGO_4MB_ALIAS_TRAP_USE_84130000 (SA39 mid-FTL vs EH55 0x84130000 SA26)",
        ),
    ]
}

pub(super) const fn pico_mmio_reg_specs() -> [(u32, &'static str, u8, u8); 31] {
    use crate::constants::{
        AIOF_CONT_OFF, AIOF_FS0_OFF, AIOF_RDDET_OFF, AIOF_SDET_OFF, AIOF_TEST1_OFF,
        AIOF_VMODDET_OFF, AIOF_VMODESEL_OFF, AIOF_VOUTSEL_OFF, AIOF_YC3_OFF, AM34_AIOF_BASE,
        AM34_AVDD_BASE, AM34_DAVIS_PRI_MMIO_BASE, AM34_DAVIS_SEC_MMIO_BASE, AM34_IC7501_TBUS_BASE,
        AM34_IC7501_TBUS_TX_OFF, AM34_IIC0_BASE, AM34_IIC1_BASE, AM34_SC0_BASE, AM34_SC0_STR_VA,
        AM34_SC0_TXB_VA, AM34_SDCD_BUS_CFG_VA, AM34_SDCD_MMIO_BASE, AM34_SYS_STATUS_REG,
        AM34_WDBC_REG, AVDD_DACOUT1_OFF, AVDD_NPALCTL0_OFF, AVDD_VDECCTL1_OFF, AVDD_VOUTMODE0_OFF,
    };
    use crate::re::davis_probe::{DAVIS_CH0_CONTROL_REG_VA, DAVIS_CH1_RESET_REG_VA};
    use crate::re::main_passthrough::{DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_ISR_CTX_PTR_VA};
    let c = P7501_MMIO_DOM_CORE;
    let v = P7501_MMIO_DOM_VIDEO;
    let s = P7501_MMIO_DOM_STORAGE;
    [
        // Core (8) — preserves exact CORE snapshot backwards compatibility
        (AM34_SC0_TXB_VA, "SC0TXB", c, 32),
        (AM34_SC0_BASE, "SC0CTR", c, 32),
        (AM34_SC0_STR_VA, "SC0STR", c, 32),
        (AM34_WDBC_REG, "OCD_CTRL", c, 32),
        (AM34_SYS_STATUS_REG, "OCD_STAT", c, 32),
        (0x1000_0000, "SYS_ID", c, 32),
        (DEV_SERIAL_CALLBACK_VA, "SER_CB", c, 32),
        (DEV_SERIAL_ISR_CTX_PTR_VA, "ISR_CTX", c, 32),
        // Video (15) — strictly 32-bit aligned; AIOF_YC3_VCLK bundles +0x64 (YC3) & +0x66 (AIOF_V)
        (AM34_AIOF_BASE + AIOF_VMODESEL_OFF, "AIOF_VMOD", v, 16),
        (AM34_AIOF_BASE + AIOF_FS0_OFF, "AIOF_FS0", v, 16),
        (AM34_AIOF_BASE + AIOF_SDET_OFF, "AIOF_SDET", v, 16),
        (AM34_AIOF_BASE + AIOF_VMODDET_OFF, "AIOF_VDET", v, 16),
        (AM34_AIOF_BASE + AIOF_VOUTSEL_OFF, "AIOF_VOUT", v, 16),
        (AM34_AIOF_BASE + AIOF_YC3_OFF, "AIOF_YC3V", v, 32),
        (AM34_AIOF_BASE + AIOF_TEST1_OFF, "AIOF_TST1", v, 16),
        (AM34_AIOF_BASE + AIOF_RDDET_OFF, "AIOF_RDET", v, 16),
        (AM34_AIOF_BASE + AIOF_CONT_OFF, "AIOF_CONT", v, 16),
        (AM34_AVDD_BASE + AVDD_DACOUT1_OFF, "AVDD_DAC1", v, 16),
        (AM34_AVDD_BASE + AVDD_VOUTMODE0_OFF, "AVDD_VOUT", v, 16),
        (AM34_AVDD_BASE + AVDD_NPALCTL0_OFF, "AVDD_NPAL", v, 16),
        (AM34_AVDD_BASE + AVDD_VDECCTL1_OFF, "AVDD_VDEC", v, 16),
        (AM34_IIC0_BASE, "IIC0_BASE", v, 32),
        (AM34_IIC1_BASE, "IIC1_BASE", v, 32),
        // Storage / Sys (8)
        (AM34_DAVIS_PRI_MMIO_BASE, "DAVIS_PRI", s, 32),
        (AM34_DAVIS_SEC_MMIO_BASE, "DAVIS_SEC", s, 32),
        (AM34_SDCD_MMIO_BASE, "SDCD_BASE", s, 32),
        (AM34_IC7501_TBUS_BASE, "TBUS_BASE", s, 32),
        (DAVIS_CH0_CONTROL_REG_VA, "DAVIS_PRST", s, 16),
        (DAVIS_CH1_RESET_REG_VA, "DAVIS_SRST", s, 16),
        (AM34_SDCD_BUS_CFG_VA, "SDCD_CFG4", s, 32),
        (
            AM34_IC7501_TBUS_BASE + AM34_IC7501_TBUS_TX_OFF,
            "TBUS_MBX0",
            s,
            32,
        ),
    ]
}

/// Verified PiE-OS / Loader call-chain anchors (`LOADER_VER061`, `MAIN3060NP`, `MAIN3410PQ`).
/// Tuple: `(name, va_3060np, va_3410pq, mmio_ref, stage_id, flags, caller)`.
pub(super) const fn pico_pieos_anchor_specs()
-> [(&'static str, u32, u32, u32, u8, u8, &'static str); 11] {
    [
        (
            "uart0_printf",
            0x4400_5DEE,
            0x4400_5DEE,
            0xD400_2000,
            1,
            0x01,
            "loader_main",
        ),
        (
            "norfrom_fvu_load_check",
            0x4400_894B,
            0x4400_894B,
            0x8404_0000,
            2,
            0x01,
            "loader_main",
        ),
        (
            "nfd_file_exists",
            0x4400_A1F1,
            0x4400_A1F1,
            0x8413_0000,
            2,
            0x01,
            "norfrom_fvu_load_check",
        ),
        (
            "crt0_data_copy_bss",
            0x5700_0B08,
            0x5700_0B08,
            0x531B_C000,
            5,
            0x00,
            "main_entry",
        ),
        (
            "aiof_ic3001_pal_m_setup",
            0x0000_0000,
            0x570D_627E,
            0x8800_0000,
            8,
            0x04,
            "sys_boot",
        ),
        (
            "rtsd_ata_hard_reset_wait",
            0x5711_226E,
            0x5710_A98C,
            0x8900_0C00,
            3,
            0x04,
            "rtsd_hdd_init",
        ),
        (
            "NFD_Init",
            0x574A_68E9,
            0x0000_0000,
            0x8413_0000,
            7,
            0x02,
            "BGO_Init_entry",
        ),
        (
            "BGO_Init_entry",
            0x574A_F2F0,
            0x0000_0000,
            0x8413_0000,
            7,
            0x00,
            "sys_boot",
        ),
        (
            "hdd_cmd_rtsd_capacity",
            0x574E_499C,
            0x574D_D1FC,
            0x8900_0D00,
            9,
            0x00,
            "rtsd_mount",
        ),
        (
            "rtsd_sleep_10ms",
            0x574E_6BBD,
            0x574D_F538,
            0x0000_0000,
            3,
            0x00,
            "rtsd_ata_hard_reset_wait",
        ),
        (
            "av_passthrough_tbc_gates",
            0x0000_0000,
            0x574F_7B8C,
            0x8800_0000,
            8,
            0x04,
            "av_passthrough_tbc_entry",
        ),
    ]
}

/// Canonical regional model suffixes projected from `panadiga_profiles::region` into C11 `.rodata`.
pub(super) const CANONICAL_REGION_SUFFIXES: &[&str] = &[
    "LB", "LBS", "P", "VP", "PC", "EC", "EB", "EG", "EP", "EF", "EE", "GC", "GCS", "GN",
];

pub(super) struct PicoRegionRuleSpec {
    pub suffix: &'static str,
    pub tv_standard: String,
    pub line_system: String,
    pub aiof_vmodesel: u16,
    pub dvd_region: u8,
    pub oem_marker: char,
    pub ic3001_addr7_primary: u8,
    pub ic3001_addr7_alt: u8,
}

pub(super) fn pico_region_rule_specs() -> Vec<PicoRegionRuleSpec> {
    CANONICAL_REGION_SUFFIXES
        .iter()
        .map(|&code| {
            let v =
                panadiga_profiles::region::interpolate_region_variant(code, "DMR-EH55", "", None);
            let aiof_vmodesel =
                u16::from_str_radix(v.aiof_vmodesel_hex.trim_start_matches("0x"), 16)
                    .expect("valid aiof_vmodesel hex");
            let oem_marker = v.oem_marker.chars().next().unwrap_or('E');
            let (ic3001_addr7_primary, ic3001_addr7_alt) =
                if matches!(code, "LB" | "LBS" | "P" | "VP" | "PC") {
                    (0x48, 0x49)
                } else {
                    (0x4A, 0x48)
                };
            PicoRegionRuleSpec {
                suffix: code,
                tv_standard: v.tv_standard,
                line_system: v.video_line_system,
                aiof_vmodesel,
                dvd_region: v.dvd_region_code,
                oem_marker,
                ic3001_addr7_primary,
                ic3001_addr7_alt,
            }
        })
        .collect()
}

/// Canonical UART boot diagnostic signature rule (`needle`, `c11_event_macro`).
pub const DIAG_SIGNATURE_RULES: &[(&str, &str)] = &[
    ("DDR CHECK: OK", "PICOFW_SSOT_DIAG_EVT_DDR_OK"),
    ("serial boot mode", "PICOFW_SSOT_DIAG_EVT_SERIAL_BOOT"),
    ("... SD init end", "PICOFW_SSOT_DIAG_EVT_SD_HW_OK"),
    ("-91006", "PICOFW_SSOT_DIAG_EVT_SD_ERR"),
    ("-90002", "PICOFW_SSOT_DIAG_EVT_SD_ERR"),
    ("-90001", "PICOFW_SSOT_DIAG_EVT_SD_ERR"),
    ("FAT Mount failed", "PICOFW_SSOT_DIAG_EVT_FAT_FAIL"),
    ("FAT Open(", "PICOFW_SSOT_DIAG_EVT_FAT_FAIL"),
    (
        "SD diag load failed",
        "PICOFW_SSOT_DIAG_EVT_SD_DIAG_LOAD_FAIL",
    ),
    (
        "diag loader uncompress failed",
        "PICOFW_SSOT_DIAG_EVT_SD_UNCOMPRESS_FAIL",
    ),
    (
        "fvu uncompress failed",
        "PICOFW_SSOT_DIAG_EVT_SD_UNCOMPRESS_FAIL",
    ),
    ("no header!", "PICOFW_SSOT_DIAG_EVT_SD_UNCOMPRESS_FAIL"),
    ("no footer!", "PICOFW_SSOT_DIAG_EVT_SD_UNCOMPRESS_FAIL"),
    (
        "check sum mismatch",
        "PICOFW_SSOT_DIAG_EVT_SD_UNCOMPRESS_FAIL",
    ),
    ("*** start diag ***", "PICOFW_SSOT_DIAG_EVT_DIAG_EXEC"),
    ("diag loader is preserved", "PICOFW_SSOT_DIAG_EVT_DIAG_EXEC"),
    ("** open tray **", "PICOFW_SSOT_DIAG_EVT_ODD_OPEN"),
    ("can not close tray", "PICOFW_SSOT_DIAG_EVT_ODD_TRAY_FAIL"),
    ("80045300", "PICOFW_SSOT_DIAG_EVT_ODD_TRAY_FAIL"),
    ("80040904", "PICOFW_SSOT_DIAG_EVT_CD_APC_ERR"),
    ("83A7", "PICOFW_SSOT_DIAG_EVT_CD_APC_ERR"),
    ("BGO driver init failed", "PICOFW_SSOT_DIAG_EVT_BGO_FAIL"),
    ("hung-up time changed", "PICOFW_SSOT_DIAG_EVT_HUNG_U99"),
    ("PANAMON1>", "PICOFW_SSOT_DIAG_EVT_PANAMON"),
];

pub(super) const FLASH_CHIPS_ORDER: &[(&str, &str)] = &[
    ("spansion_s29jl064h_8mb", "PICOFW_FLASH_ID_S29JL064H_8MB"),
    (
        "spansion_s29jl032h_4mb_top",
        "PICOFW_FLASH_ID_S29JL032H_4MB",
    ),
    (
        "spansion_s29al032d_4mb_top",
        "PICOFW_FLASH_ID_S29AL032D_4MB",
    ),
    (
        "spansion_s29al016d_2mb_top",
        "PICOFW_FLASH_ID_S29AL016D_2MB",
    ),
    ("spansion_s29gl128n_16mb", "PICOFW_FLASH_ID_S29GL128N_16MB"),
    (
        "macronix_mx29lv320_4mb_bottom",
        "PICOFW_FLASH_ID_MX29LV320B_4MB",
    ),
    (
        "macronix_mx29lv320_4mb_top",
        "PICOFW_FLASH_ID_MX29LV320T_4MB",
    ),
    ("oki_mr27v802_1mb_otp", "PICOFW_FLASH_ID_OKI_MR27V802_1MB"),
];

pub(super) const TARGET_PROFILES_ORDER: &[(&str, &str)] = &[
    ("DMR-EH55", "PICOFW_TARGET_PROFILE_DMR_EH55"),
    ("DMR-ES35V", "PICOFW_TARGET_PROFILE_DMR_ES35V"),
    ("DMR-EZ485V", "PICOFW_TARGET_PROFILE_DMR_EZ485V"),
    ("DMR-ES10", "PICOFW_TARGET_PROFILE_DMR_ES10"),
];

pub(super) const LOADER_FINGERPRINTS: &[(&str, &str, &str, bool)] = &[
    (
        "Loader ver0.61",
        "DMR-EH55",
        "PICOFW_TARGET_PROFILE_DMR_EH55",
        false,
    ),
    (
        "ver0.18",
        "DMR-ES35V",
        "PICOFW_TARGET_PROFILE_DMR_ES35V",
        true,
    ),
    (
        "EZ485",
        "DMR-EZ485V",
        "PICOFW_TARGET_PROFILE_DMR_EZ485V",
        true,
    ),
    ("ES10", "DMR-ES10", "PICOFW_TARGET_PROFILE_DMR_ES10", true),
];

/// Computes the deterministic FNV-1a contract hash over generated Pico C11 SSOT tables & wire contracts.
#[must_use]
pub fn compute_pico_ssot_hash() -> u32 {
    let mut material = generate_c11_ssot_tables().into_bytes();
    for &(sym, val) in crate::constants::events::PICOFW_WIRE_TAGS
        .iter()
        .chain(crate::constants::events::PICOFW_RES_CODES.iter())
    {
        material.push(0);
        material.extend_from_slice(sym.as_bytes());
        material.push(b'=');
        material.extend_from_slice(val.as_bytes());
    }
    header_sections::compute_fnv1a_u32(&material)
}

/// Renders compatibility wrapper `firmware/picofw/uart/picofw_pana_rsp_ssot.h` forwarding to `generated/picofw_ssot.h`.
pub fn generate_pana_rsp_ssot_header() -> String {
    String::from(
        "/* AUTO-GENERATED BY panadiga-core::ssot::pico_header -- DO NOT EDIT MANUALLY. */\n\
         #ifndef PICOFW_PANA_RSP_SSOT_H\n\
         #define PICOFW_PANA_RSP_SSOT_H\n\n\
         #include \"generated/picofw_ssot.h\"\n\n\
         #endif /* PICOFW_PANA_RSP_SSOT_H */\n",
    )
}

pub(super) fn write_if_changed_or_create(path: &Path, content: &str) -> Result<()> {
    if let Ok(existing) = fs::read_to_string(path)
        && existing.replace("\r\n", "\n") == content
    {
        return Ok(());
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed creating directory {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("Failed writing {}", path.display()))
}

/// Writes `picofw_pana_rsp_ssot.h` compatibility header to the given path.
pub fn export_pico_rsp_ssot_header(output_path: &Path) -> Result<()> {
    write_if_changed_or_create(output_path, &generate_pana_rsp_ssot_header())
}

/// Exports `generated/picofw_ssot.h`, `generated/picofw_ssot_tables.c`, and `uart/picofw_pana_rsp_ssot.h` under `pico_dir`.
pub fn export_pico_ssot_files(pico_dir: &Path) -> Result<()> {
    write_if_changed_or_create(
        &pico_dir.join("generated/picofw_ssot.h"),
        &generate_c11_ssot_header(),
    )?;
    write_if_changed_or_create(
        &pico_dir.join("generated/picofw_ssot_tables.c"),
        &generate_c11_ssot_tables(),
    )?;
    write_if_changed_or_create(
        &pico_dir.join("uart/picofw_pana_rsp_ssot.h"),
        &generate_pana_rsp_ssot_header(),
    )?;
    Ok(())
}
