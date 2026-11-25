//! `.rodata` table emitter for `firmware/picofw/generated/picofw_ssot_tables.c`.

use std::fmt::Write as _;

use crate::constants::{PANA_FEISTEL_KEYSCHED_IV, PANA_FEISTEL_SBOX_256};
use panadiga_crypto::feistel::PANA_DVD_KEY_CATALOG;

use super::super::registry::{
    EvidenceTier, FlashChipProfile, HardwareProfileRegistry, NorRegionKind, NorRegionPolicy,
    sector_range_for_aligned_span,
};
use super::{
    FLASH_CHIPS_ORDER, LOADER_FINGERPRINTS, PICO_SSOT_SIGNATURES, TARGET_PROFILES_ORDER,
    pico_mmio_reg_specs, pico_pieos_anchor_specs, pico_region_rule_specs, pico_trap_specs,
};

fn flash_enum_for_id(id: &str) -> &'static str {
    FLASH_CHIPS_ORDER
        .iter()
        .find(|&&(k, _)| k == id)
        .map(|&(_, e)| e)
        .unwrap_or_else(|| panic!("Unknown flash_ref '{id}' for C11 SSOT projection"))
}

fn derive_flash_chip_nvram_meta(
    chip: &FlashChipProfile,
    ipl_l1_size: u32,
) -> (u32, u32, u32, u16, u16) {
    if !chip.writable || chip.id == "macronix_mx29lv320_4mb_bottom" {
        return (chip.total_bytes, 0, 0, 0, 0);
    }
    let nvram_size: u32 = if chip.total_bytes >= 16 * 1024 * 1024 {
        128 * 1024
    } else {
        64 * 1024
    };
    let nvram_offset = chip.total_bytes - nvram_size;
    let (sa0, sa1) = sector_range_for_aligned_span(chip, nvram_offset, chip.total_bytes - 1)
        .unwrap_or_else(|e| panic!("NVRAM SA derivation failed for '{}': {e}", chip.id));
    (
        ipl_l1_size,
        nvram_offset,
        nvram_size,
        u16::try_from(sa0).expect("SA fit u16"),
        u16::try_from(sa1 - sa0 + 1).expect("SA count fit u16"),
    )
}

fn append_flash_chips_table(out: &mut String, reg: &HardwareProfileRegistry, ipl_l1_size: u32) {
    out.push_str(
        "const picofw_ssot_flash_chip_t k_picofw_ssot_flash_chips[PICOFW_FLASH_ID_COUNT] = {\n",
    );
    for &(chip_id, c_enum) in FLASH_CHIPS_ORDER {
        let chip = reg
            .bundle()
            .flash_chips
            .get(chip_id)
            .unwrap_or_else(|| panic!("Missing flash chip '{chip_id}' in builtin bundle"));
        let (ipl_end, nv_off, nv_sz, nv_base_sa, nv_sa_cnt) =
            derive_flash_chip_nvram_meta(chip, ipl_l1_size);
        let mfg = chip.jedec.as_ref().map_or(0u32, |j| j.mfg);
        let mut devs = [0u32; 3];
        let mut cyc = 0u8;
        if let Some(j) = &chip.jedec {
            cyc = u8::try_from(j.device_cycles.len().min(3)).unwrap_or(0);
            for (idx, val) in j.device_cycles.iter().take(3).enumerate() {
                devs[idx] = *val;
            }
        }
        let bank_count = chip.sector_banks.len().min(4);
        let _ = writeln!(out, "    /* {c_enum} */ {{");
        let _ = writeln!(out, "        .id = {c_enum},");
        let _ = writeln!(out, "        .id_str = \"{}\",", chip.id);
        let _ = writeln!(out, "        .part_number = \"{}\",", chip.part_number);
        let _ = writeln!(out, "        .total_bytes = 0x{:08X}UL,", chip.total_bytes);
        let _ = writeln!(out, "        .writable = {},", chip.writable);
        let _ = writeln!(out, "        .jedec_mfg = 0x{mfg:04X}u,");
        let _ = writeln!(
            out,
            "        .jedec_dev = {{ 0x{:04X}u, 0x{:04X}u, 0x{:04X}u }},",
            devs[0], devs[1], devs[2]
        );
        let _ = writeln!(out, "        .jedec_cycles = {cyc}u,");
        let _ = writeln!(out, "        .ipl_guard_end = 0x{ipl_end:08X}UL,");
        let _ = writeln!(out, "        .nvram_offset = 0x{nv_off:08X}UL,");
        let _ = writeln!(out, "        .nvram_size = 0x{nv_sz:08X}UL,");
        let _ = writeln!(out, "        .nvram_base_sa = {nv_base_sa}u,");
        let _ = writeln!(out, "        .nvram_sector_count = {nv_sa_cnt}u,");
        let _ = writeln!(out, "        .bank_count = {bank_count}u,");
        out.push_str("        .banks = {\n");
        let mut next_sa = 0u32;
        for idx in 0..4 {
            if let Some(bank) = chip.sector_banks.get(idx) {
                let _ = writeln!(
                    out,
                    "            {{ .start_sa = {next_sa}u, .count = {}u, .sector_size = 0x{:08X}UL }},",
                    bank.count, bank.sector_size
                );
                next_sa = next_sa.saturating_add(bank.count);
            } else {
                out.push_str("            { .start_sa = 0u, .count = 0u, .sector_size = 0UL },\n");
            }
        }
        out.push_str("        }\n    },\n");
    }
    out.push_str("};\n\n");
}

