//! Physical storage drive enumeration with host OS boot drive detection.

#[cfg(windows)]
use std::fs::OpenOptions;

/// Information regarding a discovered physical disk drive.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicalDriveInfo {
    pub device_path: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub size_gb: f64,
    pub is_system_drive: bool,
    pub safe_to_write: bool,
    pub lockout_reason: Option<String>,
}

#[cfg_attr(not(windows), allow(dead_code))]
fn parse_cim_disk_drive_entry(entry: &serde_json::Value) -> Option<PhysicalDriveInfo> {
    let idx = entry.get("Index")?.as_u64()? as u32;
    let model = entry
        .get("Model")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Physical Storage Drive");
    let iface = entry
        .get("InterfaceType")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("DISK");
    let size_bytes = entry.get("Size").and_then(|v| v.as_u64()).unwrap_or(0);
    let size_gb = size_bytes as f64 / 1_000_000_000.0;
    let is_system_drive = idx == 0;
    Some(PhysicalDriveInfo {
        device_path: format!(r"\\.\PhysicalDrive{idx}"),
        display_name: format!("{model} ({iface}, {size_gb:.1} GB)"),
        size_bytes,
        size_gb,
        is_system_drive,
        safe_to_write: !is_system_drive,
        lockout_reason: is_system_drive
            .then(|| "System OS primary boot drive (Disk 0 hard-lockout)".to_string()),
    })
}

/// Parses PowerShell `Get-CimInstance Win32_DiskDrive | ConvertTo-Json` (object or array).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn parse_cim_disk_drives_json(raw_json: &str) -> Vec<PhysicalDriveInfo> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return Vec::new();
    };
    let mut drives: Vec<PhysicalDriveInfo> = match parsed {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(parse_cim_disk_drive_entry)
            .collect(),
        serde_json::Value::Object(_) => parse_cim_disk_drive_entry(&parsed).into_iter().collect(),
        _ => Vec::new(),
    };
    drives.sort_by_key(|d| {
        d.device_path
            .strip_prefix(r"\\.\PhysicalDrive")
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(u32::MAX)
    });
    drives
}

#[cfg(windows)]
fn list_windows_cim_drives() -> Vec<PhysicalDriveInfo> {
    let Ok(output) = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_DiskDrive | Select-Object Index,Model,Size,InterfaceType | ConvertTo-Json -Compress",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_cim_disk_drives_json(&stdout)
}

/// Discovers connected physical storage drives and evaluates system safety locks.
pub fn list_physical_drives() -> anyhow::Result<Vec<PhysicalDriveInfo>> {
    #[cfg(windows)]
    {
        let cim_drives = list_windows_cim_drives();
        if !cim_drives.is_empty() {
            return Ok(cim_drives);
        }
    }

    let mut drives = Vec::new();

    #[cfg(windows)]
    {
        for idx in 0..16 {
            let dev_path = format!(r"\\.\PhysicalDrive{}", idx);
            if let Ok(file) = OpenOptions::new().read(true).open(&dev_path) {
                let metadata = file.metadata().ok();
                let len = metadata.map(|m| m.len()).unwrap_or(0);
                let is_system = idx == 0;
                drives.push(PhysicalDriveInfo {
                    device_path: dev_path.clone(),
                    display_name: format!(
                        "Physical Drive {} ({:.1} GB)",
                        idx,
                        len as f64 / 1_000_000_000.0
                    ),
                    size_bytes: len,
                    size_gb: len as f64 / 1_000_000_000.0,
                    is_system_drive: is_system,
                    safe_to_write: !is_system,
                    lockout_reason: is_system.then(|| {
                        "LOCKED: Windows Host Operating System Boot Drive (PhysicalDrive0)"
                            .to_string()
                    }),
                });
            }
        }
    }

    #[cfg(unix)]
    {
        for letter in b'a'..=b'z' {
            let dev_path = format!("/dev/sd{}", letter as char);
            if std::path::Path::new(&dev_path).exists() {
                let is_system = letter == b'a';
                drives.push(PhysicalDriveInfo {
                    device_path: dev_path.clone(),
                    display_name: format!(
                        "SCSI Disk {} (/dev/sd{})",
                        letter as char, letter as char
                    ),
                    size_bytes: 0,
                    size_gb: 0.0,
                    is_system_drive: is_system,
                    safe_to_write: !is_system,
                    lockout_reason: is_system
                        .then(|| "LOCKED: Host Root Filesystem / OS Drive (/dev/sda)".to_string()),
                });
            }
        }
    }

    Ok(drives)
}
