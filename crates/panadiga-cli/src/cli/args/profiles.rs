#![deny(unsafe_code)]

//! CLI arguments for v2 relational JSON hardware profile dictionaries (`panadiga profiles`).

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::nvram::LabOverrideCliArgs;

#[derive(Args, Debug)]
pub struct ProfilesArgs {
    #[command(subcommand)]
    pub action: ProfilesActions,
}

#[derive(Subcommand, Debug)]
pub enum ProfilesActions {
    /// Lists all resolved DIGA hardware profiles (model -> board/SoC + NOR flash + EEPROM + capabilities)
    List {
        /// Optional runtime v2 JSON overlay directory (`soc_cores/`, `socs/`, `flash_chips/`, `models/`, ...)
        #[arg(long, value_name = "DIR")]
        overlay_dir: Option<PathBuf>,

        /// Exports structured JSON list (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Resolves and displays full hex/binary address bindings & capabilities for a specific model ID or alias
    Show {
        /// DIGA model ID or alias (e.g. `DMR-EH55`, `DMR-ES35V`, `DMR-EZ485V`, `DMR-ES10`)
        #[arg(value_name = "MODEL")]
        model: String,

        /// Optional runtime v2 JSON overlay directory
        #[arg(long, value_name = "DIR")]
        overlay_dir: Option<PathBuf>,

        /// Exports structured JSON resolved profile (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Validates built-in and overlay relational JSON dictionaries against claim & silicon safety invariants
    Validate {
        /// Optional runtime v2 JSON overlay directory
        #[arg(long, value_name = "DIR")]
        overlay_dir: Option<PathBuf>,

        /// Exports structured JSON validation report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Evaluates CapabilityGate authorization for a model, runtime probes, and optional LabOverride
    Authorize {
        /// DIGA model ID or exact alias
        #[arg(value_name = "MODEL")]
        model: String,

        /// Capability (`inspect_read_only`, `nvram_sanitize_offline_cow`, `nvram_write_live`, `upper_nor_patch_offline_cow`, `upper_nor_write_live`, `official_frm_pass_through`, `custom_recovery_media`, `critical_ipl_or_lethal_mmio`)
        #[arg(long, value_name = "CAPABILITY")]
        capability: String,

        /// Optional runtime v2 JSON overlay directory
        #[arg(long, value_name = "DIR")]
        overlay_dir: Option<PathBuf>,

        /// Observed dump byte length
        #[arg(long, value_name = "BYTES")]
        dump_size: Option<u64>,

        /// Live JEDEC manufacturer ID (hex or dec, e.g. `0x01`)
        #[arg(long, value_name = "HEX")]
        jedec_mfg: Option<String>,

        /// Live JEDEC device ID cycle words comma-separated (e.g. `0x227E,0x220A,0x2201`)
        #[arg(long, value_name = "CSV")]
        jedec_dev: Option<String>,

        /// Verified WP#/ACC pin state (`true` or `false`)
        #[arg(long)]
        wp_deasserted: Option<bool>,

        /// Whether EEMUL 0xF1 active marker was probed inside top sectors
        #[arg(long)]
        has_eemul_f1: bool,

        /// Observed `.FRM` container SHA-256 hex digest
        #[arg(long, value_name = "SHA256")]
        frm_sha256: Option<String>,

        /// Observed loader banner string (e.g. `Loader ver0.61`)
        #[arg(long, value_name = "BANNER")]
        loader_banner: Option<String>,

        /// Requested SD loader profile key (e.g. `ver061`)
        #[arg(long, value_name = "KEY")]
        loader_profile: Option<String>,

        #[command(flatten)]
        lab_override: LabOverrideCliArgs,

        /// Exports authorization/denial JSON (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Exports normalized `ProfilesBundle` or resolved model array as JSON
    Export {
        /// Output format (`bundle` or `json`)
        #[arg(long, default_value = "bundle", value_name = "FORMAT")]
        format: String,

        /// Output JSON file (`-` emits stdout JSON)
        #[arg(long, value_name = "FILE")]
        out: PathBuf,

        /// Optional runtime v2 JSON overlay directory
        #[arg(long, value_name = "DIR")]
        overlay_dir: Option<PathBuf>,
    },

    /// Exports all 12 generated JSON Schema Draft 2020-12 files into `profiles/schema/`
    ExportSchema {
        /// Output schema directory
        #[arg(long, default_value = "profiles/schema", value_name = "DIR")]
        out_dir: PathBuf,
    },

    /// Evidence-gated promotion of a model claim axis tier inside `profiles/models/<model>.json`
    Promote {
        /// Target DIGA model ID (e.g. `DMR-EH56`)
        #[arg(long, value_name = "MODEL")]
        model: String,

        /// Claim axis (`board`, `flash_chip`, `nor_layout`, `train`, `eeprom`)
        #[arg(long, value_name = "AXIS")]
        axis: String,

        /// Target evidence tier (`T0`..`T4` or snake_case name)
        #[arg(long, value_name = "TIER")]
        tier: String,

        /// Primary evidence source kind (`probe_fixture`, `bench_log`, `physical_dump`, ...)
        #[arg(long, value_name = "KIND")]
        source_kind: Option<String>,

        /// Reference file path (fallback for `--fixture` or `--bench-log`)
        #[arg(long, value_name = "REF")]
        source_ref: Option<String>,

        /// Mandatory 64-char SHA-256 hex digest of verified artifact
        #[arg(long, value_name = "SHA256")]
        sha256: String,

        /// Optional probe fixture path relative to workspace root
        #[arg(long, value_name = "FIXTURE_PATH")]
        fixture: Option<String>,

        /// Bench log path under `docs/debug_logs/` (required for `bench_verified` / `T4`)
        #[arg(long, value_name = "LOG_PATH")]
        bench_log: Option<String>,

        /// Optional reviewer notes stored with the evidence record
        #[arg(long, value_name = "NOTES")]
        notes: Option<String>,

        /// Target `profiles/` root directory
        #[arg(long, default_value = "profiles", value_name = "DIR")]
        profiles_dir: PathBuf,

        /// Workspace root directory resolving fixture and bench log paths
        #[arg(long, default_value = ".", value_name = "DIR")]
        workspace_root: PathBuf,
    },
}
