#![deny(unsafe_code)]

//! Panasonic `EXTRHEAD`/`EXTRFOOT` zlib/gzip/LZSS/LZMA Codec & Partial Feistel Wrapper
//!
//! Pure-Rust implementation matching `BOOT.BIN` (`FUN_4400b5e1`, `FUN_4400b61a`,
//! `FUN_4400b6c6`, `FUN_4401e5a3`) plus Go `pana-fw` / `unixtract` multi-key & compression parity.

mod codec;
mod crypt;
mod lzss;
mod types;

#[cfg(test)]
mod tests;

pub use codec::{
    pack_extrhead, pack_extrhead_with_options, parse_extrhead_header, unpack_extrhead,
    unpack_extrhead_with_key,
};
#[cfg(test)]
pub(crate) use crypt::apply_partial_crypt_with_key;
pub(crate) use crypt::detect_extrhead_key_and_double;
pub use lzss::{compress_pana_lzss, decompress_pana_lzss};
pub use types::{ExtrheadCompressionAlgo, ExtrheadCryptMode, ExtrheadHeader, UnpackedExtrhead};
