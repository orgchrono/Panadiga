//! Panasonic DMR-EH55 DIGA Suite - Workspace Overview & Binary Hex Inspector Commands
//!
//! Single Responsibility: Tauri IPC command dispatchers for `get_system_overview`
//! and `read_hex_dump`.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::constants::{
    BYTES_PER_MIB_F64, CANDIDATE_FRM_CONTAINER_PATHS, CANDIDATE_HDD_IMAGE_PATHS,
    DEFAULT_FRM_CONTAINER_PATH, DEFAULT_HDD_IMAGE_PATH, DEFAULT_HEX_DUMP_ROWS,
    FLASH_SIZE_2MB_BYTES, FLASH_SIZE_4MB_BYTES, FLASH_SIZE_8MB_BYTES, FLASH_SIZE_16MB_BYTES,
    HEX_DUMP_HALF_ROW_BYTES, HEX_DUMP_ROW_BYTES, MAX_HEX_DUMP_ROWS, OVERVIEW_CORE_ARCH,
    OVERVIEW_DEBUG_PORT, OVERVIEW_ERROR_CODE, OVERVIEW_ERROR_DESC, OVERVIEW_FW_VERSION,
    OVERVIEW_HDD_SPECS, OVERVIEW_OPTICAL_DRIVE, OVERVIEW_OS_SYSTEM,
};
use super::dto::{
    BinaryFileStatusDto, HexDumpRowDto, SystemOverviewDto, detect_diga_model_from_slice,
    resolve_candidate, resolve_default_flash_path,
};
use super::error::IpcResult;

/// Returns the complete workspace and hardware overview for `useDashboardViewModel`.
#[tauri::command]
pub fn get_system_overview() -> IpcResult<SystemOverviewDto> {
    let check_file = |id: &str, name: &str, path: &str, role: &str| -> BinaryFileStatusDto {
        let p = Path::new(path);
        let meta = fs::metadata(p).ok();
        let exists = meta.is_some();
        let size_bytes = meta.map(|m| m.len());
        let formatted_size = size_bytes.map(|s| format!("{:.2} MB", s as f64 / BYTES_PER_MIB_F64));
        BinaryFileStatusDto {
            id: id.to_string(),
            name: name.to_string(),
            path: path.to_string(),
            exists,
            size_bytes,
            formatted_size,
            expected_role: role.to_string(),
            status: if exists {
                "ready".to_string()
            } else {
                "missing".to_string()
            },
        }
    };

    let resolved_flash = resolve_default_flash_path(None);

    let (product_model, soc_processor, flash_rom) = if let Ok(data) = fs::read(&resolved_flash) {
        let model = detect_diga_model_from_slice(&data);
        let (soc, rom) = match data.len() {
            FLASH_SIZE_2MB_BYTES => (
                "Matsushita AM33 / MN103S".to_string(),
                "2 MB NOR Flash (Top-Boot 64KB EEMUL NVRAM @ 0x801F0000)".to_string(),
            ),
            FLASH_SIZE_4MB_BYTES => (
                "Matsushita AM33 / MN103S".to_string(),
                "4 MB NOR Flash (Top-Boot 64KB EEMUL NVRAM @ 0x803F0000)".to_string(),
            ),
            FLASH_SIZE_8MB_BYTES => (
                "Matsushita AM34 / MN103S (VEP79138)".to_string(),
                "Spansion S29JL064H (8 MB NOR, Top-Boot 8x8KB SA134..SA141 @ 0x807F0000)"
                    .to_string(),
            ),
            FLASH_SIZE_16MB_BYTES => (
                "Matsushita AM34 / MN103S (VEP79138)".to_string(),
                "16 MB NOR Flash (Top-Boot 64KB EEMUL NVRAM @ 0x80FF0000)".to_string(),
            ),
            _ => (
                "Matsushita UniPhier AM34 / MN103S Core".to_string(),
                "Panasonic DIGA NOR Flash (Top-Boot EEMUL NVRAM)".to_string(),
            ),
        };
        (model, soc, rom)
    } else {
        (
            "Panasonic DIGA (Auto-Detected Target)".to_string(),
            "Matsushita UniPhier AM34 / MN103S Core".to_string(),
            "Spansion NOR Flash (Top-Boot 8x8KB EEMUL NVRAM)".to_string(),
        )
    };

    let binaries = vec![
        check_file(
            "flash_dump",
            "Panasonic DIGA NOR Flash Dump",
            &resolved_flash,
            "Bootloader IPL + 64 KB EEMUL NVRAM",
        ),
        check_file(
            "hdd_image",
            "Panasonic DIGA HDD Raw Dump",
            DEFAULT_HDD_IMAGE_PATH,
            "LBA 0 Superblock (SH) + Mirror LBA 57344 + Partitions 1 & 2",
        ),
        check_file(
            "frm_container",
            "Optical Firmware Container (PANA_DVD.FRM)",
            DEFAULT_FRM_CONTAINER_PATH,
            "64-bit ECB Encrypted AM34 OS, Servo DSP & Front Panel",
        ),
    ];

    Ok(SystemOverviewDto {
        product_model,
        soc_processor,
        core_architecture: OVERVIEW_CORE_ARCH.to_string(),
        os_system: OVERVIEW_OS_SYSTEM.to_string(),
        flash_rom,
        hdd_specs: OVERVIEW_HDD_SPECS.to_string(),
        optical_drive: OVERVIEW_OPTICAL_DRIVE.to_string(),
        debug_port: OVERVIEW_DEBUG_PORT.to_string(),
        error_code: OVERVIEW_ERROR_CODE.to_string(),
        error_description: OVERVIEW_ERROR_DESC.to_string(),
        firmware_version: OVERVIEW_FW_VERSION.to_string(),
        binaries,
    })
}

