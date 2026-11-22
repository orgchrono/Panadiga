//! Windows PowerShell CIM / Storage discovery parser and fallback enumeration.

#![cfg_attr(not(windows), allow(dead_code))]

use serde_json::Value;

use crate::constants::{EFI_SYSTEM_PARTITION_GPT_GUID, MICROSOFT_RESERVED_GPT_GUID};

use super::types::{
    MountedPartitionInfo, PhysicalDriveInfo, classify_drive_role, json_bool_lossy,
    json_str_trimmed, json_u64_lossy,
};

#[cfg(windows)]
use std::process::Command;

fn gpt_guid_is_os_protected(raw: &str) -> bool {
    let cleaned = raw
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_ascii_lowercase();
    cleaned == EFI_SYSTEM_PARTITION_GPT_GUID || cleaned == MICROSOFT_RESERVED_GPT_GUID
}

fn entry_has_os_gpt_partition(entry: &Value) -> bool {
    let has_gpt_type_list = match entry.get("GptTypes") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(Value::as_str)
            .any(gpt_guid_is_os_protected),
        Some(Value::String(s)) => gpt_guid_is_os_protected(s),
        _ => false,
    };
    if has_gpt_type_list {
        return true;
    }
    match entry.get("Partitions") {
        Some(Value::Array(arr)) => arr
            .iter()
            .any(|p| gpt_guid_is_os_protected(&json_str_trimmed(p, "GptType"))),
        Some(obj @ Value::Object(_)) => gpt_guid_is_os_protected(&json_str_trimmed(obj, "GptType")),
        _ => false,
    }
}

fn parse_cim_partitions(entry: &Value) -> Vec<MountedPartitionInfo> {
    let mut parts = Vec::new();
    let items: Vec<&Value> = match entry.get("Partitions") {
        Some(Value::Array(arr)) => arr.iter().collect(),
        Some(obj @ Value::Object(_)) => vec![obj],
        _ => return parts,
    };
    for p in items {
        let drive_letter = json_str_trimmed(p, "DriveLetter");
        if drive_letter.is_empty() {
            continue;
        }
        let mount_point = if drive_letter.ends_with('\\') {
            drive_letter.clone()
        } else {
            format!("{drive_letter}\\")
        };
        let dev_id = format!(r"\\.\{}", drive_letter.trim_end_matches(['\\', '/']));
        parts.push(MountedPartitionInfo {
            device_path: dev_id,
            label: json_str_trimmed(p, "VolumeName"),
            fs_type: json_str_trimmed(p, "FileSystem"),
            mount_point,
            size_bytes: json_u64_lossy(p.get("SizeBytes")),
        });
    }
    parts
}

fn build_windows_lockout_reason(
    idx: u32,
    norm_sys: &str,
    hosts_sys_letter: bool,
    is_system_flag: bool,
    is_boot_flag: bool,
    has_os_gpt: bool,
    has_pagefile: bool,
) -> String {
    let mut reasons: Vec<&'static str> = Vec::new();
    if hosts_sys_letter {
        reasons.push("SystemDrive");
    }
    if is_system_flag || is_boot_flag {
        reasons.push("IsSystem/IsBoot");
    }
    if has_os_gpt {
        reasons.push("EFI/MSR GPT");
    }
    if has_pagefile {
        reasons.push("Pagefile");
    }
    if reasons.is_empty() {
        reasons.push("Primary Boot Disk0");
    }
    format!(
        "Host OS protected drive (Disk {idx} / {norm_sys} hard-lockout: {})",
        reasons.join(", ")
    )
}

