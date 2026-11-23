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

use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use super::region_compat::EXPECTED_MAIN_3060NP_SIZE;
use crate::constants::AM34_MAIN_BASE_VA;

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

/// Inspection status returned by [`inspect_main_bgo_geometry`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainBgoGeometryStatus {
    pub state: String,
    pub geometry_id: Option<String>,
    pub table_va_hex: String,
    pub bgo_flash_base_hex: String,
    pub nfd_bss_end_hex: String,
}

const fn va_off(va: u32) -> usize {
    (va - AM34_MAIN_BASE_VA) as usize
}

const fn byte_site(va: u32, expect: u8, replace: u8) -> BytePatchSite {
    BytePatchSite {
        offset: va_off(va),
        expect,
        replace,
    }
}

const MAIN3060NP_WORD_SITES: &[WordPatchSite] = &[
    // Reserve 16 KiB static RAM above original BSS `_end` (`0x531B_7774 -> 0x531B_C000`).
    // Keep `0x574B_61B3` as `word_sites[0]` for `nfd_bss_end_hex`; keep crt0 cached end at `0x531B_7774`
    // while extending crt0 uncached BSS zero-end and `_sbrk` start from `0x931B_7DF8 -> 0x931B_C000`.
    WordPatchSite {
        offset: va_off(0x574B_61B3),
        expect: 0x531B_7774,
        replace: 0x531B_C000,
    },
    WordPatchSite {
        offset: va_off(0x5700_0B08),
        expect: 0x931B_7DF8,
        replace: 0x931B_C000,
    },
    WordPatchSite {
        offset: va_off(0x5777_DE0F),
        expect: 0x931B_7DF8,
        replace: 0x931B_C000,
    },
    // BGO DID group logical/physical sector counts (`28/35 -> 40/45`, `P1=3 -> 1`).
    WordPatchSite {
        offset: va_off(0x579E_D2CC),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2D0),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2D4),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2D8),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2E0),
        expect: 0x0000_0005,
        replace: 0x0000_0003,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2E4),
        expect: 0x0000_0023,
        replace: 0x0000_002D,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2E8),
        expect: 0x0000_0023,
        replace: 0x0000_002D,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2EC),
        expect: 0x0000_0023,
        replace: 0x0000_002D,
    },
    // Primary BGO descriptor (`0x579E_D32C..=0x579E_D36C`).
    WordPatchSite {
        offset: va_off(0x579E_D32C),
        expect: 0x0000_0003,
        replace: 0x0000_0001,
    },
    WordPatchSite {
        offset: va_off(0x579E_D330),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D340),
        expect: 0x0000_0A1A,
        replace: 0x0000_0F24,
    },
    WordPatchSite {
        offset: va_off(0x579E_D344),
        expect: 0x0000_0002,
        replace: 0x0000_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D350),
        expect: 0x0000_0020,
        replace: 0x0000_002E,
    },
    WordPatchSite {
        offset: va_off(0x579E_D354),
        expect: 0x8420_0000,
        replace: 0x8413_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D358),
        expect: 0x843F_0000,
        replace: 0x8440_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D35C),
        expect: 0x0000_2000,
        replace: 0x0001_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D360),
        expect: 0x0000_0020,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D364),
        expect: 0x0000_0028,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D368),
        expect: 0x843F_0000,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D36C),
        expect: 0x8440_0000,
        replace: 0xFFFF_FFFF,
    },
    // BGO RAM pointer rows (`0x579E_D398..=0x579E_D400`) relocated into `0x531B_B000..0x531B_BC00`.
    WordPatchSite {
        offset: va_off(0x579E_D398),
        expect: 0x52FE_9230,
        replace: 0x531B_B000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D39C),
        expect: 0x52FE_93C0,
        replace: 0x531B_B620,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3A0),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3A4),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3A8),
        expect: 0x52FE_9466,
        replace: 0x531B_B830,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3AC),
        expect: 0x52FE_937C,
        replace: 0x531B_B510,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3B4),
        expect: 0x52FE_9374,
        replace: 0x531B_B500,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3B8),
        expect: 0x52FE_9462,
        replace: 0x531B_B820,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3BC),
        expect: 0x52FE_93BE,
        replace: 0x531B_B612,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3C0),
        expect: 0x52FE_9469,
        replace: 0x531B_B833,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3C4),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3C8),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3D0),
        expect: 0x52FE_9376,
        replace: 0x531B_B502,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3D4),
        expect: 0x52FE_9463,
        replace: 0x531B_B821,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3D8),
        expect: 0x52FE_93BE,
        replace: 0x531B_B612,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3DC),
        expect: 0x52FE_9469,
        replace: 0x531B_B833,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3E0),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3E4),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3EC),
        expect: 0x52FE_9378,
        replace: 0x531B_B504,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3F0),
        expect: 0x52FE_9464,
        replace: 0x531B_B822,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3F4),
        expect: 0x52FE_93BE,
        replace: 0x531B_B612,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3F8),
        expect: 0x52FE_9469,
        replace: 0x531B_B833,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3FC),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D400),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    // Flash vendor bank base tables (`0x579E_D408..=0x579E_D434`).
    WordPatchSite {
        offset: va_off(0x579E_D408),
        expect: 0x0000_0003,
        replace: 0x0000_0001,
    },
    WordPatchSite {
        offset: va_off(0x579E_D40C),
        expect: 0x8420_0000,
        replace: 0x8413_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D410),
        expect: 0x8430_0000,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D414),
        expect: 0x8438_0000,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D42C),
        expect: 0x0000_0002,
        replace: 0x0000_0001,
    },
    WordPatchSite {
        offset: va_off(0x579E_D430),
        expect: 0x8420_0000,
        replace: 0x8413_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D434),
        expect: 0x8438_0000,
        replace: 0xFFFF_FFFF,
    },
];

