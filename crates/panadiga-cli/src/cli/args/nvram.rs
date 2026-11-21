//! CLI argument definitions for NOR Flash and NVRAM management.

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Reusable audited LabOverride flags unlocking offline Copy-on-Write experimentation.
#[derive(Args, Debug, Clone, Default)]
pub struct LabOverrideCliArgs {
    /// Unlocks unverified offline Copy-on-Write / R&D writes (requires disclaimer ack and reason; live hardware stays locked)
    #[arg(long)]
    pub lab_override_unverified: bool,

    /// Exact risk acknowledgment literal required alongside `--lab-override-unverified` (`I_ACCEPT_UNVERIFIED_HARDWARE_RISK`)
    #[arg(long, value_name = "ACK")]
    pub confirm_disclaimer: Option<String>,

    /// Operator justification (>= 5 chars) recorded alongside `--lab-override-unverified`
    #[arg(long, value_name = "REASON")]
    pub override_reason: Option<String>,
}

/// Policy and model-pinning flags shared across offline NVRAM / Upper NOR Copy-on-Write commands.
#[derive(Args, Debug, Clone, Default)]
pub struct NvramPolicyCliArgs {
    /// Pin explicit DIGA model ID instead of relying on structural fingerprint probes (e.g. `DMR-EH55`)
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    #[command(flatten)]
    pub lab_override: LabOverrideCliArgs,
}

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

        #[command(flatten)]
        policy: NvramPolicyCliArgs,
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

        #[command(flatten)]
        policy: NvramPolicyCliArgs,
    },

    /// Copy-on-write patch in Upper NOR App / BGO (0x040000..nvram_start-1); blocks SA0..SA10 unless --allow-critical-ipl
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

        /// Optional explicit binary backup path for affected sector(s)
        #[arg(long, value_name = "BACKUP_FILE")]
        backup: Option<PathBuf>,

        /// Explicit offline CoW override permitting writes inside Critical Boot IPL (SA0..SA10)
        #[arg(long)]
        allow_critical_ipl: bool,

        #[command(flatten)]
        policy: NvramPolicyCliArgs,
    },

    /// Audits all sectors of a Spansion NOR binary dump
    AuditNorSectors {
        /// Path to NOR binary dump
        #[arg(long, value_name = "BIN")]
        image: PathBuf,

        /// Exports structured JSON sector audit report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },
}
