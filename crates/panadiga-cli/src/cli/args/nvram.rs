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

    /// Copy-on-write patch in Upper NOR App / BGO (0x040000..nvram_start-1); blocks SA0..SA3 unless --allow-critical-ipl
    #[command(name = "patch-upper-nor")]
    PatchUpperNor {
        /// Path to raw Spansion NOR flash dump (2/4/8/16 MiB)
        #[arg(short, long, value_name = "INPUT_DUMP")]
        input: PathBuf,

        /// Destination copy-on-write output path (cannot overwrite input)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: PathBuf,

        /// Flash-relative byte offset (hex/dec inside 0x040000..nvram_start-1)
        #[arg(long, value_name = "OFFSET")]
        offset: String,

        /// Hex bytes to write (e.g. "DEADBEEF" or "A5 FF")
        #[arg(long, value_name = "HEX")]
        hex: String,

        /// Optional explicit binary backup path for affected 64 KiB sector(s)
        #[arg(long, value_name = "BACKUP_FILE")]
        backup: Option<PathBuf>,

        /// Explicit offline CoW override permitting writes inside Critical Boot IPL (SA0..SA3)
        #[arg(long)]
        allow_critical_ipl: bool,
    },
}
