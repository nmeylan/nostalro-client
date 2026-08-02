//! Follows references from one GRF entry to the next until nothing new turns up.

use super::{Missing, Need, Origin, normalize};
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsm::RsmFile;
use ragnarok_formats::rsw::{RswFile, RswObject};
use ragnarok_formats::str_effect::StrEffectFile;
use ragnarok_game::gr2_model::{Gr2Action, animation_file_path, bone_type_from_name};
use std::collections::{HashSet, VecDeque};

pub struct Walk {
    pub reached: HashSet<String>,
    pub missing: Vec<Missing>,
}

/// How badly a referenced file is wanted. A sprite cannot animate without its
/// `.act`, but an `.act` may drive a sprite it does not name — pet headgear
/// animations ride on the pet's own sprite — so that direction is optional.
fn need_for(referrer: &str, target: &str) -> Need {
    if referrer.ends_with(".act") && target.ends_with(".spr") {
        Need::Probed
    } else {
        Need::Required
    }
}

/// Sprites are named without an extension; the client loads `.spr` and `.act`
/// together. A root with no extension stands for that pair.
const SPRITE_PAIR: [&str; 2] = ["spr", "act"];

fn extension(path: &str) -> Option<&str> {
    let file = path.rsplit('/').next()?;
    file.rsplit_once('.').map(|(_, e)| e)
}

pub fn walk(grf: &GrfArchive, roots: Vec<(String, Origin, Need)>) -> Walk {
    let mut queue: VecDeque<(String, Origin, Need)> = VecDeque::new();
    for (path, origin, need) in roots {
        for p in expand_extensionless(&path) {
            queue.push_back((p, origin.clone(), need));
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut reached: HashSet<String> = HashSet::new();
    let mut missing = Vec::new();

    while let Some((path, origin, need)) = queue.pop_front() {
        if !seen.insert(path.clone()) {
            continue;
        }
        if !grf.file_exists(&path) {
            missing.push(Missing { path, origin, need });
            continue;
        }
        for next in references(grf, &path) {
            let need = need_for(&path, &next);
            queue.push_back((next, Origin::File(path.clone()), need));
        }
        reached.insert(path);
    }

    Walk { reached, missing }
}

/// A path with no extension is a sprite base name; everything else stands alone.
fn expand_extensionless(path: &str) -> Vec<String> {
    if path.ends_with('/') {
        return Vec::new(); // a directory prefix, not a file
    }
    match extension(path) {
        Some(_) => vec![path.to_string()],
        None => SPRITE_PAIR
            .iter()
            .map(|ext| format!("{path}.{ext}"))
            .collect(),
    }
}

/// Everything `path` names, as normalized GRF entry names.
fn references(grf: &GrfArchive, path: &str) -> Vec<String> {
    let Some(ext) = extension(path) else {
        return Vec::new();
    };
    // Only these formats point at other files; reading the rest would be wasted IO.
    if !matches!(ext, "rsw" | "gnd" | "rsm" | "str" | "spr" | "act" | "gr2") {
        return Vec::new();
    }

    // The sprite/animation pair references each other by name, no read needed.
    if let Some(stem) = path.strip_suffix(".spr") {
        return vec![format!("{stem}.act")];
    }
    if let Some(stem) = path.strip_suffix(".act") {
        return vec![format!("{stem}.spr")];
    }
    if ext == "gr2" {
        let file = path.rsplit('/').next().unwrap_or(path);
        let Some(bone) = bone_type_from_name(file) else {
            return Vec::new();
        };
        return Gr2Action::ALL
            .iter()
            .filter_map(|a| animation_file_path(bone, *a))
            .map(|p| normalize(&p))
            .collect();
    }

    let Ok(data) = grf.read_file(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match ext {
        "rsw" => {
            let Ok(rsw) = RswFile::parse(&data) else {
                return out;
            };
            for f in [&rsw.gnd_file, &rsw.gat_file, &rsw.ini_file] {
                if !f.is_empty() {
                    out.push(normalize(&ragnarok_resources::map::file(f)));
                }
            }
            if rsw.water.level.is_some() {
                let water_type = rsw.water.water_type.unwrap_or(0);
                out.extend(
                    ragnarok_renderer::water::water_texture_candidates(water_type)
                        .iter()
                        .map(|t| normalize(t)),
                );
            }
            for object in &rsw.objects {
                match object {
                    RswObject::Model(m) => {
                        out.push(normalize(&format!("data/model/{}", m.model_name)))
                    }
                    RswObject::Sound(s) if !s.file_name.is_empty() => {
                        out.push(normalize(&ragnarok_resources::sound::sfx(&s.file_name)))
                    }
                    RswObject::Sound(_) => {}
                    // Effects are numeric ids resolved by the effect tables, and
                    // lights reference nothing.
                    RswObject::Effect(_) | RswObject::Light(_) => {}
                }
            }
        }
        "gnd" => {
            if let Ok(gnd) = GndFile::parse(&data) {
                out.extend(
                    gnd.textures
                        .iter()
                        .map(|t| normalize(&ragnarok_resources::texture::named(t))),
                );
            }
        }
        "rsm" => {
            if let Ok(rsm) = RsmFile::parse(&data) {
                out.extend(
                    rsm.textures
                        .iter()
                        .map(|t| normalize(&ragnarok_resources::texture::named(t))),
                );
            }
        }
        "str" => {
            // STR texture names are relative to the folder holding the .str.
            let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            if let Ok(effect) = StrEffectFile::parse(&data) {
                for layer in &effect.layers {
                    out.extend(
                        layer
                            .textures
                            .iter()
                            .filter(|t| !t.is_empty())
                            .map(|t| normalize(&format!("{dir}/{t}"))),
                    );
                }
            }
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::grf::GrfArchive;

    /// A map drags in its ground, its models and their textures; a sprite drags
    /// in its animation; and a root naming a file the archive lacks is reported
    /// rather than silently dropped.
    #[test]
    fn walk_follows_references_and_reports_gaps() {
        let dir = std::env::temp_dir().join("grf_audit_closure_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.grf");

        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file("data/sprite/poring.spr", b"spr").unwrap();
        grf.add_file("data/sprite/poring.act", b"act").unwrap();
        grf.add_file("data/texture/orphan.bmp", b"bmp").unwrap();
        grf.save().unwrap();
        drop(grf);

        let grf = GrfArchive::open(&path).unwrap();
        let walk = walk(
            &grf,
            vec![
                (
                    "data/sprite/poring".to_string(),
                    Origin::Registry,
                    Need::Required,
                ),
                (
                    "data/sprite/missing.spr".to_string(),
                    Origin::Registry,
                    Need::Required,
                ),
            ],
        );

        assert!(walk.reached.contains("data/sprite/poring.spr"));
        assert!(
            walk.reached.contains("data/sprite/poring.act"),
            "the .act is reached through its .spr"
        );
        assert!(
            !walk.reached.contains("data/texture/orphan.bmp"),
            "nothing references the orphan"
        );

        let missing: Vec<&str> = walk.missing.iter().map(|m| m.path.as_str()).collect();
        assert!(missing.contains(&"data/sprite/missing.spr"));
        assert!(
            !missing.contains(&"data/sprite/poring.act"),
            "a present file is never reported missing"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
