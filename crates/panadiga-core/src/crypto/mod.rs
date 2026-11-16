//! Cryptographic Forensics, Entropy Analysis & ECB Block Frequency Leak Detection
//!
//! Analyzes encrypted Panasonic container formats (`PANA_DVD.FRM`) and correlates
//! 64-bit Feistel cipher blocks with HDD partition structures.

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

pub use codebook::{correlate_hdd_partitions, print_codebook};
pub use codec::parse_u32_radix;
pub use entropy::{analyze_8byte_frequencies, analyze_frm_container};
pub use extrhead::{
    ExtrheadCompressionAlgo, ExtrheadCryptMode, ExtrheadHeader, UnpackedExtrhead,
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
    CramFsHeaderInfo, DRV_HEADER_SIZE, DRV_TAIL_CLEAR_BYTES, DriveHeaderInfo,
    KaseikyoServiceCommand, MAIN_LIST_LEGACY_CRYPT_SPAN, MAIN_LIST_LEGACY_SEGMENT_TARGET_SIZE,
    MAIN_LIST_MODERN_CRYPT_SPAN, MAIN_LIST_MODERN_SEGMENT_TARGET_SIZE, MainListEntry,
    MainListHeader, MainPartitionSlice, PANA_KASEIKYO_SERVICE_COMMANDS, PartitionFsKind,
    RomFsHeaderInfo, UnpackedDrvModule, UnpackedMainList, checksum_affs, extract_drv_module,
    find_feistel_key_slow, network_decipher_in_place, network_encipher_in_place, pack_drv_module,
    pack_main_list, parse_main_list, split_main_partitions, verify_cramfs_header,
    verify_romfs_header,
};
pub use toc::{
    TocLayout, analyze_frm_toc_report, analyze_toc, decrypt_and_parse_frm_toc,
    decrypt_frm_toc_with_detected_key, format_toc_dump_lines, interpret_frm_target_flag,
    parse_sd_frm_toc, parse_sd_frm_toc_with_layout,
};
pub use toc_io::{dump_toc, read_and_analyze_frm_toc};
pub use types::*;
