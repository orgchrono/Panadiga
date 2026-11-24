#![no_main]

use libfuzzer_sys::fuzz_target;
use panadiga_crypto::{
    ExtrheadCryptMode, decompress_pana_lzss, parse_extrhead_header, unpack_extrhead,
};

fuzz_target!(|data: &[u8]| {
    let _ = parse_extrhead_header(data);
    let _ = decompress_pana_lzss(data, 65_536);
    let _ = unpack_extrhead(data, ExtrheadCryptMode::Auto);
});
