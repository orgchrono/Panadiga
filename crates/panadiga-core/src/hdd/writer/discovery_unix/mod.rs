//! Linux & macOS storage enumeration, mountpoint resolution, and safe unmounting.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

mod lsblk;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use anyhow::Context;
#[cfg(any(target_os = "linux", target_os = "macos", test))]
use anyhow::{Result, bail};
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::types::PhysicalDriveInfo;
#[cfg(target_os = "macos")]
use super::types::classify_drive_role;

#[cfg(any(target_os = "linux", test))]
pub(crate) use lsblk::parse_lsblk_blockdevices_json;

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
    lsblk::list_linux_sys_block_fallback(&os_disks)
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

#[cfg(any(target_os = "macos", test))]
pub(crate) fn should_run_macos_diskutil(device_path: &str) -> Result<bool> {
    if super::block_ops::is_unconditional_os_boot_disk(device_path) {
        bail!("Refusing to unmount macOS system OS disk '{device_path}'.");
    }
    let lower = device_path.trim().to_ascii_lowercase();
    Ok(lower.starts_with("/dev/disk") || lower.starts_with("/dev/rdisk"))
}

#[cfg(target_os = "macos")]
pub(crate) fn unmount_unix_partitions(device_path: &str) -> Result<Vec<String>> {
    if !should_run_macos_diskutil(device_path)? {
        return Ok(Vec::new());
    }
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
