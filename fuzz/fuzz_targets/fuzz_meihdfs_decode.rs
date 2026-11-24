#![no_main]

use libfuzzer_sys::fuzz_target;
use panadiga_core::hdd::{
    decode_journal_blocks_slice, decode_meihdfs_slice, decode_single_meihdfs_record,
    try_decode_single_meihdfs_record,
};

fuzz_target!(|data: &[u8]| {
    let _ = decode_single_meihdfs_record(data, 0);
    let _ = try_decode_single_meihdfs_record(data, 0);
    let _ = decode_meihdfs_slice(data, true, "fuzz_meihdfs");
    let _ = decode_journal_blocks_slice(data);
});
