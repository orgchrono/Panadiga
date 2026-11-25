#![deny(unsafe_code)]

//! CLI argument definitions for live PANAMON1 Spansion NOR capture and DAVIS IDE diagnostics.

use clap::Args;
use std::path::PathBuf;

/// Captures Spansion `S29JL064H` (`IC5001`) NOR slices over `PANAMON1>` (`full|bgo|nvram|loader|custom`).
#[derive(Args, Debug)]
pub struct CaptureNorArgs {
    /// Pico serial port identifier (e.g. COM10 or /dev/ttyACM0)
    #[arg(short, long, value_name = "PORT")]
    pub port: String,

    /// NOR slice preset: full (8 MiB), bgo (SA26..=SA70), nvram (64 KiB), loader (256 KiB), or custom
    #[arg(long, default_value = "full", value_name = "SLICE")]
    pub slice: String,

    /// Custom start virtual address in hex (required when --slice custom; inside 0x84000000..0x84800000)
    #[arg(long, value_name = "HEX")]
    pub start: Option<String>,

    /// Custom length in hex bytes (required when --slice custom)
    #[arg(long, value_name = "HEX")]
    pub len: Option<String>,

    /// Output path for captured NOR binary slice
    #[arg(short, long, value_name = "BIN")]
    pub out: PathBuf,

    /// Optional JSON summary export (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Runs non-destructive AM34 `DAVIS CH0/CH1` IDE & `0xEC IDENTIFY` probe over `PANAMON1>` (blocks `0xC000_100C`).
#[derive(Args, Debug)]
pub struct DavisProbeArgs {
    /// Pico serial port identifier (e.g. COM10 or /dev/ttyACM0)
    #[arg(short, long, value_name = "PORT")]
    pub port: String,

    /// Target CH1 drive selector: master (0xA0), slave (0xB0), or both
    #[arg(long, default_value = "master", value_name = "DRIVE")]
    pub drive: String,

    /// Skip post-probe ATA `0xEC` IDENTIFY DEVICE payload decoding/export
    #[arg(long)]
    pub no_identify: bool,

    /// Optional output path for extracted 512-byte `IDENTIFY DEVICE` sector
    #[arg(long, value_name = "BIN")]
    pub out_ec: Option<PathBuf>,

    /// Optional JSON probe telemetry export (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Deterministically replays a raw COM10 / `ndb>` evidence log (LE words, diffs, BCU fault & STANDBY gate).
#[derive(Args, Debug)]
pub struct SiliconReplayArgs {
    /// Path to raw COM10 / `ndb>` transcript log
    #[arg(short, long, value_name = "LOG_FILE")]
    pub log: PathBuf,

    /// Optional JSON summary export (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Captures a safe read-only `ndb> dd` hot bare-metal word snapshot via PicoFW `BRIDGE` under `STANDBY_GATE_STOP`.
#[derive(Args, Debug)]
pub struct SiliconSnapshotArgs {
    /// Pico serial port identifier (e.g. COM10 or /dev/ttyACM0)
    #[arg(short, long, value_name = "PORT")]
    pub port: String,

    /// Target AM34 virtual address in hex (blocks lethal MMIO and BCU fault aperture)
    #[arg(short, long, value_name = "HEX")]
    pub addr: String,

    /// Byte count to read (rounded up to 4-byte words; default 16)
    #[arg(short, long, default_value_t = 16)]
    pub len: usize,

    /// Optional JSON summary export (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}
