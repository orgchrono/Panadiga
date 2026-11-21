//! Compiled firmware train contracts (bootloaders, `.FRM` TOC geometry, Feistel ciphers, SDRAM load VAs, and binary symbols).

use super::evidence::ClaimEvidence;
use crate::hex_serde::{hex_u32, hex_u32_map, opt_hex_u32};
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

/// Firmware build/train profile independent of silicon ISA (`loader_ver061`, `loader_ver018`, `prog071`, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FirmwareTrainProfile {
    pub id: String,
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
    pub evidence: ClaimEvidence,
}
