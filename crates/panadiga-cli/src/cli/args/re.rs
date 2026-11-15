//! CLI argument definitions for Matsushita MN103S / AM34 static reverse engineering audits.

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct ReArgs {
    #[command(subcommand)]
    pub action: ReActions,
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
}
