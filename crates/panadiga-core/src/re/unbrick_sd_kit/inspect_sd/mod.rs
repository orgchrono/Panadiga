//! Raw SD Card / Image Forensic Auditor (MBR / GPT / Superfloppy / FAT16 BPB / 8.3 SFN).
//!
//! Decomposed cleanly into domain types (`types`), pure MBR/VBR/BPB parser (`mbr_bpb`),
//! FAT16 root & cluster-chain inspector (`fat16`), and sector-aligned I/O (`io`).

pub mod fat16;
pub mod io;
pub mod mbr_bpb;
pub mod types;

#[cfg(test)]
mod tests;

pub use io::{inspect_sd_disk_bytes, inspect_sd_reader, inspect_sd_target};
pub use types::{FatBpbReport, MbrPartitionRecord, RootDirFileReport, SdDiskInspectionReport};
