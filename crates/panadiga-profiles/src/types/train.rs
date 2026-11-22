//! Compiled firmware train contracts (bootloaders, `.FRM` TOC geometry, Feistel ciphers, SDRAM load VAs, and binary symbols).

use super::evidence::ClaimEvidence;
use crate::hex_serde::{hex_i32, hex_u8, hex_u32, hex_u32_map, hex_u32_vec, opt_hex_u32};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Expected bootloader banner fingerprint inside `0x000000..0x03FFFF`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoaderFingerprint {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub offset_start: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub offset_end: u32,
    pub ascii_marker: String,
}

/// Container and symmetric cipher parameters for `.FRM` / SD / CD recovery images.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrmContainerSpec {
    #[serde(default)]
    pub toc_entry_bytes: Option<u32>,
    #[serde(default, with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub toc_size: Option<u32>,
    #[serde(default)]
    pub cipher: Option<String>,
    #[serde(default)]
    pub key_id: Option<String>,
    #[serde(default)]
    pub official_frm_sha256: Option<String>,
}

/// Cryptographic / integrity trust model verified for stage-1.5 `DIGADIAG.FRM` boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DigadiagTrustModel {
    Checksum32Only,
    EncryptedToc,
    RsaSigned,
    Unknown,
}

/// SD stage-1.5 `DIGADIAG.FRM` specification declared per firmware train.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DigadiagSpec {
    pub format: String,
    pub filename: String,
    pub exact_bytes: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub load_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub entry_va: u32,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub panamon1_entry_va: Option<u32>,
    pub bgo_geometry_target: String,
    pub fvu_decoupled_mode: String,
    pub sd_media_requirement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_model: Option<DigadiagTrustModel>,
}

/// Container rebuild recipe defining `PANA__SD.FRM` / `PANA_DVD.FRM` assembly from live dumps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanaRebuildRecipe {
    pub output_container: String,
    pub toc_format: String,
    #[serde(default)]
    pub family_aliases: Vec<String>,
    #[serde(with = "hex_u8")]
    #[schemars(with = "String")]
    pub model_prefix_byte: u8,
    #[serde(default)]
    pub sources_from_panamon_dump: Vec<String>,
}

/// Half-open byte offset range `[start, end)` inside an unpacked `PROG` binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OffsetRange {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub start: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub end: u32,
}

/// 32-bit SDRAM pointer relocation mapping (`from -> to`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddressReloc {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub from: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub to: u32,
}

/// Verified single-byte immediate patch (`offset: expect -> replace`) inside a retargeted `PROG`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BytePatchEntry {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub offset: u32,
    #[serde(with = "hex_u8")]
    #[schemars(with = "String")]
    pub expect: u8,
    #[serde(with = "hex_u8")]
    #[schemars(with = "String")]
    pub replace: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Pure SSOT recipe to retarget a donor `PROG` binary from a factory baseline BGO geometry to a target flash geometry (e.g. `BGO_45SEC_8MB_0X84130000`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgBgoRetargetSpec {
    pub target_geometry_id: String,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub expected_prog_size: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub bgo_table_offset: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub bgo_table_marker: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub detect_logical_sectors: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub detect_flash_base: u32,
    #[serde(with = "hex_u32_vec")]
    #[schemars(with = "Vec<String>")]
    pub bgo_table_words: Vec<u32>,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub bgo_sum_beq_offset: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub nfrom_drv_start: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub nfrom_drv_end: u32,
    pub nfrom_drv_uncompressed_bytes: u32,
    pub nfrom_drv_blob_relpath: String,
    pub nfrom_drv_uncompressed_sha256: String,
    #[serde(with = "hex_u8")]
    #[schemars(with = "String")]
    pub nop_fill_byte: u8,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub bss_zero_end_offset: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub orig_bss_zero_end: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub target_bss_zero_end: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub init2_call_offset: u32,
    #[serde(with = "hex_i32")]
    #[schemars(with = "String")]
    pub orig_init2_rel32: i32,
    #[serde(with = "hex_i32")]
    #[schemars(with = "String")]
    pub target_init2_rel32: i32,
    pub hi_ram_ranges: Vec<OffsetRange>,
    pub hi_ram_relocs: Vec<AddressReloc>,
    pub byte_patches: Vec<BytePatchEntry>,
}

/// Firmware build/train profile independent of silicon ISA (`loader_ver061`, `loader_ver018`, `prog071`, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FirmwareTrainProfile {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soc_core_ref: Option<String>,
    pub loader_version: String,
    #[serde(default)]
    pub loader_date: Option<String>,
    pub loader_profile_key: String,
    #[serde(default)]
    pub loader_fingerprint: Option<LoaderFingerprint>,
    #[serde(default)]
    pub frm: Option<FrmContainerSpec>,
    #[serde(default, with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub prog_load_va: Option<u32>,
    #[serde(default, with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub main_load_va: Option<u32>,
    #[serde(default, with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub drv_load_va: Option<u32>,
    #[serde(default, with = "hex_u32_map")]
    #[schemars(with = "BTreeMap<String, String>")]
    pub symbols: BTreeMap<String, u32>,
    #[serde(default)]
    pub sd_boot_filenames: Vec<String>,
    #[serde(default)]
    pub digadiag_raw_bytes: Option<u32>,
    #[serde(default)]
    pub media_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digadiag_spec: Option<DigadiagSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pana_rebuild_recipe: Option<PanaRebuildRecipe>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgo_retarget_spec: Option<ProgBgoRetargetSpec>,
    pub evidence: ClaimEvidence,
}
