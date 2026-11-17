//! CLI handler and formatted output for `panadiga pico error-lookup`.

use std::path::PathBuf;

use anyhow::Result;
use panadiga_core::log_println;
use panadiga_core::re::error_dict::{
    FirmwareErrorEntry, all_firmware_errors, lookup_firmware_errors,
};

fn print_error_entry(idx: usize, entry: &FirmwareErrorEntry) {
    log_println!(
        "{}. [{}] {} ({:?} @ {})",
        idx + 1,
        entry.code,
        entry.title,
        entry.domain,
        entry.address
    );
    log_println!("   Original   : {}", entry.native_text);
    log_println!("   Traducao   : {}", entry.translation);
    for act in &entry.actions {
        log_println!("   -> Acao    : {}", act);
    }
}

/// Queries embedded Panasonic / Matshita firmware error records by code or keyword.
pub fn handle_error_lookup(query: &str, json_out: Option<PathBuf>) -> Result<()> {
    let hits = lookup_firmware_errors(query);
    if let Some(out) = json_out {
        super::super::json_io::emit_json_output(&hits, Some(out.as_path()))?;
        return Ok(());
    }
    log_println!("====================================================================");
    log_println!(
        "   PANADIGA FIRMWARE ERROR DICTIONARY ({} registros) | Busca: '{}'",
        all_firmware_errors().len(),
        query
    );
    log_println!("====================================================================");
    if hits.is_empty() {
        log_println!("Nenhum registro encontrado para '{}'.", query);
        return Ok(());
    }
    for (idx, hit) in hits.iter().take(25).enumerate() {
        print_error_entry(idx, hit);
    }
    if hits.len() > 25 {
        log_println!(
            "... ({} mais resultados; refine a busca ou use --json -)",
            hits.len() - 25
        );
    }
    Ok(())
}
