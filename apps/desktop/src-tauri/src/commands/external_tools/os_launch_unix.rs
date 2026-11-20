//! Unix (macOS / Linux) & Android tool launcher dispatchers for external engineering apps.

#[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
use crate::commands::constants::{DEFAULT_GDB_LISTEN_SOCKET, FIRMWARE_IMAGES_DIR_NAME};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use std::path::Path;

#[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
use super::ghidra_discovery::{collect_ghidra_home_candidates, sync_ghidra_ssot_and_user_scripts};
#[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
use super::os_launch::{spawn_detached, unsupported_tool_err};

#[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
fn discover_unix_ghidra_run() -> std::path::PathBuf {
    let mut scan_roots: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("/opt"),
        std::path::PathBuf::from("/opt/ghidra"),
        std::path::PathBuf::from("/usr/share"),
        std::path::PathBuf::from("/usr/local/share"),
        std::path::PathBuf::from("/usr/local/share/ghidra"),
        std::path::PathBuf::from("/Applications"),
    ];
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        scan_roots.push(home.join("ghidra"));
    }
    collect_ghidra_home_candidates(&scan_roots)
        .into_iter()
        .map(|dir| dir.join("ghidraRun"))
        .find(|cand| cand.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("ghidraRun"))
}

#[cfg(target_os = "linux")]
fn spawn_first_terminal(root: &Path, shell_cmd: Option<&str>) -> Result<(), std::io::Error> {
    for (term, exec_flag) in [
        ("x-terminal-emulator", "-e"),
        ("gnome-terminal", "--"),
        ("konsole", "-e"),
        ("xfce4-terminal", "-x"),
        ("kitty", "--"),
        ("alacritty", "-e"),
        ("wezterm", "start"),
        ("tilix", "-e"),
        ("mate-terminal", "-x"),
        ("lxterminal", "-e"),
        ("xterm", "-e"),
    ] {
        let result = match shell_cmd {
            Some(c) => spawn_detached(term, &[exec_flag, "sh", "-lc", c], root),
            None => spawn_detached(term, &[], root),
        };
        if result.is_ok() {
            return Ok(());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Nenhum emulador de terminal Linux encontrado no PATH.",
    ))
}

#[cfg(target_os = "macos")]
pub(super) fn launch_for_current_os(tool: &str, root: &Path) -> IpcResult<String> {
    let root_str = root.to_string_lossy().to_string();
    match tool.to_lowercase().as_str() {
        "gdb" => {
            spawn_detached(
                "osascript",
                &[
                    "-e",
                    &format!(
                        "tell application \"Terminal\" to do script \"cd '{}' && gdb-multiarch -ex 'set architecture mn10300' -ex 'target remote {}'\"",
                        root_str, DEFAULT_GDB_LISTEN_SOCKET
                    ),
                ],
                root,
            )
            .map_err(|e| {
                PanadigaIpcError::serial(format!("Falha ao iniciar Terminal com GDB: {}", e))
            })?;
            Ok(format!(
                "Terminal macOS iniciado conectando em {}.",
                DEFAULT_GDB_LISTEN_SOCKET
            ))
        }
        "ghidra_scripts" | "ghidra" | "ghidra_sync" => {
            let (synced, _) = sync_ghidra_ssot_and_user_scripts(root)?;
            let ghidra_dir = root.join("tools").join("ghidra");
            let ghidra_cmd = discover_unix_ghidra_run();
            spawn_detached(&ghidra_cmd.to_string_lossy(), &[], root)
                .or_else(|_| spawn_detached("open", &[&ghidra_dir.to_string_lossy()], root))
                .map_err(|e| {
                    PanadigaIpcError::io(format!("Falha ao abrir Ghidra / scripts no macOS: {}", e))
                })?;
            Ok(format!(
                "Ghidra / scripts Jython SSOT sincronizados ({} arquivos) e abertos no macOS.",
                synced
            ))
        }
        "ghidra_mcp" => Err(PanadigaIpcError::validation(
            "No macOS, inicie o bridge ghidra-mcp no ambiente Python local apontando para :8081.",
        )),
        "hxd" => {
            let images_dir = root.join(FIRMWARE_IMAGES_DIR_NAME);
            spawn_detached("open", &[&images_dir.to_string_lossy()], root).map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao abrir pasta de imagens: {}", e))
            })?;
            Ok("Diretório de imagens binárias aberto no Finder.".to_string())
        }
        "terminal" => {
            spawn_detached("open", &["-a", "Terminal", &root_str], root).map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao abrir Terminal macOS: {}", e))
            })?;
            Ok("Terminal macOS iniciado no workspace.".to_string())
        }
        other => Err(unsupported_tool_err(other)),
    }
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android")
))]
pub(super) fn launch_for_current_os(tool: &str, root: &Path) -> IpcResult<String> {
    match tool.to_lowercase().as_str() {
        "gdb" => {
            let cmd = format!(
                "gdb-multiarch -ex 'set architecture mn10300' -ex 'target remote {}'; exec bash",
                DEFAULT_GDB_LISTEN_SOCKET
            );
            spawn_first_terminal(root, Some(&cmd)).map_err(|e| {
                PanadigaIpcError::serial(format!("Falha ao iniciar GDB no terminal Linux: {}", e))
            })?;
            Ok("Sessão GDB Multiarch iniciada no terminal.".to_string())
        }
        "ghidra_scripts" | "ghidra" | "ghidra_sync" => {
            let (synced, _) = sync_ghidra_ssot_and_user_scripts(root)?;
            let ghidra_cmd = discover_unix_ghidra_run();
            let ghidra_res =
                spawn_detached(&ghidra_cmd.to_string_lossy(), &[], root).or_else(|_| {
                    spawn_detached("ghidra", &[], root).or_else(|_| {
                        spawn_detached(
                            "xdg-open",
                            &[&root.join("tools").join("ghidra").to_string_lossy()],
                            root,
                        )
                    })
                });
            ghidra_res.map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao iniciar Ghidra no Linux: {}", e))
            })?;
            Ok(format!(
                "Ghidra / scripts Jython SSOT ({} sincronizados em ~/ghidra_scripts) abertos.",
                synced
            ))
        }
        "ghidra_mcp" => Err(PanadigaIpcError::validation(
            "Inicie o servidor ghidra-mcp no ambiente Linux apontando para http://127.0.0.1:8081.",
        )),
        "hxd" => {
            spawn_detached(
                "xdg-open",
                &[&root.join(FIRMWARE_IMAGES_DIR_NAME).to_string_lossy()],
                root,
            )
            .map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao abrir diretório de imagens: {}", e))
            })?;
            Ok("Pasta de dumps abertas via xdg-open (use o Inspetor Hex nativo).".to_string())
        }
        "terminal" => {
            spawn_first_terminal(root, None).map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao abrir terminal Linux: {}", e))
            })?;
            Ok("Terminal Linux iniciado no workspace.".to_string())
        }
        other => Err(unsupported_tool_err(other)),
    }
}

#[cfg(target_os = "android")]
pub(super) fn launch_for_current_os(_tool: &str, _root: &Path) -> IpcResult<String> {
    Err(PanadigaIpcError::validation(
        "Ferramentas externas de bancada (GDB CLI, Ghidra, Terminal) são recursos exclusivos do host Desktop; utilize os painéis nativos In-App no Android.",
    ))
}
