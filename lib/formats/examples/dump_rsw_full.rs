use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::{RswFile, RswObject};
use std::path::Path;

fn main() {
    let grf_path = std::env::args().nth(1).unwrap_or_else(|| "data/data.grf".into());
    let maps: Vec<String> = std::env::args().skip(2).collect();
    let grf = GrfArchive::open(Path::new(&grf_path)).expect("open grf");

    for map in &maps {
        let path = format!("data/{}.rsw", map);
        let bytes = match grf.read_file(&path) {
            Ok(b) => b,
            Err(e) => { println!("[{}] read error: {}", map, e); continue; }
        };
        let rsw = match RswFile::parse(&bytes) {
            Ok(r) => r,
            Err(e) => { println!("[{}] parse error: {}", map, e); continue; }
        };
        let mut models = 0; let mut lights = 0; let mut sounds = 0; let mut effects = 0;
        let mut effect_samples: Vec<&ragnarok_formats::rsw::RswEffect> = Vec::new();
        for o in &rsw.objects {
            match o {
                RswObject::Model(_) => models += 1,
                RswObject::Light(_) => lights += 1,
                RswObject::Sound(_) => sounds += 1,
                RswObject::Effect(e) => { effects += 1; if effect_samples.len() < 5 { effect_samples.push(e); } }
            }
        }
        println!("[{}] models={} lights={} sounds={} effects={} water_type={:?} water_level={:?}",
            map, models, lights, sounds, effects, rsw.water.water_type, rsw.water.level);
        for (i, e) in effect_samples.iter().enumerate() {
            println!("    effect[{}] type={} name={:?} pos=[{:.1},{:.1},{:.1}] emit={} param={:?}",
                i, e.effect_type, e.name, e.position[0], e.position[1], e.position[2], e.emit_speed, e.param);
        }
    }
}
