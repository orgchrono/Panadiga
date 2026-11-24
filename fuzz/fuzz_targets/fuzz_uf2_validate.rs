#![no_main]

use libfuzzer_sys::fuzz_target;
use panadiga_core::pico::{Uf2TargetFamily, extract_uf2_family_slice, validate_uf2_buffer};

fuzz_target!(|data: &[u8]| {
    let _ = validate_uf2_buffer(data);
    let _ = extract_uf2_family_slice(data, Uf2TargetFamily::Rp2040);
    let _ = extract_uf2_family_slice(data, Uf2TargetFamily::Rp2350);
});
