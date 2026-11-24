#![no_main]

use libfuzzer_sys::fuzz_target;
use panadiga_core::pico::{
    NorWritePolicy, decode_mn103s_g_registers, decode_rsp_hex_memory, filter_host_rsp_packet,
    parse_rsp_reply,
};

fuzz_target!(|data: &[u8]| {
    let _ = parse_rsp_reply(data, None);
    let _ = decode_rsp_hex_memory(data);
    let _ = decode_mn103s_g_registers(data);
    let mut policy = NorWritePolicy::NvramOnly;
    let _ = filter_host_rsp_packet(data, &mut policy);
});
