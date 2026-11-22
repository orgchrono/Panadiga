#![deny(unsafe_code)]

//! Dedicated DMR-EH55 (`M6930LB` 8 MiB NOR BGO `0x84130000..0x84400000`) retarget for `PROG061`.
//!
//! Factory `PROG061 [05/11/01]` (`prog061_dec.bin`) was compiled for the DMR-EH50 (`M6930` 4 MiB NOR)
//! 2-region BGO layout (`0x84200000..0x84400000`, 35 physical / 28 logical sectors, `D_ID 0x0A01`).
//! On the DMR-EH55 (`M6930LB` 8 MiB Spansion `S29JL064H`), `Loader ver0.61` locates the `[nfrom]` BGO
//! partition at `0x84130000..0x84400000` (`45 x 64 KiB` uniform sectors `SA26..SA70`, 40 logical
//! sectors, 3876 pages, `D_ID 0x0F01..0x0F24`).
//!
//! We preserve [`Prog061BgoTargetGeometry::Eh50M6930_4Mb`] intact and retarget to
//! [`Prog061BgoTargetGeometry::Eh55M6930Lb8Mb`] by:
//! 1. Updating the `0x100`-byte BGO descriptor table at `0x036E2C..0x036F2C`.
//! 2. Transplanting `Loader ver0.61`'s `nfrom_drv.o` (`29,770` B) into `0x021E6A..0x029356` with
//!    relocated internal/table/RAM pointers, `memcpy`/`memset` `CALL32` targets, and `FUN_5040248e`
//!    entrypoint displacement (`0x0024C6`).
//! 3. Relocating the `HIGH_LEVEL_BGO` + `WRAPPER_BGO` RAM workspace pointers (`211` refs) into unused
//!    SDRAM `0x504FA000..0x504FE000`, updating `_start`'s `.bss` zero upper bound (`0x0007C0`), and
//!    patching the 34 verified `0x1CC4->0x26C4`, `D_ID 0x0Axx->0x0Fxx`, and page-map loop bounds.

use std::io::Read;

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use super::{OFF_BGO_SUM_BEQ, ORIG_SUM_BEQ, PATCH_SUM_BRA, PROG061_EXPECTED_SIZE};

const EMBEDDED_EH55_NFROM_DRV_GZ: &[u8] = include_bytes!("eh55_nfrom_drv.bin.gz");

pub const OFF_BGO_SUM_BEQ_EH55: usize = 0x0002_30CF;
pub(super) const OFF_BGO_TABLE: usize = 0x0003_6E2C;
const OFF_NFROM_DRV_START: usize = 0x0002_1E6A;
const OFF_NFROM_DRV_END: usize = 0x0002_9356;
const EH55_NFROM_DRV_LEN: usize = 29_770;
const OFF_BSS_ZERO_END: usize = 0x0000_07C0;
const ORIG_BSS_ZERO_END: u32 = 0x504F_9178;
const EH55_BSS_ZERO_END: u32 = 0x504F_E000;
const OFF_INIT_2_CALL: usize = 0x0000_24C6;
const ORIG_INIT_2_REL32: i32 = 0x0001_FE93;
const EH55_INIT_2_REL32: i32 = 0x0001_FE82;
const MN103_NOP: u8 = 0xCB;

/// Target BGO flash geometry compiled into a `PROG061` image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prog061BgoTargetGeometry {
    /// DMR-EH50 (`M6930`) 4 MiB NOR (`0x84200000..0x84400000`, 28 logical sectors, `D_ID 0x0A01`).
    #[default]
    Eh50M6930_4Mb,
    /// DMR-EH55 (`M6930LB`) 8 MiB NOR (`0x84130000..0x84400000`, 40 logical sectors, `D_ID 0x0F01`).
    Eh55M6930Lb8Mb,
}

impl Prog061BgoTargetGeometry {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eh50M6930_4Mb => "EH50_M6930_4MB",
            Self::Eh55M6930Lb8Mb => "EH55_M6930LB_8MB",
        }
    }
}

