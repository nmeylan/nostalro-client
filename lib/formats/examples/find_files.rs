use std::path::PathBuf;

use ragnarok_formats::grf::GrfArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let grf_path = args.get(1).ok_or("usage: find_files <grf> [pattern]")?;
    let pattern = args.get(2).map(|s| s.to_lowercase()).unwrap_or_default();
    let archive = GrfArchive::open(&PathBuf::from(grf_path))?;
    let mut names: Vec<String> = archive
        .file_names()
        .iter()
        .filter(|n| n.to_lowercase().contains(&pattern))
        .map(|s| s.to_string())
        .collect();
    names.sort();
    for n in &names {
        println!("{n}");
    }
    eprintln!("[{} matching files]", names.len());
    Ok(())
}
