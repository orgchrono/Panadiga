//! BGO, AV TBC passthrough, SD recovery kit builder, and Region/Model compatibility CLI args.

use clap::Args;
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
    /// Output directory for SD root recovery artifacts (defaults to firmware_and_images/sd_unbrick_kit)
    #[arg(
        short,
        long,
        default_value = panadiga_core::constants::DEFAULT_SD_UNBRICK_KIT_DIR,
        value_name = "DIR"
    )]
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

    /// Target bootloader profile: ver061 (DMR-EH55 raw 256 KiB + sd48, default) or ver063 (128 KiB EXTRHEAD + sd64)
    #[arg(long, default_value = "ver061", value_name = "PROFILE")]
    pub loader_profile: String,

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

    /// Optional analog TV standard overlay metadata (ntsc, pal-m, pal, or secam)
    #[arg(long, value_name = "STANDARD")]
    pub tv_standard: Option<String>,

    /// Optional regional chassis preset (br-pal-m, us-ntsc, eu-pal, or custom)
    #[arg(long, value_name = "PRESET")]
    pub lbs_preset: Option<String>,

    /// Apply hardened PANAMON1 ver0.61 preset (`uart_monitor` + `ver061` + dual-family parity check)
    #[arg(long)]
    pub hardened_sd: bool,

    /// Allow building an SD kit from an experimental/unverified (< T3) firmware_train profile (e.g. ver063 T2)
    #[arg(long)]
    pub allow_unverified_profile: bool,

    /// Patch PANA__SD.FRM (PROG061 + SSOT MAIN builds (3060NP/3410PQ)) 2 ms -> 1000 ms for SATA bridges (IDE2SAT2)
    #[arg(long)]
    pub sata_bridge_reset_fix: bool,

    /// Embed pre-FVU SATA HDD autoboot hook in PANA__SD.FRM PROG061 (requires --sata-bridge-reset-fix & ver061)
    #[arg(long, alias = "sata-bridge-stage2-boot")]
    pub sata_hdd_autoboot: bool,

    /// Retarget 4 MiB MAIN builds (3060NP/3410PQ) to 8 MiB EH55 NOR (0x84130000) via SSOT bgo_retarget_anchors
    #[arg(long)]
    pub main_bgo_retarget: bool,

    /// Wrap PANA__SD.FRM PROG061 in universal NOR autoboot SDRAM self-unpacker (256 B + zlib, zero NOR writes SD dry-run)
    #[arg(long, alias = "portao4-self-unpack")]
    pub nor_autoboot_capsule: bool,

    /// Exports structured JSON build & verification report
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Rebuilds a verified SD unbrick kit (`DIGADIAG.FRM` + `PANA__SD.FRM`) from an immutable NOR dump and optional HDD `MAIN` partition.
#[derive(Args, Debug)]
pub struct RebuildFromDumpArgs {
    /// Target DIGA model ID or alias resolved via `panadiga-profiles` (e.g. DMR-EH55)
    #[arg(long, default_value = "DMR-EH55", value_name = "MODEL")]
    pub model: String,

    /// Immutable physical NOR flash dump binary (`*.bin`) used to locate embedded EXTRHEAD/DRV modules
    #[arg(long, value_name = "NOR_BIN")]
    pub nor_dump: PathBuf,

    /// Optional decrypted/EXTRHEAD HDD Partition 1 (`MAIN`) binary override
    #[arg(long, value_name = "MAIN_BIN")]
    pub hdd_main: Option<PathBuf>,

    /// Optional factory `PANA_DVD.FRM` fallback container
    #[arg(long, value_name = "FRM_FILE")]
    pub frm: Option<PathBuf>,

    /// Optional `PROG061` binary override
    #[arg(long, value_name = "PROG_BIN")]
    pub prog061: Option<PathBuf>,

    /// Optional fixed target HDD capacity in GB (e.g. 250; defaults to dynamic LBA48 unlock)
    #[arg(long, value_name = "GB")]
    pub target_gb: Option<u32>,

    /// Destination directory for rebuilt SD unbrick kit
    #[arg(short, long, value_name = "DIR")]
    pub out_dir: PathBuf,

    /// Allow mixing NOR/DRV/MAIN builds whose loader version or family checksums cannot be strictly matched
    #[arg(long)]
    pub allow_unverified_mix: bool,

    /// Allow rebuilding using an experimental/unverified (< T3) firmware_train profile
    #[arg(long)]
    pub allow_unverified_profile: bool,

    /// Exports structured JSON rebuild & verification report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}
