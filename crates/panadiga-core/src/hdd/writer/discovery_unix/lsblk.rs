//! Linux `lsblk` JSON parsing and `/sys/block` fallback enumeration helpers.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use serde_json::Value;

#[cfg(target_os = "linux")]
use std::{fs, path::Path};

use super::super::types::{
    MountedPartitionInfo, PhysicalDriveInfo, classify_drive_role, json_bool_lossy,
    json_str_trimmed, json_u64_lossy,
};
use super::is_critical_os_mount;
#[cfg(target_os = "linux")]
use super::{parent_block_name_from_dev, parse_proc_mounts_entries};

fn collect_lsblk_mountpoints(node: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = node.get("mountpoints").and_then(Value::as_array) {
        for mp in arr.iter().filter_map(Value::as_str).map(str::trim) {
            if !mp.is_empty() && !out.contains(&mp.to_string()) {
                out.push(mp.to_string());
            }
        }
    }
    if let Some(mp) = node
        .get("mountpoint")
        .and_then(Value::as_str)
        .map(str::trim)
        && !mp.is_empty()
        && !out.contains(&mp.to_string())
    {
        out.push(mp.to_string());
    }
    out
}

fn collect_lsblk_children(
    children: &[Value],
    mount_points: &mut Vec<String>,
    partitions: &mut Vec<MountedPartitionInfo>,
) {
    for child in children {
        let child_mps = collect_lsblk_mountpoints(child);
        for mp in &child_mps {
            if !mount_points.contains(mp) {
                mount_points.push(mp.clone());
            }
        }
        let dev_path = json_str_trimmed(child, "path");
        let name = json_str_trimmed(child, "kname");
        let resolved_dev = if !dev_path.is_empty() {
            dev_path
        } else if !name.is_empty() {
            format!("/dev/{name}")
        } else {
            String::new()
        };
        if !resolved_dev.is_empty() {
            partitions.push(MountedPartitionInfo {
                device_path: resolved_dev,
                label: json_str_trimmed(child, "label"),
                fs_type: json_str_trimmed(child, "fstype"),
                mount_point: child_mps.into_iter().next().unwrap_or_default(),
                size_bytes: json_u64_lossy(child.get("size")),
            });
        }
        if let Some(nested) = child.get("children").and_then(Value::as_array) {
            collect_lsblk_children(nested, mount_points, partitions);
        }
    }
}

fn parse_lsblk_device_entry(
    dev: &Value,
    extra_os_root_disks: &[String],
) -> Option<PhysicalDriveInfo> {
    let dev_type = json_str_trimmed(dev, "type").to_ascii_lowercase();
    let name = json_str_trimmed(dev, "name");
    let kname = json_str_trimmed(dev, "kname");
    let raw_path = json_str_trimmed(dev, "path");
    let device_path = if !raw_path.is_empty() {
        raw_path
    } else if !name.is_empty() {
        if name.starts_with('/') {
            name
        } else {
            format!("/dev/{name}")
        }
    } else {
        return None;
    };
    let is_virtual = matches!(dev_type.as_str(), "zram" | "ram" | "rom")
        || device_path.starts_with("/dev/zram")
        || device_path.starts_with("/dev/loop");
    let mut mount_points = collect_lsblk_mountpoints(dev);
    let mut partitions = Vec::new();
    if let Some(children) = dev.get("children").and_then(Value::as_array) {
        collect_lsblk_children(children, &mut mount_points, &mut partitions);
    }
    if is_virtual && mount_points.is_empty() {
        return None;
    }
    let size_bytes = json_u64_lossy(dev.get("size"));
    if size_bytes == 0 && partitions.is_empty() {
        return None;
    }
    let size_gb = size_bytes as f64 / 1_000_000_000.0;
    let vendor = json_str_trimmed(dev, "vendor");
    let model = json_str_trimmed(dev, "model");
    let full_model = match (vendor.is_empty(), model.is_empty()) {
        (false, false)
            if !model
                .to_ascii_lowercase()
                .starts_with(&vendor.to_ascii_lowercase()) =>
        {
            format!("{vendor} {model}")
        }
        (_, false) => model,
        (false, true) => vendor,
        (true, true) => format!("Linux Block Device ({device_path})"),
    };
    let tran = json_str_trimmed(dev, "tran");
    let bus_type = if !tran.is_empty() {
        tran.to_ascii_uppercase()
    } else if kname.starts_with("nvme") {
        "NVMe".to_string()
    } else if kname.starts_with("mmcblk") {
        "MMC/SD".to_string()
    } else {
        "DISK".to_string()
    };
    let removable = json_bool_lossy(dev.get("rm"));
    let is_system_drive = mount_points.iter().any(|mp| is_critical_os_mount(mp))
        || extra_os_root_disks.iter().any(|d| d == &device_path);
    let drive_role = classify_drive_role(
        &device_path,
        &bus_type,
        &full_model,
        removable,
        size_bytes,
        is_system_drive,
    );
    Some(PhysicalDriveInfo {
        device_path: device_path.clone(),
        display_name: format!("{full_model} ({bus_type}, {size_gb:.1} GB)"),
        size_bytes,
        size_gb,
        is_system_drive,
        safe_to_write: !is_system_drive,
        lockout_reason: is_system_drive
            .then(|| format!("LOCKED: Linux OS root/system drive ({device_path})")),
        drive_role,
        bus_type,
        removable,
        is_mounted: !mount_points.is_empty(),
        mount_points,
        partitions,
    })
}

