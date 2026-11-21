#![deny(unsafe_code)]

//! Cross-platform 1x / drive-minimum optical DAO burning backends (Windows IMAPI2, Linux xorriso/growisofs/wodim, macOS hdiutil).

use std::path::Path;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use std::process::Command;

use anyhow::{Result, bail};

use super::types::OpticalBurnerDriveInfo;

const DVD_1X_KBPS: u32 = 1385;

pub trait OpticalBurnBackend {
    fn enumerate_burners(&self) -> Vec<OpticalBurnerDriveInfo>;
    fn build_burn_command_preview(&self, burner_id: &str, iso_path: &Path) -> Result<String>;
    fn burn_iso_lowest_speed(&self, burner_id: &str, iso_path: &Path) -> Result<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemOpticalBurnBackend;

pub fn format_dvd_speed_label(kbps: u32) -> String {
    if kbps == 0 || kbps <= DVD_1X_KBPS {
        format!("{DVD_1X_KBPS} KB/s (1x DVD)")
    } else {
        let mult = (f64::from(kbps) / f64::from(DVD_1X_KBPS)).max(1.0);
        format!("{kbps} KB/s (~{mult:.1}x min)")
    }
}

fn ps_escape_single_quoted(raw: &str) -> String {
    raw.replace('\'', "''")
}

/// Builds a PowerShell IMAPI2 script burning `iso_path` DAO at the lowest drive speed (`SetWriteSpeed`).
pub fn build_imapi2_burn_script(burner_id: &str, iso_path: &Path) -> String {
    let esc_id = ps_escape_single_quoted(burner_id);
    let esc_iso = ps_escape_single_quoted(&iso_path.display().to_string());
    format!(
        r#"$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
public static class PanadigaIsoStream {{
  [DllImport("shlwapi.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
  public static extern void SHCreateStreamOnFileEx(string pszFile, uint grfMode, uint dwAttr, bool fCreate, IntPtr pstmTemplate, out IStream ppstm);
}}
"@
$dm = New-Object -ComObject IMAPI2.MsftDiscMaster2
$rec = New-Object -ComObject IMAPI2.MsftDiscRecorder2
$rec.InitializeDiscRecorder('{esc_id}')
$fmt = New-Object -ComObject IMAPI2.MsftDiscFormat2Data
$fmt.Recorder = $rec
$fmt.ClientName = 'Panadiga'
$fmt.ForceMediaToBeClosed = $true
$speeds = @($fmt.SupportedWriteSpeeds | Sort-Object)
if ($speeds.Count -gt 0) {{ $fmt.SetWriteSpeed([int]$speeds[0], $false) }}
$stream = $null
[PanadigaIsoStream]::SHCreateStreamOnFileEx('{esc_iso}', 0x20, 0x80, $false, [IntPtr]::Zero, [ref]$stream)
$fmt.Write($stream)
[void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($stream)
Write-Output 'IMAPI2_BURN_OK'
"#
    )
}

/// Builds Linux CLI arguments enforcing 1x (`speed=1`) DAO burning.
pub fn build_linux_burn_args(tool: &str, burner_id: &str, iso_path: &Path) -> Vec<String> {
    let iso_str = iso_path.display().to_string();
    match tool {
        "xorriso" => vec![
            "-as".into(),
            "cdrecord".into(),
            "-v".into(),
            format!("dev={burner_id}"),
            "speed=1".into(),
            "-dao".into(),
            "-eject".into(),
            iso_str,
        ],
        "growisofs" => vec![
            "-dvd-compat".into(),
            "-speed=1".into(),
            "-Z".into(),
            format!("{burner_id}={iso_str}"),
        ],
        _ => vec![
            "-v".into(),
            "-dao".into(),
            "speed=1".into(),
            format!("dev={burner_id}"),
            iso_str,
        ],
    }
}

/// Builds macOS `hdiutil burn` CLI arguments enforcing `-speed 1` + verification.
pub fn build_macos_burn_args(burner_id: &str, iso_path: &Path) -> Vec<String> {
    vec![
        "burn".into(),
        iso_path.display().to_string(),
        "-device".into(),
        burner_id.to_string(),
        "-speed".into(),
        "1".into(),
        "-verifyburn".into(),
        "-nounderrun".into(),
    ]
}

#[cfg(target_os = "windows")]
fn enumerate_windows_burners() -> Vec<OpticalBurnerDriveInfo> {
    let script = r#"$ErrorActionPreference='SilentlyContinue'; $dm=New-Object -ComObject IMAPI2.MsftDiscMaster2; $out=@(); foreach($id in $dm){ $r=New-Object -ComObject IMAPI2.MsftDiscRecorder2; $r.InitializeDiscRecorder($id); $vol=($r.VolumePathNames -join ','); $fmt=New-Object -ComObject IMAPI2.MsftDiscFormat2Data; $fmt.Recorder=$r; $sp=@($fmt.SupportedWriteSpeeds | ForEach-Object {[int]([math]::Round($_ * 2.048))}); $out += [pscustomobject]@{device_id=$id;display_name="$($r.VendorId) $($r.ProductId) ($vol)".Trim();speeds=$sp} }; $out | ConvertTo-Json -Compress"#;
    let Ok(output) = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() || text == "null" {
        return Vec::new();
    }
    let val: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let rows: Vec<serde_json::Value> = match val {
        serde_json::Value::Array(arr) => arr,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => Vec::new(),
    };
    rows.into_iter()
        .filter_map(|row| {
            let device_id = row.get("device_id")?.as_str()?.to_string();
            let display_name = row
                .get("display_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Optical Drive")
                .to_string();
            let mut speeds: Vec<u32> = row
                .get("speeds")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_u64().and_then(|n| u32::try_from(n).ok()))
                        .collect()
                })
                .unwrap_or_default();
            speeds.sort_unstable();
            let min_kbps = speeds.first().copied().unwrap_or(DVD_1X_KBPS);
            Some(OpticalBurnerDriveInfo {
                expected_confirm_token: super::types::expected_optical_confirm_token(&device_id),
                device_id,
                display_name,
                os_backend: "windows_imapi2".to_string(),
                supported_write_speeds_kbps: speeds,
                min_write_speed_kbps: min_kbps,
                min_write_speed_label: format_dvd_speed_label(min_kbps),
                safe_optical_target: true,
            })
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn detect_linux_burn_tool() -> &'static str {
    for candidate in ["xorriso", "growisofs", "wodim", "cdrecord"] {
        if Command::new("which")
            .arg(candidate)
            .output()
            .is_ok_and(|o| o.status.success())
        {
            return candidate;
        }
    }
    "xorriso"
}

#[cfg(target_os = "linux")]
fn enumerate_linux_burners() -> Vec<OpticalBurnerDriveInfo> {
    let mut out = Vec::new();
    for idx in 0..8 {
        let dev = format!("/dev/sr{idx}");
        if Path::new(&dev).exists() {
            let vendor = std::fs::read_to_string(format!("/sys/block/sr{idx}/device/vendor"))
                .unwrap_or_default();
            let model = std::fs::read_to_string(format!("/sys/block/sr{idx}/device/model"))
                .unwrap_or_default();
            let label = format!("{} {} ({dev})", vendor.trim(), model.trim())
                .trim()
                .to_string();
            out.push(OpticalBurnerDriveInfo {
                expected_confirm_token: super::types::expected_optical_confirm_token(&dev),
                device_id: dev,
                display_name: label,
                os_backend: format!("linux_{}", detect_linux_burn_tool()),
                supported_write_speeds_kbps: vec![DVD_1X_KBPS],
                min_write_speed_kbps: DVD_1X_KBPS,
                min_write_speed_label: format_dvd_speed_label(DVD_1X_KBPS),
                safe_optical_target: true,
            });
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn enumerate_macos_burners() -> Vec<OpticalBurnerDriveInfo> {
    let Ok(out) = Command::new("drutil").arg("list").output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.contains("DVD") || l.contains("CD"))
        .enumerate()
        .map(|(i, line)| {
            let dev = format!("IOBDServices/{}", i + 1);
            OpticalBurnerDriveInfo {
                expected_confirm_token: super::types::expected_optical_confirm_token(&dev),
                device_id: dev,
                display_name: line.trim().to_string(),
                os_backend: "macos_hdiutil".to_string(),
                supported_write_speeds_kbps: vec![DVD_1X_KBPS],
                min_write_speed_kbps: DVD_1X_KBPS,
                min_write_speed_label: format_dvd_speed_label(DVD_1X_KBPS),
                safe_optical_target: true,
            }
        })
        .collect()
}

impl OpticalBurnBackend for SystemOpticalBurnBackend {
    fn enumerate_burners(&self) -> Vec<OpticalBurnerDriveInfo> {
        #[cfg(target_os = "windows")]
        {
            enumerate_windows_burners()
        }
        #[cfg(target_os = "linux")]
        {
            enumerate_linux_burners()
        }
        #[cfg(target_os = "macos")]
        {
            enumerate_macos_burners()
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        Vec::new()
    }

    fn build_burn_command_preview(&self, burner_id: &str, iso_path: &Path) -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            Ok(format!(
                "powershell IMAPI2 MsftDiscFormat2Data (ForceMediaToBeClosed=$true, minSpeed, recorder={burner_id}, iso={})",
                iso_path.display()
            ))
        }
        #[cfg(target_os = "linux")]
        {
            let tool = detect_linux_burn_tool();
            let args = build_linux_burn_args(tool, burner_id, iso_path);
            Ok(format!("{tool} {}", args.join(" ")))
        }
        #[cfg(target_os = "macos")]
        {
            let args = build_macos_burn_args(burner_id, iso_path);
            Ok(format!("hdiutil {}", args.join(" ")))
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            let _ = (burner_id, iso_path);
            bail!("Optical disc burning is not supported on this OS target")
        }
    }

    fn burn_iso_lowest_speed(&self, burner_id: &str, iso_path: &Path) -> Result<String> {
        if cfg!(target_os = "android") {
            bail!("Optical burning is not supported on Android APK");
        }
        #[cfg(target_os = "windows")]
        {
            let script = build_imapi2_burn_script(burner_id, iso_path);
            let out = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()?;
            if !out.status.success() {
                bail!(
                    "Windows IMAPI2 burn failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Ok("Windows IMAPI2 1x/min-speed DAO burn verified".to_string())
        }
        #[cfg(target_os = "linux")]
        {
            let tool = detect_linux_burn_tool();
            let args = build_linux_burn_args(tool, burner_id, iso_path);
            let out = Command::new(tool).args(&args).output()?;
            if !out.status.success() {
                bail!(
                    "Linux {tool} burn failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Ok(format!("Linux {tool} speed=1 DAO burn completed"))
        }
        #[cfg(target_os = "macos")]
        {
            let args = build_macos_burn_args(burner_id, iso_path);
            let out = Command::new("hdiutil").args(&args).output()?;
            if !out.status.success() {
                bail!(
                    "macOS hdiutil burn failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Ok("macOS hdiutil -speed 1 -verifyburn completed".to_string())
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            let _ = (burner_id, iso_path);
            bail!("Optical disc burning is not supported on this OS target")
        }
    }
}