/// 64-word DMR-EH55 BGO geometry descriptor (`Loader ver0.61 @ 0x440337A0`) with `LO_RAM` at `0x504FD000..0x504FD818`.
const EH55_BGO_TABLE_WORDS: [u32; 64] = [
    0x8000_0000,
    0x0000_0000,
    0x0000_0001,
    0x0000_0028,
    0x0000_0000,
    0x0000_0000,
    0x0000_0000,
    0x0000_0F24,
    0x0000_0000,
    0x0001_0000,
    0x0000_0001,
    0x0000_002E,
    0x8413_0000,
    0x8440_0000,
    0x0001_0000,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0x0001_0000,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0x0001_0000,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0x0000_0000,
    0x0000_0003,
    0x0000_002D,
    0x0000_002D,
    0x0000_002D,
    0x504F_D164,
    0x504F_D5C4,
    0x504F_D5B4,
    0x504F_D6BC,
    0x504F_D6BA,
    0x504F_D594,
    0xFFFF_FFFF,
    0x0000_0000,
    0x0000_0028,
    0x0000_0028,
    0x0000_0028,
    0x0000_0028,
    0x0000_0001,
    0x8413_0000,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0x0000_0001,
    0x8413_0000,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
];

const HI_RAM_RELOCS: &[(u32, u32)] = &[
    (0x5049_6D10, 0x504F_A000),
    (0x5049_6D14, 0x504F_A004),
    (0x5049_6D16, 0x504F_A006),
    (0x5049_6D18, 0x504F_A008),
    (0x5049_6D1A, 0x504F_A00A),
    (0x5049_6D1C, 0x504F_A00C),
    (0x5049_6D1E, 0x504F_A00E),
    (0x5049_6F10, 0x504F_A200),
    (0x5049_6F18, 0x504F_A208),
    (0x5049_7010, 0x504F_A300),
    (0x5049_7018, 0x504F_A308),
    (0x5049_7510, 0x504F_A800),
    (0x5049_8910, 0x504F_C600),
    (0x5049_8912, 0x504F_C602),
    (0x5049_89D0, 0x504F_C6C0),
    (0x5049_89D2, 0x504F_C6C2),
    (0x5049_89E0, 0x504F_C6D0),
    (0x5049_8BF0, 0x504F_C8E0),
    (0x5049_8DF0, 0x504F_CAE0),
];

/// High-level (`0x001F3C..0x004A80`) and wrapper (`0x00A534`) immediate geometry patches.
const HI_BYTE_PATCHES: &[(usize, u8, u8)] = &[
    // Clear size 0x1CC4 -> 0x26C4
    (0x001F78, 0x1C, 0x26),
    (0x0043A9, 0x1C, 0x26),
    // Page map block count 0..9 -> 0..14
    (0x002027, 0x09, 0x0E),
    (0x002084, 0x09, 0x0E),
    (0x0021CC, 0x09, 0x0E),
    (0x002237, 0x09, 0x0E),
    (0x0025EC, 0x09, 0x0E),
    (0x00268B, 0x09, 0x0E),
    (0x003850, 0x09, 0x0E),
    (0x0049F9, 0x09, 0x0E),
    // Backup page map D_ID 0x0A11 -> 0x0F16 (low byte + high byte)
    (0x00208F, 0x11, 0x16),
    (0x002160, 0x11, 0x16),
    (0x002195, 0x11, 0x16),
    (0x002917, 0x11, 0x16),
    // Primary D_ID 0x0Axx -> 0x0Fxx
    (0x001FD6, 0x0A, 0x0F),
    (0x002033, 0x0A, 0x0F),
    (0x002090, 0x0A, 0x0F),
    (0x0020DA, 0x0A, 0x0F),
    (0x0020E8, 0x0A, 0x0F),
    (0x00214D, 0x0A, 0x0F),
    (0x002161, 0x0A, 0x0F),
    (0x002167, 0x0A, 0x0F),
    (0x002190, 0x0A, 0x0F),
    (0x002196, 0x0A, 0x0F),
    (0x002311, 0x0A, 0x0F),
    (0x0023BD, 0x0A, 0x0F),
    (0x002449, 0x0A, 0x0F),
    (0x002457, 0x0A, 0x0F),
    (0x002619, 0x0A, 0x0F),
    (0x00270D, 0x0A, 0x0F),
    (0x002793, 0x0A, 0x0F),
    (0x002883, 0x0A, 0x0F),
    (0x002918, 0x0A, 0x0F),
    (0x0029D4, 0x0A, 0x0F),
    (0x00387D, 0x0A, 0x0F),
    (0x0043C2, 0x0A, 0x0F),
    (0x004738, 0x0A, 0x0F),
    (0x00A534, 0x0A, 0x0F),
];

