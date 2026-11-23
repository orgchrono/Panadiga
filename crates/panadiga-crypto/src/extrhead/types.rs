#![deny(unsafe_code)]

//! Domain types for Panasonic `EXTRHEAD`/`EXTRFOOT` containers.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

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
            other => {
                bail!("Unsupported compression algo '{other}': expected zlib, gzip, lzss, or lzma")
            }
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
            other => bail!("Unsupported unpack mode '{other}': expected auto, main, drv, or plain"),
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