const MAIN3060NP_BYTE_SITES: &[BytePatchSite] = &[
    // NFD control clear size & file-slot struct offsets (`0x1CC4/0x1CC0/0x1C00 -> 0x26C4/0x26C0/0x2600`).
    BytePatchSite {
        offset: va_off(0x574A_8812),
        expect: 0x1C,
        replace: 0x26,
    },
    BytePatchSite {
        offset: va_off(0x574A_A18C),
        expect: 0x1C,
        replace: 0x26,
    },
    BytePatchSite {
        offset: va_off(0x574A_A28E),
        expect: 0x1C,
        replace: 0x26,
    },
    BytePatchSite {
        offset: va_off(0x574A_A2B6),
        expect: 0x1C,
        replace: 0x26,
    },
    // Page-map block count `0x09 -> 0x0E`.
    BytePatchSite {
        offset: va_off(0x574A_888C),
        expect: 0x09,
        replace: 0x0E,
    },
    BytePatchSite {
        offset: va_off(0x574A_88AB),
        expect: 0x09,
        replace: 0x0E,
    },
    BytePatchSite {
        offset: va_off(0x574A_89D3),
        expect: 0x09,
        replace: 0x0E,
    },
    BytePatchSite {
        offset: va_off(0x574A_8A36),
        expect: 0x09,
        replace: 0x0E,
    },
    BytePatchSite {
        offset: va_off(0x574A_8E7D),
        expect: 0x09,
        replace: 0x0E,
    },
    BytePatchSite {
        offset: va_off(0x574A_8EE0),
        expect: 0x09,
        replace: 0x0E,
    },
    BytePatchSite {
        offset: va_off(0x574A_9B05),
        expect: 0x09,
        replace: 0x0E,
    },
    BytePatchSite {
        offset: va_off(0x574A_A75D),
        expect: 0x09,
        replace: 0x0E,
    },
    // Directory ID ceiling `0x0A1A -> 0x0F24`.
    BytePatchSite {
        offset: va_off(0x574A_866D),
        expect: 0x1A,
        replace: 0x24,
    },
    BytePatchSite {
        offset: va_off(0x574A_866E),
        expect: 0x0A,
        replace: 0x0F,
    },
    // Backup page-map DID `0x0A11 -> 0x0F16`.
    BytePatchSite {
        offset: va_off(0x574A_8893),
        expect: 0x11,
        replace: 0x16,
    },
    BytePatchSite {
        offset: va_off(0x574A_8894),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_895A),
        expect: 0x11,
        replace: 0x16,
    },
    BytePatchSite {
        offset: va_off(0x574A_895B),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8987),
        expect: 0x11,
        replace: 0x16,
    },
    BytePatchSite {
        offset: va_off(0x574A_8988),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_9034),
        expect: 0x11,
        replace: 0x16,
    },
    BytePatchSite {
        offset: va_off(0x574A_9035),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_9F00),
        expect: 0x11,
        replace: 0x16,
    },
    BytePatchSite {
        offset: va_off(0x574A_9F01),
        expect: 0x0A,
        replace: 0x0F,
    },
    // Primary DID bases (`0x0A01/0x0A02/0x0A07 -> 0x0F01/0x0F02/0x0F07`).
    BytePatchSite {
        offset: va_off(0x574A_884D),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8875),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_88B0),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_88B8),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_88DB),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_894E),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8960),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8983),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8A44),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8A98),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8B24),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8B2C),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8B5D),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8F46),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8FD0),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_90A6),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_A1A1),
        expect: 0x0A,
        replace: 0x0F,
    },
    // NFD free-page budget / scan ceilings (`0x0A00 -> 0x0F00`).
    BytePatchSite {
        offset: va_off(0x574A_6B84),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_8E4E),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_9ACA),
        expect: 0x0A,
        replace: 0x0F,
    },
    BytePatchSite {
        offset: va_off(0x574A_A52C),
        expect: 0x0A,
        replace: 0x0F,
    },
    // BGO sector scan limits (`39 -> 45` physical sectors in `0x574AECBB`/`0x574AF6A3`/`0x574AF72B`/`0x574AF7CC`/`0x574AF8AE`).
    BytePatchSite {
        offset: va_off(0x574A_ECC4),
        expect: 0x27,
        replace: 0x2D,
    },
    BytePatchSite {
        offset: va_off(0x574A_F6AD),
        expect: 0x27,
        replace: 0x2D,
    },
    BytePatchSite {
        offset: va_off(0x574A_F735),
        expect: 0x27,
        replace: 0x2D,
    },
    BytePatchSite {
        offset: va_off(0x574A_F7D6),
        expect: 0x27,
        replace: 0x2D,
    },
    BytePatchSite {
        offset: va_off(0x574A_F9CA),
        expect: 0x27,
        replace: 0x2D,
    },
    // P5 reserve sector count (`8 -> 4` so `P1(1) + P2(40) + P5(4) = 45`, matching native 8 MiB IPL).
    BytePatchSite {
        offset: va_off(0x574A_F32A),
        expect: 0x08,
        replace: 0x04,
    },
    // Retarget `blt` at `0x574B_2410` so `add -1, d0` branch lands on trampoline S6B (`0x574B_242B`).
    BytePatchSite {
        offset: va_off(0x574B_2411),
        expect: 0x1A,
        replace: 0x1B,
    },
    // Cross-page_map `struct nfd_ctrl` relative displacements (`±0x0A00` shift from `page_map` `0x1400 -> 0x1E00`).
    // 28 sites: `file_slot_tbl -> omap_a` (`-0x1A00 + {0,8,12}` -> `-0x2400 + {0,8,12}`)
    byte_site(0x574A_6B21, 0xE6, 0xDC),
    byte_site(0x574A_6C94, 0xE6, 0xDC),
    byte_site(0x574A_6C9B, 0xE6, 0xDC),
    byte_site(0x574A_7094, 0xE6, 0xDC),
    byte_site(0x574A_709A, 0xE6, 0xDC),
    byte_site(0x574A_79CA, 0xE6, 0xDC),
    byte_site(0x574A_79D6, 0xE6, 0xDC),
    byte_site(0x574A_7B68, 0xE6, 0xDC),
    byte_site(0x574A_7E95, 0xE6, 0xDC),
    byte_site(0x574A_7E9B, 0xE6, 0xDC),
    byte_site(0x574A_81BE, 0xE6, 0xDC),
    byte_site(0x574A_81C3, 0xE6, 0xDC),
    byte_site(0x574A_83AD, 0xE6, 0xDC),
    byte_site(0x574A_83B3, 0xE6, 0xDC),
    byte_site(0x574A_8FB1, 0xE6, 0xDC),
    byte_site(0x574A_9088, 0xE6, 0xDC),
    byte_site(0x574A_9384, 0xE6, 0xDC),
    byte_site(0x574A_9390, 0xE6, 0xDC),
    byte_site(0x574A_93EE, 0xE6, 0xDC),
    byte_site(0x574A_93F3, 0xE6, 0xDC),
    byte_site(0x574A_9468, 0xE6, 0xDC),
    byte_site(0x574A_947D, 0xE6, 0xDC),
    byte_site(0x574A_9720, 0xE6, 0xDC),
    byte_site(0x574A_97C7, 0xE6, 0xDC),
    byte_site(0x574A_993E, 0xE6, 0xDC),
    byte_site(0x574A_99E6, 0xE6, 0xDC),
    byte_site(0x574A_A5B1, 0xE6, 0xDC),
    byte_site(0x574A_A605, 0xE6, 0xDC),
    // 4 sites: `file_slot_tbl -> omap_b` (`-0x1900 + {0,8}` -> `-0x2300 + {0,8}`)
    byte_site(0x574A_8FA7, 0xE7, 0xDD),
    byte_site(0x574A_9075, 0xE7, 0xDD),
    byte_site(0x574A_A658, 0xE7, 0xDD),
    byte_site(0x574A_A6A5, 0xE7, 0xDD),
    // 7 sites: `file_slot_tbl -> hdr` (`-0x1C00 + {0,4,8}` -> `-0x2600 + {0,4,8}`)
    byte_site(0x574A_9287, 0xE4, 0xDA),
    byte_site(0x574A_94EC, 0xE4, 0xDA),
    byte_site(0x574A_9A05, 0xE4, 0xDA),
    byte_site(0x574A_9A0B, 0xE4, 0xDA),
    byte_site(0x574A_A0C0, 0xE4, 0xDA),
    byte_site(0x574A_A5B9, 0xE4, 0xDA),
    byte_site(0x574A_A65F, 0xE4, 0xDA),
    // 3 sites: `file_slot_tbl -> page_map` (`-0x1400 -> -0x1E00`)
    byte_site(0x574A_70ED, 0xEC, 0xE2),
    byte_site(0x574A_939C, 0xEC, 0xE2),
    byte_site(0x574A_9758, 0xEC, 0xE2),
    // 2 sites: `omap_a -> file_slot_tbl/next_entry` (`+0x1A02/+0x1AC2 -> +0x2402/+0x24C2`)
    byte_site(0x574A_A370, 0x1A, 0x24),
    byte_site(0x574A_A389, 0x1A, 0x24),
    // 4 sites: `hdr+8 -> file_slot_tbl` (`+0x1BF8/+0x1BFA -> +0x25F8/+0x25FA`)
    byte_site(0x574A_92B2, 0x1B, 0x25),
    byte_site(0x574A_92B9, 0x1B, 0x25),
    byte_site(0x574A_A0EA, 0x1B, 0x25),
    byte_site(0x574A_A0F1, 0x1B, 0x25),
];

