//! Default system paths, workspace root discovery, serial, and GDB network constants.

/// Default relative path to the sample Panasonic HDD 100MB dump.
pub const DEFAULT_HDD_IMAGE_PATH: &str =
    "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin";

/// Default relative path to the official Panasonic optical update container.
pub const DEFAULT_FRM_CONTAINER_PATH: &str = "firmware_and_images/PANA_DVD.FRM";

/// Default directory containing Panasonic DMR-EH55 PDF service manuals and schematics.
pub const DEFAULT_MANUALS_DIR: &str = "manuals_and_schematics";

/// Default output directory for exported Ghidra scripts.
pub const DEFAULT_TOOLS_DIR: &str = "tools";

/// Default directory for partition extraction.
pub const DEFAULT_HDD_EXTRACT_DIR: &str = "hdd_extracted";

/// Default serial port for host communication with the Pico controller.
#[cfg(target_os = "windows")]
pub const DEFAULT_SERIAL_PORT: &str = "COM5";
#[cfg(target_os = "macos")]
pub const DEFAULT_SERIAL_PORT: &str = "/dev/cu.usbmodem101";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const DEFAULT_SERIAL_PORT: &str = "/dev/ttyACM0";

/// Resolves the workspace root across CLI, Cargo workspace subcrate tests, Desktop, or Android.
/// Honors `PANADIGA_WORKSPACE_ROOT` first, then probes CWD, ancestors, and executable ancestors.
pub fn resolve_workspace_root() -> std::path::PathBuf {
    if let Ok(env_root) = std::env::var("PANADIGA_WORKSPACE_ROOT") {
        let trimmed = env_root.trim();
        if !trimmed.is_empty() {
            return std::path::PathBuf::from(trimmed);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    for ancestor in cwd.ancestors() {
        if ancestor.join("Cargo.toml").is_file()
            && (ancestor.join("docs").is_dir() || ancestor.join("crates").is_dir())
        {
            return ancestor.to_path_buf();
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            if ancestor.join("docs").is_dir() || ancestor.join("manuals_and_schematics").is_dir() {
                return ancestor.to_path_buf();
            }
        }
    }
    cwd
}

/// Resolves a writable base directory for relative outputs across repo builds and installed packages.
/// Inside a dev repo it uses `resolve_workspace_root()`; when launched from `/usr/bin`, `/`, or
/// read-only `Program Files`, it falls back to user-writable OS application storage.
pub fn resolve_writable_data_root() -> std::path::PathBuf {
    let ws = resolve_workspace_root();
    if ws.join("Cargo.toml").is_file() && ws.join("crates").is_dir() {
        return ws;
    }
    #[cfg(target_os = "windows")]
    if let Some(local) = std::env::var_os("LOCALAPPDATA").filter(|s| !s.is_empty()) {
        return std::path::PathBuf::from(local).join("Panadiga");
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|s| !s.is_empty()) {
            return std::path::PathBuf::from(xdg).join("panadiga");
        }
        if let Some(home) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
            return std::path::PathBuf::from(home).join("Panadiga");
        }
    }
    ws
}

/// Resolves a workspace-relative subdirectory (e.g. `"docs"` or `"manuals_and_schematics"`).
pub fn find_workspace_subdir(rel: &str) -> Option<std::path::PathBuf> {
    let root = resolve_workspace_root().join(rel);
    if root.is_dir() {
        return Some(root);
    }
    for prefix in [".", "..", "../..", "../../.."] {
        let cand = std::path::Path::new(prefix).join(rel);
        if cand.is_dir() {
            return Some(cand);
        }
    }
    None
}

/// Default baud rate for Pico USB CDC communication.
pub const DEFAULT_BAUD_RATE: u32 = 115_200;

/// Default TCP listen host address for the GDB RSP bridge.
pub const DEFAULT_GDB_LISTEN_HOST: &str = "127.0.0.1";

/// Default TCP listen port for the GDB RSP bridge.
pub const DEFAULT_GDB_LISTEN_PORT: u16 = 3333;

/// Default TCP listen address and port for the GDB RSP bridge.
pub const DEFAULT_GDB_LISTEN_SOCKET: &str = "127.0.0.1:3333";

/// Candidate search paths for the raw Panasonic HDD backup image across development directories.
pub const CANDIDATE_HDD_RAW_BACKUP_PATHS: &[&str] = &[
    "firmware_and_images/panasonic_dmr-eh55_hdd_raw_backup.bin",
    "../../firmware_and_images/panasonic_dmr-eh55_hdd_raw_backup.bin",
    "../../../firmware_and_images/panasonic_dmr-eh55_hdd_raw_backup.bin",
];

/// Candidate directory search paths for Panasonic service manuals and schematics.
pub const CANDIDATE_MANUALS_DIRS: &[&str] = &[
    "manuals_and_schematics",
    "../../manuals_and_schematics",
    "../../../manuals_and_schematics",
];
