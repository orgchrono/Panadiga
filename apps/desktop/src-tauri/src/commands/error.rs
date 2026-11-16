//! Structured IPC error taxonomy for Tauri commands.
//!
//! Classifies hardware, safety (`E05`), serial transport, validation, and filesystem
//! errors into deterministic machine-readable error codes while preserving human diagnostics
//! and seamless string serialization across Tauri IPC boundaries.

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Alias for every Tauri command return type enforcing structured `PanadigaIpcError`.
pub type IpcResult<T> = Result<T, PanadigaIpcError>;

/// Canonical IPC error taxonomy shared across Tauri backend handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanadigaIpcErrorCode {
    E05IplReadOnly,
    ESerial,
    EIo,
    EValidation,
    EInternal,
}

impl PanadigaIpcErrorCode {
    /// Machine-readable error code tag serialized in IPC error prefixes.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::E05IplReadOnly => "E05_IPL_READ_ONLY",
            Self::ESerial => "E_SERIAL",
            Self::EIo => "E_IO",
            Self::EValidation => "E_VALIDATION",
            Self::EInternal => "E_INTERNAL",
        }
    }
}

/// Structured IPC error payload carrying both taxonomy code and detailed diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Creates a deterministic filesystem/I/O error (`[E_IO]`).
    pub fn io(message: impl Into<String>) -> Self {
        Self::new(PanadigaIpcErrorCode::EIo, message)
    }

    /// Creates a deterministic input/path/argument validation error (`[E_VALIDATION]`).
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(PanadigaIpcErrorCode::EValidation, message)
    }

    /// Creates a deterministic hardware/serial/GDB transport error (`[E_SERIAL]`).
    pub fn serial(message: impl Into<String>) -> Self {
        Self::new(PanadigaIpcErrorCode::ESerial, message)
    }

    /// Creates a deterministic IPL read-only protection violation (`[E05_IPL_READ_ONLY]`).
    pub fn ipl_read_only(message: impl Into<String>) -> Self {
        Self::new(PanadigaIpcErrorCode::E05IplReadOnly, message)
    }

    /// Creates a deterministic unexpected internal fault (`[E_INTERNAL]`).
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(PanadigaIpcErrorCode::EInternal, message)
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
            || lower.contains("porta ")
            || lower.contains("pico")
            || lower.contains("cdc")
            || lower.contains("gdb")
            || lower.contains("sc0")
        {
            PanadigaIpcErrorCode::ESerial
        } else if lower.contains("inválid")
            || lower.contains("invalid")
            || lower.contains("não encontrad")
            || lower.contains("not found")
            || lower.contains("não suportad")
            || lower.contains("desconhecid")
        {
            PanadigaIpcErrorCode::EValidation
        } else if lower.contains("i/o")
            || lower.contains("arquivo")
            || lower.contains("file")
            || lower.contains("permission")
            || lower.contains("failed reading")
            || lower.contains("failed writing")
        {
            PanadigaIpcErrorCode::EIo
        } else {
            PanadigaIpcErrorCode::EInternal
        };
        Self { code, message }
    }
}

impl Deref for PanadigaIpcError {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl fmt::Display for PanadigaIpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for PanadigaIpcError {}

impl Serialize for PanadigaIpcError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PanadigaIpcError {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::classify(raw))
    }
}

impl From<std::io::Error> for PanadigaIpcError {
    fn from(err: std::io::Error) -> Self {
        Self::new(PanadigaIpcErrorCode::EIo, err.to_string())
    }
}

impl From<anyhow::Error> for PanadigaIpcError {
    fn from(err: anyhow::Error) -> Self {
        Self::classify(err.to_string())
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
    fn classifies_and_serializes_ipc_errors_deterministically() {
        let e05 = PanadigaIpcError::classify("E05: Regiao IPL 0x80000000 bloqueada");
        assert_eq!(e05.code, PanadigaIpcErrorCode::E05IplReadOnly);
        assert!(e05.to_string().starts_with("[E05_IPL_READ_ONLY]"));
        assert!(e05.contains("0x80000000"));
        let serialized = serde_json::to_string(&e05).unwrap();
        assert!(serialized.contains("E05_IPL_READ_ONLY"));

        let serial = PanadigaIpcError::classify("Falha ao abrir porta serial COM5");
        assert_eq!(serial.code, PanadigaIpcErrorCode::ESerial);

        let validation = PanadigaIpcError::classify("Ferramenta externa não suportada");
        assert_eq!(validation.code, PanadigaIpcErrorCode::EValidation);

        let io =
            PanadigaIpcError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        assert_eq!(io.code, PanadigaIpcErrorCode::EIo);
    }
}
