//! Low-level USB CDC transport, port discovery, and multi-chunk reader.

use serialport::SerialPortType;
use std::time::{Duration, Instant};

use panadiga_core::pico::cdc::strip_leading_prompts;

use crate::commands::constants::{
    COMPLETION_MARKERS, SERIAL_CHUNK_READ_TIMEOUT_MS, SERIAL_READ_CHUNK_BYTES,
};
use crate::commands::error::{IpcResult, PanadigaIpcError};
use crate::state::PanadigaBackendState;

use super::super::types::extract_port_name;

/// Hard upper bound (2 MiB) for a single text CLI response so runaway UART chatter cannot OOM the host.
const MAX_SERIAL_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

/// Verifies that the serial port is not currently owned by an active GDB RSP or SC0 bridge thread.
pub fn ensure_serial_port_unlocked(
    backend: &PanadigaBackendState,
    clean_port: &str,
) -> IpcResult<()> {
    backend.ensure_serial_port_unlocked(clean_port)
}

pub(crate) fn serial_response_has_completed(out: &str) -> bool {
    let body = strip_leading_prompts(out);
    if body.is_empty() {
        return false;
    }
    let trimmed_end = body.trim_end();
    if trimmed_end.ends_with("PANADIGA>") || trimmed_end.ends_with("P7501>") {
        return true;
    }
    if trimmed_end == "PONG" {
        return true;
    }
    // Only evaluate line-based completion markers on a complete line boundary,
    // preventing mid-line truncation across 64-byte USB CDC packets.
    if !out.ends_with('\n') && !out.ends_with('\r') {
        return false;
    }
    if body
        .lines()
        .any(|line| line.trim_start().starts_with("@P7501_RES "))
    {
        return true;
    }
    // When structured @P7501_ / @PROG / @EVT frames are present, wait for @P7501_RES or prompt
    // rather than breaking early on a human summary line like `<CMD>: OK`.
    let has_structured_frames = body.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("@P7501_") || t.starts_with("@PROG ") || t.starts_with("@EVT ")
    });
    if has_structured_frames {
        return false;
    }
    body.lines().any(|line| {
        let trimmed = line.trim();
        COMPLETION_MARKERS
            .iter()
            .any(|marker| trimmed == *marker || trimmed.starts_with(marker))
    })
}

