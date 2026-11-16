//! Read-only MN103S `DIGADIAG` UART NOR flash dumper stub builder and stream decoder.
//!
//! Direct pure-Rust consolidation of `scripts/bench/nor_dumper.py`: emits position-coupled
//! MN103S machine code strictly guarded to read-only cached (`0x80000000..0x80800000`) and
//! uncached (`0x84000000..0x84800000`) NOR windows, and verifies `PANADMP1..PANADONE` dumps.

mod stream;
mod stub;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::PANA_DIGADIAG_DEFAULT_LOAD_ADDR;

pub use stream::decode_nor_dump_stream;
pub(crate) use stub::PUTC_D3_BYTES;
pub use stub::{build_nor_dumper_artifact, build_readonly_nor_dumper_stub};

/// Magic header emitted at the start of every UART NOR flash dump stream.
pub const NOR_MAGIC_HEADER: &[u8; 8] = b"PANADMP1";
/// Magic footer emitted at the end of every UART NOR flash dump stream.
pub const NOR_MAGIC_FOOTER: &[u8; 8] = b"PANADONE";
/// Strictly enforced read-only MN103S NOR flash windows (cached & uncached).
pub const NOR_ALLOWED_WINDOWS: [(u32, u32); 2] =
    [(0x8000_0000, 0x8080_0000), (0x8400_0000, 0x8480_0000)];
/// Safe free SDRAM load address used by live Pico RAM injection (`$M` / `$G`).
pub const DEFAULT_NOR_STUB_LOAD_ADDR: u32 = 0x5180_0000;
/// Verified `/BOOT.BIN` `DIGADIAG.FRM` default RAM entry VA (`0x50400000`).
pub const DIGADIAG_NOR_STUB_LOAD_ADDR: u32 = PANA_DIGADIAG_DEFAULT_LOAD_ADDR;

pub(crate) const HEADER_PREFIX_LEN: usize = NOR_MAGIC_HEADER.len() + 12;

/// Selects the canonical stub load VA: `0x50400000` for wrapped `DIGADIAG.FRM` or `0x51800000` for live RAM RSP.
pub fn resolve_nor_stub_load_addr(explicit: Option<u32>, wrap_digadiag: bool) -> u32 {
    explicit.unwrap_or(if wrap_digadiag {
        DIGADIAG_NOR_STUB_LOAD_ADDR
    } else {
        DEFAULT_NOR_STUB_LOAD_ADDR
    })
}

/// Verified binary NOR dump decoded from a `PANADMP1..PANADONE` UART stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NorDumpResult {
    pub start_addr: u32,
    pub total_len: u32,
    pub block_size: u32,
    pub data: Vec<u8>,
    pub sha256: String,
    pub block_checksums: Vec<u32>,
}

/// Validates strictly read-only NOR address bounds (`0x80000000..` / `0x84000000..`) and alignment.
pub fn validate_nor_dump_range(start_addr: u32, total_len: u32, block_size: u32) -> Result<()> {
    if total_len == 0 {
        bail!("total_len must be > 0 (got 0)");
    }
    if block_size == 0 {
        bail!("block_size must be > 0 (got 0)");
    }
    if !total_len.is_multiple_of(block_size) {
        bail!("total_len ({total_len}) must be an exact multiple of block_size ({block_size})");
    }
    let start_u64 = start_addr as u64;
    let end_u64 = start_u64 + total_len as u64;
    let in_window = NOR_ALLOWED_WINDOWS
        .iter()
        .any(|&(lo, hi)| start_u64 >= lo as u64 && end_u64 <= hi as u64);
    if !in_window {
        bail!(
            "Read-only NOR guard rejected [0x{start_addr:08X}, 0x{end_u64:08X}); allowed windows: 0x80000000..0x80800000 or 0x84000000..0x84800000"
        );
    }
    Ok(())
}