fn parse_cim_disk_drive_entry_with_sys(
    entry: &Value,
    sys_letter: Option<&str>,
) -> Option<PhysicalDriveInfo> {
    let idx_val = entry.get("Index").or_else(|| entry.get("Number"))?;
    let idx = json_u64_lossy(Some(idx_val)) as u32;
    let model = ["Model", "FriendlyName"]
        .into_iter()
        .map(|k| json_str_trimmed(entry, k))
        .find(|s| !s.is_empty())
        .unwrap_or_else(|| "Physical Storage Drive".to_string());
    let iface = ["InterfaceType", "BusType"]
        .into_iter()
        .map(|k| json_str_trimmed(entry, k))
        .find(|s| !s.is_empty())
        .unwrap_or_else(|| "DISK".to_string());
    let media = json_str_trimmed(entry, "MediaType");
    let size_bytes = json_u64_lossy(entry.get("Size"));
    let size_gb = size_bytes as f64 / 1_000_000_000.0;
    let partitions = parse_cim_partitions(entry);
    let mount_points: Vec<String> = partitions
        .iter()
        .map(|p| p.mount_point.clone())
        .filter(|m| !m.is_empty())
        .collect();

    let norm_sys = sys_letter
        .map(|s| s.trim_end_matches(['\\', '/']).to_ascii_uppercase())
        .unwrap_or_else(|| "C:".to_string());
    let hosts_sys_letter = mount_points.iter().any(|mp| {
        mp.trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(&norm_sys)
    });
    let is_system_flag = json_bool_lossy(entry.get("IsSystem"));
    let is_boot_flag = json_bool_lossy(entry.get("IsBoot"));
    let has_pagefile = json_bool_lossy(entry.get("HasPagefile"));
    let has_os_gpt = entry_has_os_gpt_partition(entry);
    let explicit_boot =
        is_system_flag || is_boot_flag || has_pagefile || has_os_gpt || hosts_sys_letter;
    let is_system_drive = explicit_boot
        || (idx == 0 && entry.get("IsSystem").is_none() && partitions.is_empty())
        || (idx == 0 && norm_sys == "C:" && mount_points.is_empty());

    let removable = iface.eq_ignore_ascii_case("USB")
        || iface.eq_ignore_ascii_case("SD")
        || media.to_ascii_lowercase().contains("removable");
    let dev_path = format!(r"\\.\PhysicalDrive{idx}");
    let drive_role = classify_drive_role(
        &dev_path,
        &iface,
        &model,
        removable,
        size_bytes,
        is_system_drive,
    );

    Some(PhysicalDriveInfo {
        device_path: dev_path,
        display_name: format!("{model} ({iface}, {size_gb:.1} GB)"),
        size_bytes,
        size_gb,
        is_system_drive,
        safe_to_write: !is_system_drive,
        lockout_reason: is_system_drive.then(|| {
            build_windows_lockout_reason(
                idx,
                &norm_sys,
                hosts_sys_letter,
                is_system_flag,
                is_boot_flag,
                has_os_gpt,
                has_pagefile,
            )
        }),
        drive_role,
        bus_type: iface,
        removable,
        is_mounted: !mount_points.is_empty(),
        mount_points,
        partitions,
    })
}

