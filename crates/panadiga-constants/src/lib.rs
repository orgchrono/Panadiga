//! Panadiga L0 Leaf Architectural Constants, IPC Event Registry, Telemetry Logger & Pure Forensic Helpers.
//!
//! Zero internal workspace dependencies (`panadiga-*`), forming the acyclic foundation for
//! `panadiga-bench`, `panadiga-vault`, `panadiga-crypto`, `panadiga-ir`, and `panadiga-core`.

#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]

#[macro_use]
pub mod logger;

pub mod constants;
pub mod events;
pub mod forensic;

pub use constants::*;
pub use logger::LogLevel;
