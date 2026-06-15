pub mod buckets;
pub mod draw;
pub mod effect_queue;
pub mod effect_trait;
pub mod effects;
pub mod factory;
pub mod radial_emitter;
pub mod spec;
pub mod spr_aliases;
pub mod spr_burst;
pub mod str_aliases;
pub mod table;

pub use draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
pub use effect_queue::{
    EffectQueue, SpawnRequest, body_attached, is_count_point_effect, is_link_effect,
    is_trail_effect,
};
pub use effect_trait::{
    Afterimage, BodyAction, BodyCopy, BodyTint, BodyVertical, CameraShake, CameraView, Effect,
    EffectRenderCtx, EffectUpdateCtx,
};
pub use factory::{is_real_impl, make_effect};
pub use spec::{AlphaKeyframe, Attach, CurveParams, EffectSpec, SprBurstParams};
pub use spr_aliases::{SprDef, spr_def};
pub use spr_burst::spr_burst_params;
pub use str_aliases::str_aliases;
pub use table::{effect_spec, spawn_camera_shake};

/// Distinct GRF texture paths used by `Custom`-payload effects, for renderer
/// preload at app boot. Walks each implemented effect module's `TEXTURES`
/// constant; deprecated overlay textures from the family path aren't covered
/// here and load on first spawn.
pub fn effect_texture_paths() -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let texture_lists: &[&[&str]] = &[
        effects::begin_asura::TEXTURES,
        effects::warp::TEXTURES,
        effects::magnum_break::TEXTURES,
        effects::dome_ring::TEXTURES,
        effects::bottom_song::TEXTURES,
        effects::bottom_hermode::TEXTURES,
        effects::bottom_landprotector::TEXTURES,
        effects::bottom_light::TEXTURES,
        effects::forest_light::TEXTURES,
        effects::linelink::TEXTURES,
        effects::guard::TEXTURES,
        effects::bottom_magnus::TEXTURES,
        effects::bottom_out::TEXTURES,
        effects::bottom_vertical::TEXTURES,
        effects::hit::TEXTURES,
        effects::hit2::TEXTURES,
        effects::hit5_6::TEXTURES,
        effects::bottom_sanctuary_pillar::TEXTURES,
        effects::warp_zone::TEXTURES,
        effects::volcano::TEXTURES,
        effects::gumgang::TEXTURES,
        effects::floor_aura::TEXTURES,
        effects::casting_ring::TEXTURES,
        effects::color_casting::TEXTURES,
        effects::black_devil::TEXTURES,
        effects::electric::TEXTURES,
        effects::hit_line::TEXTURES,
        effects::sparkle_column::TEXTURES,
        effects::bash::TEXTURES,
        effects::flasher::TEXTURES,
        effects::hasteup::TEXTURES,
        effects::blessing::TEXTURES,
        effects::cast_circle::TEXTURES,
        effects::chemical::TEXTURES,
        effects::sightrasher::TEXTURES,
        effects::firesplashhit::TEXTURES,
        effects::coldhit::TEXTURES,
        effects::endure::TEXTURES,
        effects::enhance::TEXTURES,
        effects::entry::TEXTURES,
        effects::exit::TEXTURES,
        effects::glasswall::TEXTURES,
        effects::glasswall2::TEXTURES,
        effects::providence::TEXTURES,
        effects::healsp::TEXTURES,
        effects::frost_diver::TEXTURES,
        effects::fullscreen_overlay::TEXTURES,
        effects::begin_spell_6::TEXTURES,
        effects::stormgust::TEXTURES,
        effects::animated_texture_billboard::TEXTURES,
        effects::placeholder::TEXTURES,
        effects::portal::TEXTURES,
        effects::portal2::TEXTURES,
        effects::portal_wind::TEXTURES,
        effects::ready_portal::TEXTURES,
        effects::teleportation::TEXTURES,
        effects::spraypond::TEXTURES,
        effects::status_up::TEXTURES,
        effects::firearrow::TEXTURES,
        effects::napalmbeat::TEXTURES,
        effects::sandwind::TEXTURES,
        effects::yupitel::TEXTURES,
        effects::bowling_bash::TEXTURES,
        effects::overthrust::TEXTURES,
        effects::callzone::TEXTURES,
        effects::ground_sample::TEXTURES,
        effects::sonicblowhit::TEXTURES,
        effects::cartrevolution::TEXTURES,
        effects::barrier::TEXTURES,
        effects::turnundead::TEXTURES,
        effects::firepillaron::TEXTURES,
        effects::hitdark::TEXTURES,
        effects::pierce::TEXTURES,
        effects::gumgang2::TEXTURES,
        effects::defender::TEXTURES,
        effects::heal::TEXTURES,
        effects::big_portal::TEXTURES,
        effects::attack_energy::TEXTURES,
        effects::wind::TEXTURES,
        effects::bash3d::TEXTURES,
        effects::blitzbeat::TEXTURES,
        effects::curseattack::TEXTURES,
        effects::detecting::TEXTURES,
        effects::earthspike::TEXTURES,
        effects::heavensdrive::TEXTURES,
        effects::bottom_box::TEXTURES,
        effects::flowercast::TEXTURES,
        effects::fireivy::TEXTURES,
        effects::grimtooth_atk::TEXTURES,
        effects::icewall::TEXTURES,
        effects::party::TEXTURES,
        effects::foot::TEXTURES,
        effects::teihit::TEXTURES,
        effects::particle_up::TEXTURES,
        effects::peong_up::TEXTURES,
        effects::peong::TEXTURES,
        effects::heartcasting::TEXTURES,
        effects::colorpaper::TEXTURES,
        effects::gravitation::TEXTURES,
        effects::storm_kick::TEXTURES,
        effects::effect_texture::TEXTURES,
        effects::tarot_card::TEXTURES,
        effects::temp_result::TEXTURES,
        effects::toprank::TEXTURES,
        effects::lockon::TEXTURES,
        effects::waterball::TEXTURES,
        effects::yufitel2::TEXTURES,
        effects::texture_falling::TEXTURES,
        effects::aciddemon::TEXTURES,
        effects::rainbow::TEXTURES,
        effects::agiup::TEXTURES,
        effects::light_sphere::TEXTURES,
        effects::throw_item::TEXTURES,
        effects::rg_coin::TEXTURES,
        effects::cloud_projectile::TEXTURES,
        effects::twilight::TEXTURES,
        effects::pressure::TEXTURES,
        effects::mapzone::TEXTURES,
        effects::waterfall::TEXTURES,
        effects::cloud::TEXTURES,
        effects::stin::TEXTURES,
        effects::sma::TEXTURES,
        effects::soul_breaker::TEXTURES,
        effects::slash::TEXTURES,
        effects::thunderstorm2::TEXTURES,
        effects::summon_slave::TEXTURES,
        effects::bubble_drop::TEXTURES,
        effects::cartter::TEXTURES,
        effects::ice_arrow::TEXTURES,
    ];
    for list in texture_lists {
        for name in *list {
            // A name may list `|`-separated alias candidates; preload every
            // candidate so whichever the GRF actually has is in the cache.
            // Bare names resolve under the effect texture dir; names that
            // already carry a path (e.g. thrown item icons under
            // `유저인터페이스/item/`) are relative to `data/texture/`. Must
            // mirror the renderer's `texture_lookup`.
            for candidate in name.split('|') {
                if candidate.contains('/') {
                    seen.insert(format!("data/texture/{candidate}"));
                } else {
                    seen.insert(format!("data/texture/effect/{candidate}"));
                }
            }
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
        crate::arrow::SPRITES,
        effects::hit::SPRITES,
        effects::sight::SPRITES,
        effects::exit::SPRITES,
        effects::hasteup::SPRITES,
        effects::firearrow::SPRITES,
        effects::fireball::SPRITES,
        effects::soul_strike::SPRITES,
        effects::energy_drain::SPRITES,
        effects::sightrasher::SPRITES,
        effects::venomdust2::SPRITES,
        effects::healsp::SPRITES,
        effects::blessing::SPRITES,
        effects::cone::SPRITES,
        effects::dragonsmoke::SPRITES,
        effects::wink::SPRITES,
        effects::m_ef02::SPRITES,
        effects::ghost::SPRITES,
        effects::banjjakii::SPRITES,
        effects::orbit_burst::SPRITES,
        effects::hitdark::SPRITES,
        effects::spearbmr::SPRITES,
        effects::waterball2::SPRITES,
        effects::warp_zone::SPRITES,
        effects::peong::SPRITES,
        effects::super_angel::SPRITES,
        effects::summon_slave::SPRITES,
        effects::sakura::SPRITES,
        effects::bottom_song::SPRITES,
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
            paths.iter().all(|p| p.starts_with("data/texture/")),
            "all entries are GRF texture paths",
        );
        // Most are effect-dir textures; a few (thrown item icons) live
        // elsewhere under data/texture/ via the path-bearing-name convention.
        assert!(
            paths.iter().any(|p| p.starts_with("data/texture/유저인터페이스/item/")),
            "thrown item icons resolve under the item dir",
        );
        assert!(
            paths.iter().any(|p| p.ends_with("ring_yellow.tga")),
            "Warp's texture is included",
        );
    }
}
