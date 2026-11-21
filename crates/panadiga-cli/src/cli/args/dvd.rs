#![deny(unsafe_code)]

//! CLI arguments for ISO 9660 Level 1 (`1x` / minimum speed) DVD recovery creator (`panadiga dvd`).

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct DvdArgs {
    #[command(subcommand)]
    pub action: DvdActions,
}

#[derive(Subcommand, Debug)]
pub enum DvdActions {
    /// Lists host optical disc burners and their minimum supported DVD write speeds
    Targets {
        /// Exports structured JSON burner list (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Creates a Mn103Vm-gated ISO 9660 Level 1 `.iso` or burns a physical DVD-R at 1x/min speed
    Create(Box<DvdCreateArgs>),
}

#[derive(Args, Debug)]
pub struct DvdCreateArgs {
    /// Output standalone verified ISO 9660 `.iso` file path
    #[arg(long, value_name = "ISO_FILE")]
    pub out: Option<PathBuf>,

    /// Target physical optical burner ID (`/dev/sr0`, IMAPI2 ID, etc.)
    #[arg(long, value_name = "BURNER_ID")]
    pub burner: Option<String>,

    /// Safety confirmation token required for optical burning (`CONFIRM-<burner_id>`)
    #[arg(long, value_name = "TOKEN")]
    pub confirm_token: Option<String>,

    /// Target Panasonic DIGA recorder model string
    #[arg(long, default_value = "DMR-EH55")]
    pub model: String,

    /// ISO 9660 Volume Identifier (pure ASCII uppercase)
    #[arg(long, default_value = "PANADIGA_REC")]
    pub volume_label: String,

    /// Bootloader profile (`ver061` or `ver063`)
    #[arg(long, default_value = "ver061")]
    pub loader_profile: String,

    /// Optional explicit `DIGADIAG.FRM` file
    #[arg(long, value_name = "DIGADIAG_FRM")]
    pub digadiag: Option<PathBuf>,

    /// Optional `PANA_DVD.FRM` update container to include/unpack
    #[arg(long, value_name = "PANA_DVD_FRM")]
    pub pana_dvd: Option<PathBuf>,

    /// Force 100% clean-room bare-metal PANAMON1 payload
    #[arg(long)]
    pub cleanroom: bool,

    /// Validate ISO 9660 + lockouts + preview command without invoking optical burn
    #[arg(long)]
    pub dry_run: bool,

    /// Exports structured JSON report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}
