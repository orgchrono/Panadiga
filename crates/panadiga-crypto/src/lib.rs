#![forbid(unsafe_code)]

//! Cryptographic Forensics, Entropy Analysis & ECB Block Frequency Leak Detection
//!
//! Analyzes encrypted Panasonic container formats (`PANA_DVD.FRM`) and correlates
//! 64-bit Feistel cipher blocks with HDD partition structures.

#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]

#[macro_use]
pub extern crate panadiga_constants;

pub use panadiga_constants as constants;

pub(crate) mod hdd {
    pub use panadiga_constants::forensic::{ImageBuffer, calculate_entropy, open_image_buffer};
}

pub(crate) mod nvram {
    pub use panadiga_constants::forensic::{calculate_hashes, resolve_input_path};
}

pub mod aes128;
pub mod codebook;
pub mod codec;
pub mod entropy;
pub mod extrhead;
pub mod feistel;
pub mod frm_pack;
pub mod pana_fw_go;
pub mod toc;
pub mod toc_io;
pub mod types;
pub mod universal_frm;

pub use aes128::{
    FrmKeyring, PANA_DVD_AES_CATALOG, PanaAesKeyPair, PanaFeistelKey, aes128_cbc_decrypt_in_place,
    aes128_cbc_encrypt_in_place, aes128_decrypt_block, aes128_encrypt_block, derive_network_key_iv,
};
pub use codebook::{correlate_hdd_partitions, print_codebook};
pub use codec::{DecodedStbSlot, parse_u32_radix, try_decode_stb_slot};
pub use entropy::{analyze_8byte_frequencies, analyze_frm_container};
pub use extrhead::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, ExtrheadError, ExtrheadHeader, UnpackedExtrhead,
    compress_pana_lzss, decompress_pana_lzss, pack_extrhead, pack_extrhead_with_options,
    parse_extrhead_header, unpack_extrhead, unpack_extrhead_with_key,
};
pub use feistel::{
    PANA_DVD_KEY_CATALOG, adler32_panasonic_strided, derive_round_key_32b,
    derive_round_key_from_custom_key_8b, detect_frm_feistel_key, feistel_decrypt_block,
    feistel_decrypt_buffer, feistel_encrypt_block, feistel_encrypt_buffer, feistel_f_round,
    reconstruct_master_key_7b,
};
pub use frm_pack::{
    FrmContainerMode, FrmPackModule, FrmPackReport, PackedFrmModuleSummary, build_digadiag_frm,
    build_digadiag_frm_with_algo, convert_pana_dvd_to_factory_sd, pack_frm, pack_frm_with_key,
    unpack_dvd_frm_to_modules, verify_packed_frm_roundtrip,
};
pub use pana_fw_go::{
    AesHeader, AesHeaderFileEntry, CramFsHeaderInfo, DRV_HEADER_SIZE, DRV_TAIL_CLEAR_BYTES,
    DriveHeaderInfo, ElfMetadata, ExtractedFsEntry, ExtractedFsEntryKind, FsExtractReport,
    KaseikyoServiceCommand, MAIN_LIST_LEGACY_CRYPT_SPAN, MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE,
    MAIN_LIST_MODERN_CRYPT_SPAN, MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE, MainListEntry,
    MainListHeader, MainPartitionSlice, PANA_AES_HEADER_MAGIC, PANA_AES_HEADER_SIZE,
    PANA_KASEIKYO_SERVICE_COMMANDS, PartitionFsKind, RomFsHeaderInfo, UnpackedDrvModule,
    UnpackedMainList, UnwrappedAesFrm, checksum_affs, detect_and_unwrap_aes_frm,
    extract_cramfs_to_dir, extract_drv_module, extract_partition_fs, extract_romfs_to_dir,
    find_feistel_key_slow, find_feistel_key_slow_with_keyring, network_decipher_in_place,
    network_encipher_in_place, pack_drv_module, pack_main_list, parse_elf_metadata,
    parse_main_list, reassemble_main_partitions, sanitize_fs_rel_path, split_main_partitions,
    verify_cramfs_header, verify_romfs_header, wrap_aes_frm,
};
pub use toc::{
    TocLayout, analyze_frm_toc_report, analyze_toc, decrypt_and_parse_frm_toc,
    decrypt_frm_toc_with_detected_key, decrypt_frm_toc_with_keyring, format_toc_dump_lines,
    interpret_frm_target_flag, parse_sd_frm_toc, parse_sd_frm_toc_with_layout,
};
pub use toc_io::{dump_toc, read_and_analyze_frm_toc};
pub use types::*;
pub use universal_frm::{
    FrmRepackOptions, FrmTargetMode, ModuleEncryptionKind, RepackCompression,
    UniversalFrmContainer, UniversalFrmFormat, UniversalFrmModule, UniversalFrmRepackReport,
    UniversalFrmSubContainer, repack_universal_frm, unpack_universal_frm,
};