fn append_target_profiles_table(out: &mut String, reg: &HardwareProfileRegistry, ipl_l1_size: u32) {
    out.push_str(
        "const picofw_ssot_target_profile_t k_picofw_ssot_target_profiles[PICOFW_TARGET_PROFILE_COUNT] = {\n",
    );
    for &(model_id, c_enum) in TARGET_PROFILES_ORDER {
        let resolved = reg
            .resolve_model(model_id)
            .unwrap_or_else(|e| panic!("Failed resolving target profile '{model_id}': {e}"));
        let chip = resolved
            .flash_chip
            .as_ref()
            .unwrap_or_else(|| panic!("Model '{model_id}' must declare flash_chip for C11 SSOT"));
        let flash_enum = flash_enum_for_id(&chip.id);
        let layout = resolved.nor_layout.as_ref();

        let eemul_reg = layout.and_then(|l| l.find_region(NorRegionKind::Eemul));
        let writable_nvram =
            chip.writable && eemul_reg.is_some_and(|r| r.policy == NorRegionPolicy::NvramSanitize);
        let upper_unlock_ok = chip.writable
            && layout.is_some_and(|l| {
                l.regions
                    .iter()
                    .any(|r| r.policy == NorRegionPolicy::GatedUpperNor)
            });
        let full_ro = !writable_nvram && !upper_unlock_ok;
        let ipl_end = if full_ro {
            chip.total_bytes
        } else {
            ipl_l1_size
        };

        let (ro_start, ro_end) = if full_ro {
            (0u32, chip.total_bytes)
        } else if let Some(xip) = layout.and_then(|l| l.find_region(NorRegionKind::MainXip)) {
            (xip.start_offset, xip.end_offset + 1)
        } else {
            (0u32, 0u32)
        };

        let (bgo_off, bgo_sz, bgo_cnt) = if let Some(bgo) =
            layout.and_then(|l| l.find_region(NorRegionKind::Bgo))
        {
            let sz = bgo.end_offset - bgo.start_offset + 1;
            let (sa0, sa1) = sector_range_for_aligned_span(chip, bgo.start_offset, bgo.end_offset)
                .expect("BGO sector span must align");
            (
                bgo.start_offset,
                sz,
                u16::try_from(sa1 - sa0 + 1).expect("BGO count fit u16"),
            )
        } else {
            (0u32, 0u32, 0u16)
        };

        let (ee_off, ee_sz, ee_base_sa, ee_cnt) = if let Some(ee) = eemul_reg {
            let sz = ee.end_offset - ee.start_offset + 1;
            let (sa0, sa1) = sector_range_for_aligned_span(chip, ee.start_offset, ee.end_offset)
                .expect("EEMUL sector span must align");
            (
                ee.start_offset,
                sz,
                u16::try_from(sa0).expect("EEMUL SA fit u16"),
                u16::try_from(sa1 - sa0 + 1).expect("EEMUL count fit u16"),
            )
        } else {
            (0u32, 0u32, 0u16, 0u16)
        };

        let _ = writeln!(out, "    /* {c_enum} */ {{");
        let _ = writeln!(out, "        .profile_id = {c_enum},");
        let _ = writeln!(
            out,
            "        .model_name = \"{}\",",
            resolved.manifest.model_id
        );
        let _ = writeln!(out, "        .flash_id = {flash_enum},");
        let _ = writeln!(out, "        .total_bytes = 0x{:08X}UL,", chip.total_bytes);
        let _ = writeln!(out, "        .writable_nvram = {writable_nvram},");
        let _ = writeln!(
            out,
            "        .upper_nor_unlock_allowed = {upper_unlock_ok},"
        );
        let _ = writeln!(out, "        .full_nor_readonly = {full_ro},");
        let _ = writeln!(out, "        .ipl_guard_end = 0x{ipl_end:08X}UL,");
        let _ = writeln!(out, "        .protected_ro_start = 0x{ro_start:08X}UL,");
        let _ = writeln!(out, "        .protected_ro_end = 0x{ro_end:08X}UL,");
        let _ = writeln!(out, "        .bgo_offset = 0x{bgo_off:08X}UL,");
        let _ = writeln!(out, "        .bgo_size = 0x{bgo_sz:08X}UL,");
        let _ = writeln!(out, "        .bgo_sector_count = {bgo_cnt}u,");
        let _ = writeln!(out, "        .eemul_offset = 0x{ee_off:08X}UL,");
        let _ = writeln!(out, "        .eemul_size = 0x{ee_sz:08X}UL,");
        let _ = writeln!(out, "        .eemul_base_sa = {ee_base_sa}u,");
        let _ = writeln!(out, "        .eemul_sector_count = {ee_cnt}u");
        out.push_str("    },\n");
    }
    out.push_str("};\n\n");
}

