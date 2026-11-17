//! Payload extraction, NOR dumper stubs, and Matshita R6-MV4 optical emulator CLI args.

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Extracts and Feistel-decrypts DRV/PROG061/MAIN modules from PANA_DVD.FRM or PANA__SD.FRM.
#[derive(Args, Debug)]
pub struct ExtractFrmArgs {
    /// Source .FRM path (defaults to firmware_and_images/PANA_DVD.FRM)
    #[arg(
        long,
        default_value = "firmware_and_images/PANA_DVD.FRM",
        value_name = "FRM_FILE"
    )]
    pub frm: PathBuf,

    /// Destination directory for extracted module binaries
    #[arg(short, long, value_name = "DIR")]
    pub outdir: PathBuf,

    /// Also decompress EXTRHEAD/EXTRFOOT payloads (DRV/MAIN)
    #[arg(long)]
    pub unpack: bool,

    /// Exports structured JSON extraction report
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Assembles a read-only MN103S UART NOR flash dumper stub (optionally wrapped as 128 KiB DIGADIAG.FRM).
#[derive(Args, Debug)]
pub struct NorStubArgs {
    /// Physical/mapped NOR start VA (e.g. 0x84000000 or 0x80000000)
    #[arg(long, default_value = "0x84000000", value_name = "ADDR")]
    pub start: String,

    /// Total bytes to dump (default 8 MiB = 0x800000)
    #[arg(long, default_value = "0x800000", value_name = "LEN")]
    pub len: String,

    /// Block size in bytes (default 4096)
    #[arg(long, default_value_t = 4096)]
    pub block_size: u32,

    /// MN103S SDRAM load address (defaults to 0x50400000 with --wrap-digadiag, or 0x51800000 for raw RSP stub)
    #[arg(long, value_name = "ADDR")]
    pub load_addr: Option<String>,

    /// Optional output path (.bin or DIGADIAG.FRM)
    #[arg(short, long, value_name = "OUT_FILE")]
    pub out: Option<PathBuf>,

    /// Wrap assembled stub inside plaintext 128 KiB DIGADIAG.FRM
    #[arg(long)]
    pub wrap_digadiag: bool,

    /// Exports structured JSON stub metadata and hex payload (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Decodes and checksum-verifies a `PANADMP1..PANADONE` UART NOR flash capture stream.
#[derive(Args, Debug)]
pub struct DecodeNorDumpArgs {
    /// Captured UART dump stream path (or `-` for stdin bytes)
    #[arg(short, long, value_name = "STREAM_FILE")]
    pub input: PathBuf,

    /// Optional destination path to save verified raw binary image
    #[arg(short, long, value_name = "OUT_BIN")]
    pub out: Option<PathBuf>,

    /// Exports structured JSON dump verification report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
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
