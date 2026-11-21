//! Header generator for `firmware/pico-p7501/generated/p7501_ssot.h`.

use std::fmt::Write as _;

use crate::re::main_passthrough::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_HANDSHAKE_BYTES, DEV_SERIAL_ISR_CTX_PTR_VA,
    DEV_SERIAL_PC_OFFSET, passthrough_rsp_payloads,
};

use super::super::registry::{
    EvidenceTier, HardwareProfileRegistry, NorRegionKind, sector_range_for_aligned_span,
};
use super::super::spec::constants::P7501_WDT_MAX_CMD_WINDOW_MS;
use super::{
    DEV_SERIAL_NAK_BYTE, DIAG_SIGNATURE_RULES, FLASH_CHIPS_ORDER, LOADER_FINGERPRINTS,
    P7501_AM34_LETHAL_REG_LEN_SSOT, P7501_CLI_LINE_MAX_SSOT, P7501_GDB_SECTOR_CACHE_SIZE_SSOT,
    P7501_GPIO_NSW5V_HIZ_GP3, P7501_GPIO_RX_GP2, P7501_GPIO_SW_UART_GP26, P7501_GPIO_TX_GP4,
    P7501_GPIO_XRESET_OD_GP5, P7501_JIG_DEFAULT_HOLD_MS_SSOT, P7501_VDD_FLOATING_CEIL_MV,
    P7501_VDD_MIN_MV, P7501_VDD_NOMINAL_MV, P7501_VDD_PULLDOWN_MIN_MV, P7501_VDD_SAFE_MIN_MV,
    P7501_VDD_WAKE_MIN_MV, P7501_WDT_TIMEOUT_MS_SSOT, TARGET_PROFILES_ORDER,
    generate_c11_ssot_tables,
};

