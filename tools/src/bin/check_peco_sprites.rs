use ragnarok_formats::grf::GrfArchive;
use std::path::Path;

fn main() {
    let grf_path = Path::new(ragnarok_resources::grf::DEFAULT_ARCHIVE);
    let grf = GrfArchive::open(grf_path).expect("Failed to open GRF");

    let names = [
        (
            "Knight peco (13)",
            ragnarok_resources::sprite::player::PECO_KNIGHT_MALE,
        ),
        (
            "Crusader peco (21)",
            ragnarok_resources::sprite::player::GRAND_PECO_CRUSADER_MALE,
        ),
        (
            "Crusader peco F (21)",
            ragnarok_resources::sprite::player::GRAND_PECO_CRUSADER_FEMALE,
        ),
        (
            "Lord Knight peco (4014)",
            ragnarok_resources::sprite::player::LORD_PECO_MALE,
        ),
        (
            "Paladin peco (4022)",
            ragnarok_resources::sprite::player::PECO_PALADIN_MALE,
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
