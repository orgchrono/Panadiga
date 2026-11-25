//! Panasonic DMR-EH55 DIGA Suite - Tauri IPC Commands Subsystem
//!
//! Modularized by Single Responsibility Principle (SRP) and Separation of Concerns (SoC):
//! - `dto`: Data Transfer Objects matching React/TypeScript frontend models and path helpers
//! - `storage`: Spansion S29JL064H NVRAM and HDD/MEIHDFS inspection, decoding, and extraction commands
//! - `crypto_manuals`: PANA_DVD.FRM 64-bit ECB cryptanalysis, TOC, codebook, and service manuals commands
//! - `workspace`: System overview (`get_system_overview`) and binary hex dump (`read_hex_dump`) commands
//! - `hardware`: Pico USB CDC, Cold Break-In, In-Circuit Erase, GDB RSP Bridge, External Tools, and RAG commands

pub mod constants;
pub mod crypto_manuals;
pub mod dto;
pub mod engineering_docs;
pub mod error;
pub mod external_tools;
pub mod frm_pack;
pub mod hardware;
pub mod hardware_bridge;
pub mod hardware_diag;
#[cfg(any(target_os = "android", test))]
pub mod ir;
pub mod settings;
pub mod silicon_explorer;
pub mod ssot_studio;
pub mod storage;
pub mod unbrick;
pub mod unbrick_dto;
pub mod workspace;

pub use crypto_manuals::*;
pub use dto::*;
pub use engineering_docs::*;
pub use error::*;
pub use external_tools::*;
pub use frm_pack::*;
pub use hardware::*;
pub use hardware_bridge::*;
pub use hardware_diag::*;
#[cfg(any(target_os = "android", test))]
pub use ir::*;
pub use settings::*;
pub use silicon_explorer::*;
pub use ssot_studio::*;
pub use storage::*;
pub use unbrick::*;
pub use unbrick_dto::*;
pub use workspace::*;
