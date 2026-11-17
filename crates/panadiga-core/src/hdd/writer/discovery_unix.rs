//! Linux & macOS storage enumeration, mountpoint resolution, and safe unmounting.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use serde_json::Value;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use anyhow::{Context, Result, bail};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::{fs, path::Path};

use super::types::{
    MountedPartitionInfo, PhysicalDriveInfo, classify_drive_role, json_bool_lossy,
    json_str_trimmed, json_u64_lossy,
};

const CRITICAL_UNIX_MOUNTS: &[&str] = &["/", "/boot", "/boot/efi", "/usr", "/var", "/home"];

pub(crate) fn parse_proc_mounts_entries(proc_mounts: &str) -> Vec<(String, String, String)> {
    proc_mounts
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let unescape = |s: &str| s.replace(r"\040", " ").replace(r"\011", "\t");
            let src = unescape(parts.next()?);
            let target = unescape(parts.next()?);
            let fstype = parts.next().unwrap_or("").to_string();
            Some((src, target, fstype))
        })
        .collect()
}

pub(crate) fn parent_block_name_from_dev(dev_or_name: &str) -> String {
    let base = dev_or_name.trim().rsplit('/').next().unwrap_or("");
    for prefix in ["nvme", "mmcblk", "loop", "nbd"] {
        if base.starts_with(prefix)
            && let Some(pos) = base.rfind('p')
        {
            let suffix = &base[pos + 1..];
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                return base[..pos].to_string();
            }
        }
    }
    base.trim_end_matches(|c: char| c.is_ascii_digit())
        .to_string()
}

pub(crate) fn is_critical_os_mount(mount_point: &str) -> bool {
    CRITICAL_UNIX_MOUNTS.contains(&mount_point.trim())
}

pub(crate) fn find_linux_os_root_disks(mounts: &[(String, String, String)]) -> Vec<String> {
    let mut disks = Vec::new();
    for (src, target, _) in mounts {
        if is_critical_os_mount(target) && src.starts_with("/dev/") {
            let parent = parent_block_name_from_dev(src);
            if !parent.is_empty() {
                let full = format!("/dev/{parent}");
                if !disks.contains(&full) {
                    disks.push(full);
                }
            }
        }
    }
    disks
}

