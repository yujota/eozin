#![no_main]

use eozin::std_io::DynamicDecoder;
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let reader = Cursor::new(data);
    if let Ok(mut decoder) = DynamicDecoder::new(reader) {
        let _ = decoder.read_tile_as_bytes(0, 0, 0);
    }
});
