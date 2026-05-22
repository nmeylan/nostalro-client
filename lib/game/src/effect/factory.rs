//! Single dispatch point from [`EffectId`] to a concrete [`Effect`]
//! implementation. Real implementations have explicit arms in the match
//! below; any remaining id whose spec resolves to `EffectSpec::Custom`
//! falls into the placeholder catchall (pink billboard, plus the original
//! game's STR overlay for the 12 StrHybrid ids).
//!
//! The factory takes an [`EffectAnchor`] rather than a raw `Attach` — the
//! renderer's spawn pipeline (`EffectHolder::spawn`) resolves
//! `Attach::Entity` and friends to a single world position before calling
//! `make_effect`, so individual effects don't have to. Trail-shaped
//! effects (Frost Diver) unpack `EffectAnchor::Trail`; everything else
//! collapses to the caster-side anchor via `EffectAnchor::point`.

use models::enums::effect_id::EffectId;
use super::buckets::is_hybrid;
use super::effect_trait::Effect;
use super::effects;
use super::spec::EffectAnchor;
use super::str_aliases::str_aliases;

/// Build a concrete custom-effect instance. Ids with a real implementation
/// hit an explicit arm below; anything else lands on the placeholder.
pub fn make_effect(id: EffectId, anchor: EffectAnchor) -> Option<Box<dyn Effect>> {
    Some(match id {
        EffectId::Warp => Box::new(effects::warp::WarpEffect::new(anchor.point())),
        EffectId::Entry => Box::new(effects::entry::EntryEffect::new(anchor.point())),
        EffectId::Exit => Box::new(effects::exit::ExitEffect::new(anchor.point())),
        // Only the four ids without an STR file in the classic GRF
        // (firearrow, fireball, napalmbeat, sandwind) need a Custom
        // impl — everything else falls back to the canonical STR.
        EffectId::Firearrow => Box::new(effects::firearrow::FireArrowEffect::new(anchor.point())),
        EffectId::Fireball => Box::new(effects::fireball::FireballEffect::new(anchor.point())),
        EffectId::Napalmbeat => Box::new(effects::napalmbeat::NapalmBeatEffect::new(anchor.point())),
        EffectId::Sandwind => Box::new(effects::sandwind::SandwindEffect::new(anchor.point())),

        // Frost Diver — trail-shaped, unpacks both endpoints. Single-point
        // anchors (effect-viewer demo, any caller that doesn't know about
        // the trail) collapse to `from == to`, which the effect detects
        // and falls back to cluster mode for.
        EffectId::Frostdiver => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::frost_diver::FrostDiverEffect::new(
                from, to, effects::frost_diver::FROSTDIVER,
            ))
        }
        EffectId::Frostdiver2 => {
            // FrostDiver2 is a single-point burst — no trail behaviour.
            let p = anchor.point();
            Box::new(effects::frost_diver::FrostDiverEffect::new(
                p, p, effects::frost_diver::FROSTDIVER2,
            ))
        }
        EffectId::Magnumbreak => {
            Box::new(effects::magnum_break::MagnumBreakEffect::new(anchor.point()))
        }

        // Sight + Ruwach — orbit-spawn SpriteParticle pairs around the
        // entity. Same struct, different per-skill `Params`.
        EffectId::Sight => Box::new(effects::sight::OrbitEffect::new(
            anchor.point(),
            effects::sight::SIGHT,
        )),
        EffectId::Ruwach => Box::new(effects::sight::OrbitEffect::new(
            anchor.point(),
            effects::sight::RUWACH,
        )),

        // StatusUp family — crossed-texture streak particles around
        // the entity. Incagility/Incagidex rise; Decagility falls. Tints
        // differ per id.
        EffectId::Incagility => Box::new(effects::status_up::StatusUpEffect::new(
            anchor.point(),
            effects::status_up::INCAGILITY,
        )),
        EffectId::Decagility => Box::new(effects::status_up::StatusUpEffect::new(
            anchor.point(),
            effects::status_up::DECAGILITY,
        )),
        EffectId::Incagidex => Box::new(effects::status_up::StatusUpEffect::new(
            anchor.point(),
            effects::status_up::INCAGIDEX,
        )),

        // Hit family — weapon-swing impact shockwave + debris.
        // The cylinder ring + per-segment particle trails match the
        // original game's look; impact direction is the spawn-time `angle`
        // (currently defaulting to 0 since the spawn pipeline doesn't
        // carry it yet, see hit::new_with_angle docs).
        EffectId::Hit1 => Box::new(effects::hit::HitEffect::new(anchor.point(), effects::hit::HIT1)),
        EffectId::Hit2 => Box::new(effects::hit2::Hit2Effect::new(anchor.point())),
        EffectId::Hit3 => Box::new(effects::hit::HitEffect::new(anchor.point(), effects::hit::HIT3)),
        EffectId::Hit4 => Box::new(effects::hit::HitEffect::new(anchor.point(), effects::hit::HIT4)),
        EffectId::Hit5 => Box::new(effects::hit5_6::HitCrossEffect::new(
            anchor.point(),
            effects::hit5_6::HIT5,
        )),
        EffectId::Hit6 => Box::new(effects::hit5_6::HitCrossEffect::new(
            anchor.point(),
            effects::hit5_6::HIT6,
        )),
        EffectId::Stormgust => Box::new(effects::stormgust::StormgustEffect::new(anchor.point())),
        EffectId::BottomSanc => {
            Box::new(effects::bottom_sanctuary_pillar::BottomSanctuaryPillarEffect::new(anchor.point()))
        }
        EffectId::Warpzone => Box::new(effects::warp_zone::WarpZoneEffect::new(
            anchor.point(),
            effects::warp_zone::PARAMS_BURST,
        )),
        EffectId::Warpzone2 => Box::new(effects::warp_zone::WarpZoneEffect::new(
            anchor.point(),
            effects::warp_zone::PARAMS_SUSTAINED,
        )),
        EffectId::Landprotector => Box::new(effects::volcano::VolcanoEffect::new(
            anchor.point(),
            effects::volcano::LANDPROTECTOR,
        )),
        EffectId::Volcano => Box::new(effects::volcano::VolcanoEffect::new(
            anchor.point(),
            effects::volcano::VOLCANO,
        )),
        EffectId::Deluge => Box::new(effects::volcano::VolcanoEffect::new(
            anchor.point(),
            effects::volcano::DELUGE,
        )),
        EffectId::Violentgale => Box::new(effects::volcano::VolcanoEffect::new(
            anchor.point(),
            effects::volcano::VIOLENTGALE,
        )),
        EffectId::Ganbantein => Box::new(effects::volcano::VolcanoEffect::new(
            anchor.point(),
            effects::volcano::GANBANTEIN,
        )),
        EffectId::Gumgang3 => Box::new(effects::volcano::VolcanoEffect::new(
            anchor.point(),
            effects::volcano::GUMGANG3,
        )),


        EffectId::Level99 => Box::new(effects::aura::AuraEffect::new(
            anchor.point(),
            effects::aura::LV99_LARGE,
        )),
        EffectId::Level992 => Box::new(effects::aura::AuraEffect::new(
            anchor.point(),
            effects::aura::LV99_MIDDLE,
        )),
        EffectId::Level993 => Box::new(effects::aura::AuraEffect::new(
            anchor.point(),
            effects::aura::LV99_BOTTOM,
        )),
        EffectId::Level994 => Box::new(effects::aura::AuraEffect::new(
            anchor.point(),
            effects::aura::LV99_TRANSCENDANT,
        )),
        EffectId::Level995 => Box::new(effects::aura::AuraEffect::new(
            anchor.point(),
            effects::aura::LV99_TRANSCENDANT_MIDDLE,
        )),
        EffectId::Level996 => Box::new(effects::aura::AuraEffect::new(
            anchor.point(),
            effects::aura::LV99_TRANSCENDANT_BOTTOM,
        )),

        EffectId::Beginspell => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::YELLOW,
        )),
        EffectId::Beginspell2 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::WATER,
        )),
        EffectId::Beginspell3 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::FIRE,
        )),
        EffectId::Beginspell4 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::WIND,
        )),
        EffectId::Beginspell5 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::EARTH,
        )),
        EffectId::Beginspell6 => Box::new(effects::begin_spell_6::BeginSpell6Effect::new(anchor.point())),
        EffectId::Beginspell7 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::POISON,
        )),
        EffectId::Beginspellred => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::RED,
        )),
        EffectId::Beginspellwhite => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::WHITE,
        )),
        EffectId::BeginspellN => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::N_BLUE,
        )),
        EffectId::Beginasura => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA,
        )),
        EffectId::Beginasura1 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_EARTH,
        )),
        EffectId::Beginasura2 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_WIND,
        )),
        EffectId::Beginasura3 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_WATER,
        )),
        EffectId::Beginasura4 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_FIRE,
        )),
        EffectId::Beginasura5 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_UNDEAD,
        )),
        EffectId::Beginasura6 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_SHADOW,
        )),
        EffectId::Beginasura7 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_HOLY,
        )),
        EffectId::Beginasura11 => Box::new(effects::cast_circle::CastCircleEffect::new(
            anchor.point(),
            effects::cast_circle::ASURA_CHAMPION,
        )),

        // Animated texture billboard — 13-frame .bmp texture cycle on a
        // camera-facing billboard. Three colour variants share the
        // effect with different texture lists.
        EffectId::TorchRed => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::TORCH_RED,
            ),
        ),
        EffectId::TorchGreen => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::TORCH_GREEN,
            ),
        ),
        EffectId::TorchPurple => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::TORCH_PURPLE,
            ),
        ),
        EffectId::Dust => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::DUST,
            ),
        ),

        // EffectTextureSet(F1=14) — single static .bmp on the same quad as
        // the animated torch family. distance=30, alpha=50/255, no Y
        // offset; flag1[2]=4 → standard alpha quad.
        EffectId::Glow1 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::GLOW_01,
            ),
        ),
        EffectId::Glow2 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::GLOW_02,
            ),
        ),
        EffectId::Glow11 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::GLOW_11,
            ),
        ),
        EffectId::Glow12 => Box::new(
            effects::animated_texture_billboard::AnimatedTextureBillboardEffect::new(
                anchor.point(),
                effects::animated_texture_billboard::GLOW_12,
            ),
        ),

        // BottomSong — 12 Bard/Dancer ground songs that share one
        // ground-disc primitive with per-id texture and radius. The
        // Magnus/Vertical/Light/LandProtector/Hermode songs look different
        // and are deferred.
        EffectId::BottomGospel => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::GOSPEL,
        )),
        EffectId::BottomEvilland => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::EVILLAND,
        )),
        EffectId::BottomFortunekiss => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::FORTUNEKISS,
        )),
        EffectId::BottomLullaby => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::LULLABY,
        )),
        EffectId::BottomRichmankim => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::RICHMANKIM,
        )),
        EffectId::BottomDrumbattlefield => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::DRUMBATTLEFIELD,
        )),
        EffectId::BottomRingnibelungen => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::RINGNIBELUNGEN,
        )),
        EffectId::BottomIntoabyss => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::INTOABYSS,
        )),
        EffectId::BottomWhistle => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::WHISTLE,
        )),
        EffectId::BottomPoembragi => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::POEMBRAGI,
        )),
        EffectId::BottomAppleidun => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::APPLEIDUN,
        )),
        EffectId::BottomHumming => Box::new(effects::bottom_song::BottomSongEffect::new(
            anchor.point(),
            effects::bottom_song::HUMMING,
        )),

        // BottomMagnus — 4-sided square pillar via
        // `EffectPrimitiveDraw::Frustum`. BottomSanc already has its own
        // dedicated impl (`bottom_sanctuary_pillar.rs`) that renders a
        // 24-sided cylinder; only Magnus and Fogwall land here.
        // Both share the same pillar geometry, differing by tint.
        EffectId::BottomMag => Box::new(effects::bottom_magnus::BottomMagnusEffect::new(
            anchor.point(),
            effects::bottom_magnus::MAGNUS,
        )),
        EffectId::BottomFogwall => Box::new(effects::bottom_magnus::BottomMagnusEffect::new(
            anchor.point(),
            effects::bottom_magnus::FOGWALL,
        )),

        // BottomHermode — small rotating cube emitted as 6 WorldQuad
        // faces with per-face shading.
        EffectId::BottomHermode => Box::new(
            effects::bottom_hermode::BottomHermodeEffect::new(
                anchor.point(),
                effects::bottom_hermode::HERMODE,
            ),
        ),
        // BottomRokisweil — pulsing camera-facing billboards. Uses the
        // existing Billboard primitive.
        EffectId::BottomRokisweil => Box::new(
            effects::bottom_out::BottomOutEffect::new(
                anchor.point(),
                effects::bottom_out::ROKISWEIL,
            ),
        ),

        // BottomLandProtector — single horizontal square ward with
        // radially-breathing corners. 4 ids.
        EffectId::BottomLa => Box::new(
            effects::bottom_landprotector::BottomLandProtectorEffect::new(
                anchor.point(),
                effects::bottom_landprotector::LA,
            ),
        ),
        EffectId::BottomRunner => Box::new(
            effects::bottom_landprotector::BottomLandProtectorEffect::new(
                anchor.point(),
                effects::bottom_landprotector::RUNNER,
            ),
        ),
        EffectId::BottomTransfer => Box::new(
            effects::bottom_landprotector::BottomLandProtectorEffect::new(
                anchor.point(),
                effects::bottom_landprotector::TRANSFER,
            ),
        ),
        EffectId::BottomSpider => Box::new(
            effects::bottom_landprotector::BottomLandProtectorEffect::new(
                anchor.point(),
                effects::bottom_landprotector::SPIDER,
            ),
        ),

        // BottomLight — 315° curtain-cone wall built from ~20 WorldQuad
        // ribbon segments per frame. Same geometry for both ids; a flag
        // picks the tint/blend.
        EffectId::BottomEternalchaos => Box::new(
            effects::bottom_light::BottomLightEffect::new(
                anchor.point(),
                effects::bottom_light::ETERNALCHAOS,
            ),
        ),
        EffectId::BottomSiegfried => Box::new(
            effects::bottom_light::BottomLightEffect::new(
                anchor.point(),
                effects::bottom_light::SIEGFRIED,
            ),
        ),

        // BottomVertical — vertical "curtain" strips via the
        // `EffectPrimitiveDraw::WorldQuad` primitive. 5 ids.
        EffectId::BottomDissonance => Box::new(
            effects::bottom_vertical::BottomVerticalEffect::new(
                anchor.point(),
                effects::bottom_vertical::DISSONANCE,
            ),
        ),
        EffectId::BottomUglydance => Box::new(
            effects::bottom_vertical::BottomVerticalEffect::new(
                anchor.point(),
                effects::bottom_vertical::UGLYDANCE,
            ),
        ),
        EffectId::BottomAssassincross => Box::new(
            effects::bottom_vertical::BottomVerticalEffect::new(
                anchor.point(),
                effects::bottom_vertical::ASSASSINCROSS,
            ),
        ),
        EffectId::BottomDontforgetme => Box::new(
            effects::bottom_vertical::BottomVerticalEffect::new(
                anchor.point(),
                effects::bottom_vertical::DONTFORGETME,
            ),
        ),
        EffectId::BottomServiceforyou => Box::new(
            effects::bottom_vertical::BottomVerticalEffect::new(
                anchor.point(),
                effects::bottom_vertical::SERVICEFORYOU,
            ),
        ),

        // Placeholder catchall. Hybrid ids (12 effects, e.g. Stormgust,
        // Coin, Glasswall) declare an STR overlay so the original game's
        // STR animation plays alongside the pink marker. Pure-custom ids
        // (407 minus those with real impls above) get the marker only.
        other if is_hybrid(other) => Box::new(effects::placeholder::HybridPlaceholderEffect::new(
            anchor.point(),
            str_aliases(other)[0],
        )),
        _ => Box::new(effects::placeholder::PlaceholderEffect::new(anchor.point())),
    })
}

