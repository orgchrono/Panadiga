//! `_dev_SERIAL` UART0 GDB RSP `$M` frame formatting and injection planning.

use serde::{Deserialize, Serialize};

use super::patcher::PASSTHROUGH_SPECS;

/// `_dev_SERIAL` UART0 RSP protocol state table base VA (`0x515B2E60` holds state `0..4`).
pub const DEV_SERIAL_STATE_BASE_VA: u32 = 0x515B_2E00;
/// `_dev_SERIAL` registered serial callback pointer VA (`+0x40`).
pub const DEV_SERIAL_CALLBACK_VA: u32 = 0x515B_2E40;
/// Pointer to active ISR register save context frame.
pub const DEV_SERIAL_ISR_CTX_PTR_VA: u32 = 0x5316_AE8C;
/// Offset of saved `PC` within the ISR register save context frame.
pub const DEV_SERIAL_PC_OFFSET: u32 = 0x64;
/// Verified 3-byte `_dev_SERIAL` UART0 state-machine entry sequence (`0 -> 2 -> 4`, paired with `0x15` NAK).
pub const DEV_SERIAL_HANDSHAKE_BYTES: [u8; 3] = [0x0E, 0x04, 0x02];

/// Structured live `_dev_SERIAL` RSP sequence returned by MCP `rsp_plan` mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainPassthroughRspPlan {
    pub handshake_hex: String,
    pub nak_byte_hex: String,
    pub serial_state_base_va: String,
    pub serial_callback_va: String,
    pub isr_ctx_ptr_va: String,
    pub isr_saved_pc_offset: String,
    pub passthrough_frames: Vec<String>,
    pub factory_restore_frames: Vec<String>,
    pub callback_write_frame: Option<String>,
    pub pico_cli_shortcuts: Vec<String>,
}

/// Compute 8-bit modulo-256 GDB RSP checksum over a payload string.
pub fn rsp_checksum(payload: &str) -> u8 {
    payload.bytes().fold(0u8, |acc, b| acc.wrapping_add(b))
}

/// Format a payload string as `$<payload>#<xx>` with lowercase hex checksum.
pub fn format_rsp_frame(payload: &str) -> String {
    format!("${payload}#{:02x}", rsp_checksum(payload))
}

/// Format an arbitrary memory write payload as `M<ADDR>,<LEN>:<lowercase_hex>`.
pub fn format_rsp_write_mem_payload(va: u32, data: &[u8]) -> String {
    let hex_bytes = hex::encode(data);
    format!("M{va:08X},{}:{hex_bytes}", data.len())
}

/// Returns the 3 verified `MAIN` (`3410PQ`) AV TBC passthrough `$M` payloads (without `$` / `#xx`).
pub fn passthrough_rsp_payloads() -> Vec<String> {
    PASSTHROUGH_SPECS
        .iter()
        .map(|s| format_rsp_write_mem_payload(s.va, s.replace))
        .collect()
}

/// Format an arbitrary memory write as `$M<ADDR>,<LEN>:<lowercase_hex>#<xx>`.
pub fn format_rsp_write_mem_frame(va: u32, data: &[u8]) -> String {
    format_rsp_frame(&format_rsp_write_mem_payload(va, data))
}

/// Format a 32-bit little-endian word write as `$M<ADDR>,4:<le_hex>#<xx>`.
pub fn format_rsp_write_u32_frame(va: u32, value_le: u32) -> String {
    format_rsp_write_mem_frame(va, &value_le.to_le_bytes())
}

/// Build the complete `_dev_SERIAL` live RSP frame plan for Pico / UART injection.
pub fn build_passthrough_rsp_plan(callback_target_va: Option<u32>) -> MainPassthroughRspPlan {
    let passthrough_frames = PASSTHROUGH_SPECS
        .iter()
        .map(|s| format_rsp_write_mem_frame(s.va, s.replace))
        .collect();
    let factory_restore_frames = PASSTHROUGH_SPECS
        .iter()
        .map(|s| format_rsp_write_mem_frame(s.va, s.expect))
        .collect();
    let callback_write_frame =
        callback_target_va.map(|va| format_rsp_write_u32_frame(DEV_SERIAL_CALLBACK_VA, va));
    let mut pico_cli_shortcuts = vec![
        "PANARSP PASSTHROUGH".to_string(),
        "PANARSP NORMAL MAGIC PASSTHROUGH".to_string(),
    ];
    if let Some(va) = callback_target_va {
        pico_cli_shortcuts.push(format!("PANARSP CALL:{va:08X}"));
    }

    MainPassthroughRspPlan {
        handshake_hex: hex::encode(DEV_SERIAL_HANDSHAKE_BYTES),
        nak_byte_hex: "15".to_string(),
        serial_state_base_va: format!("0x{DEV_SERIAL_STATE_BASE_VA:08X}"),
        serial_callback_va: format!("0x{DEV_SERIAL_CALLBACK_VA:08X}"),
        isr_ctx_ptr_va: format!("0x{DEV_SERIAL_ISR_CTX_PTR_VA:08X}"),
        isr_saved_pc_offset: format!("0x{DEV_SERIAL_PC_OFFSET:02X}"),
        passthrough_frames,
        factory_restore_frames,
        callback_write_frame,
        pico_cli_shortcuts,
    }
}
