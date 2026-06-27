use ragnarok_formats::grf::GrfArchive;
use std::path::Path;

fn main() {
    let grf_path = Path::new("data/data.grf");
    let grf = GrfArchive::open(grf_path).expect("Failed to open GRF");

    let names = [
        (
            "Knight peco (13)",
            "data/sprite/인간족/몸통/남/페코페코_기사_남",
        ),
        (
            "Crusader peco (21)",
            "data/sprite/인간족/몸통/남/신페코크루세이더_남",
        ),
        (
            "Crusader peco F (21)",
            "data/sprite/인간족/몸통/여/신페코크루세이더_여",
        ),
        (
            "Lord Knight peco (4014)",
            "data/sprite/인간족/몸통/남/로드페코_남",
        ),
        (
            "Paladin peco (4022)",
            "data/sprite/인간족/몸통/남/페코팔라딘_남",
        ),
    ];

    println!("=== Checking peco sprite files ===");
    for (label, path) in &names {
        let spr = format!("{}.spr", path);
        let act = format!("{}.act", path);
        let has_spr = grf.file_exists(&spr);
        let has_act = grf.file_exists(&act);
        println!("{}: spr={} act={}", label, has_spr, has_act);
    }

    println!("\n=== All files containing 페코 or crusader mount patterns ===");
    for name in grf.file_names() {
        let lower = name.to_lowercase();
        if name.contains("페코") || lower.contains("peco") || name.contains("크루세이더") {
            if name.contains("sprite") {
                println!("  {}", name);
            }
        }
    }
}
