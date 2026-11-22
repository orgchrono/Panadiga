//! Panasonic DMR-EH55 DIGA Suite - In-Circuit NVRAM Erase Executor
//!
//! Single Responsibility: In-circuit execution of RAM loader injection and
//! surgical sector erase on Panasonic DIGA NOR Flash via P7501 / RAM Loader.

use std::time::{Duration, Instant};

use super::backup::{
    dump_p7501_region_bytes, dump_sram_and_ipl_backups, is_uniform_bus_pattern,
    resolve_ident_nvram_base_hex, save_live_backup_files, sha256_hex, with_p7501_handshake,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::pico::serial::read_serial_response_complete;
use crate::pico::types::InCircuitNvramEraseResult;
use crate::state::PanadigaBackendState;

pub(crate) fn query_port_vdd(port: &mut dyn serialport::SerialPort) -> f32 {
    let _ = port.clear(serialport::ClearBuffer::Input);
    let _ = port.write_all(b"STATUS\r\n");
    let _ = port.flush();
    if let Ok(resp) = read_serial_response_complete(port, Duration::from_millis(600))
        && let Some(v) = panadiga_core::pico::cdc::parse_vdd_reply(&resp)
    {
        return v;
    }
    3.30
}

/// Validates that the pre-erase NVRAM capture matches the expected byte count and contains
/// real flash contents rather than a uniform floating/shorted bus pattern (`0x00..` or `0xFF..`).
pub(crate) fn validate_pre_erase_nvram_backup(
    nv_bytes: &[u8],
    expected_bytes: usize,
) -> IpcResult<()> {
    if nv_bytes.len() != expected_bytes {
        return Err(PanadigaIpcError::serial(format!(
            "Backup pré-erase NVRAM incompleto: recebidos {} bytes, esperados {} bytes. Erase bloqueado por segurança.",
            nv_bytes.len(),
            expected_bytes
        )));
    }
    if is_uniform_bus_pattern(nv_bytes) {
        let fill = nv_bytes.first().copied().unwrap_or(0);
        return Err(PanadigaIpcError::serial(format!(
            "Backup pré-erase NVRAM uniforme (0x{:02X} repetido em {} bytes — barramento flutuante/em curto ou NVRAM já apagada). Erase bloqueado para evitar perda irreparável.",
            fill,
            nv_bytes.len()
        )));
    }
    Ok(())
}

/// Executes full in-circuit sequence:
/// 1. 4-Stage `IDENTIFY` (`@P7501_IDENT`) + Auto-Detect Flash Geometry
/// 2. Pre-Erase Live Secret & Full 64KB NVRAM Binary Dump (`nvram_live_backup.bin`, `sram_secrets.bin`, `ipl_bootcamp_header.bin`)
/// 3. Atomic on-probe `JOB NVRAM_CLEAN CONFIRM` (auto-backing up to `P2_VAULT` & cleaning Top-Boot 64KB EEMUL NVRAM)
pub fn pico_execute_in_circuit_erase_full(
    backend: &PanadigaBackendState,
    port_name: String,
) -> IpcResult<InCircuitNvramEraseResult> {
    let start_ts = Instant::now();
    with_p7501_handshake(backend, &port_name, |port, hs_out| {
        let voltage_before = query_port_vdd(port);
        let nvram_base_hex = resolve_ident_nvram_base_hex(&hs_out)?;
        let mut log_lines = vec![format!("[IDENTIFY] {}", hs_out)];

        let is_16mb = nvram_base_hex == "0x80FE0000";
        let nvram_words = if is_16mb { 32768 } else { 16384 };
        let nvram_timeout = if is_16mb { 30000 } else { 15000 };
        let erase_timeout = if is_16mb { 120000 } else { 90000 };
        let bytes_erased = if is_16mb {
            panadiga_core::constants::FLASH_16MB_NVRAM_SIZE_BYTES as u32
        } else {
            panadiga_core::constants::SPANSION_NVRAM_SIZE_BYTES as u32
        };

        // Step 2: Double-Read Pre-Erase Live Secret & Binary NVRAM Dump at auto-detected EEMUL base
        let nv_bytes_1 = dump_p7501_region_bytes(port, &nvram_base_hex, nvram_words, nvram_timeout);
        validate_pre_erase_nvram_backup(&nv_bytes_1, bytes_erased as usize)?;
        let nv_bytes_2 = dump_p7501_region_bytes(port, &nvram_base_hex, nvram_words, nvram_timeout);
        let nv_sha256 = sha256_hex(&nv_bytes_1);
        let nv_sha256_2 = sha256_hex(&nv_bytes_2);
        if nv_bytes_1 != nv_bytes_2 || nv_sha256 != nv_sha256_2 {
            return Err(PanadigaIpcError::serial(format!(
                "Double-read NVRAM divergente antes do erase (Read1={} vs Read2={}). Erase bloqueado para proteger dados forenses.",
                &nv_sha256[..16],
                &nv_sha256_2[..16]
            )));
        }
        let (sr_bytes, ipl_bytes) = dump_sram_and_ipl_backups(port);
        if sr_bytes.len() != 16384 || ipl_bytes.len() != 4096 || is_uniform_bus_pattern(&ipl_bytes)
        {
            return Err(PanadigaIpcError::serial(format!(
                "Leitura SRAM ({}/16384 B) ou IPL Boot ({}/4096 B) incompleta/uniforme (barramento flutuante ou em curto). Erase bloqueado por segurança.",
                sr_bytes.len(),
                ipl_bytes.len()
            )));
        }
        let backup_dir = save_live_backup_files(&nv_bytes_1, &sr_bytes, &ipl_bytes)?;

        log_lines.push(format!(
            "First-Contact Binary Backup ({}): nvram_live_backup.bin ({} bytes, DOUBLE-READ SHA256={}), sram_secrets.bin ({} bytes) & ipl_bootcamp_header.bin ({} bytes) salvos e verificados em {}.",
            nvram_base_hex,
            nv_bytes_1.len(),
            &nv_sha256[..16],
            sr_bytes.len(),
            ipl_bytes.len(),
            backup_dir
        ));

        // Step 3: Atomic on-probe JOB NVRAM_CLEAN CONFIRM
        let _ = port.clear(serialport::ClearBuffer::Input);
        port.write_all(b"JOB NVRAM_CLEAN CONFIRM\r\n")
            .map_err(|e| {
                PanadigaIpcError::serial(format!("Falha ao enviar JOB NVRAM_CLEAN CONFIRM: {}", e))
            })?;
        let _ = port.flush();
        let erase_out = read_serial_response_complete(port, Duration::from_millis(erase_timeout))?;
        for line in erase_out.lines() {
            log_lines.push(line.to_string());
        }
        let res_ok = panadiga_core::pico::parse_res_line(&erase_out).is_none_or(|r| r.is_ok());
        let success = erase_out.contains("JOB: OK")
            && res_ok
            && !erase_out.contains("ERROR")
            && !erase_out.contains("SAFETY INTERLOCK");
        let voltage_after = query_port_vdd(port);
        if success {
            backend.telemetry.set_ram_loader_injected(true);
        }

        Ok(InCircuitNvramEraseResult {
            success,
            elapsed_ms: start_ts.elapsed().as_millis() as u64,
            voltage_before,
            voltage_after,
            ram_loader_addr: format!("0x{:08X}", panadiga_core::constants::AM34_SRAM_LOADER_BASE),
            bytes_erased,
            verification_sha256: nv_sha256,
            log_lines,
        })
    })
}

/// Executes in-circuit surgical erase of Spansion S29JL064H NVRAM (string summary wrapper).
pub fn pico_execute_in_circuit_erase(
    backend: &PanadigaBackendState,
    port_name: String,
) -> IpcResult<String> {
    let res = pico_execute_in_circuit_erase_full(backend, port_name)?;
    Ok(res.log_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pre_erase_nvram_backup_rejects_bad_size_and_uniform_fills() {
        assert!(validate_pre_erase_nvram_backup(&[], 65536).is_err());
        assert!(validate_pre_erase_nvram_backup(&vec![0xA5; 32768], 65536).is_err());
        assert!(validate_pre_erase_nvram_backup(&vec![0x00; 65536], 65536).is_err());
        assert!(validate_pre_erase_nvram_backup(&vec![0xFF; 65536], 65536).is_err());
        assert!(validate_pre_erase_nvram_backup(&vec![0xFF; 131072], 131072).is_err());

        let mut valid_8mb = vec![0xFF; 65536];
        valid_8mb[0] = 0x55;
        valid_8mb[1] = 0xAA;
        assert!(validate_pre_erase_nvram_backup(&valid_8mb, 65536).is_ok());

        let mut valid_16mb = vec![0x00; 131072];
        valid_16mb[16] = 0x4E;
        assert!(validate_pre_erase_nvram_backup(&valid_16mb, 131072).is_ok());
    }

    #[test]
    fn test_resolve_ident_nvram_base_hex_models() {
        let cases = [
            ("DMR-EH55", "S29JL064H_8MB", Some("0x807F0000")),
            ("DMR-ES35V", "S29JL064H_8MB", Some("0x807F0000")),
            ("DMR-EZ485V", "MX29LV320B_4MB", None),
            ("DMR-ES10", "MR27V802_8MB", None),
        ];
        for (model, flash, expected) in cases {
            let line = format!(
                "@P7501_IDENT vdd_mv=3280 nsw5v=1 elec=CONNECTED loader=ver0.61 jedec=0x0 flash={flash} ipl_crc32=0x0 hw=UNVERIFIED tv=UNKNOWN state=PANARSP_READY model={model} tier=BENCH_VERIFIED caps=0x1 ssot=0x0"
            );
            assert_eq!(
                resolve_ident_nvram_base_hex(&line).ok().as_deref(),
                expected,
                "unexpected nvram resolution for {model}"
            );
        }
    }
}
