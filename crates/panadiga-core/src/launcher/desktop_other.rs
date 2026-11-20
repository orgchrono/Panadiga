//! Windows and macOS shortcut / launcher helpers.

#[cfg(not(target_os = "linux"))]
use std::path::Path;

#[cfg(not(target_os = "linux"))]
use super::LauncherInstallReport;

#[cfg(windows)]
fn create_windows_lnk(lnk_path: &Path, target_exe: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let lnk_s = lnk_path.to_string_lossy().replace('\'', "''");
    let exe_s = target_exe.to_string_lossy().replace('\'', "''");
    let work_dir = target_exe
        .parent()
        .map(|p| p.to_string_lossy().replace('\'', "''"))
        .unwrap_or_default();

    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{lnk_s}'); \
         $s.TargetPath = '{exe_s}'; \
         $s.WorkingDirectory = '{work_dir}'; \
         $s.IconLocation = '{exe_s},0'; \
         $s.Description = 'Panasonic DMR-EH55 DIGA Engineering Suite'; \
         $s.Save()"
    );
    let status = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()?;
    if !status.success() {
        anyhow::bail!("PowerShell CreateShortcut exited with {}", status);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn install_non_linux_launcher(
    exec_path: &Path,
    create_desktop_shortcut: bool,
) -> anyhow::Result<LauncherInstallReport> {
    #[cfg(windows)]
    {
        let start_menu = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(r"Microsoft\Windows\Start Menu\Programs\Panadiga.lnk");
        if let Some(parent) = start_menu.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        create_windows_lnk(&start_menu, exec_path)?;

        let desktop_shortcut = if create_desktop_shortcut {
            let desktop = std::env::var_os("USERPROFILE")
                .map(std::path::PathBuf::from)
                .map(|h| h.join("Desktop").join("Panadiga.lnk"));
            if let Some(ref lnk) = desktop
                && lnk.parent().is_some_and(|p| p.is_dir())
            {
                let _ = create_windows_lnk(lnk, exec_path);
                Some(lnk.to_string_lossy().into_owned())
            } else {
                None
            }
        } else {
            None
        };

        Ok(LauncherInstallReport {
            platform: "windows".to_string(),
            desktop_entry_path: start_menu.to_string_lossy().into_owned(),
            icon_path: Some(exec_path.to_string_lossy().into_owned()),
            shortcut_path: desktop_shortcut,
            created: true,
            message: format!("Atalho criado no Menu Iniciar: {}.", start_menu.display()),
        })
    }

    #[cfg(not(windows))]
    {
        let _ = create_desktop_shortcut;
        Ok(LauncherInstallReport {
            platform: std::env::consts::OS.to_string(),
            desktop_entry_path: exec_path.to_string_lossy().into_owned(),
            icon_path: None,
            shortcut_path: None,
            created: false,
            message: "Plataforma gerenciada nativamente pelo bundle do sistema.".to_string(),
        })
    }
}
