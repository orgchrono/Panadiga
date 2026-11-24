//! Header generator for `firmware/pico-p7501/generated/p7501_ssot.h`.

use std::fmt::Write as _;

use crate::constants::{
    AM34_AIOF_BASE, AM34_AVDD_BASE, AM34_IC7501_TBUS_BASE, AM34_IIC0_BASE, AM34_IIC1_BASE,
    AM34_SDRAM_BASE, AM34_SDRAM_SIZE, AM34_SDRAM_STUB_RAW_ADDR, IC3001_I2C_ADDR_7BIT,
    LBA_PARTITION_1_SECTOR, PANA_DIGADIAG_DEFAULT_LOAD_ADDR, PANA_LZSS_WINDOW_INIT,
    PANA_LZSS_WINDOW_SIZE,
};
use crate::hdd::capacity_patch::specs::{
    DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB, OFF_CLAMP_BRANCH_OP, OFF_SLOT_200GB, OFF_SLOT_400GB,
    OFF_SLOT_FALLBACK, OPCODE_BLS, OPCODE_BRA, WIN_LEN,
};
use crate::re::davis_probe::{
    DAVIS_IDENTIFY_BUF_DEFAULT_VA, DAVIS_IDENTIFY_META_DEFAULT_VA, DAVIS_IDENTIFY_SENTINEL,
};
use crate::re::main_passthrough::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_HANDSHAKE_BYTES, DEV_SERIAL_ISR_CTX_PTR_VA,
    DEV_SERIAL_PC_OFFSET, passthrough_rsp_payloads,
};
use crate::re::{
    NOR_AUTOBOOT_CAPSULE_HEADER_BYTES, NOR_AUTOBOOT_CAPSULE_MAGIC_LE,
    NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET,
};

use super::super::registry::{
    EvidenceTier, HardwareProfileRegistry, NorRegionKind, sector_range_for_aligned_span,
};
use super::super::spec::constants::P7501_WDT_MAX_CMD_WINDOW_MS;
use super::header_sections::{
    append_c11_struct_decls, append_diag_enums_and_externs, write_padded_define,
};
use super::{
    DEV_SERIAL_NAK_BYTE, P7501_AM34_LETHAL_REG_LEN_SSOT, P7501_AM34_LETHAL_REG2_ADDR_SSOT,
    P7501_AM34_LETHAL_REG2_LEN_SSOT, P7501_CLI_LINE_MAX_SSOT, P7501_FAST_STUB_PAGE_BYTES_SSOT,
    P7501_GDB_SECTOR_CACHE_SIZE_SSOT, P7501_GPIO_NSW5V_HIZ_GP3, P7501_GPIO_RX_GP2,
    P7501_GPIO_SW_UART_GP26, P7501_GPIO_TX_GP4, P7501_GPIO_XRESET_OD_GP5,
    P7501_IC3001_BR_NA_ADDR7_H_SSOT, P7501_IC3001_BR_NA_ADDR7_L_SSOT,
    P7501_JIG_DEFAULT_HOLD_MS_SSOT, P7501_MAX_FAST_WRITE_BYTES_SSOT, P7501_VDD_FLOATING_CEIL_MV,
    P7501_VDD_MIN_MV, P7501_VDD_NOMINAL_MV, P7501_VDD_PULLDOWN_MIN_MV, P7501_VDD_SAFE_MIN_MV,
    P7501_VDD_WAKE_MIN_MV, P7501_WDT_TIMEOUT_MS_SSOT, compute_pico_ssot_hash,
};

/// Resolved `DMR-EH55` verified NOR/MMIO context shared across C11 header emitters (`p7501_ssot.h` & `openos_ssot.h`).
pub(in crate::ssot) struct Eh55VerifiedNorContext {
    pub p7501_uart_baud: u32,
    pub lethal_mmio0: u32,
    pub sc0_uart_addr: u32,
    pub sdcd_addr: u32,
    pub davis_pri_addr: u32,
    pub davis_sec_addr: u32,
    pub nor_cached_base: u32,
    pub nor_cached_size: u32,
    pub nor_uncached_base: u32,
    pub ipl_size: u32,
    pub bgo_offset: u32,
    pub bgo_size: u32,
    pub bgo_sector_count: u32,
    pub eemul_offset: u32,
    pub eemul_size: u32,
    pub eeprom_chip_count: usize,
}

