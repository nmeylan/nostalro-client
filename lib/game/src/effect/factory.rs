//! Single dispatch point from [`EffectId`] to a concrete [`Effect`]
//! implementation. Real implementations have explicit arms in the match
//! below; any remaining id whose spec resolves to `EffectSpec::Custom`
//! falls into the placeholder catchall (pink billboard, plus the original
//! game's STR overlay for the 12 StrHybrid ids).

use models::enums::effect_id::EffectId;
use super::buckets::is_hybrid;
use super::effect_trait::Effect;
use super::effects;
use super::spec::Attach;
use super::str_aliases::str_aliases;

/// Build a concrete custom-effect instance. Ids with a real implementation
/// hit an explicit arm below; anything else lands on the placeholder.
pub fn make_effect(id: EffectId, attach: Attach) -> Option<Box<dyn Effect>> {
    Some(match id {
        EffectId::Warp => Box::new(effects::warp::WarpEffect::new(attach)),
        EffectId::Magnumbreak => {
            Box::new(effects::magnum_break::MagnumBreakEffect::new(attach))
        }

        // Hit family — weapon-swing impact shockwave + debris.
        // The cylinder ring + per-segment particle trails follow the
        // recipe; impact direction is the spawn-time `angle`
        // (currently defaulting to 0 since the spawn pipeline doesn't
        // carry it yet, see hit::new_with_angle docs).
        EffectId::Hit1 => Box::new(effects::hit::HitEffect::new(attach, effects::hit::HIT1)),
        EffectId::Hit2 => Box::new(effects::hit2::Hit2Effect::new(attach)),
        EffectId::Hit3 => Box::new(effects::hit::HitEffect::new(attach, effects::hit::HIT3)),
        EffectId::Hit4 => Box::new(effects::hit::HitEffect::new(attach, effects::hit::HIT4)),
        EffectId::Hit5 => Box::new(effects::hit5_6::HitCrossEffect::new(
            attach,
            effects::hit5_6::HIT5,
        )),
        EffectId::Hit6 => Box::new(effects::hit5_6::HitCrossEffect::new(
            attach,
            effects::hit5_6::HIT6,
        )),
        EffectId::Stormgust => Box::new(effects::stormgust::StormgustEffect::new(attach)),
        EffectId::BottomSanc => {
            Box::new(effects::bottom_sanctuary_pillar::BottomSanctuaryPillarEffect::new(attach))
        }
        EffectId::Warpzone => Box::new(effects::warp_zone::WarpZoneEffect::new(
            attach,
            effects::warp_zone::PARAMS_BURST,
        )),
        EffectId::Warpzone2 => Box::new(effects::warp_zone::WarpZoneEffect::new(
            attach,
            effects::warp_zone::PARAMS_SUSTAINED,
        )),
        EffectId::Landprotector => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::LANDPROTECTOR,
        )),
        EffectId::Volcano => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::VOLCANO,
        )),
        EffectId::Deluge => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::DELUGE,
        )),
        EffectId::Violentgale => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::VIOLENTGALE,
        )),
        EffectId::Ganbantein => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::GANBANTEIN,
        )),
        EffectId::Gumgang3 => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::GUMGANG3,
        )),


        EffectId::Level99 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_LARGE,
        )),
        EffectId::Level992 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_MIDDLE,
        )),
        EffectId::Level993 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_BOTTOM,
        )),
        EffectId::Level994 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_TRANSCENDANT,
        )),
        EffectId::Level995 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_TRANSCENDANT_MIDDLE,
        )),
        EffectId::Level996 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_TRANSCENDANT_BOTTOM,
        )),

        EffectId::Beginspell => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::YELLOW,
        )),
        EffectId::Beginspell2 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::WATER,
        )),
        EffectId::Beginspell3 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::FIRE,
        )),
        EffectId::Beginspell4 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::WIND,
        )),
        EffectId::Beginspell5 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::EARTH,
        )),
        EffectId::Beginspell6 => Box::new(effects::begin_spell_6::BeginSpell6Effect::new(attach)),
        EffectId::Beginspell7 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::POISON,
        )),
        EffectId::Beginspellred => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::RED,
        )),
        EffectId::Beginspellwhite => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::WHITE,
        )),
        EffectId::BeginspellN => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::N_BLUE,
        )),
        EffectId::Beginasura => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA,
        )),
        EffectId::Beginasura1 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_EARTH,
        )),
        EffectId::Beginasura2 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_WIND,
        )),
        EffectId::Beginasura3 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_WATER,
        )),
        EffectId::Beginasura4 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_FIRE,
        )),
        EffectId::Beginasura5 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_UNDEAD,
        )),
        EffectId::Beginasura6 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_SHADOW,
        )),
        EffectId::Beginasura7 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_HOLY,
        )),
        EffectId::Beginasura11 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_CHAMPION,
        )),

        // Animated texture billboard — 13-frame .bmp texture cycle on a
        // camera-facing billboard. Three colour variants share the
        // effect with different texture lists.
        EffectId::TorchRed => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::TORCH_RED,
            ),
        ),
        EffectId::TorchGreen => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::TORCH_GREEN,
            ),
        ),
        EffectId::TorchPurple => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::TORCH_PURPLE,
            ),
        ),
        EffectId::Dust => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::DUST,
            ),
        ),

        // EffectTextureSet(F1=14) — single static .bmp on the same quad as
        // the animated torch family. distance=30, alpha=50/255, no Y
        // offset; flag1[2]=4 → standard alpha quad.
        EffectId::Glow1 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::GLOW_01,
            ),
        ),
        EffectId::Glow2 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::GLOW_02,
            ),
        ),
        EffectId::Glow11 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::GLOW_11,
            ),
        ),
        EffectId::Glow12 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                attach,
                effects::animated_texture_billboard::GLOW_12,
            ),
        ),

        // Placeholder catchall. Hybrid ids (12 effects, e.g. Stormgust,
        // Coin, Glasswall) declare an STR overlay so the original game's
        // STR animation plays alongside the pink marker. Pure-custom ids
        // (407 minus those with real impls above) get the marker only.
        other if is_hybrid(other) => Box::new(effects::placeholder::HybridPlaceholderEffect::new(
            attach,
            str_aliases(other)[0],
        )),
        _ => Box::new(effects::placeholder::PlaceholderEffect::new(attach)),
    })
}

