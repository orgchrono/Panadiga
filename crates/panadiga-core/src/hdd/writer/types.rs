//! Pure domain types and role classifier for multi-platform HDD and SD Card discovery.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) fn json_u64_lossy(val: Option<&Value>) -> u64 {
    match val {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(Value::String(s)) => s.trim().parse::<u64>().unwrap_or(0),
        _ => 0,
    }
}

pub(crate) fn json_bool_lossy(val: Option<&Value>) -> bool {
    match val {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0) != 0,
        Some(Value::String(s)) => matches!(s.trim().to_ascii_lowercase().as_str(), "1" | "true"),
        _ => false,
    }
}

pub(crate) fn json_str_trimmed(entry: &Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

/// Functional media role distinguishing Hard Disks, SD Cards, and locked Host OS drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMediaRole {
    SystemOs,
    HardDisk,
    SdCard,
    Other,
}

/// Mounted or discovered child partition belonging to a physical drive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountedPartitionInfo {
    pub device_path: String,
    pub label: String,
    pub fs_type: String,
    pub mount_point: String,
    pub size_bytes: u64,
}

/// Information regarding a discovered physical disk drive and its mounted partitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalDriveInfo {
    pub device_path: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub size_gb: f64,
    pub is_system_drive: bool,
    pub safe_to_write: bool,
    pub lockout_reason: Option<String>,
    #[serde(default = "default_drive_role")]
    pub drive_role: StorageMediaRole,
    #[serde(default)]
    pub bus_type: String,
    #[serde(default)]
    pub removable: bool,
    #[serde(default)]
    pub is_mounted: bool,
    #[serde(default)]
    pub mount_points: Vec<String>,
    #[serde(default)]
    pub partitions: Vec<MountedPartitionInfo>,
}

const fn default_drive_role() -> StorageMediaRole {
    StorageMediaRole::HardDisk
}

const SD_CARD_CEILING_BYTES: u64 = 64_000_000_000;

/// Classifies a physical drive by OS lock status, bus protocol, media descriptor, and size.
pub fn classify_drive_role(
    device_path: &str,
    bus_type: &str,
    model: &str,
    removable: bool,
    size_bytes: u64,
    is_system_drive: bool,
) -> StorageMediaRole {
    if is_system_drive {
        return StorageMediaRole::SystemOs;
    }
    let bus_low = bus_type.trim().to_ascii_lowercase();
    let model_low = model.trim().to_ascii_lowercase();
    let path_low = device_path.trim().to_ascii_lowercase();

    let is_mmc_bus = bus_low.contains("mmc")
        || bus_low == "sd"
        || path_low.contains("mmcblk")
        || model_low.contains("sd/mmc")
        || model_low.contains("card reader")
        || model_low.contains("sdxc")
        || model_low.contains("sdhc");

    if is_mmc_bus || (removable && size_bytes > 0 && size_bytes <= SD_CARD_CEILING_BYTES) {
        return StorageMediaRole::SdCard;
    }
    StorageMediaRole::HardDisk
}

/// Filters drives cleanly by functional role (`Some(HardDisk)`, `Some(SdCard)`, or `None` for all).
pub fn filter_drives_by_role(
    drives: Vec<PhysicalDriveInfo>,
    role: Option<StorageMediaRole>,
) -> Vec<PhysicalDriveInfo> {
    match role {
        Some(target) => drives
            .into_iter()
            .filter(|d| d.drive_role == target)
            .collect(),
        None => drives,
    }
}
