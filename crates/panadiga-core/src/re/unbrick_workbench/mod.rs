//! Pre-MAIN DIGA Unbrick Workbench: Workspace Discovery, FRM Extraction, & Hardware Facts.
//!
//! Splits pure data catalog/facts (`catalog`), pure container codecs (`codec`),
//! and thin workspace/extraction I/O (`io`).

pub mod catalog;
pub mod codec;
pub mod io;

pub(crate) use catalog::bgo_mode_label;
pub use catalog::{
    ExtractedFrmModuleReport, FrmModuleExtractionReport, UnbrickHardwareFact,
    UnbrickWorkspaceReport,
};
pub(crate) use codec::{load_main_binary_and_meta, load_prog061_from_bytes};
pub(crate) use io::discover_sources;
pub use io::{extract_modules_from_frm, inspect_unbrick_workspace};
