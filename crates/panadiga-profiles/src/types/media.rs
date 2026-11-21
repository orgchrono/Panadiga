//! Authoritative physical media filesystem/container contracts (SD FAT16B 0x06, DVD ISO 9660 1x, MEIHFS).

use super::evidence::ClaimEvidence;
use crate::hex_serde::hex_u32;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MediaFormatProfile {
    pub id: String,
    pub media_kind: String,
    pub filesystem: String,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub partition_type: u32,
    pub sector_size: u32,
    pub start_lba: u32,
    pub default_volume_mb: u32,
    pub root_filenames: Vec<String>,
    pub magic_hex: String,
    pub write_speed_policy: String,
    pub evidence: ClaimEvidence,
}
