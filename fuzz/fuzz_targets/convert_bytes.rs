#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let _ = ofx_kit::convert_bytes(input, &ofx_kit::ConversionOptions::default());
});