/// Parses `lsblk -J -b -p` JSON cleanly, dynamically locking only actual OS root disks.
pub(crate) fn parse_lsblk_blockdevices_json(
    raw_json: &str,
    extra_os_root_disks: &[String],
) -> Vec<PhysicalDriveInfo> {
    let Ok(parsed) = serde_json::from_str::<Value>(raw_json.trim()) else {
        return Vec::new();
    };
    let Some(devs) = parsed.get("blockdevices").and_then(Value::as_array) else {
        return Vec::new();
    };
    devs.iter()
        .filter_map(|dev| parse_lsblk_device_entry(dev, extra_os_root_disks))
        .collect()
}

#[cfg(target_os = "linux")]
fn parse_sys_block_entry(
    entry: &fs::DirEntry,
    mounts: &[(String, String, String)],
    os_disks: &[String],
) -> Option<PhysicalDriveInfo> {
    let name = entry.file_name().to_string_lossy().into_owned();
    if !["sd", "nvme", "mmcblk", "hd", "vd"]
        .iter()
        .any(|p| name.starts_with(p))
    {
        return None;
    }
    let dev_path = format!("/dev/{name}");
    if !Path::new(&dev_path).exists() {
        return None;
    }
    let sectors = fs::read_to_string(entry.path().join("size"))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let size_bytes = sectors * 512;
    if size_bytes == 0 {
        return None;
    }
    let model = fs::read_to_string(entry.path().join("device/model"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let removable = fs::read_to_string(entry.path().join("removable"))
        .map(|s| s.trim() == "1")
        .unwrap_or(false);
    let mut mount_points = Vec::new();
    let mut partitions = Vec::new();
    for (src, mp, fstype) in mounts {
        if src == &dev_path || parent_block_name_from_dev(src) == name {
            if !mount_points.contains(mp) {
                mount_points.push(mp.clone());
            }
            partitions.push(MountedPartitionInfo {
                device_path: src.clone(),
                label: String::new(),
                fs_type: fstype.clone(),
                mount_point: mp.clone(),
                size_bytes: 0,
            });
        }
    }
    let is_system = mount_points.iter().any(|mp| is_critical_os_mount(mp))
        || os_disks.iter().any(|d| d == &dev_path);
    let bus = if name.starts_with("nvme") {
        "NVMe"
    } else if name.starts_with("mmcblk") {
        "MMC/SD"
    } else if removable {
        "USB"
    } else {
        "DISK"
    };
    let label = if model.is_empty() {
        format!("Linux Block Device ({dev_path})")
    } else {
        model
    };
    let drive_role = classify_drive_role(&dev_path, bus, &label, removable, size_bytes, is_system);
    Some(PhysicalDriveInfo {
        device_path: dev_path.clone(),
        display_name: format!("{label} ({bus}, {:.1} GB)", size_bytes as f64 / 1e9),
        size_bytes,
        size_gb: size_bytes as f64 / 1e9,
        is_system_drive: is_system,
        safe_to_write: !is_system,
        lockout_reason: is_system
            .then(|| format!("LOCKED: Linux OS root/system drive ({dev_path})")),
        drive_role,
        bus_type: bus.to_string(),
        removable,
        is_mounted: !mount_points.is_empty(),
        mount_points,
        partitions,
    })
}

#[cfg(target_os = "linux")]
pub(super) fn list_linux_sys_block_fallback(os_disks: &[String]) -> Vec<PhysicalDriveInfo> {
    let mounts = fs::read_to_string("/proc/mounts")
        .map(|txt| parse_proc_mounts_entries(&txt))
        .unwrap_or_default();
    let Ok(entries) = fs::read_dir("/sys/block") else {
        return Vec::new();
    };
    let mut drives: Vec<PhysicalDriveInfo> = entries
        .flatten()
        .filter_map(|entry| parse_sys_block_entry(&entry, &mounts, os_disks))
        .collect();
    drives.sort_by(|a, b| a.device_path.cmp(&b.device_path));
    drives
}
