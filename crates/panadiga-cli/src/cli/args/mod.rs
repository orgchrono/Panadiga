//! CLI Command and Argument Definitions
//!
//! Strongly typed Clap derivations for all Panadiga CLI commands,
//! modularized by functional domain with unified public re-exports.

pub mod crypto;
pub mod dvd;
pub mod ghidra;
pub mod hdd;
pub mod nvram;
pub mod pico;
pub mod profiles;
pub mod re;
pub mod sd;
pub mod session;
pub mod system;
pub mod unbrick;

pub use crypto::{FrmActions, FrmArgs, HeadersActions, HeadersArgs, ScanArgs};
pub use dvd::{DvdActions, DvdArgs, DvdCreateArgs};
pub use ghidra::{GhidraActions, GhidraArgs};
pub use hdd::{HddActions, HddArgs};
pub use nvram::{LabOverrideCliArgs, NvramActions, NvramArgs};
pub use pico::{GdbConfigArgs, PicoActions, PicoArgs};
pub use profiles::{ProfilesActions, ProfilesArgs};
pub use re::{ReActions, ReArgs};
pub use sd::{SdActions, SdArgs, SdCreateArgs};
pub use session::{SessionActions, SessionArgs};
pub use system::{
    ManualsArgs, McpArgs, MemoryMapArgs, RagActions, RagArgs, SsotActions, SsotArgs,
    VerifyArtifactArgs,
};
pub use unbrick::{UnbrickActions, UnbrickArgs};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "panadiga",
    author = "Antigravity Systems Engineering Team",
    version,
    about = "Panasonic DMR-EH55 & DIGA Engineering & Forensic Suite",
    long_about = "High-performance, strongly typed systems engineering suite for Panasonic DIGA recorders.\nReplaces legacy Python scripts with zero-copy memory-mapped operations, rigorous\nboundary verification, cryptographic forensics, and technical documentation lookup."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Launch interactive terminal user interface (TUI)
    #[arg(short, long)]
    pub interactive: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch native desktop GUI (WebView2 / WebKitGTK / WKWebView)
    #[command(name = "desktop")]
    Desktop,

    /// Launch native desktop GUI (alias for desktop)
    #[command(name = "gui")]
    Gui,

    /// Interactive guided terminal pipeline (TUI)
    #[command(name = "interactive", alias = "tui")]
    Interactive,

    /// NVRAM and Spansion S29JL064H NOR Flash management
    Nvram(NvramArgs),

    /// Hard Disk Drive (HDD) MEIHDFS and partition forensics
    Hdd(HddArgs),

    /// Cryptanalysis and optical update container (.FRM) inspection
    Frm(FrmArgs),

    /// Firmware partition headers, boot sectors, and formatted hex dumps
    Headers(HeadersArgs),

    /// High-performance zero-copy string extraction and keyword scanner
    Scan(ScanArgs),

    /// Index and search service manuals and schematics (PDF)
    Manuals(ManualsArgs),

    /// Hardware host controller (RP2040 / RP2350) and GDB RSP bridge
    Pico(PicoArgs),

    /// Relational JSON hardware, flash, EEPROM, and media dictionary inspector
    Profiles(ProfilesArgs),

    /// Multi-language SSOT code & schema projection exporter (`C11`, `TypeScript`, `JSON Schema`, `Ghidra`)
    Ssot(SsotArgs),

    /// Ghidra reverse engineering script generation directly from Rust SSOT
    Ghidra(GhidraArgs),

    /// Native 6-layer Zero-Error RAG Engine for reverse engineering queries
    Rag(RagArgs),

    /// Display or export official Matsushita AM34 / MN103S memory map
    #[command(name = "memory-map", alias = "memory_map")]
    MemoryMap(MemoryMapArgs),

    /// Cryptographic artifact SHA-256 verification and assertion
    #[command(name = "verify-artifact", alias = "verify")]
    VerifyArtifact(VerifyArtifactArgs),

    /// GDB RSP bridge network and serial connection parameters
    #[command(name = "gdb-config", alias = "gdb_config")]
    GdbConfig(GdbConfigArgs),

    /// Start TCP-to-Serial GDB Remote Serial Protocol (RSP) bridge (top-level alias)
    #[command(name = "gdb-bridge", alias = "gdb_bridge")]
    GdbBridge {
        /// Serial port identifier connected to Raspberry Pi Pico (e.g. COM10 or /dev/ttyACM0)
        #[arg(short, long, value_name = "PORT")]
        port: String,

        /// Serial baud rate (defaults to 115200)
        #[arg(short, long, default_value_t = 115200)]
        baud: u32,

        /// TCP listener address and port for incoming GDB connections
        #[arg(short, long, default_value = "127.0.0.1:3333")]
        listen: String,
    },

    /// Start native Rust Model Context Protocol (MCP) stdio server
    Mcp(McpArgs),

    /// Pre-MAIN Unbrick, SD Recovery Kit Builder, PROG061 BGO, & MAIN AV TBC Passthrough Workbench
    Unbrick(UnbrickArgs),

    /// Deterministic FAT16B (0x06) SD recovery card creator & safe SD target inspector
    Sd(SdArgs),

    /// ISO 9660 Level 1 (1x / minimum speed) optical DVD recovery creator & burner
    Dvd(DvdArgs),

    /// Segregated forensic workflow sessions (`raw/`, `frm/`, `recompiled/`)
    Session(SessionArgs),

    /// Static Matsushita MN103S / AM34 reverse-engineering traps, xrefs, & DRV CLI auditors
    Re(ReArgs),

    /// Installs OS application launcher menu entry, icon, and optional Desktop shortcut (Linux XDG / Windows .lnk)
    #[command(name = "install-shortcut", alias = "install_shortcut")]
    InstallShortcut {
        /// Skip creating an icon on ~/Desktop (menu launcher only)
        #[arg(long)]
        no_desktop: bool,
    },
}
