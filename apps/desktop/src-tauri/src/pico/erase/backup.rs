//! Panasonic DMR-EH55 DIGA Suite - Pre-Erase Secret Backup & Double-Read Verification
//!
//! Single Responsibility: Executes the non-destructive "What's Next" 1st-Contact
//! Read-Only Backup, double-read integrity check, and forensic binary serialization.

use panadiga_core::constants::{AM34_FLASH_BASE, AM34_SRAM_BASE};
use panadiga_core::nvram::calculate_hashes;
use std::fs;
use std::io::Write;
use std::time::Duration;

use super::parser::{parse_autodetected_nvram_base, parse_pico_dump_hex_to_bytes};
use crate::commands::dto::detect_diga_model_from_slice;
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::pico::serial::read_serial_response_complete;
use crate::pico::types::{FirstContactBackupResult, extract_port_name};
use crate::state::PanadigaBackendState;

pub(crate) fn sha256_hex(data: &[u8]) -> String {
    calculate_hashes(data).1
}

/// Executes `f` under the exclusive serial lock after opening CDC and verifying P7501 IDENTIFY.
pub(crate) fn with_p7501_handshake<R>(
    backend: &PanadigaBackendState,
    port_name: &str,
    f: impl FnOnce(&mut dyn serialport::SerialPort, String) -> IpcResult<R>,
) -> IpcResult<R> {
    let clean_port = extract_port_name(port_name);
    backend.with_exclusive_serial(&clean_port, || {
        let mut port =
            panadiga_core::pico::cdc::open_cdc(&clean_port, 115200, Duration::from_millis(10000))
                .map_err(|e| {
                    PanadigaIpcError::serial(format!(
                        "Falha ao conectar na porta serial {}: {}",
                        clean_port, e
                    ))
                })?;

        port.write_all(b"IDENTIFY\r\n")
            .map_err(|e| PanadigaIpcError::serial(format!("Falha ao enviar IDENTIFY: {}", e)))?;
        let _ = port.flush();
        let hs_out = read_serial_response_complete(&mut *port, Duration::from_millis(3000))?;
        let offline_or_missing = (!hs_out.contains("@P7501_IDENT")
            && !hs_out.contains("IDENTIFY: OK"))
            || hs_out.contains("IDENTIFY: OFFLINE");
        if offline_or_missing {
            return Err(PanadigaIpcError::serial(format!(
                "Identificação P7501 falhou ao sondar o alvo Matsushita AM34 (gravador desligado VDD=0V ou fiação solta). Resposta: {}",
                hs_out.trim()
            )));
        }
        if let Some(ident) = panadiga_core::pico::parse_ident_line(&hs_out)
            && (ident.vdd_mv < 2500 || ident.state == "STANDBY_OR_OFF")
        {
            return Err(PanadigaIpcError::serial(format!(
                "Alvo P7501 desligado/offline no IDENTIFY (model={} vdd_mv={} state={}). Conecte e ligue o gravador antes de continuar.",
                ident.model, ident.vdd_mv, ident.state
            )));
        }
        f(&mut *port, hs_out)
    })
}

/// Resolves the target EEMUL NVRAM base address from `@P7501_IDENT` (fail-closed if read-only)
/// or falls back to legacy Flash-size detection when no structured ident frame is present.
pub(crate) fn resolve_ident_nvram_base_hex(hs_out: &str) -> IpcResult<String> {
    if let Some(ident) = panadiga_core::pico::parse_ident_line(hs_out) {
        return ident.nvram_base_hex().ok_or_else(|| {
            PanadigaIpcError::serial(format!(
                "Alvo identificado '{}' (flash '{}') não possui EEMUL NVRAM gravável verificada no SSOT; escrita/erase bloqueados.",
                ident.model, ident.flash
            ))
        });
    }
    Ok(parse_autodetected_nvram_base(hs_out).to_string())
}

pub(crate) fn dump_p7501_region_bytes(
    port: &mut dyn serialport::SerialPort,
    addr_hex: &str,
    words: u32,
    timeout_ms: u64,
) -> Vec<u8> {
    let _ = port.clear(serialport::ClearBuffer::Input);
    let cmd = format!("DUMP {} {}\r\n", addr_hex, words);
    let _ = port.write_all(cmd.as_bytes());
    let _ = port.flush();
    let ascii =
        read_serial_response_complete(port, Duration::from_millis(timeout_ms)).unwrap_or_default();
    parse_pico_dump_hex_to_bytes(&ascii)
}

