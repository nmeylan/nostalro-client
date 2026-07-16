use std::path::Path;

use glam::Mat4;
use ragnarok_formats::gr2::{Gr2Container, Gr2File};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::gr2_model::{AnimationClip, SkeletonPose};

fn open_any_grf() -> Option<GrfArchive> {
    for p in ["data/data.grf", "../../data/data.grf"] {
        if Path::new(p).exists() {
            return Some(GrfArchive::open(Path::new(p)).expect("open grf"));
        }
    }
    None
}

fn load(grf: &GrfArchive, name: &str) -> Gr2File {
    let bytes = grf.read_file(name).expect("read gr2");
    let container = Gr2Container::parse(&bytes).expect("parse container");
    Gr2File::parse(&container).expect("extract file info")
}

#[test]
fn bind_pose_is_identity_palette() {
    let Some(grf) = open_any_grf() else {
        eprintln!("skip: no grf");
        return;
    };
    // inverse_world[i] == inverse(bind_world[i]), so the bind skinning palette
    // must be the identity for every bone — this validates local composition,
    // world accumulation, and the matrix convention together.
    for name in [
        "data/model/3dmob/empelium90_0.gr2",
        "data/model/3dmob/kguardian90_7.gr2",
    ] {
        let file = load(&grf, name);
        let skeleton = SkeletonPose::from_model(&file, 0).expect("skeleton");
        for m in skeleton.bind_palette() {
            assert!(
                m.abs_diff_eq(Mat4::IDENTITY, 1e-3),
                "{name}: bind palette not identity: {m:?}",
            );
        }
    }
}

#[test]
fn animation_poses_the_skeleton() {
    let Some(grf) = open_any_grf() else {
        eprintln!("skip: no grf");
        return;
    };

    // Stand animation embedded in the model file.
    let emperium = load(&grf, "data/model/3dmob/empelium90_0.gr2");
    let skeleton = SkeletonPose::from_model(&emperium, 0).expect("skeleton");
    let clip = AnimationClip::from_gr2(&emperium, 0).expect("clip");
    assert!(clip.duration > 0.0);

    let palette = clip.skinning_palette(&skeleton, clip.duration * 0.5);
    assert_eq!(palette.len(), skeleton.bone_count());
    assert!(palette.iter().all(|m| m.is_finite()));
    assert!(
        palette.iter().any(|m| !m.abs_diff_eq(Mat4::IDENTITY, 1e-3)),
        "animation produced no movement",
    );

    // A separate skeleton-less animation file applied to a guardian's skeleton,
    // matched track-to-bone by name.
    let guardian = load(&grf, "data/model/3dmob/kguardian90_7.gr2");
    let attack = load(&grf, "data/model/3dmob_bone/7_attack.gr2");
    let guardian_skeleton = SkeletonPose::from_model(&guardian, 0).expect("skeleton");
    let attack_clip = AnimationClip::from_gr2(&attack, 0).expect("attack clip");
    let posed = attack_clip.skinning_palette(&guardian_skeleton, attack_clip.duration * 0.5);
    assert_eq!(posed.len(), guardian_skeleton.bone_count());
    assert!(posed.iter().all(|m| m.is_finite()));
    assert!(posed.iter().any(|m| !m.abs_diff_eq(Mat4::IDENTITY, 1e-3)));
}