const MAIN3060NP_SLICE_SITES: &[SlicePatchSite] = &[
    // S1: `0x574A_ED28` P1 sector size (`0x2000 -> 0x10000` in d3)
    SlicePatchSite {
        offset: va_off(0x574A_ED28),
        expect: &[0x2F, 0x00, 0x20, 0x80, 0x01],
        replace: &[0xDC, 0xF8, 0x15, 0xB5, 0xFF],
    },
    // S2: `0x574B_1DD6` DID limit check (`cmp 0x20 -> 0x100, d0`)
    SlicePatchSite {
        offset: va_off(0x574B_1DD6),
        expect: &[0xA0, 0x20, 0xC6, 0x05, 0xCC, 0x55, 0xFF],
        replace: &[0xDC, 0x57, 0xE5, 0xB4, 0xFF, 0xCB, 0xCB],
    },
    // S3: `0x574B_1DE9` DID base offset (`mov 0x20 -> 0x100, d3`)
    SlicePatchSite {
        offset: va_off(0x574B_1DE9),
        expect: &[0x8F, 0x20, 0x3D, 0x21, 0x18],
        replace: &[0xDC, 0x54, 0xE5, 0xB4, 0xFF],
    },
    // S4: `0x574B_1E06` DID mask/offset (`0x1F -> 0xFF, a3`, `0x1F00 -> 0xFF00, d0`)
    SlicePatchSite {
        offset: va_off(0x574B_1E06),
        expect: &[0x9F, 0x1F, 0x58, 0x18, 0xFA, 0xC0, 0x00, 0x1F],
        replace: &[0xDC, 0x42, 0xE5, 0xB4, 0xFF, 0xCB, 0xCB, 0xCB],
    },
    // S5: `0x574B_2007` DID ceiling (`mov 0x1F -> 0xFF, d0`)
    SlicePatchSite {
        offset: va_off(0x574B_2007),
        expect: &[0x80, 0x1F, 0xF8, 0x12, 0x0C],
        replace: &[0xDC, 0x50, 0xE3, 0xB4, 0xFF],
    },
    // S6A: `0x574B_2426` (`mov 0x1F -> 0xFF, d0` path)
    SlicePatchSite {
        offset: va_off(0x574B_2426),
        expect: &[0x80, 0x1F, 0xCA, 0x04, 0x28],
        replace: &[0xDC, 0x3C, 0xDF, 0xB4, 0xFF],
    },
    // S6B: `0x574B_242B` (`add -1, d0` path via `0x574B_2410`)
    SlicePatchSite {
        offset: va_off(0x574B_242B),
        expect: &[0xFF, 0xF8, 0x12, 0x0C, 0x48],
        replace: &[0xDC, 0x43, 0xDF, 0xB4, 0xFF],
    },
    // S7: `0x574B_2677` (`cmp 0x1F -> 0xFF, d2`)
    SlicePatchSite {
        offset: va_off(0x574B_2677),
        expect: &[0xAA, 0x1F, 0xC7, 0x2B, 0x3D],
        replace: &[0xDC, 0x02, 0xDD, 0xB4, 0xFF],
    },
    // S8: `0x574B_2856` (`mov 0x20 -> 0x100, d3`)
    SlicePatchSite {
        offset: va_off(0x574B_2856),
        expect: &[0x8F, 0x20, 0x08, 0xF9, 0xE8, 0x03],
        replace: &[0xDC, 0x34, 0xDB, 0xB4, 0xFF, 0xCB],
    },
    // S9: `0x574B_3035` (`and/add 0x1F00 -> 0xFF00, d2`)
    SlicePatchSite {
        offset: va_off(0x574B_3035),
        expect: &[0xFA, 0xC2, 0x00, 0x1F, 0x88],
        replace: &[0xDC, 0x61, 0xD3, 0xB4, 0xFF],
    },
    // S10: `0x574B_416B` (`cmp 0x1F -> 0xFF, d2`)
    SlicePatchSite {
        offset: va_off(0x574B_416B),
        expect: &[0xAA, 0x1F, 0xC7, 0x1F, 0x80, 0x60],
        replace: &[0xDC, 0x37, 0xC2, 0xB4, 0xFF, 0xCB],
    },
    // S11: `0x574B_4385` (`cmp 0x20 -> 0x100, a0`)
    SlicePatchSite {
        offset: va_off(0x574B_4385),
        expect: &[0xB0, 0x20, 0xC4, 0x26, 0x72],
        replace: &[0xDC, 0x2F, 0xC0, 0xB4, 0xFF],
    },
    // Contiguous 165-byte trampoline cave in zero-xref padding at `0x5700_0320..0x5700_03C5`
    SlicePatchSite {
        offset: va_off(0x5700_0320),
        expect: &[0x00u8; 165],
        replace: &[
            // S1  @ 0x5700_0320 (13 B): mov 0x10000,d3; mov 1,d0; jmp 0x574AED2D
            0xFC, 0xCF, 0x00, 0x00, 0x01, 0x00, 0x80, 0x01, 0xDC, 0x05, 0xEA, 0x4A, 0x00,
            // S2  @ 0x5700_032D (16 B): cmp 0x100,d0; bge +7; jmp 0x574B1D2F; jmp 0x574B1DDD
            0xFA, 0xC8, 0x00, 0x01, 0xC6, 0x07, 0xDC, 0xFC, 0x19, 0x4B, 0x00, 0xDC, 0xA5, 0x1A,
            0x4B, 0x00,
            // S3  @ 0x5700_033D (11 B): mov 0x100,d3; mov sp,a1; add 0x18,a1; jmp 0x574B1DEE
            0x2F, 0x00, 0x01, 0x3D, 0x21, 0x18, 0xDC, 0xAB, 0x1A, 0x4B, 0x00,
            // S4  @ 0x5700_0348 (15 B): mov 0xFF,a3; mov (0x18,sp),d0; add 0xFF00,d0; jmp 0x574B1E0E
            0x9F, 0xFF, 0x58, 0x18, 0xFC, 0xC0, 0x00, 0xFF, 0x00, 0x00, 0xDC, 0xBC, 0x1A, 0x4B,
            0x00,
            // S5  @ 0x5700_0357 (11 B): mov 0xFF,d0; mov d0,(0x0C,a2); jmp 0x574B200C
            0x2C, 0xFF, 0x00, 0xF8, 0x12, 0x0C, 0xDC, 0xAF, 0x1C, 0x4B, 0x00,
            // S6A @ 0x5700_0362 (12 B): mov 0xFF,d0; mov d0,(0x0C,a2); inc d2; jmp 0x574B2430
            0x2C, 0xFF, 0x00, 0xF8, 0x12, 0x0C, 0x48, 0xDC, 0xC7, 0x20, 0x4B, 0x00,
            // S6B @ 0x5700_036E (11 B): add -1,d0; mov d0,(0x0C,a2); inc d2; jmp 0x574B2430
            0x28, 0xFF, 0xF8, 0x12, 0x0C, 0x48, 0xDC, 0xBC, 0x20, 0x4B, 0x00,
            // S7  @ 0x5700_0379 (17 B): cmp 0xFF,d2; bge +8; mov sp,a1; jmp 0x574B267C; jmp 0x574B26A4
            0xFA, 0xCA, 0xFF, 0x00, 0xC7, 0x08, 0x3D, 0xDC, 0xFC, 0x22, 0x4B, 0x00, 0xDC, 0x1F,
            0x23, 0x4B, 0x00,
            // S8  @ 0x5700_038A (12 B): mov 0x100,d3; clr d2; mov xr0,d3; jmp 0x574B285C
            0x2F, 0x00, 0x01, 0x08, 0xF9, 0xE8, 0x03, 0xDC, 0xCB, 0x24, 0x4B, 0x00,
            // S9  @ 0x5700_0396 (12 B): add 0xFF00,d2; mov d2,d0; jmp 0x574B303A
            0xFC, 0xC2, 0x00, 0xFF, 0x00, 0x00, 0x88, 0xDC, 0x9D, 0x2C, 0x4B, 0x00,
            // S10 @ 0x5700_03A2 (18 B): cmp 0xFF,d2; bge +9; mov 0x60,d0; jmp 0x574B4171; jmp 0x574B418C
            0xFA, 0xCA, 0xFF, 0x00, 0xC7, 0x09, 0x80, 0x60, 0xDC, 0xC7, 0x3D, 0x4B, 0x00, 0xDC,
            0xDD, 0x3D, 0x4B, 0x00,
            // S11 @ 0x5700_03B4 (17 B): cmp 0x100,a0; blt/gt +8; mov (a2),d0; jmp 0x574B438A; jmp 0x574B43AD
            0xFA, 0xD8, 0x00, 0x01, 0xC4, 0x08, 0x72, 0xDC, 0xCF, 0x3F, 0x4B, 0x00, 0xDC, 0xED,
            0x3F, 0x4B, 0x00,
        ],
    },
];

