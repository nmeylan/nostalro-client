use ragnarok_formats::grf::GrfArchive;
use std::path::Path;

fn main() {
    let grf_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ragnarok_resources::grf::DEFAULT_ARCHIVE.into());
    let patterns: Vec<String> = std::env::args().skip(2).collect();
    let grf = GrfArchive::open(Path::new(&grf_path)).expect("open grf");

    let names = grf.file_names();
    if patterns.is_empty() {
        let probes = [
            "torch_01",
            "particle1",
            "particle2",
            "particle7",
            "holyclimb",
            "blessing",
            "fireball",
            "firearrow",
            "chimneysmoke",
            "sight",
            "shadow",
            "snow",
            "maple",
            "forestlight",
            "torch_red",
            "torch_green",
            "endure",
            "alpha_down",
            "alpha_center",
            "ring_yellow",
            "bigbang",
            "ice",
            "sand1",
            "misc/",
            "이팩트",
            "effect/",
        ];
        for probe in probes {
            let mut hits: Vec<&&str> = names.iter().filter(|n| n.contains(probe)).collect();
            hits.sort();
            println!("--- probe={probe:?}  hits={}", hits.len());
            for h in hits.iter().take(8) {
                println!("    {h}");
            }
            if hits.len() > 8 {
                println!("    ... ({} more)", hits.len() - 8);
            }
        }
    } else {
        for pat in patterns {
            let mut hits: Vec<&&str> = names.iter().filter(|n| n.contains(&pat)).collect();
            hits.sort();
            println!("--- pattern={pat:?}  hits={}", hits.len());
            for h in hits {
                println!("    {h}");
            }
        }
    }
}