/// `true` when [`make_effect`] returns a concrete (non-placeholder)
/// implementation for `id`. Keep arms in sync with the explicit branches in
/// `make_effect`.
pub fn is_real_impl(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Warp
            | EffectId::Magnumbreak
            | EffectId::Hit1
            | EffectId::Hit2
            | EffectId::Hit3
            | EffectId::Hit4
            | EffectId::Hit5
            | EffectId::Hit6
            | EffectId::Stormgust
            | EffectId::BottomSanc
            | EffectId::Warpzone
            | EffectId::Warpzone2
            | EffectId::Landprotector
            | EffectId::Volcano
            | EffectId::Deluge
            | EffectId::Violentgale
            | EffectId::Ganbantein
            | EffectId::Gumgang3
            | EffectId::Level99
            | EffectId::Level992
            | EffectId::Level993
            | EffectId::Level994
            | EffectId::Level995
            | EffectId::Level996
            | EffectId::Beginspell
            | EffectId::Beginspell2
            | EffectId::Beginspell3
            | EffectId::Beginspell4
            | EffectId::Beginspell5
            | EffectId::Beginspell6
            | EffectId::Beginspell7
            | EffectId::Beginspellred
            | EffectId::Beginspellwhite
            | EffectId::BeginspellN
            | EffectId::Beginasura
            | EffectId::Beginasura1
            | EffectId::Beginasura2
            | EffectId::Beginasura3
            | EffectId::Beginasura4
            | EffectId::Beginasura5
            | EffectId::Beginasura6
            | EffectId::Beginasura7
            | EffectId::Beginasura11
            | EffectId::TorchRed
            | EffectId::TorchGreen
            | EffectId::TorchPurple
            | EffectId::Dust
            | EffectId::Glow1
            | EffectId::Glow2
            | EffectId::Glow11
            | EffectId::Glow12
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_dispatches() {
        let e = make_effect(EffectId::Warp, Attach::WorldPos([0.0; 3]));
        assert!(e.is_some());
        assert!(is_real_impl(EffectId::Warp));
    }

    #[test]
    fn unimplemented_custom_falls_back_to_placeholder() {
        // Pick an EffectId in the Custom bucket that doesn't yet have a
        // real Rust impl — factory returns the pink placeholder and
        // `is_real_impl` reports false.
        assert!(make_effect(EffectId::Aciddemon, Attach::WorldPos([0.0; 3])).is_some());
        assert!(!is_real_impl(EffectId::Aciddemon));
    }

    #[test]
    fn torch_recolours_dispatch_to_animated_texture_billboard() {
        // All recolour variants and the Glow family resolve to a real
        // impl. They must NOT fall through to the placeholder, otherwise
        // the viewer would show the pink marker instead of the cycled
        // bmp frames.
        for id in [
            EffectId::TorchRed,
            EffectId::TorchGreen,
            EffectId::TorchPurple,
            EffectId::Dust,
            EffectId::Glow1,
            EffectId::Glow2,
            EffectId::Glow11,
            EffectId::Glow12,
        ] {
            assert!(
                is_real_impl(id),
                "{:?} must have a real factory impl",
                id
            );
            let e = make_effect(id, Attach::WorldPos([0.0; 3])).unwrap();
            assert_eq!(
                e.str_overlay(),
                None,
                "{:?} is pure custom, no STR overlay",
                id
            );
        }
    }

    #[test]
    fn hybrid_placeholder_carries_str_overlay() {
        // Coin is a StrHybrid id with no real impl — factory routes it
        // through `HybridPlaceholderEffect` so its STR file still plays.
        let e = make_effect(EffectId::Coin, Attach::WorldPos([0.0; 3])).unwrap();
        assert_eq!(e.str_overlay(), Some(str_aliases(EffectId::Coin)[0]));
    }
}
