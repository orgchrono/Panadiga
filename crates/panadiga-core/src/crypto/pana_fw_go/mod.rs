//! Recompiler, Demuxer, Filesystem Validator & Stream Cipher Primitives Ported from `pana-fw` (Go)
//!
//! Brings key discoveries from `WeebDataHoarder/pana-fw` and `theubusu/unixtract` into pure Rust:
//! - Multi-segment `MainList` (`20 + 8*N` header + partial Feistel `0x1400`/`0x2800` overlap ordering)
//! - Optical drive `DRV` `0x20`-byte `DriveHeader` + tail `-48` cipher wrapper
//! - `MAIN` partition splitter (`fmaX` command line + `SWUP_ADDON`) with `RomFS` & `CramFS` verification
//! - Statistical key recovery (`find_feistel_key_slow`) over `PANA_DVD_KEY_CATALOG`
//! - `libdownloadlib.so` stream cipher (`network_decipher_in_place`) & Kaseikyo IR service table

pub mod aes_container;
mod crypto_utils;
mod drv;
pub mod fs_extract;
mod kaseikyo;
mod main_list;
mod partitions;
mod types;

#[cfg(test)]
pub(crate) mod tests;

pub use aes_container::{
    AesHeader, AesHeaderFileEntry, PANA_AES_HEADER_MAGIC, PANA_AES_HEADER_SIZE, UnwrappedAesFrm,
    detect_and_unwrap_aes_frm, wrap_aes_frm,
};
pub use crypto_utils::{
    find_feistel_key_slow, find_feistel_key_slow_with_keyring, network_decipher_in_place,
    network_encipher_in_place,
};
pub use drv::{extract_drv_module, pack_drv_module};
pub use fs_extract::{
    ElfMetadata, ExtractedFsEntry, ExtractedFsEntryKind, FsExtractReport, extract_cramfs_to_dir,
    extract_partition_fs, extract_romfs_to_dir, parse_elf_metadata, sanitize_fs_rel_path,
};
pub use kaseikyo::{
    PANA_KASEIKYO_SERVICE_COMMANDS, build_kaseikyo_wire_frame, decode_flipper_kaseikyo_command,
};
pub use main_list::{pack_main_list, parse_main_list};
pub use partitions::{
    checksum_affs, reassemble_main_partitions, split_main_partitions, verify_cramfs_header,
    verify_romfs_header,
};
pub use types::{
    CramFsHeaderInfo, DRV_HEADER_SIZE, DRV_TAIL_CLEAR_BYTES, DriveHeaderInfo,
    KaseikyoServiceCommand, MAIN_LIST_LEGACY_CRYPT_SPAN, MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE,
    MAIN_LIST_MODERN_CRYPT_SPAN, MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE, MainListEntry,
    MainListHeader, MainPartitionSlice, PartitionFsKind, RomFsHeaderInfo, UnpackedDrvModule,
    UnpackedMainList,
};
