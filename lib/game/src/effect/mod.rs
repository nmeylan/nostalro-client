pub mod buckets;
pub mod draw;
pub mod effect_queue;
pub mod effect_trait;
pub mod effects;
pub mod factory;
pub mod spec;
pub mod str_aliases;
pub mod table;

pub use draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
pub use effect_queue::{EffectQueue, SpawnRequest};
pub use effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};
pub use factory::{is_real_impl, make_effect};
pub use spec::{Attach, EffectSpec};
pub use str_aliases::str_aliases;
pub use table::effect_spec;

/// Distinct GRF texture paths used by `Custom`-payload effects, for renderer
/// preload at app boot. Walks each implemented effect module's `TEXTURES`
/// constant; deprecated overlay textures from the family path aren't covered
/// here and load on first spawn.
pub fn effect_texture_paths() -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let texture_lists: &[&[&str]] = &[
        effects::warp::TEXTURES,
        effects::magnum_break::TEXTURES,
        effects::bottom_sanctuary_pillar::TEXTURES,
        effects::warp_zone::TEXTURES,
        effects::volcano::TEXTURES,
        effects::aura::TEXTURES,
        effects::cast_circle::TEXTURES,
        effects::begin_spell_6::TEXTURES,
        effects::stormgust::TEXTURES,
        effects::placeholder::TEXTURES,
    ];
    for list in texture_lists {
        for name in *list {
            seen.insert(format!("data/texture/effect/{name}"));
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
        assert!(
            paths.iter().any(|p| p.ends_with("ring_yellow.tga")),
            "Warp's texture is included",
        );
    }
}