/// Resolves and asserts all `DMR-EH55` `BenchVerified` NOR/SoC geometry invariants once in SSOT.
pub(in crate::ssot) fn resolve_eh55_verified_nor_context() -> Eh55VerifiedNorContext {
    let reg = HardwareProfileRegistry::builtin();
    let eh55 = reg
        .resolve_model("DMR-EH55")
        .expect("DMR-EH55 v2 manifest must resolve");
    assert_eq!(
        eh55.manifest.claims.board.tier,
        EvidenceTier::BenchVerified,
        "DMR-EH55 board provenance must be BenchVerified before emitting C11 SSOT headers"
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
    let eemul_reg = layout
        .find_region(NorRegionKind::Eemul)
        .expect("Eemul region");
    let (bgo_sa0, bgo_sa1) =
        sector_range_for_aligned_span(flash, bgo_reg.start_offset, bgo_reg.end_offset)
            .expect("EH55 BGO span");

    Eh55VerifiedNorContext {
        p7501_uart_baud: soc.p7501_uart_baud,
        lethal_mmio0: soc.lethal_mmio[0],
        sc0_uart_addr: soc.mmio["sc0_uart"],
        sdcd_addr: soc.mmio["sdcd"],
        davis_pri_addr: soc.mmio["davis_pri"],
        davis_sec_addr: soc.mmio["davis_sec"],
        nor_cached_base: nor_win.base,
        nor_cached_size: nor_win.size,
        nor_uncached_base: nor_win.uncached_alias_base.expect("uncached NOR alias"),
        ipl_size: ipl_reg.end_offset - ipl_reg.start_offset + 1,
        bgo_offset: bgo_reg.start_offset,
        bgo_size: bgo_reg.end_offset - bgo_reg.start_offset + 1,
        bgo_sector_count: bgo_sa1 - bgo_sa0 + 1,
        eemul_offset: eemul_reg.start_offset,
        eemul_size: eemul_reg.end_offset - eemul_reg.start_offset + 1,
        eeprom_chip_count: reg.bundle().eeprom_chips.len(),
    }
}

/// Renders the master declarative C11 SSOT header `firmware/pico-p7501/generated/p7501_ssot.h`.
pub fn generate_c11_ssot_header() -> String {
    let ctx = resolve_eh55_verified_nor_context();
    let ipl_l1_size = ctx.ipl_size;
    let ipl_l1_start = ctx.nor_cached_base;
    let ipl_l1_mirror_start = ctx.nor_uncached_base;
    let cs0_aperture_end = ipl_l1_mirror_start
        .checked_add(ctx.nor_cached_size)
        .expect("CS0 end overflow");
    let bgo_offset = ctx.bgo_offset;
    let bgo_size = ctx.bgo_size;

    let mut out = String::from(
        "/* AUTO-GENERATED BY panadiga-core::ssot::pico_header -- DO NOT EDIT MANUALLY. */\n\
         #ifndef P7501_SSOT_GENERATED_H\n#define P7501_SSOT_GENERATED_H\n\n\
         #include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n\n\
         #ifdef __cplusplus\nextern \"C\" {\n#endif\n\n",
    );

    let core_defs: &[(&str, String)] = &[
        (
            "P7501_SSOT_DEFAULT_BAUD",
            format!("{}UL", ctx.p7501_uart_baud),
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
            "P7501_SSOT_MAX_FAST_WRITE_BYTES",
            format!("{P7501_MAX_FAST_WRITE_BYTES_SSOT}u"),
        ),
        (
            "P7501_SSOT_FAST_STUB_PAGE_BYTES",
            format!("{P7501_FAST_STUB_PAGE_BYTES_SSOT}u"),
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
            format!("0x{:08X}UL", ctx.nor_cached_base),
        ),
        (
            "P7501_SSOT_CS0_UNCACHED_BASE",
            format!("0x{ipl_l1_mirror_start:08X}UL"),
        ),
        (
            "P7501_SSOT_CS0_WINDOW_SIZE",
            format!("0x{:08X}UL", ctx.nor_cached_size),
        ),
        (
            "P7501_SSOT_CS0_APERTURE_END",
            format!("0x{cs0_aperture_end:08X}UL\n"),
        ),
        (
            "P7501_SSOT_SDRAM_CACHED_BASE",
            format!("0x{AM34_SDRAM_BASE:08X}UL"),
        ),
        (
            "P7501_SSOT_SDRAM_WINDOW_SIZE",
            format!("0x{AM34_SDRAM_SIZE:08X}UL"),
        ),
        (
            "P7501_SSOT_DIGADIAG_LOAD_ADDR",
            format!("0x{PANA_DIGADIAG_DEFAULT_LOAD_ADDR:08X}UL"),
        ),
        (
            "P7501_SSOT_SDRAM_STUB_RAW_ADDR",
            format!("0x{AM34_SDRAM_STUB_RAW_ADDR:08X}UL\n"),
        ),
        (
            "P7501_SSOT_AM34_LETHAL_REG_ADDR",
            format!("0x{:08X}UL", ctx.lethal_mmio0),
        ),
        (
            "P7501_SSOT_AM34_LETHAL_REG_LEN",
            format!("{P7501_AM34_LETHAL_REG_LEN_SSOT}UL"),
        ),
        (
            "P7501_SSOT_AM34_LETHAL_REG2_ADDR",
            format!("0x{P7501_AM34_LETHAL_REG2_ADDR_SSOT:08X}UL"),
        ),
        (
            "P7501_SSOT_AM34_LETHAL_REG2_LEN",
            format!("0x{P7501_AM34_LETHAL_REG2_LEN_SSOT:08X}UL"),
        ),
        (
            "P7501_SSOT_LZSS_WINDOW_SIZE",
            format!("0x{PANA_LZSS_WINDOW_SIZE:04X}u"),
        ),
        (
            "P7501_SSOT_LZSS_WINDOW_INIT",
            format!("0x{PANA_LZSS_WINDOW_INIT:04X}u"),
        ),
        (
            "P7501_SSOT_IC3001_BR_NA_ADDR7_L",
            format!("0x{P7501_IC3001_BR_NA_ADDR7_L_SSOT:02X}u"),
        ),
        (
            "P7501_SSOT_IC3001_BR_NA_ADDR7_H",
            format!("0x{P7501_IC3001_BR_NA_ADDR7_H_SSOT:02X}u"),
        ),
        (
            "P7501_SSOT_IC3001_EC_ADDR7",
            format!("0x{IC3001_I2C_ADDR_7BIT:02X}u"),
        ),
        ("P7501_SSOT_AIOF_BASE", format!("0x{AM34_AIOF_BASE:08X}UL")),
        ("P7501_SSOT_AVDD_BASE", format!("0x{AM34_AVDD_BASE:08X}UL")),
        ("P7501_SSOT_IIC0_BASE", format!("0x{AM34_IIC0_BASE:08X}UL")),
        ("P7501_SSOT_IIC1_BASE", format!("0x{AM34_IIC1_BASE:08X}UL")),
        (
            "P7501_SSOT_TBUS_BASE",
            format!("0x{AM34_IC7501_TBUS_BASE:08X}UL"),
        ),
        (
            "P7501_SSOT_AM34_SC0_CTR_ADDR",
            format!("0x{:08X}UL", ctx.sc0_uart_addr),
        ),
        (
            "P7501_SSOT_AM34_SDCD_BASE_ADDR",
            format!("0x{:08X}UL", ctx.sdcd_addr),
        ),
        (
            "P7501_SSOT_AM34_DAVIS_PRI_ADDR",
            format!("0x{:08X}UL", ctx.davis_pri_addr),
        ),
        (
            "P7501_SSOT_AM34_DAVIS_SEC_ADDR",
            format!("0x{:08X}UL\n", ctx.davis_sec_addr),
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
            format!("{}u\n", ctx.bgo_sector_count),
        ),
        (
            "P7501_SSOT_NOR_AUTOBOOT_HDR_BYTES",
            format!("{NOR_AUTOBOOT_CAPSULE_HEADER_BYTES}u"),
        ),
        (
            "P7501_SSOT_NOR_AUTOBOOT_MAGIC_OFF",
            format!("0x{NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET:02X}u"),
        ),
        (
            "P7501_SSOT_NOR_AUTOBOOT_MAGIC_LE",
            format!("0x{NOR_AUTOBOOT_CAPSULE_MAGIC_LE:08X}UL\n"),
        ),
        ("P7501_SSOT_HDD_CLAMP_WIN_LEN", format!("0x{WIN_LEN:02X}u")),
        (
            "P7501_SSOT_HDD_CLAMP_OFF_400GB",
            format!("0x{OFF_SLOT_400GB:02X}u"),
        ),
        (
            "P7501_SSOT_HDD_CLAMP_OFF_200GB",
            format!("0x{OFF_SLOT_200GB:02X}u"),
        ),
        (
            "P7501_SSOT_HDD_CLAMP_OFF_FALLBACK",
            format!("0x{OFF_SLOT_FALLBACK:02X}u"),
        ),
        (
            "P7501_SSOT_HDD_CLAMP_OFF_BRANCH",
            format!("0x{OFF_CLAMP_BRANCH_OP:02X}u"),
        ),
        ("P7501_SSOT_HDD_OPCODE_BLS", format!("0x{OPCODE_BLS:02X}u")),
        ("P7501_SSOT_HDD_OPCODE_BRA", format!("0x{OPCODE_BRA:02X}u")),
        (
            "P7501_SSOT_HDD_DYNAMIC_MAX_2KB",
            format!("0x{DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB:08X}UL"),
        ),
        (
            "P7501_SSOT_MEIHFS_PRIMARY_FW_LBA",
            format!("0x{LBA_PARTITION_1_SECTOR:08X}UL"),
        ),
        (
            "P7501_SSOT_MEIHFS_MIRROR_DELTA_LBA",
            "0x00020000UL".to_string(),
        ),
        (
            "P7501_SSOT_DAVIS_IDENTIFY_BUF_VA",
            format!("0x{DAVIS_IDENTIFY_BUF_DEFAULT_VA:08X}UL"),
        ),
        (
            "P7501_SSOT_DAVIS_IDENTIFY_META_VA",
            format!("0x{DAVIS_IDENTIFY_META_DEFAULT_VA:08X}UL"),
        ),
        (
            "P7501_SSOT_DAVIS_IDENTIFY_SENTINEL",
            format!("0x{DAVIS_IDENTIFY_SENTINEL:08X}UL\n"),
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

    append_c11_struct_decls(&mut out, ctx.eeprom_chip_count);
    let ssot_hash = compute_pico_ssot_hash();
    append_diag_enums_and_externs(&mut out, ssot_hash);
    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n#endif /* P7501_SSOT_GENERATED_H */\n");
    out
}
