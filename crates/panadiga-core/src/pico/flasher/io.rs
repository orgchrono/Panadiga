//! Filesystem and USB CDC serial side-effects for Pico BOOTSEL reset and UF2 flashing.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use super::uf2::{
    Uf2TargetFamily, combine_unified_uf2_bytes, extract_uf2_family_slice, validate_uf2_buffer,
};
use super::{EMBEDDED_UNIFIED_UF2, PicoFlashReport};

/// Discovers mounted BOOTSEL mass storage drives across Windows, Linux, and macOS.
pub fn detect_bootsel_drives() -> Vec<PathBuf> {
    let mut bootsel_drives = Vec::new();

    #[cfg(windows)]
    {
        for letter in b'D'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            let path = PathBuf::from(&root);
            if path.exists() {
                let info_file = path.join("INFO_UF2.TXT");
                let index_file = path.join("INDEX.HTM");
                if info_file.exists() || index_file.exists() {
                    bootsel_drives.push(path);
                }
            }
        }
    }

    #[cfg(unix)]
    {
        let mut candidate_roots: Vec<PathBuf> = vec![
            PathBuf::from("/media"),
            PathBuf::from("/run/media"),
            PathBuf::from("/mnt"),
            PathBuf::from("/Volumes"),
        ];
        if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR")
            && !xdg_runtime.trim().is_empty()
        {
            candidate_roots.push(PathBuf::from(xdg_runtime).join("media"));
        }

        let push_if_bootsel = |dir: &Path, list: &mut Vec<PathBuf>| {
            if dir.is_dir()
                && (dir.join("INFO_UF2.TXT").exists() || dir.join("INDEX.HTM").exists())
                && !list.iter().any(|existing| existing == dir)
            {
                list.push(dir.to_path_buf());
            }
        };

        for root_path in candidate_roots {
            push_if_bootsel(&root_path, &mut bootsel_drives);
            if let Ok(entries) = fs::read_dir(&root_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        push_if_bootsel(&p, &mut bootsel_drives);
                        if let Ok(sub_entries) = fs::read_dir(&p) {
                            for sub in sub_entries.flatten() {
                                push_if_bootsel(&sub.path(), &mut bootsel_drives);
                            }
                        }
                    }
                }
            }
        }

        if let Ok(mounts_txt) = fs::read_to_string("/proc/mounts") {
            for line in mounts_txt.lines() {
                if let Some(mp) = line.split_whitespace().nth(1) {
                    let clean_mp = mp.replace(r"\040", " ");
                    push_if_bootsel(Path::new(&clean_mp), &mut bootsel_drives);
                }
            }
        }
    }

    bootsel_drives
}

/// Triggers an automatic reboot into BOOTSEL mode via Panadiga CDC probe or 1200-baud touch.
pub fn trigger_1200_baud_bootsel(port_name: &str) -> Result<()> {
    // Extract the raw port name (e.g. "COM10") from descriptive UI labels like "COM10 [Raspberry Pi Pico...]"
    let clean = port_name.split_whitespace().next().unwrap_or(port_name);

    // ── Phase 1: Direct CDC BOOTSEL/RESET command probe (fastest path if supported) ──
    if let Ok(mut port) = serialport::new(clean, 115_200)
        .timeout(Duration::from_millis(500))
        .open()
    {
        let _ = port.write_data_terminal_ready(true);
        let _ = port.write_request_to_send(true);
        thread::sleep(Duration::from_millis(60));
        let _ = port.write_all(b"\r\nBOOTSEL\r\nRESET\r\n");
        let _ = port.flush();
        thread::sleep(Duration::from_millis(100));
        drop(port);
        thread::sleep(Duration::from_millis(1_200));
        if !detect_bootsel_drives().is_empty() {
            return Ok(());
        }
    }

    // ── Phase 2: Classic 1200-baud touch (official Pico SDK / Arduino standard) ────
    let mut last_err: Option<String> = None;
    for attempt in 0u8..2 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(300));
        }

        match serialport::new(clean, 1_200)
            .timeout(Duration::from_millis(500))
            .open()
        {
            Ok(mut port) => {
                let _ = port.write_data_terminal_ready(false);
                let _ = port.write_request_to_send(false);
                thread::sleep(Duration::from_millis(100));
                drop(port);
            }
            Err(e) => {
                last_err = Some(format!("attempt {}/2: {} — {}", attempt + 1, clean, e));
            }
        }

        thread::sleep(Duration::from_millis(1_500));
        if !detect_bootsel_drives().is_empty() {
            return Ok(());
        }
    }

    bail!(
        "Failed to initiate 1200-baud BOOTSEL touch after 2 attempts on port {} ({}). \
         If the drive is not visible, hold the BOOTSEL button while plugging the USB cable.",
        clean,
        last_err.unwrap_or_default()
    )
}