const MAIN3060NP_RAM_RELOCS: &[U32RelocSpec] = &[
    U32RelocSpec {
        from: 0x52FE_6CE0,
        to: 0x531B_8000,
        expected_count: 11,
    },
    U32RelocSpec {
        from: 0x52FE_6CE8,
        to: 0x531B_8008,
        expected_count: 7,
    },
    U32RelocSpec {
        from: 0x52FE_6CEA,
        to: 0x531B_800A,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_6EE0,
        to: 0x531B_8200,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_74E0,
        to: 0x531B_8800,
        expected_count: 16,
    },
    U32RelocSpec {
        from: 0x52FE_88E0,
        to: 0x531B_A600,
        expected_count: 31,
    },
    U32RelocSpec {
        from: 0x52FE_89A0,
        to: 0x531B_A6C0,
        expected_count: 3,
    },
    U32RelocSpec {
        from: 0x52FE_89A2,
        to: 0x531B_A6C2,
        expected_count: 2,
    },
    U32RelocSpec {
        from: 0x52FE_89B0,
        to: 0x531B_A6D0,
        expected_count: 4,
    },
    U32RelocSpec {
        from: 0x52FE_8BB0,
        to: 0x531B_A8D0,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_8BC0,
        to: 0x531B_A8E0,
        expected_count: 17,
    },
    U32RelocSpec {
        from: 0x52FE_8DC0,
        to: 0x531B_AAE0,
        expected_count: 1,
    },
    // BGO DID table (`40 * 12 = 480 B` in 8 MiB vs 332 B in 4 MiB; prevents flash fntable smash).
    U32RelocSpec {
        from: 0x52FE_8FB4,
        to: 0x531B_B8F4,
        expected_count: 2,
    },
    U32RelocSpec {
        from: 0x52FE_8FC0,
        to: 0x531B_B900,
        expected_count: 8,
    },
    U32RelocSpec {
        from: 0x52FE_8FC4,
        to: 0x531B_B904,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_8FC8,
        to: 0x531B_B908,
        expected_count: 1,
    },
    // BGO sector map (`45 * 2 = 90 B` in 8 MiB vs 78 B in 4 MiB; prevents partition-state smash).
    U32RelocSpec {
        from: 0x52FE_9176,
        to: 0x531B_BB00,
        expected_count: 7,
    },
    U32RelocSpec {
        from: 0x52FE_9178,
        to: 0x531B_BB02,
        expected_count: 6,
    },
    U32RelocSpec {
        from: 0x52FE_917A,
        to: 0x531B_BB04,
        expected_count: 1,
    },
    // Spansion NOR JEDEC command addresses aligned with native 8 MiB Loader ver0.61 NOR IPL.
    U32RelocSpec {
        from: 0x8420_0000,
        to: 0x8413_0000,
        expected_count: 2,
    },
    U32RelocSpec {
        from: 0x8420_0002,
        to: 0x8413_0002,
        expected_count: 3,
    },
    U32RelocSpec {
        from: 0x8420_001C,
        to: 0x8413_001C,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x8420_0554,
        to: 0x8413_0554,
        expected_count: 10,
    },
    U32RelocSpec {
        from: 0x8420_0AAA,
        to: 0x8413_0AAA,
        expected_count: 11,
    },
];

/// Verified `MAIN3060NP` (`4 MiB Top-Boot` $\leftrightarrow$ `8 MiB Uniform`) BGO retarget specification.
pub const MAIN3060NP_BGO_8MB_SPEC: MainBgoRetargetSpec = MainBgoRetargetSpec {
    target_geometry_id: MAIN_BGO_STATE_8MB,
    factory_geometry_id: MAIN_BGO_STATE_FACTORY_4MB,
    base_va: AM34_MAIN_BASE_VA,
    expected_size: EXPECTED_MAIN_3060NP_SIZE,
    reloc_scan_start: va_off(0x574A_6000),
    reloc_scan_end: va_off(0x574B_4500),
    table_detect_offset: va_off(0x579E_D354),
    table_factory_base: 0x8420_0000,
    table_target_base: 0x8413_0000,
    word_sites: MAIN3060NP_WORD_SITES,
    byte_sites: MAIN3060NP_BYTE_SITES,
    slice_sites: MAIN3060NP_SLICE_SITES,
    ram_relocs: MAIN3060NP_RAM_RELOCS,
};

fn count_u32_le(slice: &[u8], needle: u32) -> usize {
    let target = needle.to_le_bytes();
    let mut hits = 0usize;
    let mut idx = 0usize;
    while idx + 4 <= slice.len() {
        if slice[idx..idx + 4] == target {
            hits += 1;
            idx += 4;
        } else {
            idx += 1;
        }
    }
    hits
}

fn replace_u32_le(slice: &mut [u8], from: u32, to: u32) -> usize {
    let src = from.to_le_bytes();
    let dst = to.to_le_bytes();
    let mut hits = 0usize;
    let mut idx = 0usize;
    while idx + 4 <= slice.len() {
        if slice[idx..idx + 4] == src {
            slice[idx..idx + 4].copy_from_slice(&dst);
            hits += 1;
            idx += 4;
        } else {
            idx += 1;
        }
    }
    hits
}

fn classify_sites(buf: &[u8], spec: &MainBgoRetargetSpec) -> (bool, bool) {
    let scan = &buf[spec.reloc_scan_start..spec.reloc_scan_end];
    let is_factory = spec
        .word_sites
        .iter()
        .all(|s| LittleEndian::read_u32(&buf[s.offset..s.offset + 4]) == s.expect)
        && spec.byte_sites.iter().all(|s| buf[s.offset] == s.expect)
        && spec
            .slice_sites
            .iter()
            .all(|s| &buf[s.offset..s.offset + s.expect.len()] == s.expect)
        && spec.ram_relocs.iter().all(|r| {
            count_u32_le(scan, r.from) == r.expected_count && count_u32_le(scan, r.to) == 0
        });

    let is_target = spec
        .word_sites
        .iter()
        .all(|s| LittleEndian::read_u32(&buf[s.offset..s.offset + 4]) == s.replace)
        && spec.byte_sites.iter().all(|s| buf[s.offset] == s.replace)
        && spec
            .slice_sites
            .iter()
            .all(|s| &buf[s.offset..s.offset + s.replace.len()] == s.replace)
        && spec.ram_relocs.iter().all(|r| {
            count_u32_le(scan, r.to) == r.expected_count && count_u32_le(scan, r.from) == 0
        });

    (is_factory, is_target)
}

