//! Tauri IPC implementations for packing `.FRM` containers and unpacking `EXTRHEAD`/`EXTRFOOT`.

use panadiga_core::constants::{PANA_DIGADIAG_FRM_SIZE, PANA_EXTRHEAD_DEFAULT_ADLER_STEP};
use panadiga_core::crypto::{
    ExtrheadCryptMode, FrmContainerMode, FrmPackModule, FrmPackReport, build_digadiag_frm,
    pack_frm, parse_u32_radix, unpack_extrhead,
};

use crate::commands::constants::{
    DEFAULT_DIGADIAG_BUILD, DEFAULT_DIGADIAG_DST_HEX, DEFAULT_DIGADIAG_TAG, DEFAULT_FRM_HW_FAMILY,
    DEFAULT_FRM_MODEL_GROUP, DEFAULT_FRM_VERSION_HEX, FRM_UNPACK_DEFAULT_MODE,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};

use super::paths::{
    ExtrheadUnpackReportDto, PackFrmRequestDto, resolve_read_path, resolve_write_path,
};

/// Packs modules or raw diagnostic binaries into `PANA_DVD.FRM`, `PANA__SD.FRM`, or `DIGADIAG.FRM`.
#[tauri::command]
pub fn pack_frm_command(request: PackFrmRequestDto) -> IpcResult<FrmPackReport> {
    let mode_norm = request.mode.trim().to_ascii_lowercase();
    let out_path = resolve_write_path(&request.out_path)?;

    let (bytes, report) = match mode_norm.as_str() {
        "digadiag" => {
            let in_raw = request
                .input_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    PanadigaIpcError::validation(
                        "Modo 'digadiag' exige arquivo binário de entrada.",
                    )
                })?;
            let in_path = resolve_read_path(in_raw);
            let payload = std::fs::read(&in_path).map_err(|e| {
                PanadigaIpcError::io(format!("Falha lendo '{}': {}", in_path.display(), e))
            })?;
            let dst_addr = parse_u32_radix(
                request
                    .dst_addr
                    .as_deref()
                    .unwrap_or(DEFAULT_DIGADIAG_DST_HEX),
            )
            .map_err(|e| PanadigaIpcError::validation(format!("Endereço destino inválido: {e}")))?;
            let tag = request.tag.as_deref().unwrap_or(DEFAULT_DIGADIAG_TAG);
            let build = request
                .build_name
                .as_deref()
                .unwrap_or(DEFAULT_DIGADIAG_BUILD);
            build_digadiag_frm(
                &payload,
                dst_addr,
                tag,
                build,
                PANA_EXTRHEAD_DEFAULT_ADLER_STEP as u32,
                PANA_DIGADIAG_FRM_SIZE,
            )
            .map_err(|e| {
                PanadigaIpcError::validation(format!("Falha empacotando DIGADIAG.FRM: {e}"))
            })?
        }
        "sd" | "dvd" => {
            if request.modules.is_empty() {
                return Err(PanadigaIpcError::validation(format!(
                    "Modo '{mode_norm}' exige ao menos um módulo (nome + caminho)."
                )));
            }
            let ver_raw = request
                .version_u16
                .as_deref()
                .unwrap_or(DEFAULT_FRM_VERSION_HEX);
            let ver_u32 = parse_u32_radix(ver_raw)?;
            let ver_u16 = u16::try_from(ver_u32)
                .map_err(|_| PanadigaIpcError::validation("version_u16 deve caber em 16 bits."))?;
            let hw = request
                .hw_family
                .as_deref()
                .unwrap_or(DEFAULT_FRM_HW_FAMILY);
            let model = request
                .model_group
                .as_deref()
                .unwrap_or(DEFAULT_FRM_MODEL_GROUP);
            let flag = request.target_flag.unwrap_or(0);

            let mut parsed_modules = Vec::with_capacity(request.modules.len());
            for item in &request.modules {
                let name = item.name.trim();
                let path_str = item.path.trim();
                if name.is_empty() || path_str.is_empty() {
                    return Err(PanadigaIpcError::validation(
                        "Cada módulo requer nome e caminho não vazios.",
                    ));
                }
                let mod_path = resolve_read_path(path_str);
                let payload = std::fs::read(&mod_path).map_err(|e| {
                    PanadigaIpcError::io(format!(
                        "Falha lendo módulo '{}': {}",
                        mod_path.display(),
                        e
                    ))
                })?;
                let mut m = FrmPackModule::new(name, payload);
                m.version_u16 = ver_u16;
                m.hardware_family = hw.to_string();
                m.model_group = model.to_string();
                m.target_flag = flag;
                parsed_modules.push(m);
            }

            let container_mode = if mode_norm == "dvd" {
                FrmContainerMode::Dvd
            } else {
                FrmContainerMode::Sd
            };
            pack_frm(&parsed_modules, container_mode, None).map_err(|e| {
                PanadigaIpcError::validation(format!(
                    "Falha empacotando .{}: {e}",
                    mode_norm.to_uppercase()
                ))
            })?
        }
        other => {
            return Err(PanadigaIpcError::validation(format!(
                "Modo inválido '{other}': esperado 'sd', 'dvd' ou 'digadiag'."
            )));
        }
    };

    std::fs::write(&out_path, &bytes).map_err(|e| {
        PanadigaIpcError::io(format!("Falha gravando '{}': {}", out_path.display(), e))
    })?;
    Ok(report)
}

/// Unpacks and verifies an `EXTRHEAD`/`EXTRFOOT` binary container using partial Feistel decryption.
#[tauri::command]
pub fn unpack_extrhead_command(
    input_path: String,
    mode: Option<String>,
    out_path: Option<String>,
) -> IpcResult<ExtrheadUnpackReportDto> {
    let trimmed = input_path.trim();
    if trimmed.is_empty() {
        return Err(PanadigaIpcError::validation(
            "Caminho do contêiner EXTRHEAD é obrigatório.",
        ));
    }
    let resolved_in = resolve_read_path(trimmed);
    let raw = std::fs::read(&resolved_in).map_err(|e| {
        PanadigaIpcError::io(format!("Falha lendo '{}': {}", resolved_in.display(), e))
    })?;

    let crypt_mode: ExtrheadCryptMode = mode
        .as_deref()
        .unwrap_or(FRM_UNPACK_DEFAULT_MODE)
        .parse()
        .map_err(|e: anyhow::Error| PanadigaIpcError::validation(e.to_string()))?;
    let unpacked = unpack_extrhead(&raw, crypt_mode).map_err(|e| {
        PanadigaIpcError::validation(format!("Falha ao desempacotar EXTRHEAD: {e}"))
    })?;

    let written_output_path = if let Some(out_raw) = out_path.filter(|s| !s.trim().is_empty()) {
        let out_file = resolve_write_path(&out_raw)?;
        std::fs::write(&out_file, &unpacked.decompressed).map_err(|e| {
            PanadigaIpcError::io(format!("Falha gravando '{}': {}", out_file.display(), e))
        })?;
        Some(out_file.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(ExtrheadUnpackReportDto {
        file_path: resolved_in.to_string_lossy().to_string(),
        header_checksum_hex: format!("0x{:08X}", unpacked.header.header_checksum),
        calculated_checksum_hex: format!("0x{:08X}", unpacked.calculated_checksum),
        header: unpacked.header,
        decompressed_bytes: unpacked.decompressed.len(),
        checksum_valid: unpacked.checksum_valid,
        footer_present: unpacked.footer_present,
        written_output_path,
    })
}
