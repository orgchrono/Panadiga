#![deny(unsafe_code)]

//! Unified Legacy DVD/HDD + Blu-ray/UHD `.FRM` Auto-Detector, Unpacker & Repacker.
//!
//! Combines Panadiga legacy Matsushita AM34/MN103S formats (`Dvd48B`, `Dvd64B`, `Sd48B`,
//! `Sd64B`, `STB`, `DIGADIAG`) with the Blu-ray / `PANAEUSB.FRM` stack from `pana-fw` &
//! `unixtract` (`AES-128-CBC + Feistel` `AESHeader`, `BlurayFeistel` 48B Adler-32 TOC,
//! `PANASONIC\0\0\0` 48B inner header, `VGD ` exclusion, `DRV1`/`DRV2` edge cipher, and
//! `MainList` GZIP/LZSS partition streams).

mod decode;
mod decode_slot;
pub mod repack;
#[cfg(test)]
mod tests;
mod types;

pub use decode::unpack_universal_frm;
pub use repack::{
    FrmRepackOptions, FrmTargetMode, RepackCompression, UniversalFrmRepackReport,
    repack_universal_frm,
};
pub use types::{
    ModuleEncryptionKind, UniversalFrmContainer, UniversalFrmFormat, UniversalFrmModule,
    UniversalFrmSubContainer,
};