/// Inspects an unpacked `MAIN` binary against a declarative [`MainBgoRetargetSpec`] (`fail-closed`).
pub fn inspect_main_bgo_geometry_for_spec(
    buf: &[u8],
    spec: &MainBgoRetargetSpec,
) -> Result<MainBgoGeometryStatus> {
    if buf.len() != spec.expected_size {
        bail!(
            "MAIN BGO retarget '{}' requires {} B image, got {} B",
            spec.target_geometry_id,
            spec.expected_size,
            buf.len()
        );
    }
    let (is_factory, is_target) = classify_sites(buf, spec);
    let (state, geom_id) = match (is_factory, is_target) {
        (true, false) => (
            spec.factory_geometry_id.to_string(),
            Some(spec.factory_geometry_id.to_string()),
        ),
        (false, true) => (
            spec.target_geometry_id.to_string(),
            Some(spec.target_geometry_id.to_string()),
        ),
        _ => (MAIN_BGO_STATE_MISMATCH.to_string(), None),
    };
    let table_va = spec.base_va + spec.table_detect_offset as u32;
    let flash_base =
        LittleEndian::read_u32(&buf[spec.table_detect_offset..spec.table_detect_offset + 4]);
    let bss_off = spec.word_sites[0].offset;
    let nfd_bss_end = LittleEndian::read_u32(&buf[bss_off..bss_off + 4]);
    Ok(MainBgoGeometryStatus {
        state,
        geometry_id: geom_id,
        table_va_hex: format!("0x{table_va:08X}"),
        bgo_flash_base_hex: format!("0x{flash_base:08X}"),
        nfd_bss_end_hex: format!("0x{nfd_bss_end:08X}"),
    })
}

/// Applies or reverts a [`MainBgoRetargetSpec`] idempotently and fail-closed in place.
pub fn apply_main_bgo_retarget_for_spec_in_place(
    buf: &mut [u8],
    spec: &MainBgoRetargetSpec,
    enable: bool,
) -> Result<()> {
    let pre = inspect_main_bgo_geometry_for_spec(buf, spec)?;
    if enable && pre.state == spec.target_geometry_id {
        return Ok(());
    }
    if !enable && pre.state == spec.factory_geometry_id {
        return Ok(());
    }
    if enable && pre.state != spec.factory_geometry_id {
        bail!(
            "Refusing to apply MAIN BGO retarget '{}': image in state '{}' (flash={}, bss_end={})",
            spec.target_geometry_id,
            pre.state,
            pre.bgo_flash_base_hex,
            pre.nfd_bss_end_hex
        );
    }
    if !enable && pre.state != spec.target_geometry_id {
        bail!(
            "Refusing to revert MAIN BGO retarget '{}': image in state '{}'",
            spec.target_geometry_id,
            pre.state
        );
    }

    for site in spec.word_sites {
        let val = if enable { site.replace } else { site.expect };
        LittleEndian::write_u32(&mut buf[site.offset..site.offset + 4], val);
    }
    for site in spec.byte_sites {
        buf[site.offset] = if enable { site.replace } else { site.expect };
    }
    for site in spec.slice_sites {
        let slice = if enable { site.replace } else { site.expect };
        buf[site.offset..site.offset + slice.len()].copy_from_slice(slice);
    }
    let scan = &mut buf[spec.reloc_scan_start..spec.reloc_scan_end];
    for reloc in spec.ram_relocs {
        let (src, dst) = if enable {
            (reloc.from, reloc.to)
        } else {
            (reloc.to, reloc.from)
        };
        let replaced = replace_u32_le(scan, src, dst);
        if replaced != reloc.expected_count {
            bail!(
                "MAIN BGO RAM relocation 0x{src:08X}->0x{dst:08X} count mismatch: {replaced} != {}",
                reloc.expected_count
            );
        }
    }

    let post = inspect_main_bgo_geometry_for_spec(buf, spec)?;
    let expected = if enable {
        spec.target_geometry_id
    } else {
        spec.factory_geometry_id
    };
    if post.state != expected {
        bail!(
            "MAIN BGO post-retarget verification failed: expected '{expected}', got '{}'",
            post.state
        );
    }
    Ok(())
}

/// Inspects `MAIN3060NP` BGO/NFROM NOR geometry (`4 MiB Top-Boot` vs `8 MiB Uniform`).
pub fn inspect_main_bgo_geometry(buf: &[u8]) -> Result<MainBgoGeometryStatus> {
    inspect_main_bgo_geometry_for_spec(buf, &MAIN3060NP_BGO_8MB_SPEC)
}

