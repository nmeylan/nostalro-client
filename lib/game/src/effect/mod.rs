pub mod buckets;
pub mod draw;
pub mod effect_queue;
pub mod effect_trait;
pub mod effects;
pub mod factory;
pub mod spec;
pub mod spr_aliases;
pub mod spr_burst;
pub mod str_aliases;
pub mod table;

pub use draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
pub use effect_queue::{EffectQueue, SpawnRequest, is_trail_effect};
pub use effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};
pub use factory::{is_real_impl, make_effect};
pub use spec::{AlphaKeyframe, Attach, CurveParams, EffectSpec, SprBurstParams};
pub use spr_aliases::{SprDef, spr_def};
pub use spr_burst::spr_burst_params;
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
        effects::bottom_song::TEXTURES,
        effects::bottom_hermode::TEXTURES,
        effects::bottom_landprotector::TEXTURES,
        effects::bottom_light::TEXTURES,
        effects::bottom_magnus::TEXTURES,
        effects::bottom_out::TEXTURES,
        effects::bottom_vertical::TEXTURES,
        effects::hit::TEXTURES,
        effects::hit2::TEXTURES,
        effects::hit5_6::TEXTURES,
        effects::bottom_sanctuary_pillar::TEXTURES,
        effects::warp_zone::TEXTURES,
        effects::volcano::TEXTURES,
        effects::aura::TEXTURES,
        effects::bash::TEXTURES,
        effects::flasher::TEXTURES,
        effects::hasteup::TEXTURES,
        effects::blessing::TEXTURES,
        effects::cast_circle::TEXTURES,
        effects::endure::TEXTURES,
        effects::enhance::TEXTURES,
        effects::entry::TEXTURES,
        effects::exit::TEXTURES,
        effects::glasswall::TEXTURES,
        effects::healsp::TEXTURES,
        effects::frost_diver::TEXTURES,
        effects::begin_spell_6::TEXTURES,
        effects::stormgust::TEXTURES,
        effects::animated_texture_billboard::TEXTURES,
        effects::placeholder::TEXTURES,
        effects::portal::TEXTURES,
        effects::ready_portal::TEXTURES,
        effects::teleportation::TEXTURES,
        effects::spraypond::TEXTURES,
        effects::status_up::TEXTURES,
        effects::firearrow::TEXTURES,
        effects::napalmbeat::TEXTURES,
        effects::sandwind::TEXTURES,
        effects::yupitel::TEXTURES,
    ];
    for list in texture_lists {
        for name in *list {
            seen.insert(format!("data/texture/effect/{name}"));
        }
    }
    seen.into_iter().collect()
}

/// GRF sprite paths used by Custom-effect modules that emit
/// `EffectPrimitiveDraw::SpriteParticle` entries. Callers preload these
/// into the renderer's `EffectSpriteCache` so debris and other
/// per-particle SPR billboards render the first frame they're emitted
/// instead of silently skipping. Each effect module that uses
/// SpriteParticle declares a `SPRITES` constant; this aggregator walks
/// them so the preload list stays in sync with the source files.
pub fn custom_effect_sprite_paths() -> Vec<&'static str> {
    let mut seen = std::collections::BTreeSet::new();
    let sprite_lists: &[&[&str]] = &[
        effects::hit::SPRITES,
        effects::sight::SPRITES,
        effects::exit::SPRITES,
        effects::hasteup::SPRITES,
        effects::firearrow::SPRITES,
        effects::fireball::SPRITES,
        effects::soul_strike::SPRITES,
        effects::healsp::SPRITES,
        effects::blessing::SPRITES,
    ];
    for list in sprite_lists {
        for path in *list {
            seen.insert(*path);
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
