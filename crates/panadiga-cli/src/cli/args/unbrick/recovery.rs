//! BGO, AV TBC passthrough, SD recovery kit builder, and Region/Model compatibility CLI args.

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Inspects or patches PROG061 BGO (U99) recovery sites or emits 128 KiB DIGADIAG.FRM.
#[derive(Args, Debug)]
pub struct BgoArgs {
    /// Input PROG061 binary or DIGADIAG.FRM container
    #[arg(short, long, value_name = "FILE")]
    pub input: PathBuf,

    /// Output path; when omitted runs non-destructive inspection only
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// BGO patch mode: bypass (default, safe), uart_monitor, factory, or auto_format (blocked without override)
    #[arg(short, long, default_value = "bypass", value_name = "MODE")]
    pub mode: String,

    /// Write raw 256 KiB PROG061 binary instead of 128 KiB plaintext DIGADIAG.FRM
    #[arg(long)]
    pub raw_prog061: bool,

    /// Explicit unsafe override required for auto_format (D4/D5 CS1 IC5001 alias risk)
    #[arg(long)]
    pub allow_unsafe_bgo_format: bool,

    /// Exports structured JSON report
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Inspects/patches MAIN (3410PQ) AV TBC (E-E) passthrough sites or emits _dev_SERIAL RSP frames.
#[derive(Args, Debug)]
pub struct PassthroughArgs {
    /// Unpacked EH55_MAIN_3410PQ.bin path (required unless --rsp-plan is passed)
    #[arg(short, long, value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Optional non-destructive patched output file path
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Patch mode: av_passthrough (default) or factory_original
    #[arg(short, long, default_value = "av_passthrough", value_name = "MODE")]
    pub mode: String,

    /// Generates live _dev_SERIAL GDB RSP $M sequences without touching disk binaries
    #[arg(long)]
    pub rsp_plan: bool,

    /// Optional callback target VA (hex/dec) for _dev_SERIAL slot 0x515B2E40
    #[arg(long, value_name = "ADDR")]
    pub callback_va: Option<String>,

    /// Exports structured JSON report
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Builds a complete post-verified SD recovery kit (DIGADIAG.FRM + PANA__SD.FRM + SHA256SUMS.txt).
#[derive(Args, Debug)]
pub struct PrepareSdArgs {
    /// Output directory for SD root recovery artifacts
    #[arg(short, long, value_name = "DIR")]
    pub outdir: PathBuf,

    /// Optional PANA_DVD.FRM override
    #[arg(long, value_name = "FRM_FILE")]
    pub frm: Option<PathBuf>,

    /// Optional raw/EXTRHEAD PROG061 override
    #[arg(long, value_name = "FILE")]
    pub prog061: Option<PathBuf>,

    /// Optional MAIN container/binary override (e.g. Partition 1 MAIN 3410PQ)
    #[arg(long, value_name = "FILE")]
    pub main: Option<PathBuf>,

    /// Optional DRV_S211 container override
    #[arg(long, value_name = "FILE")]
    pub drv: Option<PathBuf>,

    /// PROG061 BGO mode: bypass (default, safe), uart_monitor, factory, or auto_format (blocked without override)
    #[arg(long, default_value = "bypass", value_name = "MODE")]
    pub bgo_mode: String,

    /// Explicit unsafe override required when --bgo-mode auto_format is passed
    #[arg(long)]
    pub allow_unsafe_bgo_format: bool,

    /// Optional expected MAIN build target (e.g. 3410PQ or 3060NP); rejects mismatches unless overridden
    #[arg(long, value_name = "BUILD")]
    pub expected_main_build: Option<String>,

    /// Allow MAIN build/model mismatch when composing PANA__SD.FRM
    #[arg(long)]
    pub allow_target_mismatch: bool,

    /// MAIN passthrough mode: av_passthrough (default) or factory_original
    #[arg(long, default_value = "av_passthrough", value_name = "MODE")]
    pub passthrough_mode: String,

    /// Generate DIGADIAG.FRM only without PANA__SD.FRM
    #[arg(long)]
    pub diag_only: bool,

    /// Disable dynamic LBA48 HDD capacity unlock in repacked MAIN
    #[arg(long)]
    pub no_hdd_dynamic: bool,

    /// Optional fixed target HDD capacity in GB (overrides dynamic)
    #[arg(long, value_name = "GB")]
    pub target_gb: Option<u32>,

    /// Keep extracted/patched source binaries alongside DIGADIAG.FRM and PANA__SD.FRM
    #[arg(long)]
    pub save_sources: bool,

    /// Exports structured JSON build & verification report
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum RegionCompatAction {
    /// Inspects MAIN (3410PQ / 3060NP) src/fmupre/pre_update_firm.c model & OEM comparison guard
    InspectMain {
        /// Unpacked MAIN binary path
        #[arg(short, long, value_name = "MAIN_BIN")]
        input: PathBuf,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Patches or restores MAIN <deviceinfo> model & OEM guard (bypass|factory_strict)
    PatchMain {
        /// Unpacked MAIN input binary path
        #[arg(short, long, value_name = "MAIN_BIN")]
        input: PathBuf,

        /// Patched MAIN output path (Copy-on-Write)
        #[arg(short, long, value_name = "OUT_BIN")]
        output: PathBuf,

        /// Mode: bypass (allow cross-model/OEM) or factory_strict
        #[arg(long, default_value = "bypass", value_name = "MODE")]
        mode: String,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Inspects a 512-byte DEV.BIN service container device descriptor
    InspectDev {
        /// Path to 512-byte DEV.BIN
        #[arg(short, long, value_name = "DEV_BIN")]
        input: PathBuf,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Edits model (<=8 ASCII) and optional OEM character in a 512-byte DEV.BIN
    PatchDev {
        /// Source 512-byte DEV.BIN path
        #[arg(short, long, value_name = "DEV_BIN")]
        input: PathBuf,

        /// Destination 512-byte DEV.BIN path (Copy-on-Write)
        #[arg(short, long, value_name = "OUT_DEV")]
        output: PathBuf,

        /// Target model string (1..=8 ASCII chars, e.g. EH55EG)
        #[arg(long, value_name = "MODEL")]
        model: String,

        /// Optional single ASCII OEM character (e.g. M or E)
        #[arg(long, value_name = "CHAR")]
        oem: Option<char>,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Retargets PANA_DVD.FRM / PANA__SD.FRM TOC hardware_family, model_group, oem, and target_flag
    RetargetFrm {
        /// Source .FRM container path
        #[arg(short, long, value_name = "FRM_FILE")]
        input: PathBuf,

        /// Output .FRM container path (Copy-on-Write)
        #[arg(short, long, value_name = "OUT_FRM")]
        output: PathBuf,

        /// Optional hardware family string (<=8 ASCII chars, e.g. M6930)
        #[arg(long, value_name = "HW")]
        hw: Option<String>,

        /// Optional model group string (<=6 ASCII for DVD / <=12 for SD, e.g. NP-309)
        #[arg(long, value_name = "GROUP")]
        model_group: Option<String>,

        /// Optional OEM prefix byte (1 ASCII char, 48B TOC)
        #[arg(long, value_name = "CHAR")]
        oem: Option<char>,

        /// Optional TOC FormatFlag / target_flag byte
        #[arg(long, value_name = "FLAG")]
        target_flag: Option<u8>,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },
}
