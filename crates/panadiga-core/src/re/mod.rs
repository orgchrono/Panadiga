//! Reverse Engineering, MN103S Disassembly, UART NOR Dumper & BGO Unbrick Primitives

pub mod bgo_unbrick;
pub mod main_passthrough;
pub mod mn10300;
pub mod nor_dumper;
pub mod prog061_uart_monitor;
pub mod unbrick_sd_kit;
pub mod unbrick_workbench;

pub use bgo_unbrick::{
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, PROG061_BASE_VA,
    PROG061_EXPECTED_SIZE, PROG061_NFROM_FORMAT_VA, Prog061BgoInspection, Prog061BgoPatchMode,
    Prog061BgoPatchReport, Prog061BgoSiteStatus, decode_prog061_payload, inspect_prog061_bgo_sites,
    inspect_prog061_container_bytes, patch_prog061_bgo_in_place, patch_prog061_or_build_digadiag,
};
pub use main_passthrough::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_HANDSHAKE_BYTES, DEV_SERIAL_ISR_CTX_PTR_VA,
    DEV_SERIAL_PC_OFFSET, DEV_SERIAL_STATE_BASE_VA, EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA,
    MainPassthroughMode, MainPassthroughReport, MainPassthroughRspPlan, MainPassthroughSiteReport,
    OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH, ORIG_FPC_STATUS_CTRL,
    ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH, PATCH_FPC_STATUS_CTRL, PATCH_ODD_INIT_STATUS,
    PATCH_ODD_STATE_BRANCH, apply_main_passthrough_in_place, build_passthrough_rsp_plan,
    format_rsp_frame, format_rsp_write_mem_frame, format_rsp_write_u32_frame,
    inspect_main_passthrough_sites, patch_main_passthrough_file, rsp_checksum,
};
pub use mn10300::{
    ImmediateLoadRef, InstructionRef, decode_branch_call_refs, find_callers_to,
    find_fc_immediate_loads, find_pointer_xrefs,
};
pub use nor_dumper::{
    DEFAULT_NOR_STUB_LOAD_ADDR, NOR_ALLOWED_WINDOWS, NOR_MAGIC_FOOTER, NOR_MAGIC_HEADER,
    NorDumpResult, build_nor_dumper_artifact, build_readonly_nor_dumper_stub,
    decode_nor_dump_stream, validate_nor_dump_range,
};
pub use prog061_uart_monitor::{
    OFF_UART_MONITOR_SLACK, PATCH_A2CD_UART_MONITOR, PROG061_UART_MONITOR_VA,
    UART_MONITOR_MAX_SLACK_BYTES, build_prog061_uart_monitor_payload,
};
pub use unbrick_sd_kit::{
    SdMediaVerificationReport, SdUnbrickCheck, SdUnbrickKitReport, SdUnbrickKitRequest,
    prepare_sd_unbrick_kit, verify_sd_buffers, verify_sd_unbrick_media,
};
pub use unbrick_workbench::{
    ExtractedFrmModuleReport, FrmModuleExtractionReport, UnbrickHardwareFact,
    UnbrickWorkspaceReport, extract_modules_from_frm, inspect_unbrick_workspace,
};
