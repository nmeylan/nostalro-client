//! Quick helper: list all `.str` files in the GRF that contain a pattern.
//! Usage: cargo run -p ragnarok-formats --example list_str -- <grf> [pattern]
use std::path::PathBuf;

use ragnarok_formats::grf::GrfArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let grf_path = args.get(1).ok_or("usage: list_str <grf> [pattern]")?;
    let archive = GrfArchive::open(&PathBuf::from(grf_path))?;
    let pattern = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let mut names: Vec<String> = archive
        .file_names()
        .iter()
        .filter(|n| {
            n.to_lowercase().ends_with(".str")
                && n.to_lowercase().contains(&pattern.to_lowercase())
        })
        .map(|s| s.to_string())
        .collect();
    names.sort();
    for n in &names {
        println!("{n}");
    }
    eprintln!("[{} matching files]", names.len());
    Ok(())
}
