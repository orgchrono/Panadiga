#![deny(unsafe_code)]

//! Domain types for Panasonic `.FRM` container builders.

use serde::{Deserialize, Serialize};

/// Output container format for `.FRM` image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrmContainerMode {
    Dvd,
    Dvd64B,
    Sd,
    Sd48,
}

/// Input module descriptor for packing `PANA_DVD.FRM` or `PANA__SD.FRM`.
#[derive(Debug, Clone)]
pub struct FrmPackModule {
    pub name: String,
    pub payload: Vec<u8>,
    pub version_u16: u16,
    pub hardware_family: String,
    /// Optional alias families (e.g., `"M6930"` alongside primary `"M6930LB"`) emitted as shared-payload TOC slots.
    pub hardware_family_aliases: Vec<String>,
    pub model_prefix: u8,
    pub target_flag: u8,
    pub model_group: String,
    pub tail_meta: [u8; 6],
}

impl FrmPackModule {
    /// Creates a module specification populated with EH55 defaults (`M6930`, `M`, `NP-309`).
    pub fn new(name: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            payload,
            version_u16: 0x0001,
            hardware_family: "M6930".to_string(),
            hardware_family_aliases: Vec::new(),
            model_prefix: b'M',
            target_flag: 0,
            model_group: "NP-309".to_string(),
            tail_meta: [0u8; 6],
        }
    }
}

/// Summary of a module slot written into a packed `.FRM` container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackedFrmModuleSummary {
    pub index: usize,
    pub name: String,
    pub module_offset: u32,
    pub module_size: u32,
    pub fvu_checksum_u32: u32,
    pub list_sum_u16: u16,
}

/// Structured report emitted after packing a `PANA_DVD.FRM`, `PANA__SD.FRM`, or `DIGADIAG.FRM`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrmPackReport {
    pub mode: String,
    pub total_bytes: usize,
    pub toc_region_size: usize,
    pub modules: Vec<PackedFrmModuleSummary>,
}
