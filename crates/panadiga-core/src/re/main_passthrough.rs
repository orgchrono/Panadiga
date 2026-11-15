//! Panasonic `DMR-EH55` `MAIN` (`EH55_MAIN_3410PQ.bin`) AV TBC (`E-E`) Passthrough Patcher & RSP Planner.

mod patcher;
mod rsp_plan;

#[cfg(test)]
mod tests;

pub use patcher::{
    EXPECTED_MAIN_3410PQ_SIZE, MAIN_BASE_VA, MainFirmwareIdentity, MainPassthroughMode,
    MainPassthroughReport, MainPassthroughSiteReport, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS,
    OFF_ODD_STATE_BRANCH, ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH,
    PATCH_FPC_STATUS_CTRL, PATCH_ODD_INIT_STATUS, PATCH_ODD_STATE_BRANCH,
    apply_main_passthrough_in_place, inspect_main_firmware_identity,
    inspect_main_passthrough_sites, patch_main_passthrough_file,
};
pub use rsp_plan::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_HANDSHAKE_BYTES, DEV_SERIAL_ISR_CTX_PTR_VA,
    DEV_SERIAL_PC_OFFSET, DEV_SERIAL_STATE_BASE_VA, MainPassthroughRspPlan,
    build_passthrough_rsp_plan, format_rsp_frame, format_rsp_write_mem_frame,
    format_rsp_write_mem_payload, format_rsp_write_u32_frame, passthrough_rsp_payloads,
    rsp_checksum,
};
