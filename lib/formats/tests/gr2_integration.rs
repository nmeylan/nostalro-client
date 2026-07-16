use std::path::Path;

use ragnarok_formats::gr2::{Gr2Container, Gr2File};
use ragnarok_formats::grf::GrfArchive;

fn open_any_grf() -> Option<GrfArchive> {
    for p in ["data/data.grf", "../../data/data.grf", "../../../../data/data.grf"] {
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
        let bytes = grf.read_file(name).expect("read gr2");
        let c = Gr2Container::parse(&bytes).expect("parse");

        let text = String::from_utf8_lossy(&c.data);
        let markers = ["Position", "ArtToolInfo", "Materials", "Textures"];
        assert!(
            markers.iter().any(|m| text.contains(m)),
            "{name}: no recognizable Granny type strings in decompressed data",
        );
    }
}

#[test]
fn extract_gr2_model() {
    let Some(grf) = open_any_grf() else {
        eprintln!("skip: no grf");
        return;
    };

    let bytes = grf
        .read_file("data/model/3dmob/empelium90_0.gr2")
        .expect("read emperium");
    let c = Gr2Container::parse(&bytes).expect("parse");
    let f = Gr2File::parse(&c).expect("extract");

    let model = f.models.first().expect("model");
    let skeleton_index = model.skeleton_index.expect("skeleton bound");
    assert_eq!(f.skeletons[skeleton_index].bones.len(), 18);
    assert!(!model.mesh_indices.is_empty());

    let mesh = &f.meshes[model.mesh_indices[0]];
    let vd = &f.vertex_datas[mesh.vertex_data_index.expect("vertexdata bound")];
    let topo = &f.tri_topologies[mesh.topology_index.expect("topology bound")];
    assert_eq!(vd.vertices.len(), 159);
    assert_eq!(topo.indices.len(), 414);
    assert_eq!(topo.groups[0].tri_count, 138);
    assert!(topo.indices.iter().all(|&i| (i as usize) < vd.vertices.len()));

    let tex = f.textures.first().expect("texture");
    assert_eq!((tex.width, tex.height), (256, 256));
    assert!(tex.from_file_name.contains("empelium"));

    let anim = f.animations.first().expect("animation");
    assert!(anim.duration > 0.0);
    assert!(!anim.track_group_indices.is_empty());
    let tg = &f.track_groups[anim.track_group_indices[0]];
    assert_eq!(tg.transform_tracks.len(), 18);

    // A multi-mesh guardian: every mesh must resolve its vertex data and topology,
    // and every index must stay within its own mesh's vertex buffer.
    let bytes = grf
        .read_file("data/model/3dmob/kguardian90_7.gr2")
        .expect("read guardian");
    let c = Gr2Container::parse(&bytes).expect("parse");
    let f = Gr2File::parse(&c).expect("extract");
    let model = f.models.first().expect("model");
    assert_eq!(model.mesh_indices.len(), 5);
    assert_eq!(f.skeletons[model.skeleton_index.unwrap()].bones.len(), 35);
    for &mi in &model.mesh_indices {
        let mesh = &f.meshes[mi];
        let vd = &f.vertex_datas[mesh.vertex_data_index.expect("vertexdata bound")];
        let topo = &f.tri_topologies[mesh.topology_index.expect("topology bound")];
        assert!(!vd.vertices.is_empty());
        assert!(topo.indices.iter().all(|&i| (i as usize) < vd.vertices.len()));
    }

    // A skeleton-only animation file: an animation with tracks but no meshes.
    let bytes = grf
        .read_file("data/model/3dmob_bone/9_attack.gr2")
        .expect("read animation");
    let c = Gr2Container::parse(&bytes).expect("parse");
    let f = Gr2File::parse(&c).expect("extract");
    assert!(f.meshes.is_empty());
    let anim = f.animations.first().expect("animation");
    let tg = &f.track_groups[anim.track_group_indices[0]];
    assert!(!tg.transform_tracks.is_empty());
}