/// Reads multi-chunk USB CDC serial output into a bounded byte buffer until a structured
/// `@P7501_RES ...\n` epilogue, interactive prompt (`PANADIGA>` / `P7501>`), or legacy
/// completion line is received (or `max_wait` expires).
pub fn read_serial_response_complete(
    port: &mut dyn serialport::SerialPort,
    max_wait: Duration,
) -> IpcResult<String> {
    let _ = port.set_timeout(Duration::from_millis(SERIAL_CHUNK_READ_TIMEOUT_MS));
    let start = Instant::now();
    let mut raw: Vec<u8> = Vec::with_capacity(SERIAL_READ_CHUNK_BYTES);
    let mut chunk = [0u8; SERIAL_READ_CHUNK_BYTES];

    while start.elapsed() < max_wait {
        match port.read(&mut chunk) {
            Ok(0) => continue,
            Ok(n) => {
                let remaining = MAX_SERIAL_RESPONSE_BYTES.saturating_sub(raw.len());
                if remaining == 0 {
                    break;
                }
                let take = n.min(remaining);
                let slice = &chunk[..take];
                raw.extend_from_slice(slice);
                if slice.contains(&b'\n') || slice.contains(&b'>') {
                    let decoded = String::from_utf8_lossy(&raw);
                    if serial_response_has_completed(&decoded) {
                        break;
                    }
                }
                if raw.len() >= MAX_SERIAL_RESPONSE_BYTES {
                    break;
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                // Keep waiting until an explicit terminal marker arrives or max_wait expires;
                // long captures (BOOTCAP/DIAGWATCH/JOB) legitimately pause between target lines.
                continue;
            }
            Err(err) => {
                return Err(PanadigaIpcError::serial(format!(
                    "Erro de leitura USB CDC: {err}"
                )));
            }
        }
    }
    let out = String::from_utf8_lossy(&raw);
    let clean = strip_leading_prompts(&out).trim();
    if clean.is_empty() {
        Ok(out.trim().to_string())
    } else {
        Ok(clean.to_string())
    }
}

fn port_numeric_suffix(name: &str) -> u32 {
    let digits: String = name
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse::<u32>().unwrap_or(u32::MAX)
}

fn classify_cdc_role(product: Option<&str>, is_pico: bool) -> (u8, &'static str) {
    if let Some(prod) = product {
        let lower = prod.to_ascii_lowercase();
        if lower.contains("mirror") || lower.contains("cdc1") {
            return (1, if is_pico { " CDC1/Mirror" } else { "" });
        }
        if lower.contains("cli") || lower.contains("control") || lower.contains("cdc0") {
            return (0, if is_pico { " CDC0/CLI" } else { "" });
        }
    }
    (0, "")
}

/// Lists all available USB CDC serial COM ports, prioritizing Raspberry Pi Pico CDC0 (CLI)
/// interfaces ahead of CDC1 RX mirrors and non-Pico adapters while skipping BOOTSEL drives.
pub fn list_pico_serial_ports() -> IpcResult<Vec<String>> {
    struct PortEntry {
        sort_key: (u8, u8, u32, String),
        label: String,
        pico_serial: Option<String>,
        has_explicit_role: bool,
        vid: u16,
        pid: u16,
        desc: String,
        port_name: String,
    }

    let mut entries: Vec<PortEntry> = Vec::new();
    if let Ok(available) = serialport::available_ports() {
        for p in available {
            let num = port_numeric_suffix(&p.port_name);
            if let SerialPortType::UsbPort(info) = &p.port_type {
                if info.pid == panadiga_core::pico::platforms::PICO_BOOTSEL_PID {
                    continue;
                }
                let is_pico = info.vid == panadiga_core::pico::platforms::RASPBERRY_PI_VID;
                let is_not_pico: u8 = u8::from(!is_pico);
                let (_plat, desc) = panadiga_core::pico::classify_usb_device(
                    info.vid,
                    info.pid,
                    info.product.as_deref(),
                );
                let (iface_rank, iface_tag) = classify_cdc_role(info.product.as_deref(), is_pico);
                let label = format!(
                    "{} [{}{} - VID:{:04X} PID:{:04X}]",
                    p.port_name, desc, iface_tag, info.vid, info.pid
                );
                entries.push(PortEntry {
                    sort_key: (is_not_pico, iface_rank, num, p.port_name.clone()),
                    label,
                    pico_serial: if is_pico {
                        info.serial_number.clone()
                    } else {
                        None
                    },
                    has_explicit_role: !iface_tag.is_empty(),
                    vid: info.vid,
                    pid: info.pid,
                    desc,
                    port_name: p.port_name,
                });
            } else {
                entries.push(PortEntry {
                    sort_key: (1, 2, num, p.port_name.clone()),
                    label: p.port_name.clone(),
                    pico_serial: None,
                    has_explicit_role: false,
                    vid: 0,
                    pid: 0,
                    desc: String::new(),
                    port_name: p.port_name,
                });
            }
        }
    }

    entries.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

    // When a Dual-CDC Pico shares one USB serial number without per-interface product strings,
    // label the lower-numbered port CDC0/CLI and the companion port CDC1/Mirror.
    let mut serial_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entry in &entries {
        if let Some(serial) = &entry.pico_serial {
            *serial_counts.entry(serial.clone()).or_insert(0) += 1;
        }
    }
    let mut serial_seen: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entry in &mut entries {
        if entry.has_explicit_role {
            continue;
        }
        if let Some(serial) = &entry.pico_serial
            && serial_counts.get(serial).copied().unwrap_or(0) >= 2
        {
            let idx = serial_seen.entry(serial.clone()).or_insert(0);
            let role_tag = if *idx == 0 {
                " CDC0/CLI"
            } else {
                " CDC1/Mirror"
            };
            *idx += 1;
            entry.label = format!(
                "{} [{}{} - VID:{:04X} PID:{:04X}]",
                entry.port_name, entry.desc, role_tag, entry.vid, entry.pid
            );
        }
    }

    Ok(entries.into_iter().map(|e| e.label).collect())
}

pub(super) fn open_validated_cdc(
    backend: &PanadigaBackendState,
    port_name: &str,
    baud_rate: u32,
    timeout_ms: u64,
) -> IpcResult<(String, Box<dyn serialport::SerialPort>)> {
    let clean = extract_port_name(port_name);
    backend.ensure_serial_port_unlocked(&clean)?;
    let port =
        panadiga_core::pico::cdc::open_cdc(&clean, baud_rate, Duration::from_millis(timeout_ms))
            .map_err(|e| {
                let msg = e.to_string();
                let hint = if cfg!(target_os = "linux")
                    && msg.to_ascii_lowercase().contains("permission")
                {
                    " (Linux: adicione seu usuário ao grupo 'dialout'/'uucp' via `sudo usermod -aG dialout,uucp $USER`)"
                } else {
                    ""
                };
                PanadigaIpcError::serial(format!("Falha ao abrir porta serial {clean}: {msg}{hint}"))
            })?;
    Ok((clean, port))
}

#[cfg(test)]
mod tests {
    use super::{port_numeric_suffix, serial_response_has_completed};

    #[test]
    fn serial_completion_waits_for_structured_res_newline_or_prompt() {
        assert!(!serial_response_has_completed(""));
        assert!(!serial_response_has_completed("PANADIGA> "));
        // Intermediate VDD line from STATUS must NOT terminate read before [STANDBY]/[WDT]/@P7501_RES
        assert!(!serial_response_has_completed("VDD: 3.28V (OK)\n"));
        // Partial @P7501_RES line split across USB packets must NOT terminate before '\n'
        assert!(!serial_response_has_completed(
            "VDD: 3.28V (OK)\n@P7501_RES cmd=STATUS status=OK"
        ));
        assert!(serial_response_has_completed(
            "VDD: 3.28V (OK)\n@P7501_RES cmd=STATUS status=OK code=0 elapsed_us=120\n"
        ));
        assert!(serial_response_has_completed("PONG\nPANADIGA> "));
        assert!(serial_response_has_completed("PONG"));
        assert!(serial_response_has_completed("DUMP: OK\n"));
        assert!(!serial_response_has_completed("IDENTIFY: OK"));
        assert!(!serial_response_has_completed(
            "@P7501_IDENT model=DMR-EH55\nIDENTIFY: OK (model=DMR-EH55)\n"
        ));
        assert!(serial_response_has_completed(
            "@P7501_IDENT model=DMR-EH55\nIDENTIFY: OK (model=DMR-EH55)\n@P7501_RES cmd=IDENTIFY status=OK code=0 elapsed_us=95000\n"
        ));
    }

    #[test]
    fn port_numeric_suffix_orders_com_ports_numerically() {
        assert_eq!(port_numeric_suffix("COM9"), 9);
        assert_eq!(port_numeric_suffix("COM10"), 10);
        assert_eq!(port_numeric_suffix("/dev/ttyACM0"), 0);
        assert_eq!(port_numeric_suffix("/dev/ttyACM1"), 1);
    }
}