fn has_real_bgo_table_marker(buf: &[u8]) -> bool {
    buf.len() == PROG061_EXPECTED_SIZE
        && LittleEndian::read_u32(&buf[OFF_BGO_TABLE..OFF_BGO_TABLE + 4]) == 0x8000_0000
}

/// Detects which BGO geometry (`EH50_M6930_4MB` vs `EH55_M6930LB_8MB`) is present in `PROG061`.
pub fn inspect_prog061_bgo_geometry(data: &[u8]) -> Result<Prog061BgoTargetGeometry> {
    if data.len() != PROG061_EXPECTED_SIZE {
        bail!(
            "Expected {PROG061_EXPECTED_SIZE}-byte PROG061 binary, got {} bytes",
            data.len()
        );
    }
    if !has_real_bgo_table_marker(data) {
        return Ok(Prog061BgoTargetGeometry::Eh50M6930_4Mb);
    }
    let logical_sectors = LittleEndian::read_u32(&data[OFF_BGO_TABLE + 0x0C..OFF_BGO_TABLE + 0x10]);
    let flash_base = LittleEndian::read_u32(&data[OFF_BGO_TABLE + 0x30..OFF_BGO_TABLE + 0x34]);
    if logical_sectors == 0x28 && flash_base == 0x8413_0000 {
        Ok(Prog061BgoTargetGeometry::Eh55M6930Lb8Mb)
    } else {
        Ok(Prog061BgoTargetGeometry::Eh50M6930_4Mb)
    }
}

fn decompress_eh55_nfrom_drv() -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(EMBEDDED_EH55_NFROM_DRV_GZ);
    let mut out = Vec::with_capacity(EH55_NFROM_DRV_LEN);
    decoder
        .read_to_end(&mut out)
        .context("Decompressing embedded EH55 nfrom_drv payload")?;
    if out.len() != EH55_NFROM_DRV_LEN {
        bail!(
            "Corrupt embedded EH55 nfrom_drv payload length {} (expected {EH55_NFROM_DRV_LEN})",
            out.len()
        );
    }
    Ok(out)
}

fn relocate_hi_ram_range(buf: &mut [u8], start: usize, end: usize) {
    let limit = end.saturating_sub(3);
    for off in start..limit {
        let word = LittleEndian::read_u32(&buf[off..off + 4]);
        if let Some(&(_, mapped)) = HI_RAM_RELOCS.iter().find(|&&(orig, _)| orig == word) {
            LittleEndian::write_u32(&mut buf[off..off + 4], mapped);
        }
    }
}