pub(crate) fn save_live_backup_files(nvram: &[u8], sram: &[u8], ipl: &[u8]) -> IpcResult<String> {
    // Timestamped, never-overwriting directory under writable user data root: immutable forensic evidence.
    let base = panadiga_core::constants::resolve_writable_data_root().join("first_contact_backups");
    let ts_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let expected_nv_sha = sha256_hex(nvram);
    let expected_sr_sha = sha256_hex(sram);
    let expected_ipl_sha = sha256_hex(ipl);
    let sha_tag = &expected_nv_sha[..expected_nv_sha.len().min(8)];
    fs::create_dir_all(&base).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha ao criar raiz de backups forenses '{}': {}",
            base.display(),
            e
        ))
    })?;
    let out_dir = base.join(format!("first_contact_{}_{}", ts_ms, sha_tag));
    fs::create_dir(&out_dir).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha ao criar diretório exclusivo de backup forense '{}': {}",
            out_dir.display(),
            e
        ))
    })?;

    let nv_path = out_dir.join("nvram_live_backup.bin");
    let sr_path = out_dir.join("sram_secrets.bin");
    let ipl_path = out_dir.join("ipl_bootcamp_header.bin");

    for (path, payload, label, expected_sha) in [
        (&nv_path, nvram, "NVRAM", expected_nv_sha.as_str()),
        (&sr_path, sram, "SRAM", expected_sr_sha.as_str()),
        (&ipl_path, ipl, "IPL", expected_ipl_sha.as_str()),
    ] {
        let mut file = fs::File::create(path).map_err(|e| {
            PanadigaIpcError::io(format!(
                "Falha ao criar arquivo de backup {} '{}': {}",
                label,
                path.display(),
                e
            ))
        })?;
        std::io::Write::write_all(&mut file, payload).map_err(|e| {
            PanadigaIpcError::io(format!(
                "Falha ao gravar backup {} '{}': {}",
                label,
                path.display(),
                e
            ))
        })?;
        file.sync_all().map_err(|e| {
            PanadigaIpcError::io(format!(
                "Falha ao sincronizar (fsync) backup {} '{}': {}",
                label,
                path.display(),
                e
            ))
        })?;
        drop(file);
        let readback = fs::read(path).map_err(|e| {
            PanadigaIpcError::io(format!(
                "Falha ao reler backup {} '{}' para verificação SHA-256: {}",
                label,
                path.display(),
                e
            ))
        })?;
        if readback.len() != payload.len() || sha256_hex(&readback) != expected_sha {
            return Err(PanadigaIpcError::io(format!(
                "Verificação SHA-256 pós-gravação do backup {} '{}' falhou — operação abortada.",
                label,
                path.display()
            )));
        }
    }

    let manifest = format!(
        "{}  nvram_live_backup.bin\n{}  sram_secrets.bin\n{}  ipl_bootcamp_header.bin\n",
        expected_nv_sha, expected_sr_sha, expected_ipl_sha
    );
    fs::write(out_dir.join("SHA256SUMS.txt"), manifest).map_err(|e| {
        PanadigaIpcError::io(format!(
            "Falha ao gravar manifesto SHA256SUMS.txt em '{}': {}",
            out_dir.display(),
            e
        ))
    })?;
    Ok(out_dir.display().to_string())
}

/// True when a captured buffer is empty or one repeated byte (floating or shorted bus).
pub(crate) fn is_uniform_bus_pattern(data: &[u8]) -> bool {
    match data.first() {
        None => true,
        Some(&b0) => data.iter().all(|&b| b == b0),
    }
}

/// Captures 16 KiB SRAM secrets (`AM34_SRAM_BASE`) and 4 KiB Bootloader IPL (`AM34_FLASH_BASE`).
pub(crate) fn dump_sram_and_ipl_backups(
    port: &mut dyn serialport::SerialPort,
) -> (Vec<u8>, Vec<u8>) {
    let sram_hex = format!("0x{AM34_SRAM_BASE:08X}");
    let flash_hex = format!("0x{AM34_FLASH_BASE:08X}");
    let sr_bytes = dump_p7501_region_bytes(port, &sram_hex, 4096, 6000);
    let ipl_bytes = dump_p7501_region_bytes(port, &flash_hex, 1024, 4000);
    (sr_bytes, ipl_bytes)
}

