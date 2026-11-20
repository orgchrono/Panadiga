//! Discriminated `.FRM` TOC layouts (`Dvd48B`, `Dvd64B`, `Sd48B`, `Sd64B`) and field offsets.

use serde::{Deserialize, Serialize};

use crate::constants::{PANA_FRM_TOC_ENTRY_SIZE, PANA_SD_TOC_ENTRY_SIZE};

/// Discriminated TOC layout supported by Panasonic DIGA `.FRM` containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TocLayout {
    /// Optical update `PANA_DVD.FRM`: Feistel-encrypted 48-byte (`0x30`) TOC.
    Dvd48B,
    /// Optical update `PANA_DVD.FRM`: Feistel-encrypted 64-byte (`0x40`) TOC (`EH49`, `EH59`, `EH69`).
    Dvd64B,
    /// DMR-EH55 `Loader ver0.61` (`PROG061`): plaintext 48-byte (`0x30`) SD TOC (`0x1800` header).
    Sd48B,
    /// Later `Loader ver0.63` (`BOOT.BIN`): plaintext 64-byte (`0x40`) SD TOC (`0x2000` header).
    Sd64B,
}

impl TocLayout {
    pub const fn stride_bytes(self) -> usize {
        match self {
            Self::Dvd48B | Self::Sd48B => PANA_FRM_TOC_ENTRY_SIZE,
            Self::Dvd64B | Self::Sd64B => PANA_SD_TOC_ENTRY_SIZE,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Dvd48B => "dvd-feistel",
            Self::Dvd64B => "dvd-64b-feistel",
            Self::Sd48B => "sd-48b-plaintext",
            Self::Sd64B => "sd-plaintext",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Dvd48B => "PANA_DVD.FRM (48B Feistel TOC)",
            Self::Dvd64B => "PANA_DVD.FRM (64B Feistel TOC)",
            Self::Sd48B => "PANA__SD.FRM (48B Plaintext SD TOC — Loader ver0.61)",
            Self::Sd64B => "PANA__SD.FRM (64B Plaintext SD TOC — Loader ver0.63)",
        }
    }
}

pub(super) struct TocLayoutOffsets {
    pub stride: usize,
    pub name_end: usize,
    pub ver_off: usize,
    pub off_pos: usize,
    pub hw_end: usize,
    pub flag_pos: usize,
    pub grp_start: usize,
    pub grp_end: usize,
    pub sum_off: usize,
    pub skip_empty: bool,
}

pub(super) const DVD_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    stride: PANA_FRM_TOC_ENTRY_SIZE,
    name_end: 0x0A,
    ver_off: 0x0A,
    off_pos: 0x0C,
    hw_end: 0x18,
    flag_pos: 0x19,
    grp_start: 0x1A,
    grp_end: 0x20,
    sum_off: 0x2E,
    skip_empty: false,
};

pub(super) const SD48_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    skip_empty: true,
    ..DVD_TOC_LAYOUT
};

pub(super) const SD_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    stride: PANA_SD_TOC_ENTRY_SIZE,
    name_end: 0x0A,
    ver_off: 0x0A,
    off_pos: 0x0C,
    hw_end: 0x18,
    flag_pos: 0x18,
    grp_start: 0x30,
    grp_end: 0x3C,
    sum_off: 0x3E,
    skip_empty: true,
};

pub(super) const DVD64_TOC_LAYOUT: TocLayoutOffsets = TocLayoutOffsets {
    stride: PANA_SD_TOC_ENTRY_SIZE,
    name_end: 0x0A,
    ver_off: 0x0A,
    off_pos: 0x0C,
    hw_end: 0x18,
    flag_pos: 0x19,
    grp_start: 0x1A,
    grp_end: 0x20,
    sum_off: 0x3E,
    skip_empty: true,
};
