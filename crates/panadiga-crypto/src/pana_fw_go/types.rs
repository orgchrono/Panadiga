//! Domain DTOs and layout constants for `pana-fw` (`MainList`, `DRV`, `RomFS`/`CramFS`, IR).

use serde::{Deserialize, Serialize};

use super::super::extrhead::UnpackedExtrhead;
use crate::constants::{
    PANA_DRV_HEADER_SIZE, PANA_DRV_TAIL_CLEAR_BYTES, PANA_EXTRHEAD_DRV_CRYPT_SPAN,
    PANA_MAIN_LIST_LEGACY_SEG_TARGET, PANA_MAIN_LIST_MODERN_CRYPT_SPAN,
    PANA_MAIN_LIST_MODERN_SEG_TARGET,
};

pub const MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE: u32 = PANA_MAIN_LIST_LEGACY_SEG_TARGET;
pub const MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE: u32 = PANA_MAIN_LIST_MODERN_SEG_TARGET;
pub const MAIN_LIST_LEGACY_CRYPT_SPAN: usize = PANA_EXTRHEAD_DRV_CRYPT_SPAN; // 0x2800 (10 KiB)
pub const MAIN_LIST_MODERN_CRYPT_SPAN: usize = PANA_MAIN_LIST_MODERN_CRYPT_SPAN; // 0x1400 (5 KiB)
pub const DRV_HEADER_SIZE: usize = PANA_DRV_HEADER_SIZE;
pub const DRV_TAIL_CLEAR_BYTES: usize = PANA_DRV_TAIL_CLEAR_BYTES;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainListHeader {
    pub checksum: u32,
    pub unknown1: u32,
    pub list_size: u32,
    pub decompressed_segment_size: u32,
    pub compression_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainListEntry {
    pub size: u32,
    pub checksum: u32,
    pub checksum_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedMainList {
    pub header: MainListHeader,
    pub header_checksum_valid: bool,
    pub entries: Vec<MainListEntry>,
    pub segments: Vec<UnpackedExtrhead>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriveHeaderInfo {
    pub manufacturer: String,
    pub model: String,
    pub version: String,
    pub unknown: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedDrvModule {
    pub header: Option<DriveHeaderInfo>,
    pub extrhead: Option<UnpackedExtrhead>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RomFsHeaderInfo {
    pub size: u32,
    pub checksum_be: u32,
    pub computed_checksum_be: u32,
    pub sum_valid: bool,
    pub volume_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CramFsHeaderInfo {
    pub size: u32,
    pub flags: u32,
    pub fsid_crc: u32,
    pub computed_crc: Option<u32>,
    pub crc_valid: bool,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PartitionFsKind {
    RomFs(RomFsHeaderInfo),
    CramFs(CramFsHeaderInfo),
    SwupAddon,
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainPartitionSlice {
    pub name: String,
    pub offset: usize,
    pub fs_kind: PartitionFsKind,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct KaseikyoServiceCommand {
    pub name: &'static str,
    pub fpc_bytes: [u8; 4],
    pub device: u8,
    pub subdevice: u8,
    pub command: u8,
    pub flipper_hint: &'static str,
    pub notes: &'static str,
}