pub(crate) fn resolve_mountpoint_device_in_mounts(
    mount_target: &str,
    mounts: &[(String, String, String)],
) -> Option<String> {
    let trimmed = mount_target.trim().trim_end_matches('/');
    let norm = if trimmed.is_empty() { "/" } else { trimmed };
    mounts
        .iter()
        .find(|(src, target, _)| {
            src.starts_with("/dev/")
                && (target.trim_end_matches('/') == norm || (target == "/" && norm == "/"))
        })
        .map(|(src, _, _)| src.clone())
}

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
pub(crate) fn detect_linux_host_os_disks() -> Option<Vec<String>> {
    let mounts = fs::read_to_string("/proc/mounts")
        .map(|txt| parse_proc_mounts_entries(&txt))
        .unwrap_or_default();
    let mut os_disks = find_linux_os_root_disks(&mounts);
    for (src, target, _) in mounts {
        if is_critical_os_mount(&target)
            && src.starts_with("/dev/mapper/")
            && let Ok(canon) = fs::canonicalize(&src)
            && let Some(dm) = canon.file_name().and_then(|n| n.to_str())
            && let Ok(slaves) = fs::read_dir(format!("/sys/block/{dm}/slaves"))
        {
            for s in slaves.flatten() {
                let p = parent_block_name_from_dev(&s.file_name().to_string_lossy());
                let full = format!("/dev/{p}");
                if !os_disks.contains(&full) {
                    os_disks.push(full);
                }
            }
        }
    }
    Some(os_disks)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn detect_linux_host_os_disks() -> Option<Vec<String>> {
    None
}

#[cfg(target_os = "linux")]
pub(crate) fn resolve_mount_point_to_block_device(mount_target: &str) -> Option<String> {
    let mounts = fs::read_to_string("/proc/mounts")
        .map(|txt| parse_proc_mounts_entries(&txt))
        .unwrap_or_default();
    resolve_mountpoint_device_in_mounts(mount_target, &mounts)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn resolve_mount_point_to_block_device(_mount_target: &str) -> Option<String> {
    None
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
fn list_linux_sys_block_fallback(os_disks: &[String]) -> Vec<PhysicalDriveInfo> {
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

#[cfg(target_os = "linux")]
pub(crate) fn list_linux_drives() -> Vec<PhysicalDriveInfo> {
    let os_disks = detect_linux_host_os_disks().unwrap_or_default();
    if let Ok(out) = Command::new("lsblk")
        .args([
            "-J",
            "-b",
            "-p",
            "-o",
            "NAME,KNAME,PATH,MODEL,VENDOR,SIZE,TYPE,TRAN,RM,RO,MOUNTPOINT,MOUNTPOINTS,LABEL,FSTYPE",
        ])
        .output()
        && out.status.success()
    {
        let parsed =
            parse_lsblk_blockdevices_json(&String::from_utf8_lossy(&out.stdout), &os_disks);
        if !parsed.is_empty() {
            return parsed;
        }
    }
    list_linux_sys_block_fallback(&os_disks)
}

#[cfg(target_os = "macos")]
pub(crate) fn list_macos_drives() -> Vec<PhysicalDriveInfo> {
    let mut drives = Vec::new();
    for idx in 0..16 {
        let dev_path = format!("/dev/disk{idx}");
        if std::path::Path::new(&dev_path).exists() {
            let is_system = idx <= 1;
            drives.push(PhysicalDriveInfo {
                device_path: dev_path.clone(),
                display_name: format!("macOS Disk {idx} ({dev_path})"),
                size_bytes: 0,
                size_gb: 0.0,
                is_system_drive: is_system,
                safe_to_write: !is_system,
                lockout_reason: is_system
                    .then(|| format!("LOCKED: macOS System Disk ({dev_path})")),
                drive_role: classify_drive_role(&dev_path, "DISK", "", idx > 1, 0, is_system),
                bus_type: "DISK".to_string(),
                removable: idx > 1,
                is_mounted: false,
                mount_points: Vec::new(),
                partitions: Vec::new(),
            });
        }
    }
    drives
}

#[cfg(target_os = "linux")]
pub(crate) fn unmount_unix_partitions(device_path: &str) -> Result<Vec<String>> {
    let os_disks = detect_linux_host_os_disks().unwrap_or_default();
    if os_disks.iter().any(|d| d == device_path) {
        bail!("Refusing to unmount host OS drive '{}'.", device_path);
    }
    let target_parent = parent_block_name_from_dev(device_path);
    let mounts = fs::read_to_string("/proc/mounts")
        .map(|txt| parse_proc_mounts_entries(&txt))
        .unwrap_or_default();
    let mut unmounted = Vec::new();
    for (src, target, _) in mounts {
        if src == device_path || parent_block_name_from_dev(&src) == target_parent {
            if is_critical_os_mount(&target) {
                bail!("Refusing to unmount critical OS mount '{target}' on '{src}'.");
            }
            let udisks_ok = Command::new("udisksctl")
                .args(["unmount", "-b", &src, "--no-user-interaction"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !udisks_ok {
                let out = Command::new("umount")
                    .arg(&src)
                    .output()
                    .with_context(|| format!("Failed executing umount for {src}"))?;
                if !out.status.success() {
                    bail!(
                        "Failed unmounting '{}' ({}): {}",
                        src,
                        target,
                        String::from_utf8_lossy(&out.stderr).trim()
                    );
                }
            }
            unmounted.push(format!("{src} ({target})"));
        }
    }
    Ok(unmounted)
}

#[cfg(target_os = "macos")]
pub(crate) fn unmount_unix_partitions(device_path: &str) -> Result<Vec<String>> {
    let out = Command::new("diskutil")
        .args(["unmountDisk", device_path])
        .output()
        .with_context(|| format!("Failed running diskutil unmountDisk {device_path}"))?;
    if !out.status.success() {
        bail!(
            "diskutil unmountDisk failed on {}: {}",
            device_path,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(vec![device_path.to_string()])
}
