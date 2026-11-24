//! Pure functional GDB Remote Serial Protocol (RSP) frame codec and AM34 read-only safety guards.
//!
//! Shared SSOT across CLI, Tauri bridge, and Python bench tooling for `$<payload>#<xx>` framing,
//! binary `$x` escaping, E05 IPL NOR write protection (`0x80000000..0x807EFFFF` / `0x84..`),
//! `$m`/`$g` response validation, and 28-word (`0x70`-byte) MN103S register decoding.

mod decode;
mod encode;
pub mod policy;

#[cfg(test)]
mod tests;

pub use decode::{
    G_REGISTER_BYTES, ParsedRspFrame, check_rsp_e0x_error, decode_mn103s_g_registers,
    decode_rsp_hex_memory, parse_rsp_reply, validate_dump_range,
};
pub use encode::{
    MAX_M_CHUNK, MAX_X_CHUNK, build_nor_lock_frame, build_nor_unlock_frame, build_rsp_frame,
    build_rsp_write_frame, build_rsp_write_frame_with_policy, rsp_checksum,
};
pub use policy::{
    CRITICAL_IPL_BOOT_RANGES, HostRspStreamFilterResult, IPL_READONLY_RANGES, NorWritePolicy,
    build_am34_gdb_memory_map_xml, filter_host_rsp_packet, filter_host_rsp_tcp_stream,
    is_critical_ipl_boot_address, is_ipl_readonly_address, is_nor_write_blocked_with_policy,
    parse_loopback_socket_addr,
};
