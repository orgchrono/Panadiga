#![deny(unsafe_code)]

//! CLI arguments for deterministic FAT16B (`0x06`) SD recovery card creation (`panadiga sd`).

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct SdArgs {
    #[command(subcommand)]
    pub action: SdActions,
}

#[derive(Subcommand, Debug)]
pub enum SdActions {
    /// Inspect host drives for safe SD card recovery targets and SDXC >= 64 GiB advisories
    #[command(name = "targets")]
    Targets {
        /// Exports structured JSON target drive list (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Create a deterministic FAT16B (0x06, 32 MiB default) SD recovery image or flash an SD card
    #[command(name = "create")]
    Create(Box<SdCreateArgs>),
}

#[derive(Args, Debug)]
pub struct SdCreateArgs {
    /// Output standalone raw FAT16B `.img` file path (safe offline mode)
    #[arg(long, value_name = "IMG_FILE")]
    pub out: Option<PathBuf>,

    /// Target physical SD card device (`StorageMediaRole::SdCard` enforced; e.g. `\\.\PhysicalDrive2` or `/dev/sdb`)
    #[arg(long, value_name = "DEVICE")]
    pub device: Option<String>,

    /// Safety confirmation token required for physical SD flashing (`CONFIRM-<device>`)
    #[arg(long, value_name = "TOKEN")]
    pub confirm_token: Option<String>,

    /// Target Panasonic DIGA recorder model string recorded in recovery guidance
    #[arg(long, default_value = "DMR-EH55")]
    pub model: String,

    /// FAT16B partition volume size in MiB (`32..=256`, default `32` MiB = 65536 sectors)
    #[arg(long, default_value_t = 32)]
    pub volume_mb: u32,

    /// Bootloader profile when auto-composing from `.FRM`/sources (`ver061` raw+sd48 or `ver063` extrhead+sd64)
    #[arg(long, default_value = "ver061")]
    pub loader_profile: String,

    /// Prebuilt `DIGADIAG.FRM` file placed verbatim as pure 8.3 root entry
    #[arg(long, value_name = "DIGADIAG_FRM")]
    pub digadiag: Option<PathBuf>,

    /// Prebuilt optional `PANA__SD.FRM` file placed verbatim alongside `DIGADIAG.FRM`
    #[arg(long, value_name = "PANA_SD_FRM")]
    pub pana_sd: Option<PathBuf>,

    /// Source `PANA_DVD.FRM` container when auto-composing the SD unbrick kit
    #[arg(long, value_name = "PANA_DVD_FRM")]
    pub pana_dvd: Option<PathBuf>,

    /// Include `PANA__SD.FRM` when auto-composing via `prepare_sd_unbrick_kit`
    #[arg(long)]
    pub include_pana_sd: bool,

    /// Synthesize 100% clean-room bare-metal PANAMON1 DIGADIAG.FRM without proprietary Panasonic binaries
    #[arg(long)]
    pub cleanroom: bool,

    /// Exports structured JSON creator report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}