/// Frontend `read_hex_dump` command reading binary rows for the Hex Inspector.
#[tauri::command]
pub fn read_hex_dump(
    file_path: Option<String>,
    offset: Option<usize>,
    count: Option<usize>,
) -> IpcResult<Vec<HexDumpRowDto>> {
    let path = file_path.unwrap_or_else(|| {
        resolve_candidate(
            &[
                CANDIDATE_HDD_IMAGE_PATHS[0],
                CANDIDATE_FRM_CONTAINER_PATHS[0],
            ],
            DEFAULT_HDD_IMAGE_PATH,
        )
    });
    let base_offset = offset.unwrap_or(0);
    let row_count = count
        .unwrap_or(DEFAULT_HEX_DUMP_ROWS)
        .min(MAX_HEX_DUMP_ROWS);

    let mut file = File::open(&path)
        .map_err(|e| format!("Falha ao abrir binário '{}' para Hex Dump: {}", path, e))?;
    let file_len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);

    let mut rows = Vec::with_capacity(row_count);
    let mut buf = [0u8; HEX_DUMP_ROW_BYTES];

    for r in 0..row_count {
        let cur_offset = base_offset.saturating_add(r * HEX_DUMP_ROW_BYTES);
        if cur_offset >= file_len {
            break;
        }
        file.seek(SeekFrom::Start(cur_offset as u64))
            .map_err(|e| e.to_string())?;
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let slice = &buf[..n];
        let hex_p1: Vec<String> = slice
            .iter()
            .take(HEX_DUMP_HALF_ROW_BYTES)
            .map(|b| format!("{:02X}", b))
            .collect();
        let hex_p2: Vec<String> = slice
            .iter()
            .skip(HEX_DUMP_HALF_ROW_BYTES)
            .take(HEX_DUMP_HALF_ROW_BYTES)
            .map(|b| format!("{:02X}", b))
            .collect();
        let ascii: String = slice
            .iter()
            .map(|&b| {
                if (0x20..=0x7E).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        rows.push(HexDumpRowDto {
            offset: cur_offset,
            offset_hex: format!("0x{:08X}", cur_offset),
            bytes: slice.to_vec(),
            hex_part1: hex_p1.join(" "),
            hex_part2: hex_p2.join(" "),
            ascii,
        });
    }

    Ok(rows)
}
