//! Checks which referenced wav names are missing from the GRF.
//! Reads newline-separated names from the path in arg 1 (relative to
//! `data/wav/`), prints the missing ones. Run from the repo root:
//!   cargo run -p ragnarok-formats --example check_wavs -- /tmp/wav_names.txt

use std::path::Path;

use ragnarok_formats::grf::GrfArchive;

fn main() {
    let list = std::env::args()
        .nth(1)
        .expect("usage: check_wavs <name-list>");
    let grf = GrfArchive::open(Path::new("data/data.grf")).expect("open data/data.grf");
    // Extract mode: `check_wavs --extract <out-dir>` reads GRF-internal wav paths
    // (one per line) from stdin-less arg — pass them via a file as arg 3 — and
    // dumps each to <out-dir> so the candidate can be played and ear-verified.
    if list == "--extract" {
        let out = std::env::args()
            .nth(2)
            .expect("usage: --extract <out-dir> <path-list>");
        let paths = std::env::args()
            .nth(3)
            .expect("usage: --extract <out-dir> <path-list>");
        std::fs::create_dir_all(&out).expect("create out dir");
        for p in std::fs::read_to_string(&paths)
            .expect("read path list")
            .lines()
        {
            let p = p.trim();
            if p.is_empty() {
                continue;
            }
            match grf.read_file(p) {
                Ok(bytes) => {
                    let base = p.rsplit(['/', '\\']).next().unwrap_or(p);
                    let dest = format!("{out}/{base}");
                    std::fs::write(&dest, bytes).expect("write wav");
                    println!("OK   {p} -> {dest}");
                }
                Err(_) => println!("MISS {p}"),
            }
        }
        return;
    }
    let names = std::fs::read_to_string(&list).expect("read name list");
    // Second arg = a prefix to list every GRF entry under it (e.g. "data/wav/")
    // — prints the archive's real (Korean) filenames. Otherwise, check-missing mode.
    if let Some(prefix) = std::env::args().nth(2) {
        let mut hits: Vec<&str> = grf
            .entry_names()
            .filter(|n| n.starts_with(&prefix))
            .collect();
        hits.sort_unstable();
        for n in hits {
            println!("{n}");
        }
        return;
    }
    for name in names.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let full = format!("data/wav/{name}");
        if !grf.file_exists(&full) {
            println!("{name}");
        }
    }
}
