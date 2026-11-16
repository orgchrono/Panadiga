//! Panasonic DIGA Engineering Suite - Interactive Terminal Workbench (TUI)

mod options;

use std::io::{self, Write};

use anyhow::Result;
use panadiga_core::{log_println, log_raw};

/// Prompts the user with a formatted string and captures a single trimmed line of input.
pub fn prompt_line(prompt: &str) -> String {
    log_raw!("{}", prompt);
    let _ = io::stdout().flush();
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer);
    buffer.trim().to_string()
}

/// Runs the interactive terminal user interface (TUI) loop.
pub fn run_interactive_pipeline() -> Result<()> {
    loop {
        log_println!();
        log_println!(
            "\x1b[1;36m====================================================================\x1b[0m"
        );
        log_println!(
            "\x1b[1;36m      PANADIGA ENGINEERING SUITE - INTERACTIVE WORKBENCH (TUI)\x1b[0m"
        );
        log_println!("\x1b[1;33m      Matsushita AM34 / MN103S / Spansion S29JL064H\x1b[0m");
        log_println!(
            "\x1b[1;36m====================================================================\x1b[0m"
        );
        log_println!(
            "  \x1b[1;32m[1]\x1b[0m Hardware Host Discovery & Ping (Pico RP2040 / RP2350)"
        );
        log_println!(
            "  \x1b[1;32m[2]\x1b[0m NVRAM & NOR Flash Sanitization (Surgical Microsector U99 Fix)"
        );
        log_println!(
            "  \x1b[1;32m[3]\x1b[0m HDD MEIHDFS Storage Forensics (Inspect & Extract Partitions)"
        );
        log_println!(
            "  \x1b[1;32m[4]\x1b[0m Firmware & Optical Container (.FRM Cryptanalysis & Codebook)"
        );
        log_println!("  \x1b[1;32m[5]\x1b[0m Matsushita AM34 Official Memory Map Architecture");
        log_println!("  \x1b[1;32m[6]\x1b[0m Export Ghidra Reverse Engineering Scripts (SSOT)");
        log_println!("  \x1b[1;32m[7]\x1b[0m Service Manuals & Schematic PDF Indexer & Search");
        log_println!(
            "  \x1b[1;32m[8]\x1b[0m Zero-Error Deterministic Safety RAG Engine (Ask Engineer)"
        );
        log_println!("  \x1b[1;32m[9]\x1b[0m Cryptographic Artifact Hash Verification (SHA-256)");
        log_println!("  \x1b[1;35m[D]\x1b[0m Launch Panadiga Native Desktop GUI (Tauri / React)");
        log_println!("  \x1b[1;31m[0]\x1b[0m Exit Workbench");
        log_println!(
            "\x1b[1;36m--------------------------------------------------------------------\x1b[0m"
        );

        let choice = prompt_line("\x1b[1;37mSelect an option [1-9, D, 0]: \x1b[0m");
        log_println!();

        match choice.to_uppercase().as_str() {
            "1" => options::handle_pico_option()?,
            "2" => options::handle_nvram_option()?,
            "3" => options::handle_hdd_option()?,
            "4" => options::handle_frm_option()?,
            "5" => options::print_memory_map_architecture(),
            "6" => options::handle_ghidra_export()?,
            "7" => options::handle_manuals_search()?,
            "8" => options::handle_rag_query(),
            "9" => options::handle_verify_artifact()?,
            "D" => options::handle_launch_desktop(),
            "0" | "Q" | "EXIT" => {
                log_println!("\x1b[1;32mExiting Panadiga Workbench. Have a great session!\x1b[0m");
                break;
            }
            _ => {
                log_println!("\x1b[1;31m[!] Invalid option. Please choose 1-9, D, or 0.\x1b[0m");
            }
        }

        log_println!();
        prompt_line("\x1b[1;30mPress [Enter] to continue to workbench menu...\x1b[0m");
    }

    Ok(())
}
