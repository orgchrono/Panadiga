//! CLI argument definitions for the Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Workbench.

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct UnbrickArgs {
    #[command(subcommand)]
    pub action: UnbrickActions,
}

#[derive(Subcommand, Debug)]
pub enum UnbrickActions {
    /// Inspects workspace sources, PROG061 BGO sites, MAIN TBC passthrough, _dev_SERIAL RSP plan, and SD state
    Status {
        /// Optional workspace directory override
        #[arg(long, value_name = "DIR")]
        workspace: Option<PathBuf>,

        /// Exports structured JSON report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Extracts and Feistel-decrypts DRV/PROG061/MAIN modules from PANA_DVD.FRM or PANA__SD.FRM
    ExtractFrm {
        /// Source .FRM path (defaults to firmware_and_images/PANA_DVD.FRM)
        #[arg(
            long,
            default_value = "firmware_and_images/PANA_DVD.FRM",
            value_name = "FRM_FILE"
        )]
        frm: PathBuf,

        /// Destination directory for extracted module binaries
        #[arg(short, long, value_name = "DIR")]
        outdir: PathBuf,

        /// Also decompress EXTRHEAD/EXTRFOOT payloads (DRV/MAIN)
        #[arg(long)]
        unpack: bool,

        /// Exports structured JSON extraction report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Inspects or patches PROG061 BGO (U99) recovery sites or emits 128 KiB DIGADIAG.FRM
    Bgo {
        /// Input PROG061 binary or DIGADIAG.FRM container
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output path; when omitted runs non-destructive inspection only
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// BGO patch mode: auto_format (default), bypass, uart_monitor, or factory
        #[arg(short, long, default_value = "auto_format", value_name = "MODE")]
        mode: String,

        /// Write raw 256 KiB PROG061 binary instead of 128 KiB plaintext DIGADIAG.FRM
        #[arg(long)]
        raw_prog061: bool,

        /// Exports structured JSON report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Inspects/patches MAIN (3410PQ) AV TBC (E-E) passthrough sites or emits _dev_SERIAL RSP frames
    Passthrough {
        /// Unpacked EH55_MAIN_3410PQ.bin path (required unless --rsp-plan is passed)
        #[arg(short, long, value_name = "FILE")]
        input: Option<PathBuf>,

        /// Optional non-destructive patched output file path
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Patch mode: av_passthrough (default) or factory_original
        #[arg(short, long, default_value = "av_passthrough", value_name = "MODE")]
        mode: String,

        /// Generates live _dev_SERIAL GDB RSP $M sequences without touching disk binaries
        #[arg(long)]
        rsp_plan: bool,

        /// Optional callback target VA (hex/dec) for _dev_SERIAL slot 0x515B2E40
        #[arg(long, value_name = "ADDR")]
        callback_va: Option<String>,

        /// Exports structured JSON report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Builds a complete post-verified SD recovery kit (DIGADIAG.FRM + PANA__SD.FRM + SHA256SUMS.txt)
    PrepareSd {
        /// Output directory for SD root recovery artifacts
        #[arg(short, long, value_name = "DIR")]
        outdir: PathBuf,

        /// Optional PANA_DVD.FRM override
        #[arg(long, value_name = "FRM_FILE")]
        frm: Option<PathBuf>,

        /// Optional raw/EXTRHEAD PROG061 override
        #[arg(long, value_name = "FILE")]
        prog061: Option<PathBuf>,

        /// Optional MAIN container/binary override (e.g. Partition 1 MAIN 3410PQ)
        #[arg(long, value_name = "FILE")]
        main: Option<PathBuf>,

        /// Optional DRV_S211 container override
        #[arg(long, value_name = "FILE")]
        drv: Option<PathBuf>,

        /// PROG061 BGO mode: auto_format (default), bypass, uart_monitor, or factory
        #[arg(long, default_value = "auto_format", value_name = "MODE")]
        bgo_mode: String,

        /// MAIN passthrough mode: av_passthrough (default) or factory_original
        #[arg(long, default_value = "av_passthrough", value_name = "MODE")]
        passthrough_mode: String,

        /// Generate DIGADIAG.FRM only without PANA__SD.FRM
        #[arg(long)]
        diag_only: bool,

        /// Disable dynamic LBA48 HDD capacity unlock in repacked MAIN
        #[arg(long)]
        no_hdd_dynamic: bool,

        /// Optional fixed target HDD capacity in GB (overrides dynamic)
        #[arg(long, value_name = "GB")]
        target_gb: Option<u32>,

        /// Keep extracted/patched source binaries alongside DIGADIAG.FRM and PANA__SD.FRM
        #[arg(long)]
        save_sources: bool,

        /// Exports structured JSON build & verification report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Verifies an SD card root directory (DIGADIAG.FRM, PANA__SD.FRM, and -91006 hardware checklist)
    VerifySd {
        /// SD card root or staging directory to verify
        #[arg(value_name = "SD_DIR")]
        dir: PathBuf,

        /// Exports structured JSON verification report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Assembles a read-only MN103S UART NOR flash dumper stub (optionally wrapped as 128 KiB DIGADIAG.FRM)
    NorStub {
        /// Physical/mapped NOR start VA (e.g. 0x84000000 or 0x80000000)
        #[arg(long, default_value = "0x84000000", value_name = "ADDR")]
        start: String,

        /// Total bytes to dump (default 8 MiB = 0x800000)
        #[arg(long, default_value = "0x800000", value_name = "LEN")]
        len: String,

        /// Block size in bytes (default 4096)
        #[arg(long, default_value_t = 4096)]
        block_size: u32,

        /// MN103S SDRAM load address (default 0x51800000)
        #[arg(long, default_value = "0x51800000", value_name = "ADDR")]
        load_addr: String,

        /// Optional output path (.bin or DIGADIAG.FRM)
        #[arg(short, long, value_name = "OUT_FILE")]
        out: Option<PathBuf>,

        /// Wrap assembled stub inside plaintext 128 KiB DIGADIAG.FRM
        #[arg(long)]
        wrap_digadiag: bool,
    },
}
