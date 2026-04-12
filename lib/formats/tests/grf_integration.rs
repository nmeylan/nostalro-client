use std::path::{Path, PathBuf};

use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::imf::ImfFile;
use ragnarok_formats::pal::PalFile;
use ragnarok_formats::rsm::RsmFile;
use ragnarok_formats::rsw::RswFile;
use ragnarok_formats::spr::SprFile;
use ragnarok_formats::str_effect::StrEffectFile;

fn open_grf() -> Option<GrfArchive> {
    let path = ["data/testdata", "../../data/testdata"]
        .iter()
        .map(Path::new)
        .find(|p| p.exists())?;
    Some(GrfArchive::open(path).expect("failed to open GRF"))
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
    let Some(grf) = open_grf() else {
        eprintln!("Skipping test: data/testdata not found");
        return;
    };
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
    let str_file = StrEffectFile::parse(&data).expect("failed to parse jong_mini.str");
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

fn temp_grf_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("test_grf_{}_{name}.grf", std::process::id()))
}

struct CleanupFile(PathBuf);
impl Drop for CleanupFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn create_and_add_files_roundtrip() {
    let path = temp_grf_path("create_roundtrip");
    let _cleanup = CleanupFile(path.clone());

    let content_a = b"hello world";
    let content_b = vec![0u8; 4096];
    let content_c = b"data in subfolder";

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("readme.txt", content_a).unwrap();
        grf.add_file("data/bigfile.bin", &content_b).unwrap();
        grf.add_file("data/sub/nested.txt", content_c).unwrap();
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 3);
    assert!(grf.file_exists("readme.txt"));
    assert!(grf.file_exists("data/bigfile.bin"));
    assert!(grf.file_exists("data/sub/nested.txt"));
    assert_eq!(grf.read_file("readme.txt").unwrap(), content_a);
    assert_eq!(grf.read_file("data/bigfile.bin").unwrap(), content_b);
    assert_eq!(grf.read_file("data/sub/nested.txt").unwrap(), content_c);
}

#[test]
fn remove_file_and_reopen() {
    let path = temp_grf_path("remove");
    let _cleanup = CleanupFile(path.clone());

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("keep.txt", b"keep me").unwrap();
        grf.add_file("delete.txt", b"delete me").unwrap();
        grf.save().unwrap();
    }

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        assert_eq!(grf.file_count(), 2);
        assert!(grf.remove_file("delete.txt").unwrap());
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 1);
    assert!(grf.file_exists("keep.txt"));
    assert!(!grf.file_exists("delete.txt"));
    assert_eq!(grf.read_file("keep.txt").unwrap(), b"keep me");
}

#[test]
fn repack_reclaims_space() {
    let path = temp_grf_path("repack");
    let _cleanup = CleanupFile(path.clone());

    let large_data = vec![42u8; 10_000];

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("large.bin", &large_data).unwrap();
        grf.save().unwrap();
    }

    let size_before_remove = std::fs::metadata(&path).unwrap().len();

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        grf.remove_file("large.bin").unwrap();
        grf.add_file("small.txt", b"tiny").unwrap();
        grf.save().unwrap();
    }

    let size_after_remove = std::fs::metadata(&path).unwrap().len();
    // File table shrank but orphaned data remains
    assert!(size_after_remove > 0);

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        grf.repack().unwrap();
    }

    let size_after_repack = std::fs::metadata(&path).unwrap().len();
    assert!(size_after_repack < size_before_remove);

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 1);
    assert_eq!(grf.read_file("small.txt").unwrap(), b"tiny");
}

#[test]
fn add_file_overwrites_existing() {
    let path = temp_grf_path("overwrite");
    let _cleanup = CleanupFile(path.clone());

    {
        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("data/test.txt", b"version A").unwrap();
        grf.save().unwrap();
    }

    {
        let mut grf = GrfArchive::open_rw(&path).unwrap();
        grf.add_file("data/test.txt", b"version B").unwrap();
        grf.save().unwrap();
    }

    let grf = GrfArchive::open(&path).unwrap();
    assert_eq!(grf.file_count(), 1);
    assert_eq!(grf.read_file("data/test.txt").unwrap(), b"version B");
}
