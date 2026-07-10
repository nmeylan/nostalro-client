//! Checks which referenced wav names are missing from the GRF.
//! Reads newline-separated names from the path in arg 1 (relative to
//! `data/wav/`), prints the missing ones. Run from the repo root:
//!   cargo run -p ragnarok-formats --example check_wavs -- /tmp/wav_names.txt

use std::path::Path;

use ragnarok_formats::grf::GrfArchive;

fn main() {
    let list = std::env::args().nth(1).expect("usage: check_wavs <name-list>");
    let names = std::fs::read_to_string(&list).expect("read name list");
    let grf = GrfArchive::open(Path::new("data/data.grf")).expect("open data/data.grf");
    for name in names.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let full = format!("data/wav/{name}");
        if !grf.file_exists(&full) {
            println!("{name}");
        }
    }
}
