//! External Desktop Engineering Tool Launchers & Native Zero-Error RAG Command

use std::process::{Command, Stdio};

use super::hardware::run_blocking;

/// Resolves the Panadiga workspace root via `panadiga-core` multiplatform discovery.
#[allow(dead_code)]
fn resolve_workspace_root() -> std::path::PathBuf {
    panadiga_core::constants::resolve_workspace_root()
}

/// Spawns a fully detached child process cleanly across Desktop OSes.
#[allow(dead_code)]
fn spawn_detached(
    program: &str,
    args: &[&str],
    cwd: &std::path::Path,
) -> Result<(), std::io::Error> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn().map(|_| ())
}

#[cfg(target_os = "windows")]
fn launch_tools_script(root: &std::path::Path, script: &str) -> Result<(), std::io::Error> {
    let script_path = root.join("tools").join(script);
    if !script_path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("script não encontrado: {}", script_path.display()),
        ));
    }
    let script_str = script_path.to_string_lossy().to_string();
    spawn_detached("cmd", &["/c", "start", "", &script_str], root)
}

#[cfg(target_os = "linux")]
fn spawn_first_terminal(
    root: &std::path::Path,
    shell_cmd: Option<&str>,
) -> Result<(), std::io::Error> {
    for (term, exec_flag) in [
        ("x-terminal-emulator", "-e"),
        ("gnome-terminal", "--"),
        ("konsole", "-e"),
        ("xfce4-terminal", "-x"),
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

/// Launches external engineering tools across Windows, macOS, Linux, and returns clean guidance on Android.
#[tauri::command]
pub fn launch_external_tool_command(tool: String) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        let _ = tool;
        Err(
            "Ferramentas externas de bancada (GDB CLI, Ghidra, Terminal) são recursos exclusivos do host Desktop; utilize os painéis nativos In-App no Android."
                .to_string(),
        )
    }

    #[cfg(target_os = "windows")]
    {
        let root = resolve_workspace_root();
        match tool.to_lowercase().as_str() {
            "gdb" => {
                launch_tools_script(&root, "conectar_gdb_multiarch.cmd")
                    .map_err(|e| format!("Falha ao iniciar GDB Multiarch: {}", e))?;
                Ok("Sessão interativa do GDB iniciada em novo terminal.".to_string())
            }
            "ghidra_scripts" | "ghidra" => {
                launch_tools_script(&root, "iniciar_ghidra.cmd")
                    .map_err(|e| format!("Falha ao iniciar Ghidra: {}", e))?;
                Ok("Inicializador do Ghidra disparado.".to_string())
            }
            "hxd" => {
                launch_tools_script(&root, "Abrir_HxD.cmd")
                    .map_err(|e| format!("Falha ao abrir editor hexadecimal: {}", e))?;
                Ok("Editor hexadecimal iniciado no workspace.".to_string())
            }
            "terminal" => {
                let location = format!("Set-Location -LiteralPath '{}'", root.display());
                spawn_detached(
                    "cmd",
                    &[
                        "/c",
                        "start",
                        "",
                        "powershell",
                        "-NoExit",
                        "-Command",
                        &location,
                    ],
                    &root,
                )
                .map_err(|e| format!("Falha ao abrir terminal: {}", e))?;
                Ok("Terminal PowerShell iniciado no workspace.".to_string())
            }
            other => Err(format!(
                "Ferramenta externa '{}' não suportada. Opções: gdb, ghidra, hxd, terminal",
                other
            )),
        }
    }

    #[cfg(target_os = "macos")]
    {
        let root = resolve_workspace_root();
        let root_str = root.to_string_lossy().to_string();
        match tool.to_lowercase().as_str() {
            "gdb" => {
                spawn_detached(
                    "osascript",
                    &[
                        "-e",
                        &format!(
                            "tell application \"Terminal\" to do script \"cd '{}' && gdb-multiarch -ex 'target remote 127.0.0.1:3333'\"",
                            root_str
                        ),
                    ],
                    &root,
                )
                .map_err(|e| format!("Falha ao iniciar Terminal com GDB: {}", e))?;
                Ok("Terminal macOS iniciado conectando em 127.0.0.1:3333.".to_string())
            }
            "ghidra_scripts" | "ghidra" => {
                let ghidra_dir = root.join("tools").join("ghidra");
                spawn_detached("open", &[&ghidra_dir.to_string_lossy()], &root)
                    .map_err(|e| format!("Falha ao abrir scripts do Ghidra: {}", e))?;
                Ok("Diretório de scripts Ghidra aberto no Finder.".to_string())
            }
            "hxd" => {
                let images_dir = root.join("firmware_and_images");
                spawn_detached("open", &[&images_dir.to_string_lossy()], &root)
                    .map_err(|e| format!("Falha ao abrir pasta de imagens: {}", e))?;
                Ok("Diretório de imagens binárias aberto no Finder.".to_string())
            }
            "terminal" => {
                spawn_detached("open", &["-a", "Terminal", &root_str], &root)
                    .map_err(|e| format!("Falha ao abrir Terminal macOS: {}", e))?;
                Ok("Terminal macOS iniciado no workspace.".to_string())
            }
            other => Err(format!(
                "Ferramenta externa '{}' não suportada. Opções: gdb, ghidra, hxd, terminal",
                other
            )),
        }
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "android")
    ))]
    {
        let root = resolve_workspace_root();
        match tool.to_lowercase().as_str() {
            "gdb" => {
                spawn_first_terminal(
                    &root,
                    Some("gdb-multiarch -ex 'target remote 127.0.0.1:3333'; exec bash"),
                )
                .map_err(|e| format!("Falha ao iniciar GDB no terminal Linux: {}", e))?;
                Ok("Sessão GDB Multiarch iniciada no terminal.".to_string())
            }
            "ghidra_scripts" | "ghidra" => {
                let ghidra_res = spawn_detached("ghidraRun", &[], &root).or_else(|_| {
                    spawn_detached(
                        "xdg-open",
                        &[&root.join("tools").join("ghidra").to_string_lossy()],
                        &root,
                    )
                });
                ghidra_res.map_err(|e| format!("Falha ao iniciar Ghidra no Linux: {}", e))?;
                Ok("Ghidra / diretório de scripts aberto.".to_string())
            }
            "hxd" => {
                spawn_detached(
                    "xdg-open",
                    &[&root.join("firmware_and_images").to_string_lossy()],
                    &root,
                )
                .map_err(|e| format!("Falha ao abrir diretório de imagens: {}", e))?;
                Ok("Pasta de dumps abertas via xdg-open (use o Inspetor Hex nativo).".to_string())
            }
            "terminal" => {
                spawn_first_terminal(&root, None)
                    .map_err(|e| format!("Falha ao abrir terminal Linux: {}", e))?;
                Ok("Terminal Linux iniciado no workspace.".to_string())
            }
            other => Err(format!(
                "Ferramenta externa '{}' não suportada. Opções: gdb, ghidra, hxd, terminal",
                other
            )),
        }
    }
}

/// Queries the deterministic 6-layer Zero-Error RAG Engine natively in-process.
#[tauri::command]
pub async fn query_rag_engine(
    query: String,
    _top_k: Option<usize>,
) -> Result<panadiga_rag::models::RagQueryResultDto, String> {
    run_blocking(move || Ok(panadiga_rag::query(&query).to_query_result())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_root_contains_tools_dir() {
        assert!(resolve_workspace_root().join("tools").is_dir());
    }

    #[test]
    fn unknown_external_tool_is_rejected() {
        let err = launch_external_tool_command("nope".to_string()).unwrap_err();
        assert!(err.contains("não suportada"));
    }
}
