use std::path::Path;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::pal::PalFile;
use ragnarok_formats::rsm::RsmFile;
use ragnarok_formats::rsw::RswFile;
use ragnarok_formats::spr::SprFile;
use ragnarok_formats::str_effect::StrFile;

fn grf_path() -> std::path::PathBuf {
    for candidate in ["data/testdata", "../../data/testdata"] {
        let p = Path::new(candidate);
        if p.exists() {
            return p.to_path_buf();
        }
    }
    panic!("Test GRF not found — place a GRF at data/testdata (workspace root)");
}

fn open_grf() -> GrfArchive {
    GrfArchive::open(&grf_path()).expect("failed to open GRF")
}

#[test]
fn extract_and_parse_all_formats_from_grf() {
    let grf = open_grf();
    assert!(grf.file_count() > 0);

    // GAT
    let name = grf.find_first_with_extension(".gat").expect("no .gat in GRF").to_string();
    let gat = GatFile::parse(&grf.read_file(&name).unwrap()).expect("failed to parse .gat");
    assert!(gat.width > 0 && gat.height > 0);
    assert_eq!(gat.cells.len(), (gat.width * gat.height) as usize);

    // GND
    let name = grf.find_first_with_extension(".gnd").expect("no .gnd in GRF").to_string();
    let gnd = GndFile::parse(&grf.read_file(&name).unwrap()).expect("failed to parse .gnd");
    assert!(gnd.width > 0 && gnd.height > 0);
    assert_eq!(gnd.cells.len(), (gnd.width * gnd.height) as usize);

    // RSW — also verify its GND/GAT references are parseable
    let rsw_name = grf.find_first_with_extension(".rsw").expect("no .rsw in GRF").to_string();
    let rsw = RswFile::parse(&grf.read_file(&rsw_name).unwrap()).expect("failed to parse .rsw");
    assert!(!rsw.gnd_file.is_empty());
    assert!(!rsw.gat_file.is_empty());

    let prefix = rsw_name.rsplit_once('/').map(|(p, _)| format!("{p}/")).unwrap_or_default();
    let gnd_ref = format!("{prefix}{}", rsw.gnd_file);
    if grf.file_exists(&gnd_ref) {
        GndFile::parse(&grf.read_file(&gnd_ref).unwrap()).expect("failed to parse RSW-referenced .gnd");
    }
    let gat_ref = format!("{prefix}{}", rsw.gat_file);
    if grf.file_exists(&gat_ref) {
        GatFile::parse(&grf.read_file(&gat_ref).unwrap()).expect("failed to parse RSW-referenced .gat");
    }

    // RSM
    let name = grf.find_first_with_extension(".rsm").expect("no .rsm in GRF").to_string();
    let rsm = RsmFile::parse(&grf.read_file(&name).unwrap()).expect("failed to parse .rsm");
    assert!(!rsm.nodes.is_empty());
    assert!(!rsm.root_node_names.is_empty());

    // SPR
    let name = grf.find_first_with_extension(".spr").expect("no .spr in GRF").to_string();
    let spr = SprFile::parse(&grf.read_file(&name).unwrap()).expect("failed to parse .spr");
    assert!(spr.indexed_sprites.len() + spr.rgba_sprites.len() > 0);

    // ACT
    let name = grf.find_first_with_extension(".act").expect("no .act in GRF").to_string();
    let act = ActFile::parse(&grf.read_file(&name).unwrap()).expect("failed to parse .act");
    assert!(!act.actions.is_empty());

    // STR
    let name = grf.find_first_with_extension(".str").expect("no .str in GRF").to_string();
    let str_file = StrFile::parse(&grf.read_file(&name).unwrap()).expect("failed to parse .str");
    assert!(!str_file.layers.is_empty());

    // PAL
    let name = grf.find_first_with_extension(".pal").expect("no .pal in GRF").to_string();
    PalFile::parse(&grf.read_file(&name).unwrap()).expect("failed to parse .pal");
}
