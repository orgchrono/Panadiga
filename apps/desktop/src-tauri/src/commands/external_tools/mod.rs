//! External Desktop Engineering Tool Launchers, Ghidra SSOT Sync & Zero-Error RAG Command

mod ghidra_discovery;
mod os_launch;
#[cfg(not(target_os = "windows"))]
mod os_launch_unix;

use std::path::PathBuf;

use crate::commands::error::IpcResult;
use crate::commands::hardware::run_blocking;

fn resolve_workspace_root() -> PathBuf {
    panadiga_core::constants::resolve_workspace_root()
}

/// Launches external engineering tools across Windows, macOS, Linux, and returns clean guidance on Android.
#[tauri::command]
pub fn launch_external_tool_command(tool: String) -> IpcResult<String> {
    let root = resolve_workspace_root();
    os_launch::launch_for_current_os(&tool, &root)
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

    #[test]
    fn external_tool_argv_builders_avoid_shell_invocations() {
        let args = os_launch::build_ghidra_sync_args(
            "C:/proj with spaces",
            "Panadiga_MN103S",
            "C:/Panadiga/tools/ghidra",
        );
        assert!(
            args.iter().all(|a| !a.eq_ignore_ascii_case("cmd")
                && !a.contains('>')
                && !a.contains("sh -lc"))
        );
        assert_eq!(args[0], "C:/proj with spaces");
        assert_eq!(args[2], "-process");
        assert_eq!(args[4], "-scriptPath");
    }
}
