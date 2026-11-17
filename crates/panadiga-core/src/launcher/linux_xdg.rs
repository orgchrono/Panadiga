//! Linux Freedesktop XDG launcher, Hicolor icon, and Desktop shortcut installer.

#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use std::{fs, process::Command};

#[cfg(target_os = "linux")]
use anyhow::{Context, Result};

#[cfg(target_os = "linux")]
use super::{LauncherInstallReport, render_linux_desktop_entry};

#[cfg(target_os = "linux")]
const EMBEDDED_LAUNCHER_ICON_PNG: &[u8] =
    include_bytes!("../../../../apps/desktop/src-tauri/icons/128x128.png");

#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XdgLauncherPaths {
    pub data_home: PathBuf,
    pub app_dir: PathBuf,
    pub app_entry: PathBuf,
    pub icon_dir: PathBuf,
    pub icon_file: PathBuf,
    pub desktop_shortcut: PathBuf,
}

#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
pub(crate) fn xdg_paths_from_env(
    home: Option<&str>,
    xdg_data_home: Option<&str>,
    desktop_dir_override: Option<&str>,
) -> XdgLauncherPaths {
    let home_dir = home
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let data_home = xdg_data_home
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join(".local").join("share"));
    let desktop_dir = desktop_dir_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join("Desktop"));

    let app_dir = data_home.join("applications");
    let icon_dir = data_home
        .join("icons")
        .join("hicolor")
        .join("128x128")
        .join("apps");

    XdgLauncherPaths {
        app_entry: app_dir.join("panadiga.desktop"),
        icon_file: icon_dir.join("panadiga.png"),
        data_home,
        app_dir,
        icon_dir,
        desktop_shortcut: desktop_dir.join("Panadiga.desktop"),
    }
}

#[cfg(target_os = "linux")]
fn resolve_linux_xdg_paths() -> XdgLauncherPaths {
    let home = std::env::var("HOME").ok();
    let xdg_data = std::env::var("XDG_DATA_HOME").ok();
    let xdg_desktop = Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    xdg_paths_from_env(home.as_deref(), xdg_data.as_deref(), xdg_desktop.as_deref())
}

#[cfg(target_os = "linux")]
fn set_executable_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn install_linux_xdg_launcher(
    exec_path: &Path,
    create_desktop_shortcut: bool,
) -> Result<LauncherInstallReport> {
    let paths = resolve_linux_xdg_paths();
    fs::create_dir_all(&paths.app_dir)?;
    fs::create_dir_all(&paths.icon_dir)?;
    fs::write(&paths.icon_file, EMBEDDED_LAUNCHER_ICON_PNG)
        .with_context(|| format!("Failed writing icon '{}'", paths.icon_file.display()))?;

    let content = render_linux_desktop_entry(exec_path, &paths.icon_file.to_string_lossy());
    fs::write(&paths.app_entry, &content).with_context(|| {
        format!(
            "Failed writing desktop entry '{}'",
            paths.app_entry.display()
        )
    })?;
    set_executable_permissions(&paths.app_entry);

    let shortcut_path = if create_desktop_shortcut
        && let Some(parent) = paths.desktop_shortcut.parent()
        && parent.is_dir()
    {
        fs::write(&paths.desktop_shortcut, &content).with_context(|| {
            format!(
                "Failed writing desktop shortcut '{}'",
                paths.desktop_shortcut.display()
            )
        })?;
        set_executable_permissions(&paths.desktop_shortcut);
        let _ = Command::new("gio")
            .args([
                "set",
                &paths.desktop_shortcut.to_string_lossy(),
                "metadata::trusted",
                "true",
            ])
            .status();
        Some(paths.desktop_shortcut.to_string_lossy().into_owned())
    } else {
        None
    };

    let _ = Command::new("update-desktop-database")
        .arg(&paths.app_dir)
        .status();
    let _ = Command::new("gtk-update-icon-cache")
        .args(["-q", "-t", "-f"])
        .arg(paths.data_home.join("icons").join("hicolor"))
        .status();

    Ok(LauncherInstallReport {
        platform: "linux".to_string(),
        desktop_entry_path: paths.app_entry.to_string_lossy().into_owned(),
        icon_path: Some(paths.icon_file.to_string_lossy().into_owned()),
        shortcut_path,
        created: true,
        message: format!(
            "Launcher XDG registrado em {} com ícone em {}.",
            paths.app_entry.display(),
            paths.icon_file.display()
        ),
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn ensure_linux_xdg_launcher_once(exec_path: &Path) {
    if Path::new("/usr/share/applications/panadiga.desktop").is_file() {
        return;
    }
    let paths = resolve_linux_xdg_paths();
    if paths.app_entry.is_file() && paths.icon_file.is_file() {
        return;
    }
    let _ = install_linux_xdg_launcher(exec_path, false);
}