/// Executes the non-destructive "What's Next" 1st-Contact Read-Only Backup:
/// 1. `HANDSHAKE` + Passive Address Aliasing Auto-Detect (`2MB` / `4MB` / `8MB` / `16MB`)
/// 2. Double-Read of 64KB NVRAM at auto-detected EEMUL base verifying identical SHA-256
/// 3. Read of 16KB SRAM (`0x00000000`) & 4KB Bootloader IPL Header (`0x80000000`)
/// 4. Auto-Detection of Panasonic DIGA Model (`DMR-EH55`, `DMR-ES15`, `DMR-EX75`, `DMR-EZ27`, etc.)
pub fn pico_execute_first_contact_backup(
    backend: &PanadigaBackendState,
    port_name: String,
) -> IpcResult<FirstContactBackupResult> {
    with_p7501_handshake(backend, &port_name, |port, hs_out| {
        let nvram_base_hex = resolve_ident_nvram_base_hex(&hs_out)?;
        let mut log_lines = vec![hs_out];

        let is_16mb = nvram_base_hex == "0x80FE0000";
        let nvram_words = if is_16mb { 32768 } else { 16384 };
        let nvram_timeout = if is_16mb { 30000 } else { 15000 };

        // Double-Read NVRAM at auto-detected EEMUL base to confirm zero bit-error on P7501 debug link
        let nv_bytes_1 = dump_p7501_region_bytes(port, &nvram_base_hex, nvram_words, nvram_timeout);
        let nv_bytes_2 = dump_p7501_region_bytes(port, &nvram_base_hex, nvram_words, nvram_timeout);
        let sha_1 = sha256_hex(&nv_bytes_1);
        let sha_2 = sha256_hex(&nv_bytes_2);

        let (sr_bytes, ipl_bytes) = dump_sram_and_ipl_backups(port);

        // A floating (0xFF..) or shorted (0x00..) bus yields identical reads; that is NOT a valid capture.
        // Require explicit IDENTIFY confirmation and a non-uniform bootloader header.
        let bus_alive = (log_lines[0].contains("@P7501_IDENT")
            || log_lines[0].contains("IDENTIFY: OK"))
            && !is_uniform_bus_pattern(&ipl_bytes);
        let double_read_verified = bus_alive && !nv_bytes_1.is_empty() && sha_1 == sha_2;

        // Auto-detect DIGA model from IPL Header or NVRAM strings
        let mut combined_probe = ipl_bytes.clone();
        combined_probe.extend_from_slice(&nv_bytes_1);
        let detected_model = detect_diga_model_from_slice(&combined_probe);

        if double_read_verified {
            let dir = save_live_backup_files(&nv_bytes_1, &sr_bytes, &ipl_bytes)?;
            log_lines.push(format!(
                "[DOUBLE-READ VERIFIED] SHA-256 idêntico nas 2 leituras ({} @ {}). Modelo detectado: {}. Salvo em {}/",
                &sha_1[..16],
                nvram_base_hex,
                detected_model,
                dir
            ));
        } else if !bus_alive {
            log_lines.push(
                "[ABORTADO] CPU não confirmada em HALT ou barramento flutuante/em curto (IPL uniforme). Nada foi salvo. Verifique a pinagem e o GND.".to_string(),
            );
        } else {
            log_lines.push(format!(
                "[ALERTA DE INTEGRIDADE] Leituras divergentes no barramento (Read1={} vs Read2={}). Verifique o cabo de 6 pinos!",
                &sha_1[..12],
                &sha_2[..12]
            ));
        }

        Ok(FirstContactBackupResult {
            success: double_read_verified,
            detected_model,
            nvram_bytes: nv_bytes_1.len(),
            nvram_sha256: sha_1,
            double_read_verified,
            sram_bytes: sr_bytes.len(),
            ipl_bytes: ipl_bytes.len(),
            log_lines,
        })
    })
}