fn compute_fnv1a_u32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811C_9DC5;
    for &b in bytes {
        hash ^= u32::from(b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn write_padded_define(out: &mut String, width: usize, name: &str, value: &str) {
    let _ = writeln!(out, "#define {name:<width$}{value}");
}

fn append_c11_struct_decls(out: &mut String, eeprom_count: usize) {
    out.push_str("typedef enum p7501_ssot_flash_id {\n");
    for (idx, &(_, c_enum)) in FLASH_CHIPS_ORDER.iter().enumerate() {
        let _ = writeln!(out, "    {c_enum} = {idx},");
    }
    let _ = writeln!(
        out,
        "    P7501_FLASH_ID_COUNT = {}",
        FLASH_CHIPS_ORDER.len()
    );
    out.push_str("} p7501_ssot_flash_id_t;\n\n");

    out.push_str(
        "typedef struct p7501_ssot_sector_bank {\n\
         \x20   uint16_t start_sa;\n\
         \x20   uint16_t count;\n\
         \x20   uint32_t sector_size;\n\
         } p7501_ssot_sector_bank_t;\n\n\
         typedef struct p7501_ssot_flash_chip {\n\
         \x20   p7501_ssot_flash_id_t id;\n\
         \x20   const char *id_str;\n\
         \x20   const char *part_number;\n\
         \x20   uint32_t total_bytes;\n\
         \x20   bool writable;\n\
         \x20   uint16_t jedec_mfg;\n\
         \x20   uint16_t jedec_dev[3];\n\
         \x20   uint8_t jedec_cycles;\n\
         \x20   uint32_t ipl_guard_end;\n\
         \x20   uint32_t nvram_offset;\n\
         \x20   uint32_t nvram_size;\n\
         \x20   uint16_t nvram_base_sa;\n\
         \x20   uint16_t nvram_sector_count;\n\
         \x20   uint8_t bank_count;\n\
         \x20   p7501_ssot_sector_bank_t banks[4];\n\
         } p7501_ssot_flash_chip_t;\n\n",
    );

    out.push_str("typedef enum p7501_ssot_target_profile_id {\n");
    for (idx, &(_, c_enum)) in TARGET_PROFILES_ORDER.iter().enumerate() {
        let _ = writeln!(out, "    {c_enum} = {idx},");
    }
    let _ = writeln!(
        out,
        "    P7501_TARGET_PROFILE_COUNT = {}",
        TARGET_PROFILES_ORDER.len()
    );
    out.push_str("} p7501_ssot_target_profile_id_t;\n\n");

    out.push_str(
        "typedef struct p7501_ssot_target_profile {\n\
         \x20   p7501_ssot_target_profile_id_t profile_id;\n\
         \x20   const char *model_name;\n\
         \x20   p7501_ssot_flash_id_t flash_id;\n\
         \x20   uint32_t total_bytes;\n\
         \x20   bool writable_nvram;\n\
         \x20   bool upper_nor_unlock_allowed;\n\
         \x20   bool full_nor_readonly;\n\
         \x20   uint32_t ipl_guard_end;\n\
         \x20   uint32_t protected_ro_start;\n\
         \x20   uint32_t protected_ro_end;\n\
         \x20   uint32_t bgo_offset;\n\
         \x20   uint32_t bgo_size;\n\
         \x20   uint16_t bgo_sector_count;\n\
         \x20   uint32_t eemul_offset;\n\
         \x20   uint32_t eemul_size;\n\
         \x20   uint16_t eemul_base_sa;\n\
         \x20   uint16_t eemul_sector_count;\n\
         } p7501_ssot_target_profile_t;\n\n",
    );

    let _ = writeln!(
        out,
        "#define P7501_SSOT_EEPROM_COUNT {}u\n\
         typedef struct p7501_ssot_eeprom_chip {{\n\
         \x20   const char *id;\n\
         \x20   const char *part_number;\n\
         \x20   uint8_t i2c_addr_7bit;\n\
         \x20   uint32_t total_bytes;\n\
         \x20   uint16_t page_size;\n\
         \x20   uint16_t voltage_mv;\n\
         \x20   bool is_timer_mcu;\n\
         \x20   bool is_optical_cal;\n\
         }} p7501_ssot_eeprom_chip_t;\n",
        eeprom_count
    );
}

fn append_diag_enums_and_externs(out: &mut String, ssot_hash: u32) {
    out.push_str(
        "enum {\n\
         \x20   P7501_SSOT_DIAG_EVT_DDR_OK             = (1u << 0),\n\
         \x20   P7501_SSOT_DIAG_EVT_SERIAL_BOOT        = (1u << 1),\n\
         \x20   P7501_SSOT_DIAG_EVT_SD_ERR             = (1u << 2),\n\
         \x20   P7501_SSOT_DIAG_EVT_ODD_OPEN           = (1u << 3),\n\
         \x20   P7501_SSOT_DIAG_EVT_CD_APC_ERR         = (1u << 4),\n\
         \x20   P7501_SSOT_DIAG_EVT_BGO_FAIL           = (1u << 5),\n\
         \x20   P7501_SSOT_DIAG_EVT_HUNG_U99           = (1u << 6),\n\
         \x20   P7501_SSOT_DIAG_EVT_PANAMON            = (1u << 7),\n\
         \x20   P7501_SSOT_DIAG_EVT_SD_HW_OK           = (1u << 8),\n\
         \x20   P7501_SSOT_DIAG_EVT_SD_DIAG_LOAD_FAIL  = (1u << 9),\n\
         \x20   P7501_SSOT_DIAG_EVT_SD_UNCOMPRESS_FAIL = (1u << 10),\n\
         \x20   P7501_SSOT_DIAG_EVT_DIAG_EXEC          = (1u << 11),\n\
         \x20   P7501_SSOT_DIAG_EVT_ODD_TRAY_FAIL      = (1u << 12),\n\
         \x20   P7501_SSOT_DIAG_EVT_FAT_FAIL           = (1u << 13)\n\
         };\n\n",
    );

    let _ = writeln!(
        out,
        "#define P7501_SSOT_DIAG_RULE_COUNT {}u",
        DIAG_SIGNATURE_RULES.len()
    );
    out.push_str("#define P7501_SSOT_DIAG_RULES_INIT { \\\n");
    for &(needle, evt_flag) in DIAG_SIGNATURE_RULES {
        let escaped = needle.replace('"', "\\\"");
        let _ = writeln!(
            out,
            "    {{ \"{escaped}\", {}u, {evt_flag} }}, \\",
            needle.len()
        );
    }
    out.push_str("}\n\n");

    let _ = writeln!(out, "#define P7501_SSOT_HASH_U32 0x{ssot_hash:08X}UL");
    let _ = writeln!(out, "#define P7501_SSOT_HASH_STR \"{ssot_hash:08X}\"\n");
    let _ = writeln!(
        out,
        "#define P7501_SSOT_LOADER_FP_COUNT {}u\n\
         typedef struct p7501_ssot_loader_fp {{\n\
         \x20   const char *needle;\n\
         \x20   p7501_ssot_target_profile_id_t profile_id;\n\
         \x20   const char *hw_board;\n\
         \x20   const char *tv_system;\n\
         \x20   bool lock_xip_main_ro;\n\
         }} p7501_ssot_loader_fp_t;\n",
        LOADER_FINGERPRINTS.len()
    );
    out.push_str(
        "extern const p7501_ssot_flash_chip_t k_p7501_ssot_flash_chips[P7501_FLASH_ID_COUNT];\n\
         extern const p7501_ssot_target_profile_t k_p7501_ssot_target_profiles[P7501_TARGET_PROFILE_COUNT];\n\
         extern const p7501_ssot_eeprom_chip_t k_p7501_ssot_eeprom_chips[P7501_SSOT_EEPROM_COUNT];\n\
         extern const p7501_ssot_loader_fp_t k_p7501_ssot_loader_fingerprints[P7501_SSOT_LOADER_FP_COUNT];\n\n",
    );
}

/// Renders the master declarative C11 SSOT header `firmware/pico-p7501/generated/p7501_ssot.h`.
pub fn generate_c11_ssot_header() -> String {
    let reg = HardwareProfileRegistry::builtin();
    let eh55 = reg
        .resolve_model("DMR-EH55")
        .expect("DMR-EH55 v2 manifest must resolve");
    assert_eq!(
        eh55.manifest.claims.board.tier,
        EvidenceTier::BenchVerified,
        "DMR-EH55 board provenance must be BenchVerified before emitting C11 P7501 SSOT header"
    );
    let soc = eh55.soc.as_ref().expect("DMR-EH55 SoC required");
    let flash = eh55
        .flash_chip
        .as_ref()
        .expect("DMR-EH55 flash chip required");
    let layout = eh55
        .nor_layout
        .as_ref()
        .expect("DMR-EH55 NOR layout required");
    assert!(
        soc.p7501_nsw5v_hiz_only,
        "DMR-EH55 NSW5V Hi-Z invariant must hold"
    );

    let nor_win = soc
        .bus_windows
        .iter()
        .find(|w| w.name == "nor_cached")
        .expect("nor_cached");
    let ipl_reg = layout.find_region(NorRegionKind::Ipl).expect("Ipl region");
    let bgo_reg = layout.find_region(NorRegionKind::Bgo).expect("Bgo region");
    let ipl_l1_size = ipl_reg.end_offset - ipl_reg.start_offset + 1;
    let ipl_l1_start = nor_win.base + ipl_reg.start_offset;
    let ipl_l1_mirror_start = nor_win.uncached_alias_base.expect("uncached NOR alias");
    let cs0_aperture_end = ipl_l1_mirror_start
        .checked_add(nor_win.size)
        .expect("CS0 end overflow");
    let bgo_offset = bgo_reg.start_offset;
    let bgo_size = bgo_reg.end_offset - bgo_reg.start_offset + 1;
    let (bgo_sa0, bgo_sa1) =
        sector_range_for_aligned_span(flash, bgo_reg.start_offset, bgo_reg.end_offset)
            .expect("EH55 BGO span");

    let mut out = String::from(
        "/* AUTO-GENERATED BY panadiga-core::ssot::pico_header -- DO NOT EDIT MANUALLY. */\n\
         #ifndef P7501_SSOT_GENERATED_H\n#define P7501_SSOT_GENERATED_H\n\n\
         #include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n\n\
         #ifdef __cplusplus\nextern \"C\" {\n#endif\n\n",
    );

    let core_defs: &[(&str, String)] = &[
        (
            "P7501_SSOT_DEFAULT_BAUD",
            format!("{}UL", soc.p7501_uart_baud),
        ),
        ("P7501_SSOT_VDD_MIN_MV", format!("{P7501_VDD_MIN_MV}u")),
        (
            "P7501_SSOT_VDD_WAKE_MIN_MV",
            format!("{P7501_VDD_WAKE_MIN_MV}u"),
        ),
        (
            "P7501_SSOT_VDD_PULLDOWN_MIN_MV",
            format!("{P7501_VDD_PULLDOWN_MIN_MV}u"),
        ),
        (
            "P7501_SSOT_VDD_SAFE_MIN_MV",
            format!("{P7501_VDD_SAFE_MIN_MV}u"),
        ),
        (
            "P7501_SSOT_VDD_NOMINAL_MV",
            format!("{P7501_VDD_NOMINAL_MV}u"),
        ),
        (
            "P7501_SSOT_VDD_FLOATING_CEIL_MV",
            format!("{P7501_VDD_FLOATING_CEIL_MV}u"),
        ),
        (
            "P7501_SSOT_WDT_TIMEOUT_MS",
            format!("{P7501_WDT_TIMEOUT_MS_SSOT}u"),
        ),
        (
            "P7501_SSOT_WDT_MAX_CMD_WINDOW_MS",
            format!("{P7501_WDT_MAX_CMD_WINDOW_MS}u"),
        ),
        (
            "P7501_SSOT_CLI_LINE_MAX",
            format!("{P7501_CLI_LINE_MAX_SSOT}u"),
        ),
        (
            "P7501_SSOT_JIG_DEFAULT_HOLD_MS",
            format!("{P7501_JIG_DEFAULT_HOLD_MS_SSOT}u"),
        ),
        (
            "P7501_SSOT_GDB_SECTOR_CACHE_SIZE",
            format!("{P7501_GDB_SECTOR_CACHE_SIZE_SSOT}u\n"),
        ),
    ];
    for (name, val) in core_defs {
        write_padded_define(&mut out, 35, name, val);
    }

    let hex_elems: Vec<String> = DEV_SERIAL_HANDSHAKE_BYTES
        .iter()
        .map(|b| format!("0x{b:02X}u"))
        .collect();
    let _ = writeln!(
        out,
        "#define P7501_RSP_HANDSHAKE_LEN {}u",
        DEV_SERIAL_HANDSHAKE_BYTES.len()
    );
    let _ = writeln!(
        out,
        "#define P7501_RSP_HANDSHAKE_INIT {{ {} }}",
        hex_elems.join(", ")
    );
    let _ = writeln!(
        out,
        "#define P7501_RSP_NAK_BYTE 0x{DEV_SERIAL_NAK_BYTE:02X}u\n"
    );

    write_padded_define(
        &mut out,
        30,
        "P7501_AM34_ISR_CTX_PTR_ADDR",
        &format!("0x{DEV_SERIAL_ISR_CTX_PTR_VA:08X}UL"),
    );
    write_padded_define(
        &mut out,
        30,
        "P7501_AM34_ISR_SAVED_PC_OFF",
        &format!("0x{DEV_SERIAL_PC_OFFSET:02X}UL"),
    );
    write_padded_define(
        &mut out,
        30,
        "P7501_AM34_SERIAL_CB_PTR_ADDR",
        &format!("0x{DEV_SERIAL_CALLBACK_VA:08X}UL\n"),
    );

    let addr_defs: &[(&str, String)] = &[
        (
            "P7501_SSOT_IPL_LAYER1_START",
            format!("0x{ipl_l1_start:08X}UL"),
        ),
        (
            "P7501_SSOT_IPL_LAYER1_END",
            format!("0x{:08X}UL", ipl_l1_start + ipl_l1_size - 1),
        ),
        (
            "P7501_SSOT_IPL_LAYER1_MIRROR_START",
            format!("0x{ipl_l1_mirror_start:08X}UL"),
        ),
        (
            "P7501_SSOT_IPL_LAYER1_MIRROR_END",
            format!("0x{:08X}UL", ipl_l1_mirror_start + ipl_l1_size - 1),
        ),
        (
            "P7501_SSOT_IPL_LAYER1_SIZE",
            format!("0x{ipl_l1_size:08X}UL"),
        ),
        (
            "P7501_SSOT_CS0_CACHED_BASE",
            format!("0x{:08X}UL", nor_win.base),
        ),
        (
            "P7501_SSOT_CS0_UNCACHED_BASE",
            format!("0x{ipl_l1_mirror_start:08X}UL"),
        ),
        (
            "P7501_SSOT_CS0_WINDOW_SIZE",
            format!("0x{:08X}UL", nor_win.size),
        ),
        (
            "P7501_SSOT_CS0_APERTURE_END",
            format!("0x{cs0_aperture_end:08X}UL\n"),
        ),
        (
            "P7501_SSOT_AM34_LETHAL_REG_ADDR",
            format!("0x{:08X}UL", soc.lethal_mmio[0]),
        ),
        (
            "P7501_SSOT_AM34_LETHAL_REG_LEN",
            format!("{P7501_AM34_LETHAL_REG_LEN_SSOT}UL"),
        ),
        (
            "P7501_SSOT_AM34_SC0_CTR_ADDR",
            format!("0x{:08X}UL", soc.mmio["sc0_uart"]),
        ),
        (
            "P7501_SSOT_AM34_SDCD_BASE_ADDR",
            format!("0x{:08X}UL", soc.mmio["sdcd"]),
        ),
        (
            "P7501_SSOT_AM34_DAVIS_PRI_ADDR",
            format!("0x{:08X}UL", soc.mmio["davis_pri"]),
        ),
        (
            "P7501_SSOT_AM34_DAVIS_SEC_ADDR",
            format!("0x{:08X}UL\n", soc.mmio["davis_sec"]),
        ),
        ("P7501_SSOT_GPIO_RXD_GP2", format!("{P7501_GPIO_RX_GP2}u")),
        (
            "P7501_SSOT_GPIO_NSW5V_HIZ_GP3",
            format!("{P7501_GPIO_NSW5V_HIZ_GP3}u"),
        ),
        ("P7501_SSOT_GPIO_TXD_GP4", format!("{P7501_GPIO_TX_GP4}u")),
        (
            "P7501_SSOT_GPIO_XRESET_OD_GP5",
            format!("{P7501_GPIO_XRESET_OD_GP5}u"),
        ),
        (
            "P7501_SSOT_GPIO_SW_UART_GP26",
            format!("{P7501_GPIO_SW_UART_GP26}u\n"),
        ),
        ("P7501_SSOT_BGO_OFFSET", format!("0x{bgo_offset:08X}UL")),
        (
            "P7501_SSOT_BGO_HW_BASE",
            format!("0x{:08X}UL", ipl_l1_mirror_start + bgo_offset),
        ),
        ("P7501_SSOT_BGO_SIZE", format!("0x{bgo_size:08X}UL")),
        (
            "P7501_SSOT_BGO_SECTOR_COUNT",
            format!("{}u\n", bgo_sa1 - bgo_sa0 + 1),
        ),
    ];
    for (name, val) in addr_defs {
        write_padded_define(&mut out, 35, name, val);
    }

    let payloads = passthrough_rsp_payloads();
    let _ = writeln!(
        out,
        "#define P7501_TBC_PASSTHROUGH_SEQ_LEN {}u",
        payloads.len()
    );
    out.push_str("#define P7501_TBC_PASSTHROUGH_SEQ_INIT { \\\n");
    for payload in &payloads {
        let _ = writeln!(out, "    \"{payload}\", \\");
    }
    out.push_str("}\n\n");

    append_c11_struct_decls(&mut out, reg.bundle().eeprom_chips.len());
    let ssot_hash = compute_fnv1a_u32(generate_c11_ssot_tables().as_bytes());
    append_diag_enums_and_externs(&mut out, ssot_hash);
    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n#endif /* P7501_SSOT_GENERATED_H */\n");
    out
}
