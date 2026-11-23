#![deny(unsafe_code)]

//! Data structures for parsed Legacy DVD/SD and Blu-ray `.FRM` firmware containers.

use serde::{Deserialize, Serialize};

use crate::aes128::PanaAesKeyPair;
use crate::pana_fw_go::AesHeader;

/// Discriminated Panasonic `.FRM` container format covering Legacy DVD/HDD, SD, and Blu-ray lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UniversalFrmFormat {
    LegacyDvd48B,
    LegacyDvd64B,
    SdPlain48B,
    SdPlain64B,
    BlurayFeistel,
    BlurayAes,
}

/// Outer/inner encryption strategy for a single firmware module payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleEncryptionKind {
    Plain,
    FullFeistel,
    BlurayDrvEdge,
    MainListPartial,
}

/// Decoded single module slot inside a `.FRM` container or Blu-ray sub-container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalFrmModule {
    pub index: usize,
    pub name: String,
    pub toc_tag_4b: String,
    pub version_ascii: String,
    pub version_u16: u16,
    pub module_offset: u32,
    pub module_size: u32,
    pub hardware_family: String,
    pub oem: u8,
    pub target_flag: u8,
    pub model_group: String,
    pub checksum_u32: u32,
    pub toc_compression_type: u8,
    pub toc_sum_valid: bool,
    pub data_sum_valid: bool,
    pub encoding: String,
    pub compression_algo: Option<String>,
    pub build_name: Option<String>,
    pub dst_addr_hex: Option<String>,
    pub partitions: Vec<String>,
    pub main_list_meta: Option<(u32, u32, bool)>,
    pub extrhead_meta: Option<(u16, String, u32, u32, u16)>,
    pub encryption_kind: ModuleEncryptionKind,
    #[serde(skip)]
    pub raw_toc_entry: Vec<u8>,
    #[serde(skip)]
    pub pre_decode_slot_bytes: Vec<u8>,
    #[serde(skip)]
    pub payload: Vec<u8>,
}

/// Decoded inner `.FRM` container (directly the file for Legacy/BD-Feistel, or `1..=4` children for `BlurayAes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalFrmSubContainer {
    pub sub_index: usize,
    pub file_offset: u32,
    pub format: UniversalFrmFormat,
    pub feistel_key_name: Option<String>,
    pub toc_encrypted: bool,
    pub has_panasonic_header: bool,
    pub toc_stride_bytes: usize,
    pub toc_region_size: usize,
    pub uses_adler32_sums: bool,
    pub modules: Vec<UniversalFrmModule>,
    #[serde(skip)]
    pub feistel_round_key: [u8; 32],
    #[serde(skip)]
    pub raw_outer_48b: Option<[u8; 48]>,
}

/// Complete decoded representation of any Panasonic `.FRM` firmware update file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalFrmContainer {
    pub format: UniversalFrmFormat,
    pub aes_key_name: Option<String>,
    pub aes_header: Option<AesHeader>,
    pub sub_containers: Vec<UniversalFrmSubContainer>,
    #[serde(skip)]
    pub aes_key_pair: Option<PanaAesKeyPair>,
}
