//! Section builders (`FNV-1a`, C11 struct definitions, and diagnostic signature enum/extern blocks)
//! for `firmware/pico-p7501/generated/p7501_ssot.h`.

use std::fmt::Write as _;

use super::{DIAG_SIGNATURE_RULES, FLASH_CHIPS_ORDER, LOADER_FINGERPRINTS, TARGET_PROFILES_ORDER};

pub(super) fn compute_fnv1a_u32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811C_9DC5;
    for &b in bytes {
        hash ^= u32::from(b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

pub(super) fn write_padded_define(out: &mut String, width: usize, name: &str, value: &str) {
    let _ = writeln!(out, "#define {name:<width$}{value}");
}

pub(super) fn append_c11_struct_decls(out: &mut String, eeprom_count: usize) {
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

pub(super) fn append_diag_enums_and_externs(out: &mut String, ssot_hash: u32) {
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
