pub mod buckets;
pub mod consumable_effects;
pub mod draw;
pub mod effect_queue;
pub mod effect_trait;
pub mod effects;
pub mod factory;
pub mod projectile;
pub mod radial_emitter;
pub mod skill_effects;
pub mod skill_units;
pub mod spec;
pub mod spr_aliases;
pub mod spr_burst;
pub mod status_buff;
pub mod str_aliases;
pub mod table;

pub use draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
pub use effect_queue::{
    EffectQueue, ProjectileFlight, SpawnRequest, body_attached, is_caster_link_effect,
    is_count_point_effect, is_link_effect, is_trail_effect, trail_arrival_secs,
};
pub use effect_trait::{
    Afterimage, BodyAction, BodyCopy, BodyTint, BodyVertical, CameraShake, CameraView, Effect,
    EffectRenderCtx, EffectUpdateCtx, NumberRequest,
};
pub use factory::{is_real_impl, make_effect};
pub use skill_effects::{
    CasterSkillEffects, CastingSkill, TargetSkillEffects, begin_cast_effect, beginspell_for_element,
    caster_cast_on_use, caster_skill_effects, casting_skill, derive_hit_effect, fire_glyph_effect,
    ground_placed_effect, is_cast_circle, is_ground_cast, target_skill_effects,
};
pub use skill_units::{
    UNT_USED_TRAPS, skill_unit_effect, skill_unit_sprite_paths, trap_model_name,
    trap_trigger_effect,
};
pub use spec::{AlphaKeyframe, Attach, CurveParams, EffectSpec, SprBodyRecolor, SprBurstParams};
pub use spr_aliases::{SprDef, spr_def};
pub use spr_burst::spr_burst_params;
pub use status_buff::{BuffEffect, buff_effect};
pub use str_aliases::str_aliases;
pub use table::{effect_spec, spawn_camera_shake};

pub const ARROW_SPRITE: &str = "data/sprite/몬스터/skel_archer_arrow";

pub const SPRITES: &[&str] = &[ARROW_SPRITE];

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
        effects::begin_spell::TEXTURES,
        effects::begin_spell_8::TEXTURES,
        effects::aura_blade::TEXTURES,
        effects::couple_casting::TEXTURES,
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
        effects::magic_bolt::FIRE_TEXTURES,
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
        effects::magic_bolt::ICE_TEXTURES,
    ];
    for list in texture_lists {
        for name in *list {
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

pub fn effect_str_names() -> Vec<&'static [&'static str]> {
    use models::enums::EnumWithNumberValue;
    use models::enums::effect_id::EffectId;

    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for value in 0..3000usize {
        let Ok(id) = EffectId::try_from_value(value) else {
            continue;
        };
        if !matches!(effect_spec(id), Some(spec::EffectSpec::Str { .. })) {
            continue;
        }
        let aliases = str_aliases::str_aliases(id);
        if !aliases.is_empty() && seen.insert(aliases[0]) {
            out.push(aliases);
        }
    }
    out
}

pub fn custom_effect_sprite_paths() -> Vec<&'static str> {
    let mut seen = std::collections::BTreeSet::new();
    let sprite_lists: &[&[&str]] = &[
        SPRITES,
        effects::hit::SPRITES,
        effects::sight::SPRITES,
        effects::exit::SPRITES,
        effects::hasteup::SPRITES,
        effects::magic_bolt::FIRE_SPRITES,
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
        assert!(
            paths
                .iter()
                .any(|p| p.starts_with("data/texture/유저인터페이스/item/")),
            "thrown item icons resolve under the item dir",
        );
        assert!(
            paths.iter().any(|p| p.ends_with("ring_yellow.tga")),
            "Warp's texture is included",
        );
    }
}
