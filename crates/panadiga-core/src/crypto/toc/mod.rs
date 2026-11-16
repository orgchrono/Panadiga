//! Pure deterministic parser, validator, and formatter for Panasonic `.FRM` TOC tables.
//!
//! Zero filesystem, mmap, or terminal I/O belongs in this module; file loading lives in `toc_io`.

mod layout;
mod parse;
mod report;

#[cfg(test)]
mod tests;

pub use layout::TocLayout;
pub use parse::{
    decrypt_and_parse_frm_toc, decrypt_frm_toc_with_detected_key, interpret_frm_target_flag,
    parse_sd_frm_toc, parse_sd_frm_toc_with_layout,
};
pub use report::{analyze_frm_toc_report, analyze_toc, format_toc_dump_lines};
