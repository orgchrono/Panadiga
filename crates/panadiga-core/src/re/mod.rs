//! Reverse Engineering, MN103S Disassembly, UART NOR Dumper & BGO Unbrick Primitives

pub mod bgo_unbrick;
pub mod mn10300;
pub mod nor_dumper;

pub use bgo_unbrick::{
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, PROG061_BASE_VA,
    PROG061_EXPECTED_SIZE, PROG061_NFROM_FORMAT_VA, Prog061BgoInspection, Prog061BgoPatchMode,
    Prog061BgoPatchReport, Prog061BgoSiteStatus, inspect_prog061_bgo_sites,
    patch_prog061_bgo_in_place, patch_prog061_or_build_digadiag,
};
pub use mn10300::{
    ImmediateLoadRef, InstructionRef, decode_branch_call_refs, find_callers_to,
    find_fc_immediate_loads, find_pointer_xrefs,
};
pub use nor_dumper::{
    DEFAULT_NOR_STUB_LOAD_ADDR, NOR_ALLOWED_WINDOWS, NOR_MAGIC_FOOTER, NOR_MAGIC_HEADER,
    NorDumpResult, build_readonly_nor_dumper_stub, decode_nor_dump_stream, validate_nor_dump_range,
};