/// Retargets a decoded `256 KiB` `PROG061` binary to [`Prog061BgoTargetGeometry`].
///
/// Preserves [`Prog061BgoTargetGeometry::Eh50M6930_4Mb`] untouched and translates a factory EH50
/// `PROG061` image to [`Prog061BgoTargetGeometry::Eh55M6930Lb8Mb`] idempotently, preserving any
/// active `bgo_sum` bypass state (`0xC8 0x0A` vs `0xCA 0x0A`).
pub fn retarget_prog061_bgo_geometry_in_place(
    raw_prog061: &mut [u8],
    target: Prog061BgoTargetGeometry,
) -> Result<()> {
    let current = inspect_prog061_bgo_geometry(raw_prog061)?;
    if target == Prog061BgoTargetGeometry::Eh50M6930_4Mb {
        if current == Prog061BgoTargetGeometry::Eh55M6930Lb8Mb {
            bail!(
                "Cannot downgrade already-retargeted EH55 M6930LB PROG061 back to EH50 4 MiB in-place; start from factory PROG061"
            );
        }
        return Ok(());
    }

    if !has_real_bgo_table_marker(raw_prog061)
        || current == Prog061BgoTargetGeometry::Eh55M6930Lb8Mb
    {
        return Ok(());
    }

    let bgo_sum_was_bypassed = raw_prog061[OFF_BGO_SUM_BEQ..OFF_BGO_SUM_BEQ + 2] == PATCH_SUM_BRA;

    // 1. Write 256-byte DMR-EH55 BGO geometry descriptor table (`0x036E2C..0x036F2C`).
    for (idx, &word) in EH55_BGO_TABLE_WORDS.iter().enumerate() {
        let off = OFF_BGO_TABLE + idx * 4;
        LittleEndian::write_u32(&mut raw_prog061[off..off + 4], word);
    }

    // 2. Transplant relocated `Loader ver0.61` `nfrom_drv.o` into `0x021E6A..0x029356`.
    let nfrom_drv = decompress_eh55_nfrom_drv()?;
    raw_prog061[OFF_NFROM_DRV_START..OFF_NFROM_DRV_START + nfrom_drv.len()]
        .copy_from_slice(&nfrom_drv);
    raw_prog061[OFF_NFROM_DRV_START + nfrom_drv.len()..OFF_NFROM_DRV_END].fill(MN103_NOP);
    if bgo_sum_was_bypassed {
        raw_prog061[OFF_BGO_SUM_BEQ_EH55..OFF_BGO_SUM_BEQ_EH55 + 2].copy_from_slice(&PATCH_SUM_BRA);
    } else if raw_prog061[OFF_BGO_SUM_BEQ_EH55..OFF_BGO_SUM_BEQ_EH55 + 2] != ORIG_SUM_BEQ {
        bail!(
            "Unexpected byte sequence at EH55 OFF_BGO_SUM_BEQ_EH55 (0x{OFF_BGO_SUM_BEQ_EH55:06X})"
        );
    }

    // 3. Relocate FUN_5040248e init_2 CALL32 displacement (`0x0024C6`).
    if LittleEndian::read_i32(&raw_prog061[OFF_INIT_2_CALL + 1..OFF_INIT_2_CALL + 5])
        == ORIG_INIT_2_REL32
    {
        LittleEndian::write_i32(
            &mut raw_prog061[OFF_INIT_2_CALL + 1..OFF_INIT_2_CALL + 5],
            EH55_INIT_2_REL32,
        );
    }

    // 4. Relocate high-level & wrapper BGO RAM pointers into unused SDRAM `0x504FA000..0x504FE000`.
    relocate_hi_ram_range(raw_prog061, 0x001F3C, 0x004A80);
    relocate_hi_ram_range(raw_prog061, 0x00A294, 0x00AF8C);

    // 5. Patch verified 0x1CC4->0x26C4, page-map loop bounds (9->14), and D_ID 0x0Axx->0x0Fxx.
    for &(off, expected, replacement) in HI_BYTE_PATCHES {
        if raw_prog061[off] != expected && raw_prog061[off] != replacement {
            bail!(
                "EH55 retarget mismatch at 0x{off:06X}: expected 0x{expected:02X}, found 0x{:02X}",
                raw_prog061[off]
            );
        }
        raw_prog061[off] = replacement;
    }

    // 6. Expand `_start` BSS clear end (`0x504F9178 -> 0x504FE000`) so relocated tables clear on boot.
    if LittleEndian::read_u32(&raw_prog061[OFF_BSS_ZERO_END..OFF_BSS_ZERO_END + 4])
        == ORIG_BSS_ZERO_END
    {
        LittleEndian::write_u32(
            &mut raw_prog061[OFF_BSS_ZERO_END..OFF_BSS_ZERO_END + 4],
            EH55_BSS_ZERO_END,
        );
    }

    Ok(())
}