fn append_eeprom_chips_table(out: &mut String, reg: &HardwareProfileRegistry) {
    out.push_str(
        "const picofw_ssot_eeprom_chip_t k_picofw_ssot_eeprom_chips[PICOFW_SSOT_EEPROM_COUNT] = {\n",
    );
    for eep in reg.bundle().eeprom_chips.values() {
        let fn_str = serde_json::to_string(&eep.function).unwrap_or_else(|_| "\"unknown\"".into());
        let is_timer = fn_str.contains("timer_main_config");
        let is_opt = fn_str.contains("optical_calibration");
        let _ = writeln!(
            out,
            "    {{ .id = \"{}\", .part_number = \"{}\", .i2c_addr_7bit = 0x{:02X}u, .total_bytes = 0x{:04X}u, .page_size = {}u, .voltage_mv = {}u, .is_timer_mcu = {}, .is_optical_cal = {} }},",
            eep.id,
            eep.part_number,
            eep.i2c_addr_7bit,
            eep.total_bytes,
            eep.page_size,
            eep.voltage_mv,
            is_timer,
            is_opt,
        );
    }
    out.push_str("};\n\n");
}

fn append_loader_fingerprints_table(out: &mut String, reg: &HardwareProfileRegistry) {
    out.push_str(
        "const picofw_ssot_loader_fp_t k_picofw_ssot_loader_fingerprints[PICOFW_SSOT_LOADER_FP_COUNT] = {\n",
    );
    for &(needle, model_id, c_enum, lock_xip_main_ro) in LOADER_FINGERPRINTS {
        let resolved = reg
            .resolve_model(model_id)
            .unwrap_or_else(|e| panic!("Failed resolving fingerprint model '{model_id}': {e}"));
        let hw_board = resolved
            .board
            .as_ref()
            .filter(|b| {
                resolved.manifest.claims.board.tier != EvidenceTier::CommunityUnverified
                    && !b.pcb_code.contains("UNVERIFIED")
            })
            .map_or("UNVERIFIED", |b| b.pcb_code.as_str());
        let tv_system = resolved
            .manifest
            .tv_standard
            .as_deref()
            .unwrap_or("UNKNOWN");
        let _ = writeln!(
            out,
            "    {{ \"{needle}\", {c_enum}, \"{hw_board}\", \"{tv_system}\", {lock_xip_main_ro} }},"
        );
    }
    out.push_str("};\n\n");
}

