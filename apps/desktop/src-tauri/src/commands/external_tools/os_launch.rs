//! OS-specific detached process launchers (shared helpers + Windows implementation).

#[cfg(not(target_os = "android"))]
use std::path::Path;
#[cfg(not(target_os = "android"))]
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use crate::commands::constants::{
    DEFAULT_GDB_LISTEN_SOCKET, WIN_SCRIPT_GDB, WIN_SCRIPT_GHIDRA, WIN_SCRIPT_HXD,
};
#[cfg(not(target_os = "android"))]
use crate::commands::error::{IpcResult, PanadigaIpcError};

#[cfg(target_os = "windows")]
use super::ghidra_discovery::{
    discover_ghidra_install_and_project, sync_ghidra_ssot_and_user_scripts,
};

#[cfg(not(target_os = "windows"))]
pub(super) use super::os_launch_unix::launch_for_current_os;

#[cfg(not(target_os = "android"))]
pub(super) fn unsupported_tool_err(tool: &str) -> PanadigaIpcError {
    PanadigaIpcError::validation(format!(
        "Ferramenta externa '{}' não suportada. Opções: gdb, ghidra, ghidra_sync, ghidra_mcp, hxd, terminal",
        tool
    ))
}

#[cfg(not(target_os = "android"))]
pub(super) fn spawn_detached(
    program: &str,
    args: &[&str],
    cwd: &Path,
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

#[cfg(target_os = "windows")]
pub(super) fn launch_for_current_os(tool: &str, root: &Path) -> IpcResult<String> {
    match tool.to_lowercase().as_str() {
        "gdb" => {
            launch_tools_script(root, WIN_SCRIPT_GDB).map_err(|e| {
                PanadigaIpcError::serial(format!("Falha ao iniciar GDB Multiarch: {}", e))
            })?;
            Ok(format!(
                "Sessão interativa do GDB iniciada em {}.",
                DEFAULT_GDB_LISTEN_SOCKET
            ))
        }
        "ghidra_scripts" | "ghidra" => {
            let (synced, _) = sync_ghidra_ssot_and_user_scripts(root)?;
            let discovery = discover_ghidra_install_and_project();
            if let Some(run_bat) = discovery.run_bat {
                let run_str = run_bat.to_string_lossy().to_string();
                let cwd = run_bat.parent().unwrap_or(root);
                if let Some(project) = discovery.project {
                    let gpr_str = project.gpr.to_string_lossy().to_string();
                    spawn_detached("cmd", &["/c", "call", &run_str, &gpr_str], cwd).map_err(
                        |e| PanadigaIpcError::io(format!("Falha ao iniciar Ghidra GUI: {}", e)),
                    )?;
                    return Ok(format!(
                        "Ghidra aberto diretamente no projeto {} ({} scripts Jython SSOT sincronizados em ~/ghidra_scripts).",
                        project.gpr.display(),
                        synced
                    ));
                }
                spawn_detached("cmd", &["/c", "call", &run_str], cwd).map_err(|e| {
                    PanadigaIpcError::io(format!("Falha ao iniciar Ghidra GUI: {}", e))
                })?;
                return Ok(format!(
                    "Ghidra iniciado sem janelas CMD extras ({} scripts Jython SSOT sincronizados em ~/ghidra_scripts).",
                    synced
                ));
            }
            launch_tools_script(root, WIN_SCRIPT_GHIDRA)
                .map_err(|e| PanadigaIpcError::io(format!("Falha ao iniciar Ghidra: {}", e)))?;
            Ok(format!(
                "Inicializador do Ghidra disparado ({} scripts Jython sincronizados).",
                synced
            ))
        }
        "ghidra_sync" => {
            let (synced, _) = sync_ghidra_ssot_and_user_scripts(root)?;
            let discovery = discover_ghidra_install_and_project();
            let headless = discovery.headless_bat.ok_or_else(|| {
                PanadigaIpcError::validation(
                    "analyzeHeadless.bat do Ghidra não encontrado (defina GHIDRA_HOME ou instale em C:\\ghidra).",
                )
            })?;
            let project = discovery.project.ok_or_else(|| {
                PanadigaIpcError::validation(
                    "Projeto Panadiga_MN103S.gpr não encontrado em C:\\ghidra\\projects.",
                )
            })?;
            let scripts_path = root.join("tools").join("ghidra");
            let log_path = project.dir.join("panadiga_jython_sync.log");
            let batch_line = format!(
                "\"{}\" \"{}\" \"{}\" -process -noanalysis -scriptPath \"{}\" -postScript ghidra_am34_memory_map.py -postScript ghidra_import_symbols_panadiga.py -postScript ghidra_mn103_fix_decompilation.py -postScript ghidra_auto_unbrick_audit.py > \"{}\" 2>&1",
                headless.display(),
                project.dir.display(),
                project.name,
                scripts_path.display(),
                log_path.display(),
            );
            spawn_detached("cmd", &["/c", &batch_line], root).map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao executar analyzeHeadless Jython: {}", e))
            })?;
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
                .map_err(|e| {
                    PanadigaIpcError::io(format!("Falha ao iniciar ghidra-mcp.exe: {}", e))
                })?;
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
                .map_err(|e| {
                    PanadigaIpcError::io(format!("Falha ao iniciar bridge Ghidra MCP: {}", e))
                })?;
                return Ok("Ponte GhidraMCP iniciada em http://127.0.0.1:8081.".to_string());
            }
            Err(PanadigaIpcError::validation(
                "ghidra-mcp não encontrado em C:\\ghidra\\.venv\\Scripts\\ghidra-mcp.exe.",
            ))
        }
        "hxd" => {
            launch_tools_script(root, WIN_SCRIPT_HXD).map_err(|e| {
                PanadigaIpcError::io(format!("Falha ao abrir editor hexadecimal: {}", e))
            })?;
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
                root,
            )
            .map_err(|e| PanadigaIpcError::io(format!("Falha ao abrir terminal: {}", e)))?;
            Ok("Terminal PowerShell iniciado no workspace.".to_string())
        }
        other => Err(unsupported_tool_err(other)),
    }
}
