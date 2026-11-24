//! Section builders (`FNV-1a`, C11 struct definitions, and diagnostic signature enum/extern blocks)
//! for `firmware/pico-p7501/generated/p7501_ssot.h`.

use std::fmt::Write as _;

use crate::constants::PANA_FEISTEL_SBOX_256;
use panadiga_crypto::feistel::PANA_DVD_KEY_CATALOG;

use super::{
    DIAG_SIGNATURE_RULES, FLASH_CHIPS_ORDER, LOADER_FINGERPRINTS, P7501_MMIO_DOM_CORE,
    P7501_MMIO_DOM_STORAGE, P7501_MMIO_DOM_VIDEO, P7501_TRAP_E, P7501_TRAP_R, P7501_TRAP_SEV_DENY,
    P7501_TRAP_SEV_WARN, P7501_TRAP_W, PICO_SSOT_SIGNATURES, TARGET_PROFILES_ORDER,
    pico_mmio_reg_specs, pico_pieos_anchor_specs, pico_trap_specs,
};

pub(super) fn compute_fnv1a_u32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811C_9DC5;
    for &b in bytes {
        hash ^= u32::from(b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

pub(super) fn write_padded_define(out: &mut String, width: usize, name: &str, value: &str) {
    let pad = width.saturating_sub(name.len()).max(1);
    let _ = writeln!(out, "#define {name}{}{value}", " ".repeat(pad));
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
    let _ = writeln!(out, "#define P7501_SSOT_HASH_STR \"{ssot_hash:08X}\"");
    for &(sym, tag) in crate::constants::events::P7501_WIRE_TAGS {
        let name = format!("P7501_WIRE_TAG_{sym}");
        let _ = writeln!(out, "#define {name:<32} \"{tag}\"");
    }
    for &(sym, code) in crate::constants::events::P7501_RES_CODES {
        let name = format!("P7501_RES_CODE_{sym}");
        let _ = writeln!(out, "#define {name:<32} \"{code}\"");
    }
    out.push('\n');
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
    let _ = writeln!(
        out,
        "#define P7501_SSOT_FEISTEL_KEY_COUNT {}u\n\
         typedef struct p7501_ssot_feistel_key {{\n\
         \x20   const char *name;\n\
         \x20   uint8_t key8[8];\n\
         }} p7501_ssot_feistel_key_t;\n\n\
         #define P7501_SSOT_TRAP_R {P7501_TRAP_R}u\n\
         #define P7501_SSOT_TRAP_W {P7501_TRAP_W}u\n\
         #define P7501_SSOT_TRAP_E {P7501_TRAP_E}u\n\
         #define P7501_SSOT_TRAP_SEV_WARN {P7501_TRAP_SEV_WARN}u\n\
         #define P7501_SSOT_TRAP_SEV_DENY {P7501_TRAP_SEV_DENY}u\n\
         #define P7501_SSOT_TRAP_COUNT {}u\n\
         typedef struct p7501_ssot_trap {{\n\
         \x20   const char *id;\n\
         \x20   uint32_t start;\n\
         \x20   uint32_t len;\n\
         \x20   uint8_t mask_rwe;\n\
         \x20   uint8_t severity;\n\
         \x20   const char *reason;\n\
         }} p7501_ssot_trap_t;\n\n\
         #define P7501_SSOT_MMIO_DOM_CORE {P7501_MMIO_DOM_CORE}u\n\
         #define P7501_SSOT_MMIO_DOM_VIDEO {P7501_MMIO_DOM_VIDEO}u\n\
         #define P7501_SSOT_MMIO_DOM_STORAGE {P7501_MMIO_DOM_STORAGE}u\n\
         #define P7501_SSOT_MMIO_DOM_ALL {}u\n\
         #define P7501_SSOT_MMIO_REG_COUNT {}u\n\
         typedef struct p7501_ssot_mmio_reg {{\n\
         \x20   uint32_t addr;\n\
         \x20   const char *name;\n\
         \x20   uint8_t domain;\n\
         \x20   uint8_t width_bits;\n\
         }} p7501_ssot_mmio_reg_t;\n\n\
         #define P7501_SSOT_SIG_COUNT {}u\n\
         typedef struct p7501_ssot_sig {{\n\
         \x20   const char *name;\n\
         \x20   uint8_t len;\n\
         \x20   uint8_t bytes[16];\n\
         }} p7501_ssot_sig_t;\n\n\
         #define P7501_SSOT_PIEOS_ANCHOR_COUNT {}u\n\
         typedef struct p7501_ssot_pieos_anchor {{\n\
         \x20   const char *name;\n\
         \x20   uint32_t va_3060np;\n\
         \x20   uint32_t va_3410pq;\n\
         \x20   uint32_t mmio_ref;\n\
         \x20   uint8_t stage_id;\n\
         \x20   uint8_t flags;\n\
         \x20   const char *caller;\n\
         }} p7501_ssot_pieos_anchor_t;\n\n\
         #define P7501_SSOT_REGION_RULE_COUNT {}u\n\
         typedef struct p7501_ssot_region_rule {{\n\
         \x20   const char *suffix;\n\
         \x20   const char *tv_standard;\n\
         \x20   const char *line_system;\n\
         \x20   uint16_t aiof_vmodesel;\n\
         \x20   uint8_t dvd_region;\n\
         \x20   char oem_marker;\n\
         \x20   uint8_t ic3001_addr7_primary;\n\
         \x20   uint8_t ic3001_addr7_alt;\n\
         }} p7501_ssot_region_rule_t;\n\n",
        PANA_DVD_KEY_CATALOG.len(),
        pico_trap_specs().len(),
        P7501_MMIO_DOM_CORE | P7501_MMIO_DOM_VIDEO | P7501_MMIO_DOM_STORAGE,
        pico_mmio_reg_specs().len(),
        PICO_SSOT_SIGNATURES.len(),
        pico_pieos_anchor_specs().len(),
        super::CANONICAL_REGION_SUFFIXES.len(),
    );
    let _ = PANA_FEISTEL_SBOX_256.len();

    out.push_str(
        "extern const p7501_ssot_flash_chip_t k_p7501_ssot_flash_chips[P7501_FLASH_ID_COUNT];\n\
         extern const p7501_ssot_target_profile_t k_p7501_ssot_target_profiles[P7501_TARGET_PROFILE_COUNT];\n\
         extern const p7501_ssot_eeprom_chip_t k_p7501_ssot_eeprom_chips[P7501_SSOT_EEPROM_COUNT];\n\
         extern const p7501_ssot_loader_fp_t k_p7501_ssot_loader_fingerprints[P7501_SSOT_LOADER_FP_COUNT];\n\
         extern const uint8_t k_p7501_ssot_feistel_sbox[256];\n\
         extern const uint8_t k_p7501_ssot_feistel_iv[4];\n\
         extern const p7501_ssot_feistel_key_t k_p7501_ssot_feistel_keys[P7501_SSOT_FEISTEL_KEY_COUNT];\n\
         extern const p7501_ssot_trap_t k_p7501_ssot_traps[P7501_SSOT_TRAP_COUNT];\n\
         extern const p7501_ssot_mmio_reg_t k_p7501_ssot_mmio_regs[P7501_SSOT_MMIO_REG_COUNT];\n\
         extern const p7501_ssot_sig_t k_p7501_ssot_sigs[P7501_SSOT_SIG_COUNT];\n\
         extern const p7501_ssot_pieos_anchor_t k_p7501_ssot_pieos_anchors[P7501_SSOT_PIEOS_ANCHOR_COUNT];\n\
         extern const p7501_ssot_region_rule_t k_p7501_ssot_region_rules[P7501_SSOT_REGION_RULE_COUNT];\n\
         extern const uint8_t k_p7501_ssot_hdd_clamp_prefix[14];\n\n",
    );
}
