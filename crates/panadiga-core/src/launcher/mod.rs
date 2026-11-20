//! Cross-platform application launcher, icon, and desktop shortcut registration.

mod desktop_other;
mod linux_xdg;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Result report returned after installing OS application menu and desktop shortcuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherInstallReport {
    pub platform: String,
    pub desktop_entry_path: String,
    pub icon_path: Option<String>,
    pub shortcut_path: Option<String>,
    pub created: bool,
    pub message: String,
}

/// Pure Freedesktop `.desktop` specification generator compatible with GNOME, KDE, XFCE, and LXDE.
pub fn render_linux_desktop_entry(exec_path: &Path, icon_value: &str) -> String {
    let raw = exec_path.to_string_lossy();
    let quoted_exec = if raw.contains(' ') {
        format!("\"{raw}\"")
    } else {
        raw.into_owned()
    };
    format!(
        "[Desktop Entry]\n\
         Version=1.0\n\
         Type=Application\n\
         Name=Panadiga\n\
         GenericName=Panasonic DIGA Engineering Suite\n\
         Comment=Panasonic DMR-EH55 Forensic, NVRAM, HDD & P7501 UART Suite\n\
         Exec={quoted_exec} %U\n\
         Icon={icon_value}\n\
         Terminal=false\n\
         Categories=Development;Utility;Electronics;\n\
         Keywords=Panasonic;DIGA;DMR-EH55;MEIHDFS;Pico;MN103S;U99;\n\
         StartupNotify=true\n\
         StartupWMClass=panadiga-desktop\n"
    )
}

/// Resolves the GUI binary (`panadiga-desktop`) near the active executable or in `/usr/bin` / `PATH`.
pub fn resolve_desktop_executable_path() -> PathBuf {
    let bin_name = if cfg!(windows) {
        "panadiga-desktop.exe"
    } else {
        "panadiga-desktop"
    };
    if let Ok(exe) = std::env::current_exe() {
        if exe
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("panadiga-desktop"))
        {
            return exe;
        }
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(bin_name);
            if sibling.is_file() {
                return sibling;
            }
        }
        #[cfg(target_os = "linux")]
        if Path::new("/usr/bin/panadiga-desktop").is_file() {
            return PathBuf::from("/usr/bin/panadiga-desktop");
        }
        return exe;
    }
    PathBuf::from(bin_name)
}

/// Installs OS menu launcher, application icon, and optional Desktop shortcut.
pub fn install_desktop_launcher(
    create_desktop_shortcut: bool,
) -> anyhow::Result<LauncherInstallReport> {
    let exec_path = resolve_desktop_executable_path();
    #[cfg(target_os = "linux")]
    {
        linux_xdg::install_linux_xdg_launcher(&exec_path, create_desktop_shortcut)
    }
    #[cfg(not(target_os = "linux"))]
    {
        desktop_other::install_non_linux_launcher(&exec_path, create_desktop_shortcut)
    }
}

/// Ensures Linux portable/tarball launches register an XDG app-menu entry & icon automatically.
pub fn ensure_linux_desktop_launcher_if_missing() {
    #[cfg(target_os = "linux")]
    {
        let exec_path = resolve_desktop_executable_path();
        linux_xdg::ensure_linux_xdg_launcher_once(&exec_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_valid_freedesktop_desktop_entry_with_wm_class_and_spaces() {
        let entry = render_linux_desktop_entry(
            Path::new("/home/daniel/My Apps/panadiga-desktop"),
            "/home/daniel/.local/share/icons/hicolor/128x128/apps/panadiga.png",
        );
        assert!(entry.contains("[Desktop Entry]"));
        assert!(entry.contains("Exec=\"/home/daniel/My Apps/panadiga-desktop\" %U"));
        assert!(entry.contains("StartupWMClass=panadiga-desktop"));
        assert!(entry.contains("Terminal=false"));
    }

    #[test]
    fn resolves_xdg_launcher_and_desktop_paths_cleanly() {
        let paths = linux_xdg::xdg_paths_from_env(
            Some("/home/diga"),
            Some("/home/diga/.local/share"),
            Some("/home/diga/Area de Trabalho"),
        );
        assert_eq!(
            paths.app_entry,
            PathBuf::from("/home/diga/.local/share/applications/panadiga.desktop")
        );
        assert_eq!(
            paths.icon_file,
            PathBuf::from("/home/diga/.local/share/icons/hicolor/128x128/apps/panadiga.png")
        );
        assert_eq!(
            paths.desktop_shortcut,
            PathBuf::from("/home/diga/Area de Trabalho/Panadiga.desktop")
        );
    }
}