/// Applies or reverts the `8 MiB` (`BGO_45SEC_8MB_0X84130000`) retarget on `MAIN3060NP` in place.
pub fn apply_main_bgo_retarget_in_place(buf: &mut [u8], enable: bool) -> Result<()> {
    apply_main_bgo_retarget_for_spec_in_place(buf, &MAIN3060NP_BGO_8MB_SPEC, enable)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::constants::{AM34_FLASH_UNCACHED_BASE, AM34_MAIN_BASE_VA};
    use crate::nvram::calculate_hashes;
    use crate::re::Mn103Vm;

    use super::*;

    const VA_NFD_INIT: u32 = 0x574A_68E9;
    const VA_NFD_READ: u32 = 0x574A_6BA8;
    const VA_BGO_INIT: u32 = 0x574A_8B82;
    const VA_BGO_TASK_5C: u32 = 0x574B_04A2;
    const VA_BGO_TASK_5C_STACK_TOP: u32 = 0x5180_1300;
    const VA_ITRON_SET_FLG: u32 = 0x5772_A65F;
    const VA_ITRON_CLR_FLG: u32 = 0x5772_A726;
    const VA_ITRON_WAI_FLG: u32 = 0x5772_A76F;
    const VA_ITRON_POL_FLG: u32 = 0x5772_A789;
    const VA_ITRON_STA_TSK: u32 = 0x5772_F088;
    const VA_ITRON_DLY_TSK: u32 = 0x5772_F78F;
    const VA_MAIN_CALLER_STUB: u32 = 0x5080_F000;
    const VA_MAIN_STACK_TOP: u32 = 0x50FF_F800;
    const VA_MAIN_OUT_BUF: u32 = 0x50FE_0000;

    fn build_synthetic_main3060np_bgo_fixture() -> Vec<u8> {
        let s = &MAIN3060NP_BGO_8MB_SPEC;
        let mut buf = vec![0xCBu8; s.expected_size];
        for site in s.word_sites {
            LittleEndian::write_u32(&mut buf[site.offset..site.offset + 4], site.expect);
        }
        for site in s.byte_sites {
            buf[site.offset] = site.expect;
        }
        for site in s.slice_sites {
            buf[site.offset..site.offset + site.expect.len()].copy_from_slice(site.expect);
        }
        let mut cursor = s.reloc_scan_start;
        for reloc in s.ram_relocs {
            for _ in 0..reloc.expected_count {
                LittleEndian::write_u32(&mut buf[cursor..cursor + 4], reloc.from);
                cursor += 4;
            }
        }
        buf
    }

    fn patch_ret0_stub(main_seg: &mut [u8], va: u32) {
        let off = va_off(va);
        // CLR D0; RET [], 0 -> returns 0 cleanly to CALL callers.
        main_seg[off..off + 4].copy_from_slice(&[0x00, 0xDF, 0x00, 0x00]);
    }

    fn map_main_bgo_vm_with_policy(
        main_img: &[u8],
        nor_8mb: &[u8],
        stub: Vec<u8>,
        allow_upper_nor_writes: bool,
    ) -> Mn103Vm {
        let mut main_seg = main_img.to_vec();
        // Stub ONLY external non-BGO RTOS/assert/printf wrappers (the 6 ITRON sync primitives
        // sta_tsk/dly_tsk/set_flg/clr_flg/wai_flg/pol_flg are intercepted by ItronTwoTaskRunner).
        for ext_va in [
            0x570B_97C5, // System assert! / debug print
            0x574A_A7AC, // NFD assert post hook
            0x5750_E2AD, // Post-init notification
            0x5769_7915, // Misc error callback
            0x5772_A12B,
            0x5772_A146,
            0x5772_A166,
            0x5772_B531,
            0x5772_B626,
            0x5772_F6D7,
            0x5773_00D8,
            0x5773_018F,
        ] {
            patch_ret0_stub(&mut main_seg, ext_va);
        }
        // External I2C EEPROM reader (not mapped in NOR-only VM): `MOV 0x0960, D0; RET [], 0`.
        let eeprom_off = va_off(0x5740_E2BA);
        main_seg[eeprom_off..eeprom_off + 6].copy_from_slice(&[0x2C, 0x60, 0x09, 0xDF, 0x00, 0x00]);

        let mut vm = Mn103Vm::new();
        if allow_upper_nor_writes {
            vm.nor_write_policy =
                crate::pico::rsp_codec::policy::NorWritePolicy::UpperNorAppUnlocked;
        }
        vm.map_segment(AM34_MAIN_BASE_VA, main_seg, false);
        vm.map_segment(
            AM34_FLASH_UNCACHED_BASE,
            nor_8mb.to_vec(),
            allow_upper_nor_writes,
        );
        vm.map_segment(0xDB00_0000, vec![0u8; 0x1000], true);
        let data_rom_start = va_off(0x57C9_95D0);
        let data_len = 0x3_4E2Cusize;
        vm.map_segment(
            0x5159_8000,
            main_img[data_rom_start..data_rom_start + data_len].to_vec(),
            true,
        );
        vm.map_segment(0x5177_4000, vec![0u8; 0x2000], true);
        vm.map_segment(0x5180_0000, vec![0u8; 0x2000], true);
        vm.map_segment(0x52FE_6000, vec![0u8; 0x7000], true);
        vm.map_segment(0x5306_9000, vec![0u8; 0x1000], true);
        vm.map_segment(0x531B_8000, vec![0u8; 0x4000], true);
        vm.map_segment(VA_MAIN_OUT_BUF, vec![0u8; 0x4000], true);
        vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
        vm.map_segment(VA_MAIN_CALLER_STUB, stub, false);
        vm
    }

    #[derive(Clone, Default)]
    struct FiberCtx {
        d: [u32; 4],
        a: [u32; 4],
        r: [u32; 8],
        sp: u32,
        pc: u32,
        mdr: u32,
        mdrq: u32,
        mcrh: u32,
        mcrl: u32,
        mcvf: u32,
        lir: u32,
        lar: u32,
        psw: u16,
        epsw: u32,
        psw_undef: u16,
    }

    impl FiberCtx {
        fn from_vm(vm: &Mn103Vm) -> Self {
            Self {
                d: vm.d,
                a: vm.a,
                r: vm.r,
                sp: vm.sp,
                pc: vm.pc,
                mdr: vm.mdr,
                mdrq: vm.mdrq,
                mcrh: vm.mcrh,
                mcrl: vm.mcrl,
                mcvf: vm.mcvf,
                lir: vm.lir,
                lar: vm.lar,
                psw: vm.psw,
                epsw: vm.epsw,
                psw_undef: vm.psw_undef,
            }
        }

        fn apply_to_vm(&self, vm: &mut Mn103Vm) {
            vm.d = self.d;
            vm.a = self.a;
            vm.r = self.r;
            vm.sp = self.sp;
            vm.pc = self.pc;
            vm.mdr = self.mdr;
            vm.mdrq = self.mdrq;
            vm.mcrh = self.mcrh;
            vm.mcrl = self.mcrl;
            vm.mcvf = self.mcvf;
            vm.lir = self.lir;
            vm.lar = self.lar;
            vm.psw = self.psw;
            vm.epsw = self.epsw;
            vm.psw_undef = self.psw_undef;
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum FiberState {
        Idle,
        Runnable,
        WaitFlg {
            flgid: u32,
            waiptn: u32,
            wfmode: u32,
            p_flgptn: u32,
        },
    }

    struct ItronTwoTaskRunner {
        vm: Mn103Vm,
        flags: std::collections::HashMap<u32, u32>,
        active: usize,
        other: FiberCtx,
        states: [FiberState; 2],
    }

    impl ItronTwoTaskRunner {
        fn new(vm: Mn103Vm) -> Self {
            Self {
                vm,
                flags: std::collections::HashMap::new(),
                active: 0,
                other: FiberCtx::default(),
                states: [FiberState::Runnable, FiberState::Idle],
            }
        }

        fn flg_matches(cur: u32, waiptn: u32, wfmode: u32) -> bool {
            if (wfmode & 0x02) != 0 {
                (cur & waiptn) != 0
            } else {
                (cur & waiptn) == waiptn
            }
        }

        fn switch_fiber(&mut self) {
            let cur_saved = FiberCtx::from_vm(&self.vm);
            self.other.apply_to_vm(&mut self.vm);
            self.other = cur_saved;
            self.active = 1 - self.active;
        }

        fn format_err(&self, label: &str, ring: &std::collections::VecDeque<String>) -> String {
            let trace: Vec<String> = ring.iter().cloned().collect();
            format!(
                "{label} (fiber={}, pc=0x{:08X}, d0=0x{:X}, sp=0x{:08X}, flags={:?})\ntrace:\n  {}",
                self.active,
                self.vm.pc,
                self.vm.d[0],
                self.vm.sp,
                self.flags,
                trace.join("\n  ")
            )
        }

        fn complete_flg_wait(
            &mut self,
            flgid: u32,
            wfmode: u32,
            p_flgptn: u32,
            cur: u32,
        ) -> Result<(), String> {
            if p_flgptn != 0 {
                self.vm
                    .write_u32(p_flgptn, cur)
                    .map_err(|e| format!("p_flgptn write err: {e:?}"))?;
            }
            if (wfmode & 0x01) != 0 {
                self.flags.insert(flgid, 0);
            }
            Ok(())
        }

        fn handle_itron_svc(&mut self) -> Result<Option<&'static str>, String> {
            let pc = self.vm.pc;
            let ret_pc = self
                .vm
                .read_u32(self.vm.sp)
                .map_err(|e| format!("ITRON stack ret read failed @ 0x{pc:08X}: {e:?}"))?;
            match pc {
                VA_ITRON_STA_TSK => {
                    let tid = self.vm.d[0];
                    self.vm.d[0] = 0;
                    self.vm.pc = ret_pc;
                    if tid == 0x5C && self.states[1] == FiberState::Idle {
                        self.other = FiberCtx {
                            pc: VA_BGO_TASK_5C,
                            sp: VA_BGO_TASK_5C_STACK_TOP,
                            ..Default::default()
                        };
                        self.states[1] = FiberState::Runnable;
                    }
                    Ok(None)
                }
                VA_ITRON_DLY_TSK => {
                    self.vm.d[0] = 0;
                    self.vm.pc = ret_pc;
                    if self.states[1 - self.active] == FiberState::Runnable {
                        self.switch_fiber();
                    }
                    Ok(None)
                }
                VA_ITRON_SET_FLG => {
                    let flgid = self.vm.d[0];
                    let setptn = self.vm.d[1];
                    let cur = {
                        let e = self.flags.entry(flgid).or_default();
                        *e |= setptn;
                        *e
                    };
                    let peer = 1 - self.active;
                    if let FiberState::WaitFlg {
                        flgid: w_id,
                        waiptn,
                        wfmode,
                        p_flgptn,
                    } = self.states[peer]
                        && w_id == flgid
                        && Self::flg_matches(cur, waiptn, wfmode)
                    {
                        self.complete_flg_wait(flgid, wfmode, p_flgptn, cur)?;
                        self.other.d[0] = 0;
                        self.states[peer] = FiberState::Runnable;
                    }
                    self.vm.d[0] = 0;
                    self.vm.pc = ret_pc;
                    Ok(None)
                }
                VA_ITRON_CLR_FLG => {
                    let flgid = self.vm.d[0];
                    let clrptn = self.vm.d[1];
                    let e = self.flags.entry(flgid).or_default();
                    *e &= clrptn;
                    self.vm.d[0] = 0;
                    self.vm.pc = ret_pc;
                    Ok(None)
                }
                VA_ITRON_POL_FLG | VA_ITRON_WAI_FLG => {
                    let p_flgptn = self.vm.d[0];
                    let flgid = self.vm.d[1];
                    let waiptn = self
                        .vm
                        .read_u32(self.vm.sp.wrapping_add(0x0C))
                        .map_err(|e| format!("waiptn read err: {e:?}"))?;
                    let wfmode = self
                        .vm
                        .read_u32(self.vm.sp.wrapping_add(0x10))
                        .map_err(|e| format!("wfmode read err: {e:?}"))?;
                    let cur = *self.flags.get(&flgid).unwrap_or(&0);
                    if Self::flg_matches(cur, waiptn, wfmode) {
                        self.complete_flg_wait(flgid, wfmode, p_flgptn, cur)?;
                        self.vm.d[0] = 0;
                        self.vm.pc = ret_pc;
                        Ok(None)
                    } else if pc == VA_ITRON_POL_FLG {
                        self.vm.d[0] = 0xFFFF_FFCE;
                        self.vm.pc = ret_pc;
                        Ok(None)
                    } else {
                        let peer = 1 - self.active;
                        if self.states[peer] != FiberState::Runnable {
                            return Ok(Some("Deadlock: both ITRON fibers blocked on wai_flg"));
                        }
                        self.vm.pc = ret_pc;
                        self.states[self.active] = FiberState::WaitFlg {
                            flgid,
                            waiptn,
                            wfmode,
                            p_flgptn,
                        };
                        self.switch_fiber();
                        Ok(None)
                    }
                }
                _ => Ok(None),
            }
        }

        fn run_caller_until(&mut self, stop_pc: u32, max_steps: u64) -> Result<(), String> {
            let start_steps = self.vm.steps;
            let mut ring = std::collections::VecDeque::with_capacity(48);
            while !(self.active == 0 && self.vm.pc == stop_pc) {
                if self.vm.steps - start_steps >= max_steps {
                    return Err(self.format_err("StepLimit", &ring));
                }
                if matches!(
                    self.vm.pc,
                    VA_ITRON_SET_FLG
                        | VA_ITRON_CLR_FLG
                        | VA_ITRON_WAI_FLG
                        | VA_ITRON_POL_FLG
                        | VA_ITRON_STA_TSK
                        | VA_ITRON_DLY_TSK
                ) {
                    if ring.len() == 48 {
                        ring.pop_front();
                    }
                    ring.push_back(format!(
                        "f{} SVC pc=0x{:08X} d0=0x{:X} d1=0x{:X} sp=0x{:08X}",
                        self.active, self.vm.pc, self.vm.d[0], self.vm.d[1], self.vm.sp
                    ));
                    if let Some(deadlock_msg) = self.handle_itron_svc()? {
                        return Err(self.format_err(deadlock_msg, &ring));
                    }
                    continue;
                }
                let op = self.vm.read_u8(self.vm.pc).unwrap_or(0xFF);
                if ring.len() == 48 {
                    ring.pop_front();
                }
                ring.push_back(format!(
                    "f{} pc=0x{:08X} op=0x{:02X} sp=0x{:08X} d0=0x{:X} d1=0x{:X} d2=0x{:X} a0=0x{:08X}",
                    self.active, self.vm.pc, op, self.vm.sp, self.vm.d[0], self.vm.d[1], self.vm.d[2], self.vm.a[0]
                ));
                if let Err(e) = self.vm.step() {
                    return Err(self.format_err(&format!("{e:?}"), &ring));
                }
            }
            Ok(())
        }
    }

    fn call_main_va_with_policy(
        main_img: &[u8],
        nor_8mb: &[u8],
        fn_va: u32,
        d0: u32,
        d1: u32,
        a0: u32,
        allow_upper_nor_writes: bool,
    ) -> Result<(u32, String), String> {
        let mut stub = vec![0xCBu8; 0x100];
        let rel = fn_va.wrapping_sub(VA_MAIN_CALLER_STUB).to_le_bytes();
        stub[0..7].copy_from_slice(&[0xDD, rel[0], rel[1], rel[2], rel[3], 0x00, 0x00]);
        let mut vm = map_main_bgo_vm_with_policy(main_img, nor_8mb, stub, allow_upper_nor_writes);

        vm.d[0] = d0;
        vm.d[1] = d1;
        vm.a[0] = a0;
        vm.sp = VA_MAIN_STACK_TOP;
        vm.pc = VA_MAIN_CALLER_STUB;
        let mut runner = ItronTwoTaskRunner::new(vm);
        runner.run_caller_until(VA_MAIN_CALLER_STUB + 7, 4_000_000)?;
        let (_, nor_sha) = calculate_hashes(
            runner
                .vm
                .segment_bytes(AM34_FLASH_UNCACHED_BASE)
                .ok_or_else(|| "Missing NOR segment".to_string())?,
        );
        Ok((runner.vm.d[0], nor_sha))
    }

    #[test]
    fn main3060np_bgo_retarget_roundtrips_and_fails_closed_on_corruption() {
        let orig = build_synthetic_main3060np_bgo_fixture();
        let (_, orig_sha) = calculate_hashes(&orig);
        let mut work = orig;

        assert_eq!(
            inspect_main_bgo_geometry(&work).unwrap().state,
            MAIN_BGO_STATE_FACTORY_4MB
        );
        apply_main_bgo_retarget_in_place(&mut work, true).unwrap();
        // Idempotent reapply
        apply_main_bgo_retarget_in_place(&mut work, true).unwrap();
        let st8 = inspect_main_bgo_geometry(&work).unwrap();
        assert_eq!(st8.state, MAIN_BGO_STATE_8MB);
        assert_eq!(st8.bgo_flash_base_hex, "0x84130000");
        assert_eq!(st8.nfd_bss_end_hex, "0x531BC000");

        apply_main_bgo_retarget_in_place(&mut work, false).unwrap();
        assert_eq!(calculate_hashes(&work).1, orig_sha);

        // Corrupt a single word site -> fail-closed
        work[va_off(0x579E_D354)] ^= 0x01;
        assert_eq!(
            inspect_main_bgo_geometry(&work).unwrap().state,
            MAIN_BGO_STATE_MISMATCH
        );
        assert!(apply_main_bgo_retarget_in_place(&mut work, true).is_err());
    }

    #[test]
    fn mn103vm_proves_main3060np_bgo_8mb_retarget_against_real_eh55_nor() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let main_path = root
            .join("firmware_and_images/extracted_partitions/frm_unpacked/main3060np_unpacked.bin");
        let nor_path = root.join("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
        if !main_path.is_file() || !nor_path.is_file() {
            return;
        }

        let main_orig = std::fs::read(&main_path).unwrap();
        let nor_8mb = std::fs::read(&nor_path).unwrap();
        let (_, orig_sha) = calculate_hashes(&main_orig);
        let (_, nor_before_sha) = calculate_hashes(&nor_8mb);

        // 1. Negative control: unpatched MAIN3060NP fails BGO_Init() / NFD_Init() on 8 MiB EH55 NOR!
        //    Simulate real silicon Upper-NOR writability on an in-memory clone (IPL SA0..SA10 stays RO).
        let (neg_rc, _) =
            call_main_va_with_policy(&main_orig, &nor_8mb, VA_BGO_INIT, 0, 0, 0, true)
                .expect("Unpatched MAIN3060NP BGO_Init() execution faulted");
        assert_eq!(
            neg_rc, 1,
            "Unpatched MAIN3060NP must reproduce live BGO driver init failed[1] against real 8 MiB NOR"
        );

        // 2. Apply retarget 4 MiB -> 8 MiB in place and execute real BGO_Init(), NFD_Init(),
        //    and NFD_Read(17/22/12) in Mn103Vm against real 8 MiB EH55 NOR (IPL SA0..SA10 stays E05 RO;
        //    0x574AFCA7 idempotent re-strobe executes on clone and NOR SHA-256 must be unchanged).
        let mut main_8mb = main_orig;
        assert_eq!(
            inspect_main_bgo_geometry(&main_8mb).unwrap().state,
            MAIN_BGO_STATE_FACTORY_4MB
        );
        apply_main_bgo_retarget_in_place(&mut main_8mb, true).unwrap();
        assert_eq!(
            inspect_main_bgo_geometry(&main_8mb).unwrap().state,
            MAIN_BGO_STATE_8MB
        );

        let (bgo_rc, bgo_nor_sha) =
            call_main_va_with_policy(&main_8mb, &nor_8mb, VA_BGO_INIT, 0, 0, 0, true)
                .expect("Retargeted BGO_Init() failed inside Mn103Vm");
        assert_eq!(bgo_rc, 0, "Retargeted MAIN3060NP BGO_Init() must return 0");
        assert_eq!(
            bgo_nor_sha, nor_before_sha,
            "Retargeted BGO_Init() mutated NOR bits on clean 8 MiB image"
        );

        // Run full NFD_Init() AND NFD_Read(17/22/12) inside one continuous VM session!
        let mut stub = vec![0xCBu8; 0x100];
        let rel_init = VA_NFD_INIT.wrapping_sub(VA_MAIN_CALLER_STUB).to_le_bytes();
        stub[0..7].copy_from_slice(&[
            0xDD,
            rel_init[0],
            rel_init[1],
            rel_init[2],
            rel_init[3],
            0x00,
            0x00,
        ]);
        let rel_read = VA_NFD_READ
            .wrapping_sub(VA_MAIN_CALLER_STUB + 16)
            .to_le_bytes();
        stub[16..23].copy_from_slice(&[
            0xDD,
            rel_read[0],
            rel_read[1],
            rel_read[2],
            rel_read[3],
            0x00,
            0x00,
        ]);
        let mut vm = map_main_bgo_vm_with_policy(&main_8mb, &nor_8mb, stub, true);
        vm.sp = VA_MAIN_STACK_TOP;
        vm.pc = VA_MAIN_CALLER_STUB;
        let mut runner = ItronTwoTaskRunner::new(vm);
        runner
            .run_caller_until(VA_MAIN_CALLER_STUB + 7, 4_000_000)
            .unwrap_or_else(|e| panic!("NFD_Init failed:\n{e}"));
        assert_eq!(runner.vm.d[0], 0, "Retargeted NFD_Init() must return 0");
        assert_ne!(
            runner.vm.read_u32(0x515B_34C0).unwrap(),
            0,
            "NFD initialized flag 0x515B_34C0 must be non-zero"
        );
        assert_eq!(
            runner.vm.read_u16(0x515B_34DC).unwrap(),
            0x0001,
            "Spansion manufacturer ID must be autodetected cleanly"
        );
        assert_eq!(
            runner.vm.read_u16(0x515B_34DE).unwrap(),
            0x2202,
            "S29JL064H extended device ID cycle 2 stored after reading 0x227E at +0x02"
        );
        let init_nor_sha =
            calculate_hashes(runner.vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()).1;
        assert_eq!(
            init_nor_sha, nor_before_sha,
            "Retargeted NFD_Init() mutated NOR bits on clean 8 MiB image"
        );

        fn run_nfd_read_check(runner: &mut ItronTwoTaskRunner, file_no: u32, out_va: u32) {
            runner.vm.d[0] = file_no;
            runner.vm.d[1] = out_va;
            runner.vm.sp = VA_MAIN_STACK_TOP;
            runner.vm.pc = VA_MAIN_CALLER_STUB + 16;
            runner
                .run_caller_until(VA_MAIN_CALLER_STUB + 23, 4_000_000)
                .unwrap_or_else(|e| panic!("NFD_Read({file_no}) failed:\n{e}"));
            assert_eq!(
                runner.vm.d[0], 0,
                "Retargeted NFD_Read({file_no}) must return 0"
            );
        }

        // Lock Upper NOR strictly read-only before reading BGO files into VA_MAIN_OUT_BUF:
        // - file_no=17 ("update", sz=0 on repaired EH55 NOR) returns 0
        // - file_no=22 ("sys_hdd", sz=0x1F8) returns 0 with populated OSD payload
        // - file_no=12 ("rtsd1", sz=0x30E4) returns 0 across multi-page NFD page-map read
        runner.vm.nor_write_policy = crate::pico::rsp_codec::policy::NorWritePolicy::NvramOnly;
        run_nfd_read_check(&mut runner, 17, VA_MAIN_OUT_BUF);

        run_nfd_read_check(&mut runner, 22, VA_MAIN_OUT_BUF);
        assert!(
            runner.vm.segment_bytes(VA_MAIN_OUT_BUF).unwrap()[..0x1F8]
                .iter()
                .any(|&b| b != 0),
            "NFD_Read(22 'sys_hdd') produced all-zero buffer"
        );

        run_nfd_read_check(&mut runner, 12, VA_MAIN_OUT_BUF + 0x0800);
        assert!(
            runner.vm.segment_bytes(VA_MAIN_OUT_BUF).unwrap()[0x0800..0x0800 + 0x30E4]
                .iter()
                .any(|&b| b != 0),
            "NFD_Read(12 'rtsd1') produced all-zero buffer"
        );

        let read_nor_sha =
            calculate_hashes(runner.vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()).1;
        assert_eq!(
            read_nor_sha, nor_before_sha,
            "Retargeted NFD_Read mutated NOR bits"
        );

        // 3. Reversible roundtrip back to exact original SHA-256.
        apply_main_bgo_retarget_in_place(&mut main_8mb, false).unwrap();
        assert_eq!(calculate_hashes(&main_8mb).1, orig_sha);
    }
}
