//! Static Matsushita MN103S / AM34 Trap 1-7 + AM33 Sleigh gap analysis & resolution helpers.
//!
//! Resolves the recurring disassembly/decompilation traps catalogued in
//! `docs/pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md` across `BOOT.BIN`, `PROG061`, `DRV`, and `MAIN`:
//! 1. Orphan functions & pointer/CLI dispatch tables (`<8sIII>`, ATAPI CDB table, `0x5040ea30`, `0x44035eb0`)
//! 2. Indirect `BCLR 0xff` / `BSET` RMW side effects (`0x5771ac87`, `0x40130efa`)
//! 3. Post-call unreachable block pruning & `SETLB`/stack-return anomalies (`0x574dd208`)
//! 4. Mid-instruction branch overlap collisions (`BRA 0x5040ea2f` inside `0x5040ea2e`, `0x4400493b` `CALL`)
//! 5. CS1 NOR Flash CFI command cycles (`0x84200554`, `0x84200AAA`, `0x84200000`)
//! 6. `SETLB`..`Lcc` hardware loops (`in_LAR - 4`) & negative `A3` frame accesses (`0x50404f12`)
//! 7. Cross-revision symbol shifts (`DRV_decomp` vs `S211`), CP932 table (`0x400e779a`), and mid-insn LE32 false hits (`0x4006f5e2`)
//! 8. AM33/AM34 Sleigh gaps in `GhidraMn10300`: `0xF9 0xEA` (`MOVHU (Rm0+), Rn2`), `0xFF` pre-`RETS` barrier, and `0xF0 0xC0`.

pub mod overlaps;
pub mod revisions;
pub mod tables;

#[cfg(test)]
mod tests;

pub use overlaps::{
    build_instruction_spans, detect_mid_instruction_overlaps, filter_mid_instruction_constant_hits,
    is_flash_cfi_command_va, scan_am33_f9_ea_postinc_loads, scan_am33_ff_pre_rets_sites,
    scan_cfi_flash_constant_hits,
};
pub use revisions::identify_firmware_revision;
pub use tables::{
    classify_drv_cli_command, parse_atapi_cdb_dispatch, parse_drv_cli_table,
    parse_drv_mecha_error_table, scan_code_pointer_tables,
};

use super::mn10300::decode_setlb_loops;
use serde::{Deserialize, Serialize};

pub const DRV_DECOMP_PASS_HANDLER_VA: u32 = 0x4013_420A;
pub const DRV_S211_PASS_HANDLER_VA: u32 = 0x4013_0EFA;
pub const DRV_DECOMP_MMODE_HANDLER_VA: u32 = 0x4013_4382;
pub const DRV_DECOMP_SLOGON_HANDLER_VA: u32 = 0x4013_3ED0;
pub const DRV_DECOMP_SLOGOFF_HANDLER_VA: u32 = 0x4013_3EDE;
pub const DRV_DECOMP_CLI_TABLE_VA: u32 = 0x400D_F5C0;
pub const DRV_ATAPI_WRITE_BUFFER_3B_VA: u32 = 0x4002_168D;
pub const DRV_ATAPI_READ_BUFFER_3C_VA: u32 = 0x4002_1476;
pub const DRV_ATAPI_VENDOR_ED_VA: u32 = 0x4002_09F3;
pub const DRV_ATAPI_VENDOR_FD_VA: u32 = 0x4002_0147;
pub const DRV_ATAPI_VENDOR_FE_VA: u32 = 0x4002_025E;
pub const DRV_ATAPI_STUB_UNIMPL_VA: u32 = 0x4001_4559;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrvCliEntry {
    pub index: usize,
    pub entry_va: u32,
    pub name: String,
    pub handler_va: u32,
    pub flags: u32,
    pub arg_mask: u32,
    pub classification: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtapiCdbDispatchSlot {
    pub cdb0: u8,
    pub table_entry_va: u32,
    pub handler_va: u32,
    pub is_active: bool,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionSpan {
    pub start_va: u32,
    pub end_va: u32,
    pub mnemonic: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidInstructionOverlap {
    pub source_va: u32,
    pub target_va: u32,
    pub containing_start_va: u32,
    pub containing_end_va: u32,
    pub mnemonic: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrvMechaErrorRecord {
    pub offset: usize,
    pub code: u16,
    pub code_hex: String,
    pub raw_len: usize,
    pub ascii_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareRevisionProfile {
    pub stage: String,
    pub revision: String,
    pub pass_handler_va: Option<u32>,
    pub cli_table_va: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedOpcodeHit {
    pub va: u32,
    pub bytes_hex: String,
    pub mnemonic: String,
    pub operands: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SevenTrapsSummary {
    pub orphan_code_pointers: Vec<(u32, u32)>,
    pub cfi_constant_sites: Vec<u32>,
    pub mid_instruction_overlaps: Vec<MidInstructionOverlap>,
    pub setlb_loop_count: usize,
    pub revision_profile: FirmwareRevisionProfile,
    pub am33_f9_ea_sites: Vec<ExtendedOpcodeHit>,
    pub am33_ff_pre_rets_sites: Vec<u32>,
}

/// Runs an integrated static Trap 1..7 + AM33 Sleigh diagnostic summary over a firmware slice.
pub fn analyze_seven_traps_in_slice(
    data: &[u8],
    base_va: u32,
    code_lo: u32,
    code_hi: u32,
) -> SevenTrapsSummary {
    SevenTrapsSummary {
        orphan_code_pointers: scan_code_pointer_tables(
            data,
            base_va,
            code_lo,
            code_hi,
            4,
            &[DRV_ATAPI_STUB_UNIMPL_VA],
        ),
        cfi_constant_sites: scan_cfi_flash_constant_hits(data, base_va),
        mid_instruction_overlaps: detect_mid_instruction_overlaps(data, base_va, &[]),
        setlb_loop_count: decode_setlb_loops(data, base_va, None, None, 64).len(),
        revision_profile: identify_firmware_revision(data),
        am33_f9_ea_sites: scan_am33_f9_ea_postinc_loads(data, base_va),
        am33_ff_pre_rets_sites: scan_am33_ff_pre_rets_sites(data, base_va),
    }
}
