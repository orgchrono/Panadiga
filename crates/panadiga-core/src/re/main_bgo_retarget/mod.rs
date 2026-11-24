#![deny(unsafe_code)]

//! Declarative `MAIN` BGO/NFROM NOR geometry retargeter (`35-sector 4 MiB Top-Boot`
//! $\leftrightarrow$ `45-sector 8 MiB Uniform`) with fail-closed static & `Mn103Vm` verification.
//!
//! When a Panasonic DIGA main application binary (`MAIN3060NP`) compiled for a `4 MiB` Top-Boot
//! NOR layout (`0x8420_0000..0x8440_0000`, `P1=3`, `P2=28`, `max_did=0x0A1A`) boots on an `8 MiB`
//! uniform-BGO chassis (`0x8413_0000..0x8440_0000`, `P1=1`, `P2=40`, `max_did=0x0F24`),
//! `NFD_Init()` fails with `[nfrom] BGO driver init failed[1].` (`src/nfd_api.c:228`).
//!
//! [`apply_main_bgo_retarget_in_place`] expands the `NFD`/`BGO` control structs (`0x1CC4 -> 0x26C4`)
//! into a freshly reserved static BSS tail above `_end` (`0x531B_7774 -> 0x531B_C000`), retargets
//! the descriptor/bank tables (`0x8420_0000 -> 0x8413_0000`), and updates every verified `0x0Axx`
//! directory-ID constant in `NFD_API`. Every site uses an exact fail-closed expectation check.

use serde::{Deserialize, Serialize};

mod engine;
mod sites;
mod ssot_synth;
mod tables;
#[cfg(test)]
pub(crate) mod tests;

pub use engine::{
    apply_main_bgo_retarget_for_spec_in_place, apply_main_bgo_retarget_in_place,
    inspect_main_bgo_geometry, inspect_main_bgo_geometry_for_spec,
    inspect_or_verify_main_bgo_geometry,
};
pub use ssot_synth::build_main_bgo_retarget_spec_from_ssot;

pub const MAIN_BGO_STATE_FACTORY_4MB: &str = "BGO_35SEC_4MB_0X84200000";
pub const MAIN_BGO_STATE_8MB: &str = "BGO_45SEC_8MB_0X84130000";
pub const MAIN_BGO_STATE_MISMATCH: &str = "UNKNOWN_OR_MISMATCH";

/// Exact 32-bit word patch site inside an unpacked `MAIN` binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordPatchSite {
    pub offset: usize,
    pub expect: u32,
    pub replace: u32,
}

/// Exact single-byte immediate patch site inside an unpacked `MAIN` binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BytePatchSite {
    pub offset: usize,
    pub expect: u8,
    pub replace: u8,
}

/// Exact multi-byte slice patch site (e.g. code-cave trampolines & stub blocks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlicePatchSite {
    pub offset: usize,
    pub expect: &'static [u8],
    pub replace: &'static [u8],
}

/// Reversible 32-bit literal relocation rule verified by exact expected match count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U32RelocSpec {
    pub from: u32,
    pub to: u32,
    pub expected_count: usize,
}

/// Declarative per-binary descriptor for `MAIN` BGO NOR geometry retargeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBgoRetargetSpec {
    pub target_geometry_id: &'static str,
    pub factory_geometry_id: &'static str,
    pub base_va: u32,
    pub expected_size: usize,
    pub reloc_scan_start: usize,
    pub reloc_scan_end: usize,
    pub table_detect_offset: usize,
    pub table_factory_base: u32,
    pub table_target_base: u32,
    pub word_sites: &'static [WordPatchSite],
    pub byte_sites: &'static [BytePatchSite],
    pub slice_sites: &'static [SlicePatchSite],
    pub ram_relocs: &'static [U32RelocSpec],
}

/// Owned multi-byte slice patch site synthesized from SSOT [`MainBuildSpec`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedSlicePatchSite {
    pub offset: usize,
    pub expect: Vec<u8>,
    pub replace: Vec<u8>,
}

/// Owned `MAIN` BGO NOR geometry retarget specification synthesized dynamically from SSOT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedMainBgoRetargetSpec {
    pub target_geometry_id: String,
    pub factory_geometry_id: String,
    pub base_va: u32,
    pub expected_size: usize,
    pub reloc_scan_start: usize,
    pub reloc_scan_end: usize,
    pub table_detect_offset: usize,
    pub word_sites: Vec<WordPatchSite>,
    pub byte_sites: Vec<BytePatchSite>,
    pub slice_sites: Vec<OwnedSlicePatchSite>,
    pub ram_relocs: Vec<U32RelocSpec>,
}

impl MainBgoRetargetSpec {
    /// Materializes this static descriptor into an [`OwnedMainBgoRetargetSpec`] for shared execution.
    #[must_use]
    pub fn to_owned_spec(&self) -> OwnedMainBgoRetargetSpec {
        OwnedMainBgoRetargetSpec {
            target_geometry_id: self.target_geometry_id.to_string(),
            factory_geometry_id: self.factory_geometry_id.to_string(),
            base_va: self.base_va,
            expected_size: self.expected_size,
            reloc_scan_start: self.reloc_scan_start,
            reloc_scan_end: self.reloc_scan_end,
            table_detect_offset: self.table_detect_offset,
            word_sites: self.word_sites.to_vec(),
            byte_sites: self.byte_sites.to_vec(),
            slice_sites: self
                .slice_sites
                .iter()
                .map(|s| OwnedSlicePatchSite {
                    offset: s.offset,
                    expect: s.expect.to_vec(),
                    replace: s.replace.to_vec(),
                })
                .collect(),
            ram_relocs: self.ram_relocs.to_vec(),
        }
    }
}

/// Inspection status returned by [`inspect_main_bgo_geometry`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainBgoGeometryStatus {
    pub state: String,
    pub geometry_id: Option<String>,
    pub table_va_hex: String,
    pub bgo_flash_base_hex: String,
    pub nfd_bss_end_hex: String,
}
