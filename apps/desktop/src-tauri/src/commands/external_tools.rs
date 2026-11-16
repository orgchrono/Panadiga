//! External Desktop Engineering Tool Launchers, Ghidra SSOT Sync & Zero-Error RAG Command

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[allow(unused_imports)]
use super::constants::{
    DEFAULT_GDB_LISTEN_SOCKET, FIRMWARE_IMAGES_DIR_NAME, WIN_SCRIPT_GDB, WIN_SCRIPT_GHIDRA,
    WIN_SCRIPT_HXD,
};
use super::error::{IpcResult, PanadigaIpcError};
use super::hardware::run_blocking;

/// Resolves the Panadiga workspace root via `panadiga-core` multiplatform discovery.
#[allow(dead_code)]
fn resolve_workspace_root() -> PathBuf {
    panadiga_core::constants::resolve_workspace_root()
}

/// Regenerates all Ghidra Jython scripts from Rust SSOT and mirrors them into `~/ghidra_scripts`.
#[allow(dead_code)]
fn sync_ghidra_ssot_and_user_scripts(root: &Path) -> Result<(usize, Option<PathBuf>), String> {
    let tools_ghidra = root.join("tools").join("ghidra");
    panadiga_core::ghidra::export_all_ghidra_scripts(&tools_ghidra)
        .map_err(|e| format!("Falha ao exportar scripts Ghidra da SSOT Rust: {}", e))?;

    let home_dir = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    let user_scripts_dir = home_dir.map(|h| h.join("ghidra_scripts"));
    let mut synced = 0usize;

    if let Some(ref dest_dir) = user_scripts_dir {
        std::fs::create_dir_all(dest_dir)
            .map_err(|e| format!("Falha ao criar {}: {}", dest_dir.display(), e))?;
        if let Ok(entries) = std::fs::read_dir(&tools_ghidra) {
            for entry in entries.flatten() {
                let src_path = entry.path();
                if src_path.extension().and_then(|s| s.to_str()) == Some("py")
                    && let Some(fname) = src_path.file_name()
                {
                    std::fs::copy(&src_path, dest_dir.join(fname)).map_err(|e| {
                        format!("Falha ao sincronizar {}: {}", fname.to_string_lossy(), e)
                    })?;
                    synced += 1;
                }
            }
        }
    }

    Ok((synced, user_scripts_dir))
}

#[cfg(target_os = "windows")]
struct GhidraProjectLocation {
    dir: PathBuf,
    name: String,
    gpr: PathBuf,
}

#[cfg(target_os = "windows")]
struct GhidraInstallDiscovery {
    run_bat: Option<PathBuf>,
    headless_bat: Option<PathBuf>,
    project: Option<GhidraProjectLocation>,
}

