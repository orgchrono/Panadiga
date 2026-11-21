#![deny(unsafe_code)]

//! CLI arguments for strict forensic vault sessions (`panadiga session`).

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub action: SessionActions,
}

#[derive(Subcommand, Debug)]
pub enum SessionActions {
    /// Initializes a segregated forensic session vault (`raw/`, `frm/`, `recompiled/`)
    Init {
        /// Panasonic DIGA target model ID (e.g. `DMR-EH55`)
        #[arg(long, default_value = "DMR-EH55")]
        model: String,

        /// Bootloader loader profile (`ver061` or `ver063`)
        #[arg(long, default_value = "ver061")]
        loader_profile: String,

        /// Optional deterministic session identifier (single clean path segment)
        #[arg(long)]
        session_id: Option<String>,

        /// Optional base vault root directory (defaults to `<writable_data_root>`)
        #[arg(long, value_name = "DIR")]
        vault_root: Option<PathBuf>,

        /// Exports structured JSON session manifest (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Auto-classifies and ingests a `.bin`/`.img` (`raw/`) or `.FRM` (`frm/`) into a session vault
    Ingest {
        /// Target session directory (`<base_root>/sessions/<session_id>`)
        #[arg(long, value_name = "SESSION_DIR")]
        session_dir: PathBuf,

        /// Source artifact path to copy immutably into the vault
        #[arg(long, value_name = "FILE")]
        file: PathBuf,

        /// Optional explicit bucket override (`raw`, `frm`, or `recompiled`)
        #[arg(long, value_name = "BUCKET")]
        bucket: Option<String>,

        /// Provenance label recorded in `session_manifest.json`
        #[arg(long, default_value = "cli_manual_ingest")]
        origin: String,

        /// Exports structured JSON artifact record (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Displays current workflow status of one session or lists all sessions under `--vault-root`
    Status {
        /// Specific session directory to inspect
        #[arg(long, value_name = "SESSION_DIR")]
        session_dir: Option<PathBuf>,

        /// Base vault root directory when listing all sessions
        #[arg(long, value_name = "DIR")]
        vault_root: Option<PathBuf>,

        /// Exports structured JSON manifest/list (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Extracts an embedded `EXTRHEAD DRV ` container from an 8 MiB NOR dump into `raw/drv_from_nor.bin`
    ExtractDrv {
        /// Target session directory (`<base_root>/sessions/<session_id>`)
        #[arg(long, value_name = "SESSION_DIR")]
        session_dir: PathBuf,

        /// Path to captured 8 MiB Spansion NOR binary dump
        #[arg(long, value_name = "NOR_BIN")]
        nor_dump: PathBuf,

        /// Exports structured JSON artifact record (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },
}
