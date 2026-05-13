use std::path::Path;

fn main() {
    let grf_path = std::env::args().nth(1).unwrap_or_else(|| "data/data.grf".into());
    let out_dir = std::env::args().nth(2).unwrap_or_else(|| "/tmp/lub_extract".into());
    let grf = ragnarok_formats::grf::GrfArchive::open(Path::new(&grf_path)).expect("open grf");
    
    let dir = Path::new(&out_dir);
    std::fs::create_dir_all(dir).expect("create dir");
    
    for info in grf.file_list() {
        if info.name.to_lowercase().contains("skilleffectinfo") && info.name.ends_with(".lub") {
            let data = grf.read_file(&info.name).expect("read");
            let out_path = dir.join(info.name.split('/').last().unwrap());
            std::fs::write(&out_path, &data).expect("write");
            println!("Extracted: {} -> {:?} ({} bytes)", info.name, out_path, data.len());
        }
    }
}
