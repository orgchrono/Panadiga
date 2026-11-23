//! Panasonic DIGA Core Library (`panadiga-core`)
//!
//! Pure cross-platform domain models, hardware SSOT, Matsushita AM34/MN103S constants,
//! Spansion NOR Flash / NVRAM surgery, MEIHFS HDD forensics, PANA_DVD.FRM cryptanalysis,
//! PDF manuals indexing, Ghidra Jython code generation, and Panadiga P7501 Async UART Pico host protocol.

#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]

#[macro_use]
pub extern crate panadiga_constants;

pub use panadiga_constants as constants;
pub use panadiga_constants::logger;
pub use panadiga_constants::{
    __panadiga_log_dispatch, log_debug, log_error, log_info, log_println, log_raw, log_trace,
    log_warn,
};

pub mod bench;
pub use panadiga_crypto as crypto;
pub mod ghidra;
pub mod hdd;
pub mod headers;
pub mod ingest;
#[cfg(any(target_os = "android", test, feature = "android-ir"))]
pub use panadiga_ir as ir;
pub mod launcher;
pub mod manuals;
pub mod nvram;
pub mod pico;
pub mod profile;
pub mod re;
pub mod scan;
pub mod settings;
pub mod ssot;
pub mod vault;

pub use logger::LogLevel;
