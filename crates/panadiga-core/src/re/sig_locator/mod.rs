#![deny(unsafe_code)]

//! Autonomous MN103S function locator via string cross-references and prologue/CFG patterns.

pub mod pattern_cfg;
pub mod string_xref;

use serde::{Deserialize, Serialize};

pub use pattern_cfg::{detect_prologue_at, discover_cfg_functions, summarize_function_cfg};
pub use string_xref::locate_functions_by_string;

/// Cross-reference locating an embedded ASCII string inside an MN103S binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringXrefHit {
    pub needle: String,
    pub string_va: u32,
    pub xref_va: u32,
    pub xref_kind: String,
}

/// Discovered enclosing MN103S function referencing a target string needle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocatedFunctionMatch {
    pub func_va: u32,
    pub needle: String,
    pub string_va: u32,
    pub xref_va: u32,
    pub prologue_kind: String,
}

/// Control-flow and frame summary of a candidate MN103S function discovered by prologue/CFG scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfgFunctionCandidate {
    pub entry_va: u32,
    pub end_va: u32,
    pub prologue_kind: String,
    pub branch_count: usize,
    pub call_count: usize,
    pub hardware_loop_count: usize,
}
