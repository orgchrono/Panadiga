#![deny(unsafe_code)]

//! Deterministic FAT16B (`0x06`) SD recovery card image synthesizer matching AM34 IPL `SDDIAG`.

pub mod layout;
pub mod synth;

use serde::{Deserialize, Serialize};

pub use layout::{
    FAT16_DEFAULT_VOLUME_MB, FAT16_MAX_VOLUME_MB, FAT16_MIN_VOLUME_MB, FAT16_PARTITION_TYPE_LBA,
    FAT16_VBR_LBA_START, Fat16SdGeometry, compute_fat16_sd_geometry, encode_dos_8_3_name,
};
pub use synth::{Fat16SdFileInput, SynthesizedSdImage, synthesize_fat16_sd_image};

/// Summary metadata for a deterministic FAT16B recovery card file entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fat16PlacedFileReport {
    pub name_8_3: String,
    pub size_bytes: usize,
    pub first_cluster: u16,
    pub cluster_count: usize,
    pub sha256: String,
}
