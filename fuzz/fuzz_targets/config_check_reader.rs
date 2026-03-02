#![no_main]

use std::io::Cursor;
use std::path::PathBuf;

use hatch::config::Config;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = Config::check_reader(
        PathBuf::from("fuzz-input.yaml"),
        Cursor::new(data),
    );
});
