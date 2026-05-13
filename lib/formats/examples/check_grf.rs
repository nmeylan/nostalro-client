use std::path::Path;

fn main() {
    let grf_path = std::env::args().nth(1).unwrap_or_else(|| "data/data_old.grf".into());
    let grf = ragnarok_formats::grf::GrfArchive::open(Path::new(&grf_path)).expect("open grf");
    
    for info in grf.file_list() {
        if info.name.to_lowercase().contains("skilleffect") {
            println!("Found: {} ({} bytes)", info.name, info.uncompressed_size);
        }
    }
}
