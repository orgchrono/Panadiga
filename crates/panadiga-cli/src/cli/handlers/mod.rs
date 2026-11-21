//! CLI Command Handlers and Dispatch Engine
//!
//! Modularized by functional domain with unified `dispatch` and handler re-exports.

pub mod crypto;
pub mod dvd;
pub mod frm_pack;
pub mod ghidra;
pub mod hdd;
pub mod json_io;
pub mod nvram;
pub mod pico;
pub mod pico_bench;
pub mod profiles;
pub mod re;
pub mod re_extra;
pub mod sd;
pub mod session;
pub mod system;
pub mod unbrick;

pub use crypto::{handle_frm, handle_headers, handle_scan};
pub use dvd::handle_dvd;
pub use ghidra::handle_ghidra;
pub use hdd::handle_hdd;
pub use nvram::handle_nvram;
pub use pico::{handle_gdb_config, handle_pico};
pub use profiles::handle_profiles;
pub use re::handle_re;
pub use sd::handle_sd;
pub use session::handle_session;
pub use system::{
    handle_manuals, handle_mcp, handle_memory_map, handle_rag, handle_ssot, handle_verify_artifact,
};
pub use unbrick::handle_unbrick;

use super::args::Commands;
use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Locates and launches the sibling `panadiga-desktop` native Tauri GUI executable across OSes.
pub fn launch_desktop_executable() -> Result<()> {
    let bin_name = if cfg!(windows) {
        "panadiga-desktop.exe"
    } else {
        "panadiga-desktop"
    };

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join(bin_name));
    }
    let root = panadiga_core::constants::resolve_workspace_root();
    candidates.push(root.join("target").join("release").join(bin_name));
    candidates.push(root.join("target").join("debug").join(bin_name));

    for cand in candidates {
        if cand.is_file() {
            Command::new(&cand).spawn()?;
            return Ok(());
        }
    }

    anyhow::bail!(
        "Native desktop executable '{}' not found alongside CLI. Build with `npm run build` or use `-i` for TUI.",
        bin_name
    )
}

/// Dispatches a parsed CLI command to its specialized handler.
pub fn dispatch(command: Commands) -> Result<()> {
    match command {
        Commands::Desktop | Commands::Gui => launch_desktop_executable(),
        Commands::Interactive => crate::tui::run_interactive_pipeline(),
        Commands::Nvram(args) => handle_nvram(args),
        Commands::Hdd(args) => handle_hdd(args),
        Commands::Frm(args) => handle_frm(args),
        Commands::Headers(args) => handle_headers(args),
        Commands::Scan(args) => handle_scan(args),
        Commands::Manuals(args) => handle_manuals(args),
        Commands::Pico(args) => handle_pico(args),
        Commands::Profiles(args) => handle_profiles(args),
        Commands::Ssot(args) => handle_ssot(args),
        Commands::Ghidra(args) => handle_ghidra(args),
        Commands::Rag(args) => handle_rag(args),
        Commands::MemoryMap(args) => handle_memory_map(args),
        Commands::VerifyArtifact(args) => handle_verify_artifact(args),
        Commands::GdbConfig(args) => handle_gdb_config(args),
        Commands::GdbBridge { port, baud, listen } => {
            panadiga_core::pico::run_gdb_bridge(&port, baud, &listen)
        }
        Commands::Mcp(args) => handle_mcp(args),
        Commands::Unbrick(args) => handle_unbrick(args),
        Commands::Sd(args) => handle_sd(args),
        Commands::Dvd(args) => handle_dvd(args),
        Commands::Session(args) => handle_session(args),
        Commands::Re(args) => handle_re(args),
        Commands::InstallShortcut { no_desktop } => {
            let report = panadiga_core::launcher::install_desktop_launcher(!no_desktop)?;
            panadiga_core::log_println!("[OK] {}", report.message);
            if let Some(shortcut) = report.shortcut_path {
                panadiga_core::log_println!("     Atalho Desktop: {}", shortcut);
            }
            Ok(())
        }
    }
}
