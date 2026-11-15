//! Panasonic DMR-EH55 Ghidra Integration Generator.
//!
//! Splits pure immutable hardware specs (`specs`), deterministic Python/Jython
//! renderers (`render`), and thin filesystem emission (`io`).

pub mod io;
pub mod render;
pub mod specs;

pub use io::export_all_ghidra_scripts;
pub use render::{
    generate_ghidra_auto_unbrick_audit, generate_ghidra_memory_map,
    generate_ghidra_mn103_fix_decompilation, generate_ghidra_symbols,
};
