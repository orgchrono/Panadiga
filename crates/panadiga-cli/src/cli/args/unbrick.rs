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

        /// BGO patch mode: bypass (default, safe), uart_monitor, factory, or auto_format (blocked without override)
        #[arg(short, long, default_value = "bypass", value_name = "MODE")]
        mode: String,

        /// Write raw 256 KiB PROG061 binary instead of 128 KiB plaintext DIGADIAG.FRM
        #[arg(long)]
        raw_prog061: bool,

        /// Explicit unsafe override required for auto_format (D4/D5 CS1 IC5001 alias risk)
        #[arg(long)]
        allow_unsafe_bgo_format: bool,

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

        /// PROG061 BGO mode: bypass (default, safe), uart_monitor, factory, or auto_format (blocked without override)
        #[arg(long, default_value = "bypass", value_name = "MODE")]
        bgo_mode: String,

        /// Explicit unsafe override required when --bgo-mode auto_format is passed
        #[arg(long)]
        allow_unsafe_bgo_format: bool,

        /// Optional expected MAIN build target (e.g. 3410PQ or 3060NP); rejects mismatches unless overridden
        #[arg(long, value_name = "BUILD")]
        expected_main_build: Option<String>,

        /// Allow MAIN build/model mismatch when composing PANA__SD.FRM
        #[arg(long)]
        allow_target_mismatch: bool,

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

    /// Audits raw SD card MBR/GPT/Superfloppy, FAT16 BPB, 8.3 root entries, and cluster chain
    InspectSd {
        /// Drive letter (`E:`), Windows volume (`\\.\E:`), `\\.\PhysicalDriveN`, or raw `.img` file
        #[arg(value_name = "TARGET")]
        target: String,

        /// Exports structured JSON disk audit report
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

        /// MN103S SDRAM load address (defaults to 0x50400000 with --wrap-digadiag, or 0x51800000 for raw RSP stub)
        #[arg(long, value_name = "ADDR")]
        load_addr: Option<String>,

        /// Optional output path (.bin or DIGADIAG.FRM)
        #[arg(short, long, value_name = "OUT_FILE")]
        out: Option<PathBuf>,

        /// Wrap assembled stub inside plaintext 128 KiB DIGADIAG.FRM
        #[arg(long)]
        wrap_digadiag: bool,

        /// Exports structured JSON stub metadata and hex payload (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Decodes and checksum-verifies a `PANADMP1..PANADONE` UART NOR flash capture stream
    DecodeNorDump {
        /// Captured UART dump stream path (or `-` for stdin bytes)
        #[arg(short, long, value_name = "STREAM_FILE")]
        input: PathBuf,

        /// Optional destination path to save verified raw binary image
        #[arg(short, long, value_name = "OUT_BIN")]
        out: Option<PathBuf>,

        /// Exports structured JSON dump verification report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Matshita R6-MV4 optical drive emulator, boot verifier, ATAPI write injector, and SD/CD .FRM unlock tool
    MatshitaEmu {
        #[command(subcommand)]
        action: MatshitaEmuAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum MatshitaEmuAction {
    /// Runs the /BOOT.BIN optical boot sequence verifier for a bench scenario
    Boot {
        /// Scenario: healthy_with_disc, unlock_disc_frm, tray_cannot_close_80045300, laser_apc_failure_80040904
        #[arg(long, default_value = "healthy_with_disc", value_name = "SCENARIO")]
        scenario: String,

        /// Optional PANA_DVD.FRM path to mount at virtual LBA 16 before booting
        #[arg(long, value_name = "FRM_FILE")]
        frm: Option<PathBuf>,

        /// Exports structured JSON boot sequence report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Injects payload via ATAPI 0x3B WRITE BUFFER or vendor 0xED 0x91/0x93 and verifies 0x3C readback
    InjectWrite {
        /// Buffer offset in bytes
        #[arg(long, default_value_t = 0)]
        offset: usize,

        /// ATAPI WRITE BUFFER mode (e.g. 2 = data, 5 = download microcode)
        #[arg(long, default_value_t = 2)]
        mode: u8,

        /// Use Matshita vendor CDB 0xED (0x91 stage VA -> 0x93 write) instead of 0x3B
        #[arg(long)]
        vendor_ed: bool,

        /// Optional MN103S target VA staged via 0xED 0x91 (IPL NOR 0x8000..0x807E / 0x8400..0x847E blocked E05)
        #[arg(long, value_name = "ADDR")]
        va: Option<String>,

        /// Hex payload bytes to inject (e.g. "DEADBEEF" or "4D 41 54 53")
        #[arg(long, value_name = "HEX")]
        payload_hex: Option<String>,

        /// Binary file whose bytes to inject (mutually exclusive with --payload-hex)
        #[arg(long, value_name = "BIN_FILE")]
        payload_file: Option<PathBuf>,

        /// Exports structured JSON injection & snapshot report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Executes a DRV serial console command (PASS 0x4d415453 0x48495441, MMODE, SLOGON, WB, WW, RB, RW)
    SerialCmd {
        /// Pre-authenticate session with PASS 0x4d415453 0x48495441 before running command
        #[arg(long)]
        unlock_pass: bool,

        /// Serial CLI command string
        #[arg(value_name = "COMMAND")]
        cmd: String,

        /// Exports structured JSON response
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Decodes and verifies a CD/DVD (PANA_DVD.FRM) or SD (PANA__SD.FRM / DIGADIAG.FRM) unlock container
    VerifyFrm {
        /// Container kind: dvd, sd, sd48, or digadiag
        #[arg(long, default_value = "digadiag", value_name = "KIND")]
        kind: String,

        /// Path to .FRM container
        #[arg(value_name = "FRM_FILE")]
        input: PathBuf,

        /// Exports structured JSON verification report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Generates and cryptographically verifies an unlock CD/DVD (PANA_DVD.FRM) or SD (PANA__SD.FRM) container
    PackUnlockFrm {
        /// Container kind: dvd (or cd), sd, sd48
        #[arg(long, default_value = "sd", value_name = "KIND")]
        kind: String,

        /// Optional custom PROG061 binary payload path
        #[arg(long, value_name = "BIN_FILE")]
        payload_file: Option<PathBuf>,

        /// Destination .FRM path
        #[arg(short, long, value_name = "OUT_FRM")]
        out: PathBuf,

        /// Exports structured JSON verification report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },
}