/// Flashes a validated UF2 image buffer directly onto a detected BOOTSEL drive.
pub fn flash_uf2_to_drive(drive_path: &Path, uf2_bytes: &[u8]) -> Result<PicoFlashReport> {
    let (blocks_count, family_name) = validate_uf2_buffer(uf2_bytes)?;

    let target_file = drive_path.join("panadiga_p7501_uart.uf2");
    fs::write(&target_file, uf2_bytes).with_context(|| {
        format!(
            "Failed to write UF2 firmware to destination drive at {}",
            target_file.display()
        )
    })?;

    let (_, hash_hex) = crate::nvram::calculate_hashes(uf2_bytes);

    Ok(PicoFlashReport {
        target_drive: drive_path.to_string_lossy().to_string(),
        uf2_size_bytes: uf2_bytes.len(),
        blocks_written: blocks_count,
        board_family: family_name,
        sha256_hash: hash_hex,
        success: true,
        message: format!(
            "Successfully flashed {} blocks to {}. Pico will reboot into P7501 Async UART mode.",
            blocks_count,
            drive_path.display()
        ),
    })
}

/// Automatically flashes the embedded Panadiga Pico firmware to any connected BOOTSEL drive.
/// Inspects INFO_UF2.TXT to auto-detect whether the target chip is RP2040 or RP2350.
pub fn flash_embedded_firmware(target_override: Option<&str>) -> Result<PicoFlashReport> {
    let drives = detect_bootsel_drives();
    if drives.is_empty() {
        bail!(
            "No Raspberry Pi BOOTSEL drives (RPI-RP2 / RP2350) detected! Hold BOOTSEL button while plugging USB."
        );
    }
    let target_drive = &drives[0];

    let info_txt_path = target_drive.join("INFO_UF2.TXT");
    let info_content = fs::read_to_string(&info_txt_path).unwrap_or_default();

    if let Some(family) = Uf2TargetFamily::detect(&info_content, target_override) {
        let sliced = extract_uf2_family_slice(EMBEDDED_UNIFIED_UF2, family)?;
        flash_uf2_to_drive(target_drive, &sliced)
    } else {
        flash_uf2_to_drive(target_drive, EMBEDDED_UNIFIED_UF2)
    }
}

/// Merges compiled RP2040 and RP2350 `.uf2` images into a single RP2350-E10 compliant universal UF2.
pub fn merge_unified_uf2(
    rp2040_path: &Path,
    rp2350_path: &Path,
    out_path: &Path,
) -> Result<(usize, String)> {
    let rp2040_bytes = fs::read(rp2040_path)
        .with_context(|| format!("Failed reading RP2040 UF2 '{}'", rp2040_path.display()))?;
    let rp2350_bytes = fs::read(rp2350_path)
        .with_context(|| format!("Failed reading RP2350 UF2 '{}'", rp2350_path.display()))?;

    let (combined, total_blocks, sha256_hex) = combine_unified_uf2_bytes(
        &rp2040_bytes,
        &rp2040_path.display().to_string(),
        &rp2350_bytes,
        &rp2350_path.display().to_string(),
    )?;

    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, &combined)
        .with_context(|| format!("Failed writing unified UF2 '{}'", out_path.display()))?;

    Ok((total_blocks, sha256_hex))
}
