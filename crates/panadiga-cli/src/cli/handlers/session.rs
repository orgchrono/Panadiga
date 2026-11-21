#![deny(unsafe_code)]

//! Handlers for `panadiga session` (`init`, `ingest`, `status`, `extract-drv`).

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use panadiga_core::constants::resolve_writable_data_root;
use panadiga_core::log_println;
use panadiga_core::re::unbrick_sd_kit::SdLoaderProfile;
use panadiga_core::vault::{
    VaultBucket, auto_classify_and_ingest_file, extract_nor_drv_into_vault, init_vault_session,
    list_vault_sessions, load_vault_session,
};

use super::json_io::emit_json_output;
use crate::cli::args::{SessionActions, SessionArgs};

fn parse_bucket_override(raw: Option<&str>) -> Result<Option<VaultBucket>> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::parse::<VaultBucket>)
        .transpose()
}

fn resolve_vault_root(explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(resolve_writable_data_root)
}

/// Dispatches `panadiga session` subcommands.
pub fn handle_session(args: SessionArgs) -> Result<()> {
    match args.action {
        SessionActions::Init {
            model,
            loader_profile,
            session_id,
            vault_root,
            json,
        } => {
            let root = resolve_vault_root(vault_root);
            let profile: SdLoaderProfile = loader_profile.parse()?;
            let manifest = init_vault_session(&root, &model, profile, session_id.as_deref())?;
            log_println!(
                "[OK] Vault session ready: {} (model={}, dir={})",
                manifest.session_id,
                manifest.model_id,
                manifest.session_dir
            );
            emit_json_output(&manifest, json.as_deref())?;
        }
        SessionActions::Ingest {
            session_dir,
            file,
            bucket,
            origin,
            json,
        } => {
            let bucket_opt = parse_bucket_override(bucket.as_deref())?;
            let rec =
                auto_classify_and_ingest_file(&session_dir, &file, bucket_opt, &origin, None)?;
            log_println!(
                "[OK] Ingested {} ({} B, sha256={}, origin={})",
                rec.relative_path,
                rec.size_bytes,
                rec.sha256,
                rec.origin
            );
            emit_json_output(&rec, json.as_deref())?;
        }
        SessionActions::Status {
            session_dir,
            vault_root,
            json,
        } => {
            if let Some(dir) = session_dir {
                let manifest = load_vault_session(&dir)?;
                log_println!(
                    "Session {} | model={} | step={:?} | raw={} frm={} recompiled={}",
                    manifest.session_id,
                    manifest.model_id,
                    manifest.current_step,
                    manifest.raw_artifacts.len(),
                    manifest.frm_artifacts.len(),
                    manifest.recompiled_artifacts.len()
                );
                emit_json_output(&manifest, json.as_deref())?;
            } else {
                let root = resolve_vault_root(vault_root);
                let sessions = list_vault_sessions(&root)?;
                log_println!(
                    "Found {} vault session(s) in {}",
                    sessions.len(),
                    root.display()
                );
                for s in &sessions {
                    log_println!(
                        "  {} | {} | step={:?}",
                        s.session_id,
                        s.model_id,
                        s.current_step
                    );
                }
                emit_json_output(&sessions, json.as_deref())?;
            }
        }
        SessionActions::ExtractDrv {
            session_dir,
            nor_dump,
            json,
        } => {
            let nor_bytes = fs::read(&nor_dump)?;
            let rec = extract_nor_drv_into_vault(&session_dir, &nor_bytes)?;
            log_println!(
                "[OK] Extracted NOR DRV into {} ({} B, sha256={})",
                rec.relative_path,
                rec.size_bytes,
                rec.sha256
            );
            emit_json_output(&rec, json.as_deref())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::Cli;
    use crate::cli::handlers::dispatch;
    use clap::Parser;
    use panadiga_core::vault::load_vault_session;

    #[test]
    fn cli_session_init_ingest_segregates_raw_and_frm_and_blocks_cross_bucket() {
        let base =
            std::env::temp_dir().join(format!("panadiga_cli_session_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        let cli_init = Cli::try_parse_from([
            "panadiga",
            "session",
            "init",
            "--model",
            "DMR-EH55",
            "--session-id",
            "cli_sess",
            "--vault-root",
            base.to_str().unwrap(),
        ])
        .unwrap();
        dispatch(cli_init.command.unwrap()).unwrap();

        let sess_dir = base.join("sessions").join("cli_sess");
        let frm_file = base.join("PANA_DVD.FRM");
        let bin_file = base.join("ic5001.bin");
        fs::write(&frm_file, b"PANA_DVD_DUMMY").unwrap();
        fs::write(&bin_file, vec![0xA5u8; 64]).unwrap();

        let cli_frm = Cli::try_parse_from([
            "panadiga",
            "session",
            "ingest",
            "--session-dir",
            sess_dir.to_str().unwrap(),
            "--file",
            frm_file.to_str().unwrap(),
        ])
        .unwrap();
        dispatch(cli_frm.command.unwrap()).unwrap();

        let cli_bin = Cli::try_parse_from([
            "panadiga",
            "session",
            "ingest",
            "--session-dir",
            sess_dir.to_str().unwrap(),
            "--file",
            bin_file.to_str().unwrap(),
        ])
        .unwrap();
        dispatch(cli_bin.command.unwrap()).unwrap();

        let cross_bucket = Cli::try_parse_from([
            "panadiga",
            "session",
            "ingest",
            "--session-dir",
            sess_dir.to_str().unwrap(),
            "--file",
            frm_file.to_str().unwrap(),
            "--bucket",
            "raw",
        ])
        .unwrap();
        assert!(dispatch(cross_bucket.command.unwrap()).is_err());

        let m = load_vault_session(&sess_dir).unwrap();
        assert_eq!(m.frm_artifacts.len(), 1);
        assert_eq!(m.raw_artifacts.len(), 1);
        assert!(sess_dir.join("frm").join("PANA_DVD.FRM").is_file());
        assert!(sess_dir.join("raw").join("ic5001.bin").is_file());

        let _ = fs::remove_dir_all(&base);
    }
}