pub(crate) fn parse_cim_disk_drives_json_with_sys(
    raw_json: &str,
    system_drive_letter: Option<&str>,
) -> Vec<PhysicalDriveInfo> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let Ok(parsed) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let mut drives: Vec<PhysicalDriveInfo> = match parsed {
        Value::Array(items) => items
            .iter()
            .filter_map(|e| parse_cim_disk_drive_entry_with_sys(e, system_drive_letter))
            .collect(),
        Value::Object(_) => parse_cim_disk_drive_entry_with_sys(&parsed, system_drive_letter)
            .into_iter()
            .collect(),
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
fn run_powershell_hidden(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let ps = std::env::var("SystemRoot")
        .ok()
        .map(|sr| {
            std::path::PathBuf::from(sr).join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
        })
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "powershell".to_string());

    let output = Command::new(ps)
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(windows)]
pub(crate) fn list_windows_drives() -> Vec<PhysicalDriveInfo> {
    let sys_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    // Primary query joins Get-Disk + Get-Partition + Get-Volume + active Pagefile drive letters
    // so SATA/secondary disks hosting Windows boot, EFI/MSR GPT partitions, SystemDrive, or pagefile
    // are unconditionally protected even outside PhysicalDrive0.
    let storage_script = concat!(
        "$pfLetters = @((Get-CimInstance Win32_PageFileUsage -ErrorAction SilentlyContinue | ForEach-Object { if ($_.Name -and $_.Name.Length -ge 2) { $_.Name.Substring(0,2).ToUpperInvariant() } }) | Sort-Object -Unique);",
        "$disks = @(Get-Disk -ErrorAction Stop | Sort-Object Number);",
        "$out = foreach ($d in $disks) {",
        "  $partsRaw = @(Get-Partition -DiskNumber $d.Number -ErrorAction SilentlyContinue);",
        "  $gptTypes = @($partsRaw | Where-Object { $_.GptType } | ForEach-Object { [string]$_.GptType });",
        "  $parts = @(); $hasPf = $false;",
        "  foreach ($p in $partsRaw) {",
        "    $dl = if ($p.DriveLetter -and [int][char]$p.DriveLetter -ne 0) { \"$($p.DriveLetter):\" } else { '' };",
        "    if ($dl -and ($pfLetters -contains $dl.ToUpperInvariant())) { $hasPf = $true; }",
        "    $vol = if ($dl) { Get-Volume -DriveLetter $p.DriveLetter -ErrorAction SilentlyContinue | Select-Object -First 1 } else { $null };",
        "    $parts += [PSCustomObject]@{ DriveLetter=$dl; VolumeName=if($vol){[string]$vol.FileSystemLabel}else{''}; FileSystem=if($vol){[string]$vol.FileSystem}else{''}; SizeBytes=[uint64]$p.Size; GptType=if($p.GptType){[string]$p.GptType}else{''} }",
        "  }",
        "  [PSCustomObject]@{ Index=$d.Number; Model=$d.FriendlyName; Size=[uint64]$d.Size; InterfaceType=[string]$d.BusType; MediaType=if($d.BusType -eq 'USB'){'Removable Media'}else{'Fixed'}; IsSystem=[bool]$d.IsSystem; IsBoot=[bool]$d.IsBoot; HasPagefile=$hasPf; GptTypes=@($gptTypes); Partitions=@($parts) }",
        "};",
        "@($out) | ConvertTo-Json -Depth 5 -Compress"
    );
    if let Some(stdout) = run_powershell_hidden(storage_script) {
        let parsed = parse_cim_disk_drives_json_with_sys(&stdout, Some(&sys_drive));
        if !parsed.is_empty() {
            return parsed;
        }
    }

    let cim_fallback = concat!(
        "$disks = @(Get-CimInstance Win32_DiskDrive);",
        "$out = foreach ($d in $disks) {",
        "  $parts = @();",
        "  foreach ($p in (Get-CimAssociatedInstance -InputObject $d -ResultClassName Win32_DiskPartition -ErrorAction SilentlyContinue)) {",
        "    foreach ($l in (Get-CimAssociatedInstance -InputObject $p -ResultClassName Win32_LogicalDisk -ErrorAction SilentlyContinue)) {",
        "      $parts += [PSCustomObject]@{ DriveLetter=$l.DeviceID; VolumeName=$l.VolumeName; FileSystem=$l.FileSystem; SizeBytes=$l.Size }",
        "    }",
        "  }",
        "  [PSCustomObject]@{ Index=$d.Index; Model=$d.Model; Size=$d.Size; InterfaceType=$d.InterfaceType; MediaType=$d.MediaType; Partitions=@($parts) }",
        "};",
        "@($out) | ConvertTo-Json -Depth 4 -Compress"
    );
    if let Some(stdout) = run_powershell_hidden(cim_fallback) {
        let parsed = parse_cim_disk_drives_json_with_sys(&stdout, Some(&sys_drive));
        if !parsed.is_empty() {
            return parsed;
        }
    }

    use std::fs::OpenOptions;
    let mut drives = Vec::new();
    for idx in 0..16 {
        let dev_path = format!(r"\\.\PhysicalDrive{idx}");
        if let Ok(file) = OpenOptions::new().read(true).open(&dev_path) {
            let len = file.metadata().map(|m| m.len()).unwrap_or(0);
            let is_system = idx == 0;
            drives.push(PhysicalDriveInfo {
                device_path: dev_path.clone(),
                display_name: format!("Physical Drive {} ({:.1} GB)", idx, len as f64 / 1e9),
                size_bytes: len,
                size_gb: len as f64 / 1e9,
                is_system_drive: is_system,
                safe_to_write: !is_system,
                lockout_reason: is_system.then(|| {
                    "LOCKED: Windows Host Operating System Boot Drive (PhysicalDrive0)".to_string()
                }),
                drive_role: classify_drive_role(&dev_path, "DISK", "", false, len, is_system),
                bus_type: "DISK".to_string(),
                removable: false,
                is_mounted: false,
                mount_points: Vec::new(),
                partitions: Vec::new(),
            });
        }
    }
    drives
}

/// Returns detected Windows host OS / boot / EFI / pagefile disk paths across all indices.
#[cfg(windows)]
pub(crate) fn detect_windows_host_os_disks() -> Option<Vec<String>> {
    let os_disks: Vec<String> = list_windows_drives()
        .into_iter()
        .filter(|d| d.is_system_drive)
        .map(|d| d.device_path)
        .collect();
    (!os_disks.is_empty()).then_some(os_disks)
}

#[cfg(not(windows))]
pub(crate) fn detect_windows_host_os_disks() -> Option<Vec<String>> {
    None
}
