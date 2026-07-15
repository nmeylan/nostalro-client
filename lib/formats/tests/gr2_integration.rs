use std::path::Path;

use ragnarok_formats::gr2::Gr2Container;
use ragnarok_formats::grf::GrfArchive;

fn open_any_grf() -> Option<GrfArchive> {
    for p in ["data/data.grf", "../../data/data.grf"] {
        if Path::new(p).exists() {
            return Some(GrfArchive::open(Path::new(p)).expect("open grf"));
        }
    }
    None
}

#[test]
fn list_gr2_entries() {
    let Some(grf) = open_any_grf() else {
        eprintln!("skip: no grf");
        return;
    };
    let mut names: Vec<&str> = grf
        .entry_names()
        .filter(|n| n.to_ascii_lowercase().ends_with(".gr2"))
        .collect();
    names.sort();
    eprintln!("found {} gr2 entries", names.len());
    for n in names.iter().take(40) {
        eprintln!("  {n}");
    }
}

#[test]
fn probe_gr2() {
    let Some(grf) = open_any_grf() else {
        eprintln!("skip: no grf");
        return;
    };
    for name in [
        "data/model/3dmob/empelium90_0.gr2",
        "data/model/3dmob/kguardian90_7.gr2",
        "data/model/3dmob_bone/9_attack.gr2",
    ] {
        match grf.read_file(name) {
            Ok(bytes) => {
                eprintln!("read {name}: {} bytes", bytes.len());
                let c = Gr2Container::parse(&bytes).expect("parse");
                eprintln!(
                    "  parsed v{} data={} sectors={}",
                    c.version,
                    c.data.len(),
                    c.sectors.len()
                );
                // Collect printable ASCII runs to eyeball recognizable Granny strings.
                let mut run = Vec::new();
                let mut found = Vec::new();
                for &b in &c.data {
                    if b.is_ascii_graphic() || b == b' ' {
                        run.push(b);
                    } else {
                        if run.len() >= 5 {
                            found.push(String::from_utf8_lossy(&run).into_owned());
                        }
                        run.clear();
                    }
                }
                eprintln!("  strings sample: {:?}", &found[..found.len().min(30)]);
            }
            Err(e) => eprintln!("read {name} FAILED: {e:?}"),
        }
    }
}
