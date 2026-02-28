use std::path::Path;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::imf::ImfFile;
use ragnarok_formats::pal::PalFile;
use ragnarok_formats::rsm::RsmFile;
use ragnarok_formats::rsw::RswFile;
use ragnarok_formats::spr::SprFile;
use ragnarok_formats::str_effect::StrFile;

fn open_grf() -> GrfArchive {
    let path = ["data/testdata", "../../data/testdata"]
        .iter()
        .map(Path::new)
        .find(|p| p.exists())
        .expect("Test GRF not found — place a GRF at data/testdata (workspace root)");
    GrfArchive::open(path).expect("failed to open GRF")
}

fn open_v1_grf() -> Option<GrfArchive> {
    let path = ["data/data.grf", "../../data/data.grf"]
        .iter()
        .map(Path::new)
        .find(|p| p.exists())?;
    Some(GrfArchive::open(path).expect("failed to open v1.x GRF"))
}

#[test]
fn extract_and_parse_all_formats_from_grf() {
    let grf = open_grf();
    assert!(grf.file_count() > 0);

    // GAT — morroc ruins walkability grid
    let data = grf.read_file("data/moc_ruins.gat").unwrap();
    let gat = GatFile::parse(&data).expect("failed to parse moc_ruins.gat");
    assert!(gat.width > 0 && gat.height > 0);
    assert_eq!(gat.cells.len(), (gat.width * gat.height) as usize);

    // GND — morroc ruins ground mesh
    let data = grf.read_file("data/moc_ruins.gnd").unwrap();
    let gnd = GndFile::parse(&data).expect("failed to parse moc_ruins.gnd");
    assert!(gnd.width > 0 && gnd.height > 0);
    assert_eq!(gnd.cells.len(), (gnd.width * gnd.height) as usize);

    // RSW — morroc ruins world descriptor, verify its GND/GAT references parse
    let data = grf.read_file("data/moc_ruins.rsw").unwrap();
    let rsw = RswFile::parse(&data).expect("failed to parse moc_ruins.rsw");
    assert!(!rsw.gnd_file.is_empty());
    assert!(!rsw.gat_file.is_empty());

    let gnd_ref = format!("data/{}", rsw.gnd_file);
    GndFile::parse(&grf.read_file(&gnd_ref).unwrap())
        .unwrap_or_else(|e| panic!("failed to parse RSW-referenced {gnd_ref}: {e}"));
    let gat_ref = format!("data/{}", rsw.gat_file);
    GatFile::parse(&grf.read_file(&gat_ref).unwrap())
        .unwrap_or_else(|e| panic!("failed to parse RSW-referenced {gat_ref}: {e}"));

    // RSM — tree model
    let data = grf.read_file("data/model/나무잡초꽃/나무01.rsm").unwrap();
    let rsm = RsmFile::parse(&data).expect("failed to parse rsm");
    assert!(!rsm.nodes.is_empty());
    assert!(!rsm.root_node_names.is_empty());

    // SPR — mandragora monster sprite
    let data = grf.read_file("data/sprite/몬스터/mandragora.spr").unwrap();
    let spr = SprFile::parse(&data).expect("failed to parse mandragora.spr");
    assert!(spr.indexed_sprites.len() + spr.rgba_sprites.len() > 0);

    // ACT — mandragora animation
    let data = grf.read_file("data/sprite/몬스터/mandragora.act").unwrap();
    let act = ActFile::parse(&data).expect("failed to parse mandragora.act");
    assert!(!act.actions.is_empty());

    // STR — visual effect
    let data = grf.read_file("data/sprite/이팩트/jong_mini.str").unwrap();
    let str_file = StrFile::parse(&data).expect("failed to parse jong_mini.str");
    assert!(!str_file.layers.is_empty());

    // PAL — swordsman body palette
    let data = grf.read_file("data/palette/몸/검사_남_0.pal").unwrap();
    PalFile::parse(&data).expect("failed to parse pal");

    // IMF — swordsman sprite layer metadata
    let data = grf.read_file("data/imf/검사_남.imf").unwrap();
    let imf = ImfFile::parse(&data).expect("failed to parse imf");
    assert!(!imf.layers.is_empty());
}

#[test]
fn open_v1_grf_and_read_file() {
    let Some(grf) = open_v1_grf() else { return };
    assert!(grf.file_count() > 0);

    let gat_file = grf.find_first_with_extension(".gat")
        .expect("v1 GRF should contain at least one .gat file");
    let data = grf.read_file(gat_file).expect("failed to read .gat from v1 GRF");
    let gat = GatFile::parse(&data).expect("failed to parse .gat from v1 GRF");
    assert!(gat.width > 0 && gat.height > 0);
}
