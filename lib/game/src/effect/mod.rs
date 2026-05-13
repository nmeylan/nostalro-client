pub mod effect_queue;
pub mod effect_id;
pub mod spec;
pub mod str_aliases;
pub mod table;

pub use effect_queue::{EffectQueue, SpawnRequest};
pub use effect_id::{
    classified_family, default_duration_ms, default_str_file, effect_ef_name, effect_name,
    skill_effect, EffectId, ALL_EFFECT_IDS,
};
pub use spec::{
    Attach, CustomFamily, CustomFamilyParams, EffectBlend, EffectSpec, GroundRingParams,
};
pub use str_aliases::str_aliases;
pub use table::effect_spec;

/// Distinct GRF texture paths referenced by any `EffectSpec::Custom` in the
/// effect table. Empty-string textures are skipped. Used by the renderer to
/// preload effect textures at app boot so first-spawn doesn't hitch.
pub fn effect_texture_paths() -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    for &id in ALL_EFFECT_IDS {
        let Some(spec) = effect_spec(id) else {
            continue;
        };
        let EffectSpec::Custom { params, .. } = spec else {
            continue;
        };
        match params {
            CustomFamilyParams::GroundRing(p) if !p.texture.is_empty() => {
                seen.insert(format!("data/texture/effect/{}", p.texture));
            }
            _ => {}
        }
    }
    seen.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_texture_paths_returns_unique_names() {
        let paths = effect_texture_paths();
        let mut sorted = paths.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(paths.len(), sorted.len(), "no duplicates");
        assert!(paths.iter().all(|p| !p.is_empty()), "no empty entries");
        assert!(
            paths.iter().all(|p| p.starts_with("data/texture/effect/")),
            "all entries are GRF effect paths",
        );
        // Hand-curated rows landed in `table.rs` — at least the five GroundRing
        // overrides should appear here.
        assert!(
            paths.iter().any(|p| p.ends_with("magic_target.tga")),
            "Warp texture is included",
        );
    }
}
