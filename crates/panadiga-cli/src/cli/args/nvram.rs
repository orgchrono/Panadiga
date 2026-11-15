//! CLI argument definitions for NOR Flash and NVRAM management.

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct NvramArgs {
    #[command(subcommand)]
    pub action: NvramActions,
}

#[derive(Subcommand, Debug)]
pub enum NvramActions {
    /// Cleans corrupted NVRAM sectors (0x007F0000..0x007FFFFF), backs up corrupt state, and audits microsectors
    Clean {
        /// Path to raw 8 MB Spansion S29JL064H flash dump
        #[arg(value_name = "INPUT_DUMP")]
        input: PathBuf,

        /// Path for the sanitized clean firmware output (defaults to <INPUT>_CLEAN.bin)
        #[arg(value_name = "OUTPUT_CLEAN")]
        output: Option<PathBuf>,
    },

    /// Copy-on-write guarded byte patch strictly inside writable NVRAM microsectors (blocks IPL NOR)
    Patch {
        /// Path to raw 8 MB / 4 MB Spansion NOR flash dump
        #[arg(short, long, value_name = "INPUT_DUMP")]
        input: PathBuf,

        /// Destination copy-on-write path (cannot overwrite input; defaults to <INPUT>_PATCHED.<ext>)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,

        /// NVRAM-relative byte offset (hex/dec inside 0..65535)
        #[arg(long, value_name = "OFFSET")]
        offset: String,

        /// Hex bytes to write (e.g. "DEADBEEF" or "A5 FF")
        #[arg(long, value_name = "HEX")]
        hex: String,
    },
}
