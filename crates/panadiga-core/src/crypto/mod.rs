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
pub mod toc;
pub mod toc_io;
pub mod types;

pub use codebook::{correlate_hdd_partitions, print_codebook};
pub use codec::parse_u32_radix;
pub use entropy::{analyze_8byte_frequencies, analyze_frm_container};
pub use extrhead::{
    ExtrheadCryptMode, ExtrheadHeader, UnpackedExtrhead, pack_extrhead, parse_extrhead_header,
    unpack_extrhead,
};
pub use feistel::{
    adler32_panasonic_strided, derive_round_key_32b, feistel_decrypt_block, feistel_decrypt_buffer,
    feistel_encrypt_block, feistel_encrypt_buffer, feistel_f_round, reconstruct_master_key_7b,
};
pub use frm_pack::{
    FrmContainerMode, FrmPackModule, FrmPackReport, PackedFrmModuleSummary, build_digadiag_frm,
    convert_pana_dvd_to_factory_sd, pack_frm, unpack_dvd_frm_to_modules,
    verify_packed_frm_roundtrip,
};
pub use toc::{
    TocLayout, analyze_frm_toc_report, analyze_toc, decrypt_and_parse_frm_toc,
    format_toc_dump_lines, interpret_frm_target_flag, parse_sd_frm_toc,
    parse_sd_frm_toc_with_layout,
};
pub use toc_io::{dump_toc, read_and_analyze_frm_toc};
pub use types::*;
