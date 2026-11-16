//! CLI argument definitions for Matsushita MN103S / AM34 static reverse engineering audits.

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct ReArgs {
    #[command(subcommand)]
    pub action: ReActions,
}

#[derive(Args, Debug)]
pub struct ReCommonInspectArgs {
    /// Sub-inspection kind selector
    #[arg(long, value_name = "KIND")]
    pub kind: String,

    /// Input binary path (or `-` for stdin bytes)
    #[arg(short, long, value_name = "BINARY")]
    pub input: Option<PathBuf>,

    /// Image base virtual address in hex (default 0x44000000)
    #[arg(long, default_value = "0x44000000", value_name = "HEX_VA")]
    pub base: String,

    /// JSON output path (`-` emits compact JSON to stdout)
    #[arg(long, value_name = "JSON_OUT")]
    pub json: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum ReActions {
    /// Audit an MN103S binary for Seven Traps (orphan tables, CFI literals, mid-insn overlaps, SETLB, AM33 Sleigh gaps)
    Audit {
        /// Path to unpacked MN103S binary (BOOT.BIN, PROG061, DRV_decomp, EH55_MAIN_3410PQ.bin)
        #[arg(short, long, value_name = "BINARY")]
        input: PathBuf,

        /// Image base virtual address in hex (auto-inferred from firmware signature when omitted)
        #[arg(long, value_name = "HEX_VA")]
        base: Option<String>,

        /// Start of executable code window in hex (defaults to base)
        #[arg(long, value_name = "HEX_VA")]
        code_lo: Option<String>,

        /// End of executable code window in hex (defaults to base + size)
        #[arg(long, value_name = "HEX_VA")]
        code_hi: Option<String>,

        /// Optional JSON output path
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Resolve MN103S branch/call callers and LE32 pointer cross-references to a target virtual address
    Xrefs {
        /// Path to unpacked MN103S binary
        #[arg(short, long, value_name = "BINARY")]
        input: PathBuf,

        /// Target virtual address in hex (e.g. 0x44007ba5)
        #[arg(short, long, value_name = "HEX_VA")]
        target: String,

        /// Image base virtual address in hex (auto-inferred when omitted)
        #[arg(long, value_name = "HEX_VA")]
        base: Option<String>,

        /// Optional JSON output path
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Parse Matshita R6-MV4 `<8sIII>` serial factory CLI table and classify PASS guard status
    #[command(name = "drv-cli", alias = "drv_cli")]
    DrvCli {
        /// Path to decompressed DRV binary (e.g. DRV_decomp.bin)
        #[arg(short, long, value_name = "DRV_BIN")]
        input: PathBuf,

        /// Byte offset of table within DRV image (default 0xDF5C0 for DRV_decomp)
        #[arg(long, default_value = "0xDF5C0", value_name = "HEX_OFF")]
        offset: String,

        /// DRV virtual load base (default 0x40000000)
        #[arg(long, default_value = "0x40000000", value_name = "HEX_VA")]
        base: String,

        /// Maximum table rows to scan
        #[arg(long, default_value_t = 128)]
        max_entries: usize,

        /// Evaluate write commands as unlocked (`PASS 0x4d415453 0x48495441`)
        #[arg(long, default_value_t = false)]
        pass_unlocked: bool,

        /// Optional JSON output path
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Parse 52-byte (`0x34`) DRV mecha error dictionary slice (`0x83A7`, etc.)
    #[command(name = "drv-errors", alias = "drv_errors")]
    DrvErrors {
        /// Path to DRV binary
        #[arg(short, long, value_name = "DRV_BIN")]
        input: PathBuf,

        /// Start byte offset (default 0xE779A)
        #[arg(long, default_value = "0xE779A", value_name = "HEX_OFF")]
        start: String,

        /// End byte offset (default 0xECE00)
        #[arg(long, default_value = "0xECE00", value_name = "HEX_OFF")]
        end: String,

        /// Optional JSON output path
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Inspect MN103S branches, Bcc, CALL frames, SETLB loops, 0xFC immediate loads, or regmasks
    Mn103 {
        #[command(flatten)]
        common: ReCommonInspectArgs,

        /// Optional window start VA in hex
        #[arg(long, value_name = "HEX_VA")]
        va_start: Option<String>,

        /// Optional window end VA in hex
        #[arg(long, value_name = "HEX_VA")]
        va_end: Option<String>,

        /// Single target VA in hex (for pointer-xrefs / callers)
        #[arg(long, value_name = "HEX_VA")]
        target: Option<String>,

        /// Comma-separated target VAs in hex (for callers)
        #[arg(long, value_name = "HEX_LIST")]
        targets: Option<String>,

        /// Max relative branch/call displacement in bytes (hex or dec)
        #[arg(long, default_value = "0x200000", value_name = "MAX_DISP")]
        max_abs_disp: String,

        /// Max SETLB loop body scan window in bytes
        #[arg(long, default_value_t = 64)]
        max_body_bytes: usize,

        /// 8-bit MN103S CALL/RET register mask (for `--kind regmask`)
        #[arg(long, default_value_t = 0)]
        regmask: u8,
    },

    /// Run Trap 1..7 static MN103S safety & revision detectors
    Traps {
        #[command(flatten)]
        common: ReCommonInspectArgs,

        /// Executable window lower VA
        #[arg(long, value_name = "HEX_VA")]
        code_lo: Option<String>,

        /// Executable window upper VA
        #[arg(long, value_name = "HEX_VA")]
        code_hi: Option<String>,

        /// Minimum run of consecutive pointer slots
        #[arg(long, default_value_t = 4)]
        min_run: usize,

        /// Comma-separated stub VAs excluded from pointer tables
        #[arg(long, value_name = "HEX_LIST")]
        stub_vas: Option<String>,

        /// Target VA in hex (for `--kind cfi`)
        #[arg(long, value_name = "HEX_VA")]
        va: Option<String>,

        /// JSON array of extra InstructionSpan records
        #[arg(long, value_name = "JSON_STR")]
        extra_spans: Option<String>,

        /// Comma-separated hit VAs in hex (for `--kind filter-hits`)
        #[arg(long, value_name = "HEX_LIST")]
        hits: Option<String>,

        /// Leading opcode byte (for `--kind insn-len`)
        #[arg(long, default_value_t = 0)]
        opcode: u8,
    },

    /// Parse Panasonic BOOT.BIN / DRV_decomp EXTRHEAD, ATAPI dispatch, Sense LUT, and CLI policy
    #[command(name = "pana-frm", alias = "pana_frm")]
    PanaFrm {
        /// Inspection kind: decompress-drv|extrhead|atapi-dispatch|sense-lut|classify-cli
        #[arg(long, value_name = "KIND")]
        kind: String,

        /// Input binary path (or `-` for stdin bytes)
        #[arg(short, long, value_name = "BINARY")]
        input: Option<PathBuf>,

        /// Image base virtual address in hex
        #[arg(long, default_value = "0x40000000", value_name = "HEX_VA")]
        base: String,

        /// Table byte offset within binary
        #[arg(long, default_value = "0x0", value_name = "HEX_OFF")]
        offset: String,

        /// Entry count for ATAPI/Sense LUT scans
        #[arg(long, default_value_t = 256)]
        count: usize,

        /// Leading header size skipped during `--kind decompress-drv`
        #[arg(long, default_value_t = 0x40)]
        header_size: usize,

        /// Default stub handler VA for ATAPI dispatch classification
        #[arg(long, default_value = "0x40014559", value_name = "HEX_VA")]
        default_handler: String,

        /// CLI command name for `--kind classify-cli`
        #[arg(long, value_name = "CMD")]
        cmd_name: Option<String>,

        /// Command flags dword for `--kind classify-cli`
        #[arg(long, default_value_t = 0)]
        flags: u32,

        /// Evaluate command classification as unlocked (`PASS 0x4d415453 0x48495441`)
        #[arg(long, default_value_t = false)]
        pass_unlocked: bool,

        /// JSON output path (`-` emits compact JSON to stdout)
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },
}