#[cfg(target_os = "windows")]
fn discover_ghidra_install_and_project() -> GhidraInstallDiscovery {
    let mut home_candidates: Vec<PathBuf> = Vec::new();
    if let Ok(env_home) = std::env::var("GHIDRA_HOME") {
        let trimmed = env_home.trim();
        if !trimmed.is_empty() {
            home_candidates.push(PathBuf::from(trimmed));
        }
    }
    for base in [
        PathBuf::from(r"C:\ghidra"),
        PathBuf::from(r"C:\tools\ghidra"),
        PathBuf::from(r"D:\ghidra"),
        PathBuf::from(r"C:\Program Files\ghidra"),
    ] {
        if let Ok(entries) = std::fs::read_dir(&base) {
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
    if let Some(user_profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        let user_ghidra = user_profile.join("ghidra");
        if let Ok(entries) = std::fs::read_dir(&user_ghidra) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
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

/// Spawns a fully detached child process cleanly across Desktop OSes.
#[allow(dead_code)]
fn spawn_detached(program: &str, args: &[&str], cwd: &Path) -> Result<(), std::io::Error> {
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
fn launch_tools_script(root: &Path, script: &str) -> Result<(), std::io::Error> {
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
fn spawn_first_terminal(root: &Path, shell_cmd: Option<&str>) -> Result<(), std::io::Error> {
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
pub fn launch_external_tool_command(tool: String) -> IpcResult<String> {
    #[cfg(target_os = "android")]
    {
        let _ = tool;
        Err(PanadigaIpcError::classify(
            "Ferramentas externas de bancada (GDB CLI, Ghidra, Terminal) são recursos exclusivos do host Desktop; utilize os painéis nativos In-App no Android.",
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let root = resolve_workspace_root();
        match tool.to_lowercase().as_str() {
            "gdb" => {
                launch_tools_script(&root, WIN_SCRIPT_GDB)
                    .map_err(|e| format!("Falha ao iniciar GDB Multiarch: {}", e))?;
                Ok(format!(
                    "Sessão interativa do GDB iniciada em {}.",
                    DEFAULT_GDB_LISTEN_SOCKET
                ))
            }
            "ghidra_scripts" | "ghidra" => {
                let (synced, _) = sync_ghidra_ssot_and_user_scripts(&root)?;
                let discovery = discover_ghidra_install_and_project();
                if let Some(run_bat) = discovery.run_bat {
                    let run_str = run_bat.to_string_lossy().to_string();
                    let cwd = run_bat.parent().unwrap_or(&root);
                    if let Some(project) = discovery.project {
                        let gpr_str = project.gpr.to_string_lossy().to_string();
                        spawn_detached("cmd", &["/c", "call", &run_str, &gpr_str], cwd)
                            .map_err(|e| format!("Falha ao iniciar Ghidra GUI: {}", e))?;
                        return Ok(format!(
                            "Ghidra aberto diretamente no projeto {} ({} scripts Jython SSOT sincronizados em ~/ghidra_scripts).",
                            project.gpr.display(),
                            synced
                        ));
                    }
                    spawn_detached("cmd", &["/c", "call", &run_str], cwd)
                        .map_err(|e| format!("Falha ao iniciar Ghidra GUI: {}", e))?;
                    return Ok(format!(
                        "Ghidra iniciado sem janelas CMD extras ({} scripts Jython SSOT sincronizados em ~/ghidra_scripts).",
                        synced
                    ));
                }
                launch_tools_script(&root, WIN_SCRIPT_GHIDRA)
                    .map_err(|e| format!("Falha ao iniciar Ghidra: {}", e))?;
                Ok(format!(
                    "Inicializador do Ghidra disparado ({} scripts Jython sincronizados).",
                    synced
                ))
            }
            "ghidra_sync" => {
                let (synced, _) = sync_ghidra_ssot_and_user_scripts(&root)?;
                let discovery = discover_ghidra_install_and_project();
                let headless = discovery.headless_bat.ok_or_else(|| {
                    "analyzeHeadless.bat do Ghidra não encontrado (defina GHIDRA_HOME ou instale em C:\\ghidra).".to_string()
                })?;
                let project = discovery.project.ok_or_else(|| {
                    "Projeto Panadiga_MN103S.gpr não encontrado em C:\\ghidra\\projects."
                        .to_string()
                })?;
                let proj_dir = project.dir;
                let proj_name = project.name;
                let scripts_path = root.join("tools").join("ghidra");
                let log_path = proj_dir.join("panadiga_jython_sync.log");
                let batch_line = format!(
                    "\"{}\" \"{}\" \"{}\" -process -noanalysis -scriptPath \"{}\" -postScript ghidra_am34_memory_map.py -postScript ghidra_import_symbols_panadiga.py -postScript ghidra_mn103_fix_decompilation.py -postScript ghidra_auto_unbrick_audit.py > \"{}\" 2>&1",
                    headless.display(),
                    proj_dir.display(),
                    proj_name,
                    scripts_path.display(),
                    log_path.display(),
                );
                spawn_detached("cmd", &["/c", &batch_line], &root)
                    .map_err(|e| format!("Falha ao executar analyzeHeadless Jython: {}", e))?;
                Ok(format!(
                    "Sync Headless Jython iniciado em segundo plano ({} scripts sincronizados; log em {}).",
                    synced,
                    log_path.display()
                ))
            }
            "ghidra_mcp" => {
                let gui_bridge_cmd = Path::new(r"C:\ghidra\start_ghidra_mcp_gui_bridge.cmd");
                let mcp_exe = Path::new(r"C:\ghidra\.venv\Scripts\ghidra-mcp.exe");
                if mcp_exe.is_file() {
                    spawn_detached(
                        &mcp_exe.to_string_lossy(),
                        &[
                            "--ghidra-server",
                            "http://127.0.0.1:8080/",
                            "--transport",
                            "streamable-http",
                            "--mcp-host",
                            "127.0.0.1",
                            "--mcp-port",
                            "8081",
                        ],
                        Path::new(r"C:\ghidra"),
                    )
                    .map_err(|e| format!("Falha ao iniciar ghidra-mcp.exe: {}", e))?;
                    return Ok(
                        "Ponte GhidraMCP ativa em http://127.0.0.1:8081 (aguardando servidor Ghidra UI :8080)."
                            .to_string(),
                    );
                }
                if gui_bridge_cmd.is_file() {
                    spawn_detached(
                        "cmd",
                        &["/c", "call", &gui_bridge_cmd.to_string_lossy()],
                        Path::new(r"C:\ghidra"),
                    )
                    .map_err(|e| format!("Falha ao iniciar bridge Ghidra MCP: {}", e))?;
                    return Ok("Ponte GhidraMCP iniciada em http://127.0.0.1:8081.".to_string());
                }
                Err(PanadigaIpcError::classify(
                    "ghidra-mcp não encontrado em C:\\ghidra\\.venv\\Scripts\\ghidra-mcp.exe.",
                ))
            }
            "hxd" => {
                launch_tools_script(&root, WIN_SCRIPT_HXD)
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
            other => Err(PanadigaIpcError::classify(format!(
                "Ferramenta externa '{}' não suportada. Opções: gdb, ghidra, ghidra_sync, ghidra_mcp, hxd, terminal",
                other
            ))),
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
                            "tell application \"Terminal\" to do script \"cd '{}' && gdb-multiarch -ex 'set architecture mn10300' -ex 'target remote {}'\"",
                            root_str, DEFAULT_GDB_LISTEN_SOCKET
                        ),
                    ],
                    &root,
                )
                .map_err(|e| format!("Falha ao iniciar Terminal com GDB: {}", e))?;
                Ok(format!(
                    "Terminal macOS iniciado conectando em {}.",
                    DEFAULT_GDB_LISTEN_SOCKET
                ))
            }
            "ghidra_scripts" | "ghidra" | "ghidra_sync" => {
                let (synced, _) = sync_ghidra_ssot_and_user_scripts(&root)?;
                let ghidra_dir = root.join("tools").join("ghidra");
                spawn_detached("open", &[&ghidra_dir.to_string_lossy()], &root)
                    .map_err(|e| format!("Falha ao abrir scripts do Ghidra: {}", e))?;
                Ok(format!(
                    "Scripts Jython SSOT sincronizados ({} arquivos) e diretório aberto no Finder.",
                    synced
                ))
            }
            "ghidra_mcp" => Err(PanadigaIpcError::classify(
                "No macOS, inicie o bridge ghidra-mcp no ambiente Python local apontando para :8081.",
            )),
            "hxd" => {
                let images_dir = root.join(FIRMWARE_IMAGES_DIR_NAME);
                spawn_detached("open", &[&images_dir.to_string_lossy()], &root)
                    .map_err(|e| format!("Falha ao abrir pasta de imagens: {}", e))?;
                Ok("Diretório de imagens binárias aberto no Finder.".to_string())
            }
            "terminal" => {
                spawn_detached("open", &["-a", "Terminal", &root_str], &root)
                    .map_err(|e| format!("Falha ao abrir Terminal macOS: {}", e))?;
                Ok("Terminal macOS iniciado no workspace.".to_string())
            }
            other => Err(PanadigaIpcError::classify(format!(
                "Ferramenta externa '{}' não suportada. Opções: gdb, ghidra, ghidra_sync, ghidra_mcp, hxd, terminal",
                other
            ))),
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
                let cmd = format!(
                    "gdb-multiarch -ex 'set architecture mn10300' -ex 'target remote {}'; exec bash",
                    DEFAULT_GDB_LISTEN_SOCKET
                );
                spawn_first_terminal(&root, Some(&cmd))
                    .map_err(|e| format!("Falha ao iniciar GDB no terminal Linux: {}", e))?;
                Ok("Sessão GDB Multiarch iniciada no terminal.".to_string())
            }
            "ghidra_scripts" | "ghidra" | "ghidra_sync" => {
                let (synced, _) = sync_ghidra_ssot_and_user_scripts(&root)?;
                let ghidra_res = spawn_detached("ghidraRun", &[], &root).or_else(|_| {
                    spawn_detached(
                        "xdg-open",
                        &[&root.join("tools").join("ghidra").to_string_lossy()],
                        &root,
                    )
                });
                ghidra_res.map_err(|e| format!("Falha ao iniciar Ghidra no Linux: {}", e))?;
                Ok(format!(
                    "Ghidra / scripts Jython SSOT ({} sincronizados em ~/ghidra_scripts) abertos.",
                    synced
                ))
            }
            "ghidra_mcp" => Err(PanadigaIpcError::classify(
                "Inicie o servidor ghidra-mcp no ambiente Linux apontando para http://127.0.0.1:8081.",
            )),
            "hxd" => {
                spawn_detached(
                    "xdg-open",
                    &[&root.join(FIRMWARE_IMAGES_DIR_NAME).to_string_lossy()],
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
            other => Err(PanadigaIpcError::classify(format!(
                "Ferramenta externa '{}' não suportada. Opções: gdb, ghidra, ghidra_sync, ghidra_mcp, hxd, terminal",
                other
            ))),
        }
    }
}

/// Queries the deterministic 6-layer Zero-Error RAG Engine natively in-process.
#[tauri::command]
pub async fn query_rag_engine(
    query: String,
    _top_k: Option<usize>,
) -> IpcResult<panadiga_rag::models::RagQueryResultDto> {
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