/// `true` when [`make_effect`] returns a concrete (non-placeholder)
/// implementation for `id`. Keep arms in sync with the explicit branches in
/// `make_effect`.
pub fn is_real_impl(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Warp
            | EffectId::Entry
            | EffectId::Exit
            | EffectId::Firearrow
            | EffectId::Fireball
            | EffectId::Napalmbeat
            | EffectId::Sandwind
            | EffectId::Frostdiver
            | EffectId::Frostdiver2
            | EffectId::Magnumbreak
            | EffectId::Sight
            | EffectId::Ruwach
            | EffectId::Incagility
            | EffectId::Decagility
            | EffectId::Incagidex
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
            | EffectId::BottomGospel
            | EffectId::BottomEvilland
            | EffectId::BottomFortunekiss
            | EffectId::BottomLullaby
            | EffectId::BottomRichmankim
            | EffectId::BottomDrumbattlefield
            | EffectId::BottomRingnibelungen
            | EffectId::BottomIntoabyss
            | EffectId::BottomWhistle
            | EffectId::BottomPoembragi
            | EffectId::BottomAppleidun
            | EffectId::BottomHumming
            | EffectId::BottomMag
            | EffectId::BottomFogwall
            | EffectId::BottomDissonance
            | EffectId::BottomUglydance
            | EffectId::BottomAssassincross
            | EffectId::BottomDontforgetme
            | EffectId::BottomServiceforyou
            | EffectId::BottomEternalchaos
            | EffectId::BottomSiegfried
            | EffectId::BottomLa
            | EffectId::BottomRunner
            | EffectId::BottomTransfer
            | EffectId::BottomSpider
            | EffectId::BottomHermode
            | EffectId::BottomRokisweil
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_dispatches() {
        let e = make_effect(EffectId::Warp, EffectAnchor::Point([0.0; 3]));
        assert!(e.is_some());
        assert!(is_real_impl(EffectId::Warp));
    }

    #[test]
    fn effect_anchor_propagates_to_first_draw_call() {
        // Sociable test: the whole point of the EffectAnchor refactor —
        // an effect's primitives render at the anchor position, not at
        // the world origin. Magnum Break's parent ring emits a
        // `GroundDisc` centred on the spawn point, so spawning at a
        // non-origin anchor must produce a non-origin centre. Before
        // the refactor every effect's `new(attach)` fell back to
        // `[0.0; 3]` for anything other than `Attach::WorldPos`, which
        // silently broke entity-attached and trail-attached spawns;
        // locking this assertion stops that regression.
        use crate::effect::draw::{EffectDrawList, EffectPrimitiveDraw};
        use crate::effect::effect_trait::{EffectRenderCtx, EffectUpdateCtx};

        let anchor_pos = [10.0, 0.0, 20.0];
        let mut effect = make_effect(
            EffectId::Magnumbreak,
            EffectAnchor::Point(anchor_pos),
        )
        .expect("magnum break must dispatch");
        // Step one tick so the effect has age > 0 (some effects skip
        // emission at exactly age 0 — magnum_break doesn't but the
        // pattern matters for future effects).
        effect.update(&EffectUpdateCtx {
            delta: 1.0 / 60.0,
            camera_target: None,
        });
        let mut draws = EffectDrawList::new();
        effect.collect_draws(
            &mut draws,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 800.0,
                screen_h: 600.0,
                elapsed: 0.0,
            },
        );
        // Find the first GroundDisc — should be centred on anchor_pos.
        let center = draws
            .primitives
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::GroundDisc { center, .. } => Some(*center),
                _ => None,
            })
            .expect("magnum_break emits a GroundDisc");
        assert!(
            (center[0] - anchor_pos[0]).abs() < 1e-3
                && (center[2] - anchor_pos[2]).abs() < 1e-3,
            "GroundDisc centre {center:?} should match anchor {anchor_pos:?}",
        );
    }

    #[test]
    fn effect_anchor_point_helper_collapses_both_variants() {
        // The Trail variant's `from` becomes the single-point anchor
        // for non-trail effects; Point is its own value. Locks the
        // helper since factory arms rely on `anchor.point()`.
        assert_eq!(
            EffectAnchor::Point([1.0, 2.0, 3.0]).point(),
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            EffectAnchor::Trail {
                from: [7.0, 0.0, 8.0],
                to: [50.0, 0.0, 60.0],
            }
            .point(),
            [7.0, 0.0, 8.0],
        );
    }

    #[test]
    fn unimplemented_custom_falls_back_to_placeholder() {
        // Pick an EffectId in the Custom bucket that doesn't yet have a
        // real Rust impl — factory returns the pink placeholder and
        // `is_real_impl` reports false.
        assert!(make_effect(EffectId::Aciddemon, EffectAnchor::Point([0.0; 3])).is_some());
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
            let e = make_effect(id, EffectAnchor::Point([0.0; 3])).unwrap();
            assert_eq!(
                e.str_overlay(),
                None,
                "{:?} is pure custom, no STR overlay",
                id
            );
        }
    }

    #[test]
    fn bottom_vertical_variants_dispatch_to_world_quad_strips() {
        // 5 BottomVertical ids must land on the BottomVertical custom
        // effect (WorldQuad primitives), not the placeholder.
        for id in [
            EffectId::BottomDissonance,
            EffectId::BottomUglydance,
            EffectId::BottomAssassincross,
            EffectId::BottomDontforgetme,
            EffectId::BottomServiceforyou,
        ] {
            assert!(is_real_impl(id), "{:?} must have a real impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3])).unwrap();
            assert_eq!(e.str_overlay(), None);
        }
    }

    #[test]
    fn bottom_hermode_dispatches_to_world_quad_cube() {
        assert!(is_real_impl(EffectId::BottomHermode));
        let e = make_effect(EffectId::BottomHermode, EffectAnchor::Point([0.0; 3])).unwrap();
        assert_eq!(e.str_overlay(), None);
    }

    #[test]
    fn bottom_rokisweil_dispatches_to_billboard_pulse() {
        assert!(is_real_impl(EffectId::BottomRokisweil));
        let e = make_effect(EffectId::BottomRokisweil, EffectAnchor::Point([0.0; 3])).unwrap();
        assert_eq!(e.str_overlay(), None);
    }

    #[test]
    fn bottom_landprotector_variants_dispatch_to_world_quad_square() {
        // 4 BottomLandProtector ids must land on the BottomLandProtector
        // custom effect (single WorldQuad horizontal square), not the
        // placeholder.
        for id in [
            EffectId::BottomLa,
            EffectId::BottomRunner,
            EffectId::BottomTransfer,
            EffectId::BottomSpider,
        ] {
            assert!(is_real_impl(id), "{:?} must have a real impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3])).unwrap();
            assert_eq!(e.str_overlay(), None);
        }
    }

    #[test]
    fn bottom_light_variants_dispatch_to_world_quad_curtain() {
        // 2 BottomLight ids must land on the BottomLight custom effect
        // (WorldQuad ribbon segments), not the placeholder.
        for id in [EffectId::BottomEternalchaos, EffectId::BottomSiegfried] {
            assert!(is_real_impl(id), "{:?} must have a real impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3])).unwrap();
            assert_eq!(e.str_overlay(), None);
        }
    }

    #[test]
    fn bottom_magnus_variants_dispatch_to_frustum_pillar() {
        // BottomMag + BottomFogwall must land on the BottomMagnus
        // custom effect (Frustum sides=4), not the placeholder.
        for id in [EffectId::BottomMag, EffectId::BottomFogwall] {
            assert!(is_real_impl(id), "{:?} must have a real impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3])).unwrap();
            assert_eq!(e.str_overlay(), None);
        }
    }

    #[test]
    fn bottom_songs_dispatch_to_real_impl() {
        // Sociable test: 12 BottomSong ids must route to the
        // BottomSong custom effect rather than the pink placeholder.
        // No STR overlay (Bard/Dancer songs aren't classified as
        // StrHybrid in our table).
        for id in [
            EffectId::BottomGospel,
            EffectId::BottomEvilland,
            EffectId::BottomFortunekiss,
            EffectId::BottomLullaby,
            EffectId::BottomRichmankim,
            EffectId::BottomDrumbattlefield,
            EffectId::BottomRingnibelungen,
            EffectId::BottomIntoabyss,
            EffectId::BottomWhistle,
            EffectId::BottomPoembragi,
            EffectId::BottomAppleidun,
            EffectId::BottomHumming,
        ] {
            assert!(
                is_real_impl(id),
                "{:?} must have a real factory impl",
                id
            );
            let e = make_effect(id, EffectAnchor::Point([0.0; 3])).unwrap();
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
        let e = make_effect(EffectId::Coin, EffectAnchor::Point([0.0; 3])).unwrap();
        assert_eq!(e.str_overlay(), Some(str_aliases(EffectId::Coin)[0]));
    }
}
