//! Tauri IPC commands for physical drive discovery, guarded MEIHDFS writing, and HDD capacity unlocking.

use std::path::Path;

use crate::commands::constants::{
    CANDIDATE_HDD_IMAGE_PATHS, CANDIDATE_MAIN_CAPACITY_PATHS, DEFAULT_HDD_IMAGE_PATH,
};
use crate::commands::dto::resolve_candidate;
use crate::commands::error::{IpcResult, PanadigaIpcError};

/// Discovers connected physical storage drives & mounted volumes (optionally filtered by role).
#[tauri::command]
pub fn list_physical_drives(
    role: Option<String>,
) -> IpcResult<Vec<panadiga_core::hdd::PhysicalDriveInfo>> {
    let parsed_role = role
        .as_deref()
        .and_then(|r| match r.trim().to_ascii_lowercase().as_str() {
            "hdd" | "hard_disk" | "harddisk" => {
                Some(panadiga_core::hdd::StorageMediaRole::HardDisk)
            }
            "sd" | "sd_card" | "sdcard" | "mmc" => {
                Some(panadiga_core::hdd::StorageMediaRole::SdCard)
            }
            "system" | "os" => Some(panadiga_core::hdd::StorageMediaRole::SystemOs),
            _ => None,
        });
    panadiga_core::hdd::list_drives_by_role(parsed_role).map_err(PanadigaIpcError::from)
}

/// Dedicated enumerator discovering Hard Disk Drives (SATA/PATA/NVMe/USB enclosures).
#[tauri::command]
pub fn list_hdd_drives() -> IpcResult<Vec<panadiga_core::hdd::PhysicalDriveInfo>> {
    panadiga_core::hdd::list_hard_disk_drives().map_err(PanadigaIpcError::from)
}

/// Dedicated enumerator discovering SD Cards and removable flash media.
#[tauri::command]
pub fn list_sd_card_drives() -> IpcResult<Vec<panadiga_core::hdd::PhysicalDriveInfo>> {
    panadiga_core::hdd::list_sd_card_drives().map_err(PanadigaIpcError::from)
}

fn require_hdd_write_nonce(
    state: &tauri::State<'_, crate::state::PanadigaBackendState>,
    nonce_id: Option<&str>,
    device_path: &str,
    input_ref: &str,
) -> IpcResult<()> {
    crate::commands::hardware_diag::require_destructive_nonce(
        state.inner(),
        nonce_id,
        panadiga_core::bench::OpKind::NorWrite,
        device_path.trim(),
        input_ref,
    )
}

/// Unmounts non-OS partitions of `device_path` (e.g. via `udisksctl` / `umount` on Linux).
#[tauri::command]
pub fn unmount_physical_drive(
    state: tauri::State<'_, crate::state::PanadigaBackendState>,
    device_path: String,
    nonce_id: Option<String>,
) -> IpcResult<Vec<String>> {
    require_hdd_write_nonce(&state, nonce_id.as_deref(), &device_path, "unmount")?;
    panadiga_core::hdd::unmount_drive_partitions(&device_path).map_err(PanadigaIpcError::from)
}

/// Formats a physical disk with procedural MEIHDFS filesystem structures (mkfs.meihdfs).
#[tauri::command]
pub fn format_hdd_meihdfs(
    state: tauri::State<'_, crate::state::PanadigaBackendState>,
    device_path: String,
    total_sectors: u64,
    confirmation_token: String,
    nonce_id: Option<String>,
) -> IpcResult<panadiga_core::hdd::DriveWriteReport> {
    require_hdd_write_nonce(&state, nonce_id.as_deref(), &device_path, "format_meihdfs")?;
    let layout = panadiga_core::hdd::mkfs::generate_meihdfs_layout(total_sectors, None)?;
    panadiga_core::hdd::write_meihdfs_procedural_to_drive(
        &device_path,
        &layout,
        &confirmation_token,
    )
    .map_err(PanadigaIpcError::from)
}

/// Safely writes an existing raw image file to a verified physical drive.
#[tauri::command]
pub fn write_hdd_image(
    state: tauri::State<'_, crate::state::PanadigaBackendState>,
    device_path: String,
    image_path: Option<String>,
    confirmation_token: String,
    nonce_id: Option<String>,
) -> IpcResult<panadiga_core::hdd::DriveWriteReport> {
    let input_ref = image_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("default_hdd_image");
    require_hdd_write_nonce(&state, nonce_id.as_deref(), &device_path, input_ref)?;
    let img_path = match image_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => crate::commands::frm_pack::resolve_safe_read_path(raw)?,
        None => std::path::PathBuf::from(resolve_candidate(
            CANDIDATE_HDD_IMAGE_PATHS,
            DEFAULT_HDD_IMAGE_PATH,
        )),
    };
    let buf = panadiga_core::hdd::open_image_buffer(&img_path).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Failed reading image file {}: {e}",
            img_path.display()
        ))
    })?;
    panadiga_core::hdd::write_raw_image_to_drive(&device_path, &buf, &confirmation_token)
        .map_err(PanadigaIpcError::from)
}

/// Atomically restores primary MEIHDFS structures from dual redundancy mirrors.
#[tauri::command]
pub fn restore_hdd_mirrors(
    state: tauri::State<'_, crate::state::PanadigaBackendState>,
    device_path: String,
    confirmation_token: String,
    nonce_id: Option<String>,
) -> IpcResult<panadiga_core::hdd::DriveWriteReport> {
    require_hdd_write_nonce(&state, nonce_id.as_deref(), &device_path, "restore_mirrors")?;
    panadiga_core::hdd::restore_meihdfs_from_mirrors(&device_path, &confirmation_token)
        .map_err(PanadigaIpcError::from)
}

fn resolve_default_main_capacity_path(input_path: Option<String>) -> String {
    if let Some(p) = input_path.filter(|s| !s.trim().is_empty()) {
        return p;
    }
    resolve_candidate(
        CANDIDATE_MAIN_CAPACITY_PATHS,
        CANDIDATE_MAIN_CAPACITY_PATHS[0],
    )
}

/// Inspects `src/hdd_cmd.c` (`MaxProductLBA`) HDD capacity ceiling in a MAIN binary, EXTRHEAD Partition 1, or HDD image.
#[tauri::command]
pub fn inspect_hdd_capacity_patch(
    input_path: Option<String>,
) -> IpcResult<panadiga_core::hdd::MainCapacityInspectionReport> {
    let resolved = resolve_default_main_capacity_path(input_path);
    panadiga_core::hdd::inspect_main_hdd_capacity(Path::new(&resolved))
        .map_err(PanadigaIpcError::from)
}

/// Applies a dynamic or fixed GB HDD capacity patch (`src/hdd_cmd.c` + EXTRHEAD repack + optional TBC passthrough).
#[tauri::command]
pub fn apply_hdd_capacity_patch(
    input_path: Option<String>,
    output_path: Option<String>,
    mode: String,
    target_gb: Option<u32>,
    passthrough_mode: Option<String>,
) -> IpcResult<panadiga_core::hdd::MainCapacityPatchReport> {
    let in_str = resolve_default_main_capacity_path(input_path);
    let in_p = Path::new(&in_str);
    let out_p = output_path
        .filter(|s| !s.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            panadiga_core::hdd::default_capacity_patched_output_path(in_p, &mode, target_gb)
        });
    panadiga_core::hdd::patch_main_hdd_capacity_with_passthrough(
        in_p,
        &out_p,
        &mode,
        target_gb,
        passthrough_mode.as_deref(),
    )
    .map_err(PanadigaIpcError::from)
}