fn format_hex_byte_list(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("0x{b:02X}u"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn append_feistel_tables(out: &mut String) {
    out.push_str("const uint8_t k_picofw_ssot_feistel_sbox[256] = {\n");
    for chunk in PANA_FEISTEL_SBOX_256.chunks(16) {
        let _ = writeln!(out, "    {},", format_hex_byte_list(chunk));
    }
    out.push_str("};\n\n");

    let _ = writeln!(
        out,
        "const uint8_t k_picofw_ssot_feistel_iv[4] = {{ {} }};\n",
        format_hex_byte_list(&PANA_FEISTEL_KEYSCHED_IV)
    );

    out.push_str(
        "const picofw_ssot_feistel_key_t k_picofw_ssot_feistel_keys[PICOFW_SSOT_FEISTEL_KEY_COUNT] = {\n",
    );
    for (name, key8) in PANA_DVD_KEY_CATALOG {
        let _ = writeln!(
            out,
            "    {{ \"{name}\", {{ {} }} }},",
            format_hex_byte_list(key8)
        );
    }
    out.push_str("};\n\n");
}

fn append_traps_mmio_and_sigs_tables(out: &mut String) {
    out.push_str("const picofw_ssot_trap_t k_picofw_ssot_traps[PICOFW_SSOT_TRAP_COUNT] = {\n");
    for (id, start, len, mask, sev, reason) in pico_trap_specs() {
        let _ = writeln!(
            out,
            "    {{ \"{id}\", 0x{start:08X}UL, 0x{len:08X}UL, 0x{mask:02X}u, {sev}u, \"{reason}\" }},"
        );
    }
    out.push_str("};\n\n");

    out.push_str(
        "const picofw_ssot_mmio_reg_t k_picofw_ssot_mmio_regs[PICOFW_SSOT_MMIO_REG_COUNT] = {\n",
    );
    for (addr, name, dom, width) in pico_mmio_reg_specs() {
        let _ = writeln!(
            out,
            "    {{ 0x{addr:08X}UL, \"{name}\", {dom}u, {width}u }},"
        );
    }
    out.push_str("};\n\n");

    out.push_str("const picofw_ssot_sig_t k_picofw_ssot_sigs[PICOFW_SSOT_SIG_COUNT] = {\n");
    for &(name, raw) in PICO_SSOT_SIGNATURES {
        let mut padded = [0u8; 16];
        let copy_len = raw.len().min(padded.len());
        padded[..copy_len].copy_from_slice(&raw[..copy_len]);
        let _ = writeln!(
            out,
            "    {{ \"{name}\", {}u, {{ {} }} }},",
            raw.len(),
            format_hex_byte_list(&padded)
        );
    }
    out.push_str("};\n\n");

    out.push_str(
        "const picofw_ssot_pieos_anchor_t k_picofw_ssot_pieos_anchors[PICOFW_SSOT_PIEOS_ANCHOR_COUNT] = {\n",
    );
    for (name, va3060, va3410, mmio_ref, stage, flags, caller) in pico_pieos_anchor_specs() {
        let _ = writeln!(
            out,
            "    {{ \"{name}\", 0x{va3060:08X}UL, 0x{va3410:08X}UL, 0x{mmio_ref:08X}UL, {stage}u, 0x{flags:02X}u, \"{caller}\" }},"
        );
    }
    out.push_str("};\n\n");

    out.push_str(
        "const picofw_ssot_region_rule_t k_picofw_ssot_region_rules[PICOFW_SSOT_REGION_RULE_COUNT] = {\n",
    );
    for r in pico_region_rule_specs() {
        let _ = writeln!(
            out,
            "    {{ \"{}\", \"{}\", \"{}\", 0x{:04X}u, {}u, '{}', 0x{:02X}u, 0x{:02X}u }},",
            r.suffix,
            r.tv_standard,
            r.line_system,
            r.aiof_vmodesel,
            r.dvd_region,
            r.oem_marker,
            r.ic3001_addr7_primary,
            r.ic3001_addr7_alt,
        );
    }
    out.push_str("};\n\n");

    out.push_str("const uint8_t k_picofw_ssot_hdd_clamp_prefix[14] = { ");
    out.push_str(&format_hex_byte_list(
        &crate::hdd::capacity_patch::specs::PREFIX_SIG,
    ));
    out.push_str(" };\n");
}

/// Renders `firmware/picofw/generated/picofw_ssot_tables.c` defining single-copy `.rodata` tables.
pub fn generate_c11_ssot_tables() -> String {
    let reg = HardwareProfileRegistry::builtin();
    let eh55 = reg
        .resolve_model("DMR-EH55")
        .expect("DMR-EH55 v2 manifest must resolve");
    let layout = eh55.nor_layout.as_ref().expect("DMR-EH55 NOR layout");
    let ipl = layout
        .find_region(NorRegionKind::Ipl)
        .expect("DMR-EH55 IPL region");
    let ipl_l1_size = ipl.end_offset - ipl.start_offset + 1;

    let mut out = String::new();
    out.push_str(
        "/* AUTO-GENERATED BY panadiga-core::ssot::pico_header -- DO NOT EDIT MANUALLY. */\n",
    );
    out.push_str("#include \"generated/picofw_ssot.h\"\n\n");
    append_flash_chips_table(&mut out, reg, ipl_l1_size);
    append_target_profiles_table(&mut out, reg, ipl_l1_size);
    append_eeprom_chips_table(&mut out, reg);
    append_loader_fingerprints_table(&mut out, reg);
    append_feistel_tables(&mut out);
    append_traps_mmio_and_sigs_tables(&mut out);
    out
}
