//! Structured IPC error taxonomy for Tauri commands.
//!
//! Classifies hardware, safety (`E05`), serial transport, and filesystem errors into
//! deterministic machine-readable error codes while preserving human diagnostics.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical IPC error taxonomy shared across Tauri backend handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PanadigaIpcErrorCode {
    E05IplReadOnly,
    ESerial,
    EIo,
    EValidation,
    EInternal,
}

/// Structured IPC error payload carrying both taxonomy code and detailed diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanadigaIpcError {
    pub code: PanadigaIpcErrorCode,
    pub message: String,
}

impl PanadigaIpcError {
    /// Creates an IPC error with an explicit taxonomy code.
    pub fn new(code: PanadigaIpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Classifies an arbitrary domain/hardware error string into a structured `PanadigaIpcError`.
    pub fn classify(raw: impl Into<String>) -> Self {
        let message = raw.into();
        let lower = message.to_ascii_lowercase();
        let code = if lower.contains("e05")
            || lower.contains("ipl")
            || lower.contains("somente leitura")
            || lower.contains("read-only")
            || lower.contains("safety interlock")
        {
            PanadigaIpcErrorCode::E05IplReadOnly
        } else if lower.contains("serial")
            || lower.contains("porta com")
            || lower.contains("pico")
            || lower.contains("cdc")
            || lower.contains("gdb")
        {
            PanadigaIpcErrorCode::ESerial
        } else if lower.contains("inválid")
            || lower.contains("invalid")
            || lower.contains("não encontrado")
            || lower.contains("not found")
        {
            PanadigaIpcErrorCode::EValidation
        } else if lower.contains("i/o")
            || lower.contains("arquivo")
            || lower.contains("file")
            || lower.contains("permission")
        {
            PanadigaIpcErrorCode::EIo
        } else {
            PanadigaIpcErrorCode::EInternal
        };
        Self { code, message }
    }
}

impl fmt::Display for PanadigaIpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for PanadigaIpcError {}

impl From<std::io::Error> for PanadigaIpcError {
    fn from(err: std::io::Error) -> Self {
        Self::new(PanadigaIpcErrorCode::EIo, err.to_string())
    }
}

impl From<String> for PanadigaIpcError {
    fn from(err: String) -> Self {
        Self::classify(err)
    }
}

impl From<&str> for PanadigaIpcError {
    fn from(err: &str) -> Self {
        Self::classify(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_e05_and_serial_errors_deterministically() {
        let e05 = PanadigaIpcError::classify("E05: Regiao IPL 0x80000000 bloqueada");
        assert_eq!(e05.code, PanadigaIpcErrorCode::E05IplReadOnly);
        assert!(e05.to_string().contains("E05IplReadOnly"));

        let serial = PanadigaIpcError::classify("Falha ao abrir porta serial COM5");
        assert_eq!(serial.code, PanadigaIpcErrorCode::ESerial);

        let io =
            PanadigaIpcError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        assert_eq!(io.code, PanadigaIpcErrorCode::EIo);
    }
}
