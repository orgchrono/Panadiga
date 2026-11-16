//! Ghidra installation/project discovery and SSOT Jython script synchronization.

#[cfg(not(target_os = "android"))]
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
pub(super) struct GhidraProjectLocation {
    pub dir: PathBuf,
    pub name: String,
    pub gpr: PathBuf,
}

#[cfg(target_os = "windows")]
pub(super) struct GhidraInstallDiscovery {
    pub run_bat: Option<PathBuf>,
    pub headless_bat: Option<PathBuf>,
    pub project: Option<GhidraProjectLocation>,
}

#[cfg(not(target_os = "android"))]
use crate::commands::error::{IpcResult, PanadigaIpcError};

/// Regenerates all Ghidra Jython scripts from Rust SSOT and mirrors them into `~/ghidra_scripts`.
#[cfg(not(target_os = "android"))]
pub(super) fn sync_ghidra_ssot_and_user_scripts(
    root: &Path,
) -> IpcResult<(usize, Option<PathBuf>)> {
    let tools_ghidra = root.join("tools").join("ghidra");
    panadiga_core::ghidra::export_all_ghidra_scripts(&tools_ghidra).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha ao exportar scripts Ghidra da SSOT Rust: {}",
            e
        ))
    })?;

    let home_dir = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    let user_scripts_dir = home_dir.map(|h| h.join("ghidra_scripts"));
    let mut synced = 0usize;

    if let Some(ref dest_dir) = user_scripts_dir {
        std::fs::create_dir_all(dest_dir).map_err(|e| {
            PanadigaIpcError::io(format!("Falha ao criar {}: {}", dest_dir.display(), e))
        })?;
        if let Ok(entries) = std::fs::read_dir(&tools_ghidra) {
            for entry in entries.flatten() {
                let src_path = entry.path();
                if src_path.extension().and_then(|s| s.to_str()) == Some("py")
                    && let Some(fname) = src_path.file_name()
                {
                    std::fs::copy(&src_path, dest_dir.join(fname)).map_err(|e| {
                        PanadigaIpcError::io(format!(
                            "Falha ao sincronizar {}: {}",
                            fname.to_string_lossy(),
                            e
                        ))
                    })?;
                    synced += 1;
                }
            }
        }
    }

    Ok((synced, user_scripts_dir))
}

#[cfg(target_os = "windows")]
pub(super) fn discover_ghidra_install_and_project() -> GhidraInstallDiscovery {
    let mut home_candidates: Vec<PathBuf> = Vec::new();
    if let Ok(env_home) = std::env::var("GHIDRA_HOME") {
        let trimmed = env_home.trim();
        if !trimmed.is_empty() {
            home_candidates.push(PathBuf::from(trimmed));
        }
    }
    let mut scan_roots = vec![
        PathBuf::from(r"C:\ghidra"),
        PathBuf::from(r"C:\tools\ghidra"),
        PathBuf::from(r"D:\ghidra"),
        PathBuf::from(r"C:\Program Files\ghidra"),
    ];
    if let Some(user_profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        scan_roots.push(user_profile.join("ghidra"));
    }
    for base in &scan_roots {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("ghidra_"))
                {
                    home_candidates.push(p);
                }
            }
        }
    }

    let ghidra_home = home_candidates
        .into_iter()
        .find(|dir| dir.join("ghidraRun.bat").is_file());
    let run_bat = ghidra_home.as_ref().map(|h| h.join("ghidraRun.bat"));
    let headless_bat = ghidra_home
        .as_ref()
        .map(|h| h.join("support").join("analyzeHeadless.bat"))
        .filter(|p| p.is_file());

    let project_candidates = [
        GhidraProjectLocation {
            dir: PathBuf::from(r"C:\ghidra\projects"),
            name: "Panadiga_MN103S".to_string(),
            gpr: PathBuf::from(r"C:\ghidra\projects\Panadiga_MN103S.gpr"),
        },
        GhidraProjectLocation {
            dir: PathBuf::from(r"C:\ghidra\projects\Panadiga_MN103S"),
            name: "Panadiga_MN103S".to_string(),
            gpr: PathBuf::from(r"C:\ghidra\projects\Panadiga_MN103S\Panadiga_MN103S.gpr"),
        },
    ];
    let project = project_candidates.into_iter().find(|loc| loc.gpr.is_file());

    GhidraInstallDiscovery {
        run_bat,
        headless_bat,
        project,
    }
}
