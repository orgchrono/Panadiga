#![deny(unsafe_code)]

//! Domain types for Panasonic `EXTRHEAD`/`EXTRFOOT` containers.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Typed errors returned while parsing or unpacking `EXTRHEAD`/`EXTRFOOT` containers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtrheadError {
    #[error("Invalid EXTRHEAD container header ({len} bytes)")]
    InvalidHeader { len: usize },
    #[error("Unsupported compression algo '{0}': expected zlib, gzip, lzss, or lzma")]
    UnsupportedCompressionAlgo(String),
    #[error("Unsupported unpack mode '{0}': expected auto, main, drv, or plain")]
    UnsupportedCryptMode(String),
    #[error("EXTRHEAD nesting exceeded max recursion depth ({max_depth})")]
    RecursionLimitExceeded { max_depth: u8 },
    #[error("EXTRHEAD container truncated: expected {expected} bytes, got {actual}")]
    TruncatedContainer { expected: usize, actual: usize },
    #[error("EXTRHEAD checksum mismatch: expected 0x{expected:08X}, got 0x{actual:08X}")]
    ChecksumMismatch { expected: u32, actual: u32 },
}

/// Partial encryption profile for an `EXTRHEAD`/`EXTRFOOT` container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtrheadCryptMode {
    Auto,
    Main,
    Drv,
    Plain,
}

/// Supported inner payload compression algorithms for `EXTRHEAD`/`EXTRFOOT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum ExtrheadCompressionAlgo {
    Zlib = 0,
    Gzip = 1,
    Lzss = 2,
    Lzma = 3,
}

impl std::str::FromStr for ExtrheadCompressionAlgo {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "zlib" | "0" => Ok(Self::Zlib),
            "gzip" | "gz" | "1" => Ok(Self::Gzip),
            "lzss" | "2" => Ok(Self::Lzss),
            "lzma" | "3" => Ok(Self::Lzma),
            other => Err(ExtrheadError::UnsupportedCompressionAlgo(other.to_string()).into()),
        }
    }
}

impl std::str::FromStr for ExtrheadCryptMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "main" => Ok(Self::Main),
            "drv" => Ok(Self::Drv),
            "plain" => Ok(Self::Plain),
            other => Err(ExtrheadError::UnsupportedCryptMode(other.to_string()).into()),
        }
    }
}

/// Parsed 64-byte (`0x40`) Panasonic `EXTRHEAD` header block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtrheadHeader {
    pub tag: String,
    pub flag: u16,
    pub algo: u16,
    pub dst_len: u32,
    pub dst_addr: u32,
    pub src_len: u32,
    #[serde(default)]
    pub src_addr: u32,
    pub footer_offset: u32,
    pub header_checksum: u32,
    pub adler_step: u32,
    pub build_name: String,
}

/// Result of decrypting, decompressing, and validating an `EXTRHEAD`/`EXTRFOOT` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpackedExtrhead {
    pub header: ExtrheadHeader,
    pub decompressed: Vec<u8>,
    pub calculated_checksum: u32,
    pub checksum_valid: bool,
    pub footer_present: bool,
}
