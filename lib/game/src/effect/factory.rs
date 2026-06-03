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
// `bash` lives here in alphabetical order; the match arm is below near
// the other bucket-0-50 ids.
use super::effect_trait::Effect;
use super::effects;
use super::spec::EffectAnchor;
use super::str_aliases::str_aliases;

/// Build a concrete custom-effect instance. Ids with a real implementation
/// hit an explicit arm below; anything else lands on the placeholder.
pub fn make_effect(id: EffectId, anchor: EffectAnchor, hit_count: Option<u8>) -> Option<Box<dyn Effect>> {
    Some(match id {
        EffectId::Warp => Box::new(effects::warp::WarpEffect::new(anchor.point())),
        EffectId::Bash => Box::new(effects::bash::BashEffect::new(anchor.point())),
        EffectId::Hasteup => Box::new(effects::hasteup::HasteUpEffect::new(anchor.point())),
        EffectId::Flasher => Box::new(effects::flasher::FlasherEffect::new(anchor.point())),
        EffectId::Blessing => Box::new(effects::blessing::BlessingEffect::new(anchor.point())),
        EffectId::Endure => Box::new(effects::endure::EndureEffect::new(anchor.point())),
        EffectId::Enhance => Box::new(effects::enhance::EnhanceEffect::new(anchor.point())),
        EffectId::Entry => Box::new(effects::entry::EntryEffect::new(anchor.point())),
        EffectId::Exit => Box::new(effects::exit::ExitEffect::new(anchor.point())),
        EffectId::Glasswall => Box::new(effects::glasswall::GlasswallEffect::new(anchor.point())),
        EffectId::Healsp => Box::new(effects::healsp::HealSpEffect::new(anchor.point())),
        EffectId::Guard => Box::new(effects::guard::GuardEffect::new(anchor.point(), effects::guard::GUARD)),
        EffectId::Guard3 => Box::new(effects::guard::GuardEffect::new(anchor.point(), effects::guard::GUARD3)),
        EffectId::Guard2 => Box::new(effects::guard::GuardEffect::new(anchor.point(), effects::guard::GUARD2)),
        EffectId::Portal => Box::new(effects::portal::PortalEffect::new(anchor.point())),
        EffectId::Portal2 => Box::new(effects::portal2::Portal2Effect::new(
            anchor.point(),
            effects::portal2::PORTAL2,
        )),
        EffectId::Portal3 => Box::new(effects::portal2::Portal2Effect::new(
            anchor.point(),
            effects::portal2::PORTAL3,
        )),
        EffectId::Portal4 => Box::new(effects::portal_wind::PortalWindEffect::new(
            anchor.point(),
            effects::portal_wind::PORTAL4,
        )),
        EffectId::Portal5 => Box::new(effects::portal_wind::PortalWindEffect::new(
            anchor.point(),
            effects::portal_wind::PORTAL5,
        )),
        EffectId::Readyportal => Box::new(effects::ready_portal::ReadyPortalEffect::new(anchor.point())),
        EffectId::Teleportation => Box::new(effects::teleportation::TeleportationEffect::new(anchor.point())),
        EffectId::Spraypond => Box::new(effects::spraypond::SpraypondEffect::new(anchor.point())),
        // Only the four ids without an STR file in the classic GRF
        // (firearrow, fireball, napalmbeat, sandwind) need a Custom
        // impl — everything else falls back to the canonical STR.
        EffectId::Firearrow => Box::new(effects::firearrow::FireArrowEffect::new(anchor.point())),
        EffectId::Fireball => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::fireball::FireballEffect::new(from, to))
        }
        EffectId::Yufitel => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::yupitel::YupitelEffect::new(from, to))
        }
        EffectId::Blitzbeat => {
            Box::new(effects::blitzbeat::BlitzbeatEffect::new(anchor.point()))
        }
        EffectId::Waterball => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::waterball::WaterballEffect::new(from, to))
        }
        EffectId::Waterfall => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL,
        )),
        EffectId::Waterfall90 => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL_90,
        )),
        EffectId::WaterfallSmall => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL_SMALL,
        )),
        EffectId::WaterfallSmall90 => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL_SMALL_90,
        )),
        EffectId::WaterfallT2 => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL_T2,
        )),
        EffectId::WaterfallT290 => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL_T2_90,
        )),
        EffectId::WaterfallSmallT2 => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL_SMALL_T2,
        )),
        EffectId::WaterfallSmallT290 => Box::new(effects::waterfall::WaterfallEffect::new(
            anchor.point(),
            effects::waterfall::WATERFALL_SMALL_T2_90,
        )),
        EffectId::Cloud => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD)),
        EffectId::Cloud2 => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD2)),
        EffectId::Cloud3 => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD3)),
        EffectId::Cloud4 => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD4)),
        EffectId::Cloud5 => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD5)),
        EffectId::Cloud6 => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD6)),
        EffectId::Cloud7 => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD7)),
        EffectId::Cloud8 => Box::new(effects::cloud::CloudEffect::new(anchor.point(), effects::cloud::CLOUD8)),
        EffectId::Fireivy => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::fireivy::FireivyEffect::new(from, to))
        }
        EffectId::Detecting => {
            Box::new(effects::detecting::DetectingEffect::new(anchor.point()))
        }
        EffectId::Toprank => Box::new(effects::toprank::ToprankEffect::new(anchor.point())),
        EffectId::Party => Box::new(effects::party::PartyEffect::new(anchor.point())),
        EffectId::Curseattack => {
            Box::new(effects::curseattack::CurseattackEffect::new(anchor.point()))
        }
        EffectId::Napalmbeat => Box::new(effects::napalmbeat::NapalmBeatEffect::new(anchor.point())),
        EffectId::Sandwind => Box::new(effects::sandwind::SandwindEffect::new(anchor.point())),

        // Batch FR — see `table.rs`. Flowercast2/3 have no factory arm; the
        // original game's procedural dispatch for them is an empty break, so
        // they fall through to their STR alias (or render nothing).
        EffectId::Heavensdrive => {
            Box::new(effects::heavensdrive::HeavensDriveEffect::new(anchor.point()))
        }
        EffectId::Bottom => Box::new(effects::bottom_box::BottomBoxEffect::bottom(anchor.point())),
        EffectId::Bottom2 => Box::new(effects::bottom_box::BottomBoxEffect::bottom2(anchor.point())),
        EffectId::Cone => Box::new(effects::cone::ConeEffect::new(anchor.point())),
        EffectId::Flowercast => {
            Box::new(effects::flowercast::FlowerCastEffect::new(anchor.point()))
        }

        // Caster body-tint buffs (recolour + STR overlay): tint the actor +
        // play twohand.str. One struct, per-buff colour/sound param set.
        EffectId::Twohandquicken => {
            Box::new(effects::body_buff::BodyBuffEffect::new(effects::body_buff::TWOHAND_QUICKEN))
        }
        EffectId::Spearquicken => {
            Box::new(effects::body_buff::BodyBuffEffect::new(effects::body_buff::SPEAR_QUICKEN))
        }
        EffectId::Lkconcentration => {
            Box::new(effects::body_buff::BodyBuffEffect::new(effects::body_buff::LK_CONCENTRATION))
        }

        // Body-shake effects — shake the attached actor's sprite, emit no
        // primitives. One struct, four timing/amplitude param sets.
        EffectId::Quakebody => {
            Box::new(effects::quakebody::QuakeBodyEffect::new(effects::quakebody::QUAKEBODY))
        }
        EffectId::Quakebody2 => {
            Box::new(effects::quakebody::QuakeBodyEffect::new(effects::quakebody::QUAKEBODY2))
        }
        EffectId::Quakebody3 => {
            Box::new(effects::quakebody::QuakeBodyEffect::new(effects::quakebody::QUAKEBODY3))
        }
        EffectId::Quakebody4 => {
            Box::new(effects::quakebody::QuakeBodyEffect::new(effects::quakebody::QUAKEBODY4))
        }

        // Batch STR-B10 — Aciddemon swirling cone funnel; Rainbow arch.
        EffectId::Aciddemon => Box::new(effects::aciddemon::AcidDemonEffect::new(anchor.point())),
        EffectId::Rainbow => Box::new(effects::rainbow::RainbowEffect::new(anchor.point())),
        EffectId::Agiup => Box::new(effects::agiup::AgiUpEffect::new(anchor.point())),
        EffectId::Lightsphere => Box::new(effects::light_sphere::LightSphereEffect::new(
            anchor.point(),
            effects::light_sphere::LIGHTSPHERE,
        )),
        EffectId::Lightsphere2 => Box::new(effects::light_sphere::LightSphereEffect::new(
            anchor.point(),
            effects::light_sphere::LIGHTSPHERE2,
        )),

        // Batch MAPZONE — `Map_MagicZone` spinning ground rings + sparkle motes
        // / pika floor + flared aura. Persistent map-scale zones.
        EffectId::MapMagiczone => Box::new(effects::mapzone::MapZoneEffect::new(
            anchor.point(),
            effects::mapzone::MAP_MAGICZONE,
        )),
        EffectId::MapMagiczone2 => Box::new(effects::mapzone::MapZoneEffect::new(
            anchor.point(),
            effects::mapzone::MAP_MAGICZONE2,
        )),
        EffectId::Glow4 => Box::new(effects::mapzone::MapZoneEffect::new(
            anchor.point(),
            effects::mapzone::GLOW4,
        )),

        // Batch STR-B9 — Texture3DQuad. Both anchor at a hit point.
        EffectId::Yufitel2 => Box::new(effects::yufitel2::Yufitel2Effect::new(anchor.point())),
        EffectId::TextureFalling => Box::new(effects::texture_falling::FallingTrailEffect::new(
            anchor.point(),
            effects::texture_falling::TEXTURE_FALLING,
        )),

        // Footprint family — flat ground decals oriented along the
        // caster→target direction, one struct parameterised by texture + size
        // matching the original game's footprint decals. The trail's `from`
        // is the footprint anchor, `to` gives the facing direction.
        EffectId::Foot
        | EffectId::Foot2
        | EffectId::Foot3
        | EffectId::Foot4
        | EffectId::Foot5
        | EffectId::Foot6 => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            let params = match id {
                EffectId::Foot => effects::foot::FOOT,
                EffectId::Foot2 => effects::foot::FOOT2,
                EffectId::Foot3 => effects::foot::FOOT3,
                EffectId::Foot4 => effects::foot::FOOT4,
                EffectId::Foot5 => effects::foot::FOOT5,
                _ => effects::foot::FOOT6,
            };
            Box::new(effects::foot::FootEffect::new(from, to, params))
        }

        // Teihit streak-burst family — radial streaks. Teihit2 /
        // Backstap use the different teihit2 spray and are not yet done.
        EffectId::Teihit1 => {
            Box::new(effects::teihit::TeihitEffect::new(anchor.point(), effects::teihit::TEIHIT1))
        }
        EffectId::Teihit1x => {
            Box::new(effects::teihit::TeihitEffect::new(anchor.point(), effects::teihit::TEIHIT1X))
        }
        EffectId::Teihit3 => {
            Box::new(effects::teihit::TeihitEffect::new(anchor.point(), effects::teihit::TEIHIT3))
        }

        // ParticleUp family — rising sparkle bursts. (Firstaid uses a
        // different recipe and is not yet done.)
        EffectId::Hptime => {
            Box::new(effects::particle_up::ParticleUpEffect::new(anchor.point(), effects::particle_up::HPTIME))
        }
        EffectId::Sptime => {
            Box::new(effects::particle_up::ParticleUpEffect::new(anchor.point(), effects::particle_up::SPTIME))
        }
        EffectId::Hated => {
            Box::new(effects::particle_up::ParticleUpEffect::new(anchor.point(), effects::particle_up::HATED))
        }
        EffectId::Hated2 => {
            Box::new(effects::particle_up::ParticleUpEffect::new(anchor.point(), effects::particle_up::HATED2))
        }
        EffectId::SmaReady => {
            Box::new(effects::particle_up::ParticleUpEffect::new(anchor.point(), effects::particle_up::SMAREADY))
        }
        EffectId::Sprinklesand => {
            Box::new(effects::particle_up::ParticleUpEffect::new(anchor.point(), effects::particle_up::SPRINKLESAND))
        }

        // EffectTextureSet flat ground-texture effects.
        EffectId::Hittexture => {
            Box::new(effects::effect_texture::EffectTextureEffect::new(anchor.point(), effects::effect_texture::HITTEXTURE))
        }

        // Camera-facing tarot cards + the slow-cast clock — same alpha
        // curve, one texture each.
        EffectId::Tarotcard1 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(0))),
        EffectId::Tarotcard2 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(1))),
        EffectId::Tarotcard3 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(2))),
        EffectId::Tarotcard4 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(3))),
        EffectId::Tarotcard5 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(4))),
        EffectId::Tarotcard6 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(5))),
        EffectId::Tarotcard7 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(6))),
        EffectId::Tarotcard8 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(7))),
        EffectId::Tarotcard9 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(8))),
        EffectId::Tarotcard10 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(9))),
        EffectId::Tarotcard11 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(10))),
        EffectId::Tarotcard12 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(11))),
        EffectId::Tarotcard13 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(12))),
        EffectId::Tarotcard14 => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::tarot_params(13))),
        EffectId::NpcSlowcast => Box::new(effects::tarot_card::TarotCardEffect::new(anchor.point(), effects::tarot_card::NPC_SLOWCAST)),

        // Status-overlay family (BLIND / POISON): camera-locked full-viewport
        // tint via one FullscreenOverlay quad. Devil1-10 share the BLIND-level
        // params (the original game only varies the vignette zoom across them).
        EffectId::Blind => Box::new(effects::fullscreen_overlay::FullscreenOverlayEffect::new(anchor.point(), effects::fullscreen_overlay::BLIND)),
        EffectId::Devil1
        | EffectId::Devil2
        | EffectId::Devil3
        | EffectId::Devil4
        | EffectId::Devil5
        | EffectId::Devil6
        | EffectId::Devil7
        | EffectId::Devil8
        | EffectId::Devil9
        | EffectId::Devil10 => Box::new(effects::fullscreen_overlay::FullscreenOverlayEffect::new(anchor.point(), effects::fullscreen_overlay::DEVIL)),
        EffectId::DevilRed => Box::new(effects::fullscreen_overlay::FullscreenOverlayEffect::new(anchor.point(), effects::fullscreen_overlay::DEVIL_RED)),
        EffectId::Poison => Box::new(effects::fullscreen_overlay::FullscreenOverlayEffect::new(anchor.point(), effects::fullscreen_overlay::POISON)),
        EffectId::Bleeding => Box::new(effects::fullscreen_overlay::FullscreenOverlayEffect::new(anchor.point(), effects::fullscreen_overlay::BLEEDING)),
        EffectId::CrystalBlue => Box::new(effects::fullscreen_overlay::FullscreenOverlayEffect::new(anchor.point(), effects::fullscreen_overlay::CRYSTAL_BLUE)),

        // Camera-facing result banners above the caster.
        EffectId::TempOk => {
            Box::new(effects::temp_result::TempResultEffect::new(anchor.point(), effects::temp_result::TEMP_OK))
        }
        EffectId::TempFail => {
            Box::new(effects::temp_result::TempResultEffect::new(anchor.point(), effects::temp_result::TEMP_FAIL))
        }

        // ForestLight family: faint green pentagonal light-beam columns
        // rising above the caster. One struct, per-variant params. ItemLight
        // fades and self-terminates; the four Forestlight ids are
        // persistent ambient beams.
        EffectId::ItemLight => Box::new(effects::forest_light::ForestLightEffect::new(
            anchor.point(),
            effects::forest_light::ITEM_LIGHT,
        )),
        EffectId::Forestlight => Box::new(effects::forest_light::ForestLightEffect::new(
            anchor.point(),
            effects::forest_light::FORESTLIGHT,
        )),
        EffectId::Forestlight2 => Box::new(effects::forest_light::ForestLightEffect::new(
            anchor.point(),
            effects::forest_light::FORESTLIGHT2,
        )),
        EffectId::Forestlight3 => Box::new(effects::forest_light::ForestLightEffect::new(
            anchor.point(),
            effects::forest_light::FORESTLIGHT3,
        )),
        EffectId::Forestlight4 => Box::new(effects::forest_light::ForestLightEffect::new(
            anchor.point(),
            effects::forest_light::FORESTLIGHT4,
        )),

        // Wink / Fvoice: directional emotes that
        // pick their fly-off action from the camera angle, so they're Custom
        // effects, not `spr_def`. Same handler, different sprite.
        EffectId::Wink => Box::new(effects::wink::WinkEffect::new(anchor.point(), effects::wink::WINK)),
        EffectId::Fvoice => Box::new(effects::wink::WinkEffect::new(anchor.point(), effects::wink::FVOICE)),

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
        EffectId::Grimtooth => {
            // The travelling small-spike trail — reuses FrostDiver's
            // projectile with stone.bmp.
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::frost_diver::FrostDiverEffect::new(
                from, to, effects::frost_diver::GRIMTOOTH,
            ))
        }
        EffectId::Grimtoothatk => {
            Box::new(effects::grimtooth_atk::GrimToothAtkEffect::new(anchor.point()))
        }
        EffectId::Earthspike => {
            Box::new(effects::earthspike::EarthSpikeEffect::new(anchor.point(), effects::earthspike::EARTHSPIKE))
        }
        EffectId::Hyousensou => {
            Box::new(effects::earthspike::EarthSpikeEffect::new(anchor.point(), effects::earthspike::HYOUSENSOU))
        }
        EffectId::Icewall => {
            // `to` supplies the cast direction so the wall stands across the
            // targeted line; collapses to a default-oriented row at a point.
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::icewall::IceWallEffect::new(from, to))
        }
        EffectId::Soulstrike => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::soul_strike::SoulStrikeEffect::new(from, to, hit_count.unwrap_or(1)))
        }
        EffectId::Blooddrain => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::energy_drain::DrainEffect::new(from, to, effects::energy_drain::BLOOD_DRAIN))
        }
        EffectId::Energydrain => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::energy_drain::DrainEffect::new(from, to, effects::energy_drain::ENERGY_DRAIN))
        }
        EffectId::Energydrain2 => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::energy_drain::DrainEffect::new(from, to, effects::energy_drain::ENERGY_DRAIN2))
        }
        EffectId::Energydrain3 => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::energy_drain::DrainEffect::new(from, to, effects::energy_drain::ENERGY_DRAIN3))
        }
        EffectId::Magnumbreak => {
            Box::new(effects::magnum_break::MagnumBreakEffect::new(anchor.point()))
        }

        // Throw Item family — ballistic-arc item projectiles. `from`/`to`
        // give the caster→target heading; one struct, per-variant params.
        // Throwitem4 is a composite (two staggered projectiles).
        EffectId::Throwitem
        | EffectId::Throwitem2
        | EffectId::Throwitem3
        | EffectId::Throwitem4
        | EffectId::Throwitem5
        | EffectId::Throwitem6
        | EffectId::Throwitem7
        | EffectId::Throwitem8
        | EffectId::Throwitem9
        | EffectId::Throwitem10 => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            use effects::throw_item as ti;
            let variants: &[ti::ThrowItemParams] = match id {
                EffectId::Throwitem => &[ti::THROW_BOTTLES],
                EffectId::Throwitem2 => &[ti::THROW_ITEM2],
                EffectId::Throwitem3 => &[ti::THROW_STONE],
                EffectId::Throwitem4 => &[ti::THROW_MOLOTOV, ti::THROW_ITEM3],
                EffectId::Throwitem5 => &[ti::THROW_ITEM4],
                EffectId::Throwitem6 => &[ti::THROW_ITEM6],
                EffectId::Throwitem7 => &[ti::THROW_ITEM7],
                EffectId::Throwitem8 => &[ti::THROW_ITEM8],
                EffectId::Throwitem9 => &[ti::THROW_ITEM9],
                _ => &[ti::THROW_COIN],
            };
            Box::new(ti::ThrowItemEffect::new(from, to, variants))
        }

        // Chemical streak family — radial wedges (Protection, point-anchored)
        // and caster→target streak lines (Chemical2/3/4, dash, Smatk).
        EffectId::Chemicalprotection => {
            Box::new(effects::chemical::ChemicalEffect::new(anchor.point(), effects::chemical::CHEMICALPROTECTION))
        }
        EffectId::Mgattack2 => {
            Box::new(effects::chemical::ChemicalEffect::new(anchor.point(), effects::chemical::MGATTACK2))
        }
        EffectId::Chemical2
        | EffectId::Chemical2dash
        | EffectId::Chemical3
        | EffectId::Chemical4
        | EffectId::Smatk1
        | EffectId::Smatk2
        | EffectId::Smatk3
        | EffectId::Smatk4 => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            use effects::chemical as ch;
            let params = match id {
                EffectId::Chemical2 => ch::CHEMICAL2,
                EffectId::Chemical2dash => ch::CHEMICAL2DASH,
                EffectId::Chemical3 => ch::CHEMICAL3,
                EffectId::Chemical4 => ch::CHEMICAL4,
                EffectId::Smatk1 => ch::SMATK1,
                EffectId::Smatk2 => ch::SMATK2,
                EffectId::Smatk3 => ch::SMATK3,
                _ => ch::SMATK4,
            };
            Box::new(ch::ChemicalEffect::new_dir(from, to, params))
        }

        // STIN wind-card family — flying spinning cards with a motion trail.
        // Stin/Stin2/Stin4 aim along the caster→target heading (trail anchor).
        EffectId::Stin
        | EffectId::Stin2
        | EffectId::Stin4 => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            use effects::stin as st;
            let params = match id {
                EffectId::Stin => st::STIN,
                EffectId::Stin2 => st::STIN2,
                _ => st::STIN4,
            };
            Box::new(st::StinEffect::new(from, to, params))
        }
        EffectId::Stin5 => Box::new(effects::stin::StinEffect::new(
            anchor.point(),
            anchor.point(),
            effects::stin::STIN5,
        )),

        // SMA wind-spiral family — travelling emitter (Sma/Stin3) + the
        // standalone rising spiral ribbon (Sma2) + particle path (Sma3).
        EffectId::Sma | EffectId::Stin3 => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            let kind = if matches!(id, EffectId::Stin3) {
                effects::sma::SmaKind::Particles
            } else {
                effects::sma::SmaKind::Bands
            };
            Box::new(effects::sma::SmaEffect::new(from, to, kind))
        }
        EffectId::Sma2 => Box::new(effects::sma::Sma2Effect::new(anchor.point())),
        EffectId::Sma3 => Box::new(effects::particle_up::ParticleUpEffect::new(
            anchor.point(),
            effects::particle_up::SMA3,
        )),

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
        // EF_SONICBLOWHIT — single horizontal cone yawed along the strike
        // heading. Trail anchor carries caster→target so
        // the cone aims correctly; single-point anchor falls back to 0°.
        EffectId::Sonicblowhit => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::sonicblowhit::SonicBlowHitEffect::new_with_trail(from, to))
        }
        // EF_CARTREVOLUTION — twin ground ring + sphere burst, with the
        // `CartRevolution.str` overlay playing alongside (Hybrid).
        EffectId::Cartrevolution => {
            Box::new(effects::cartrevolution::CartRevolutionEffect::new(anchor.point()))
        }

        // STR-C hybrids — each builds the primitive layer the STR file
        // alone is missing, and re-declares its STR via `str_overlay()`.
        // EF_GLASSWALL2 — pink rising column + SafetyWall.str.
        EffectId::Glasswall2 => {
            Box::new(effects::glasswall2::Glasswall2Effect::new(anchor.point()))
        }
        // EF_PROVIDENCE — light funnel column + providence.str angel.
        EffectId::Providence => {
            Box::new(effects::providence::ProvidenceEffect::new(anchor.point()))
        }
        // MAPPILLAR family — rotating ring columns (batch 8).
        EffectId::Mappillar => Box::new(effects::mappillar::MappillarEffect::new(
            anchor.point(),
            effects::mappillar::MAPPILLAR,
        )),
        EffectId::Mappillar2 => Box::new(effects::mappillar::MappillarEffect::new(
            anchor.point(),
            effects::mappillar::MAPPILLAR2,
        )),
        EffectId::Mappillar3 => Box::new(effects::mappillar::MappillarEffect::new(
            anchor.point(),
            effects::mappillar::MAPPILLAR3,
        )),
        EffectId::Mappillar4 => Box::new(effects::mappillar::MappillarEffect::new(
            anchor.point(),
            effects::mappillar::MAPPILLAR4,
        )),
        // EF_KOUENKA — sakura sprite scatter + firehit.str.
        EffectId::Kouenka => {
            Box::new(effects::kouenka::KouenkaEffect::new(anchor.point()))
        }
        // EF_NAPALMVALCAN — five timed Hit2 bursts.
        EffectId::Napalmvalcan => {
            Box::new(effects::napalmvalcan::NapalmValcanEffect::new(anchor.point()))
        }
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
        // EF_GUMGANG2 — vertical pillar of light. Reusing the volcano
        // effect produces the wrong silhouette here: its per-frame
        // flame-blade sine envelope makes it look like Gumgang3 with the
        // flame wreath rather than a clean column. A dedicated
        // cylinder-stack impl matches the reference gif's clean vertical
        // column instead, so we don't share the volcano code for this
        // id.
        EffectId::Gumgang2 => Box::new(effects::gumgang2::Gumgang2Effect::new(anchor.point())),

        EffectId::Defender => Box::new(effects::defender::DefenderEffect::new(anchor.point())),

        EffectId::Wind => Box::new(effects::wind::WindEffect::new(anchor.point())),

        EffectId::Bash3d => Box::new(effects::bash3d::Bash3dEffect::new(
            anchor.point(),
            effects::bash3d::BASH3D,
        )),
        EffectId::Bash3d2 => Box::new(effects::bash3d::Bash3dEffect::new(
            anchor.point(),
            effects::bash3d::BASH3D2,
        )),
        EffectId::Bash3d3 => Box::new(effects::bash3d::Bash3dEffect::new(
            anchor.point(),
            effects::bash3d::BASH3D3,
        )),
        EffectId::Bash3d4 => Box::new(effects::bash3d::Bash3dEffect::new(
            anchor.point(),
            effects::bash3d::BASH3D4,
        )),
        EffectId::Bash3d5 => Box::new(effects::bash3d::Bash3dEffect::new(
            anchor.point(),
            effects::bash3d::BASH3D5,
        )),

        // LEVEL99 aura family — each EF_LEVEL99* is a distinct primitive layer
        // the server composes together (ring + halo + sparkles), not a size
        // variant of one billboard.
        EffectId::Level99 => Box::new(effects::casting_ring::CastingRingEffect::new(
            anchor.point(),
            effects::casting_ring::LV99,
        )),
        EffectId::Level995 => Box::new(effects::casting_ring::CastingRingEffect::new(
            anchor.point(),
            effects::casting_ring::LV995,
        )),
        EffectId::Level992 => Box::new(effects::floor_aura::FloorAuraEffect::new(
            anchor.point(),
            effects::floor_aura::LV99_BLUE,
        )),
        EffectId::Level996 => Box::new(effects::floor_aura::FloorAuraEffect::new(
            anchor.point(),
            effects::floor_aura::LV99_GREEN,
        )),
        EffectId::Level993 => Box::new(effects::sparkle_column::SparkleColumnEffect::new(
            anchor.point(),
            effects::sparkle_column::FREEZING,
        )),
        EffectId::Level994 => Box::new(effects::sparkle_column::SparkleColumnEffect::new(
            anchor.point(),
            effects::sparkle_column::WHITELIGHT,
        )),
        EffectId::MapGhost => Box::new(effects::sparkle_column::SparkleColumnEffect::new(
            anchor.point(),
            effects::sparkle_column::GHOST,
        )),

        EffectId::Beginspell => Box::new(effects::begin_spell::BeginSpellEffect::new(anchor.point())),
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
        EffectId::Beginasura => Box::new(effects::begin_asura::BeginAsuraEffect::base(
            anchor.point(),
        )),
        EffectId::Beginasura1 => Box::new(effects::begin_asura::BeginAsuraEffect::elemental(
            anchor.point(),
            0,
        )),
        EffectId::Beginasura2 => Box::new(effects::begin_asura::BeginAsuraEffect::elemental(
            anchor.point(),
            1,
        )),
        EffectId::Beginasura3 => Box::new(effects::begin_asura::BeginAsuraEffect::elemental(
            anchor.point(),
            2,
        )),
        EffectId::Beginasura4 => Box::new(effects::begin_asura::BeginAsuraEffect::elemental(
            anchor.point(),
            3,
        )),
        EffectId::Beginasura5 => Box::new(effects::begin_asura::BeginAsuraEffect::elemental(
            anchor.point(),
            4,
        )),
        EffectId::Beginasura6 => Box::new(effects::begin_asura::BeginAsuraEffect::elemental(
            anchor.point(),
            5,
        )),
        EffectId::Beginasura7 => Box::new(effects::begin_asura::BeginAsuraEffect::elemental(
            anchor.point(),
            6,
        )),
        EffectId::Beginasura11 => Box::new(effects::begin_asura::BeginAsuraEffect::champion(
            anchor.point(),
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

        // Bottom volcano — single-slot radial particle column rising at
        // 80°. NOT a BottomSong variant despite the BottomVo/De/Vi/Suiton
        // naming: in the original game these ids render as the rising
        // volcano column, not a ground-song disc, so they get their own
        // effect here.
        EffectId::BottomVo => Box::new(effects::bottom_volcano::BottomVolcanoEffect::new(
            anchor.point(),
            effects::bottom_volcano::VOLCANO_RED,
        )),
        EffectId::BottomDe => Box::new(effects::bottom_volcano::BottomVolcanoEffect::new(
            anchor.point(),
            effects::bottom_volcano::VOLCANO_BLUE,
        )),
        EffectId::BottomVi => Box::new(effects::bottom_volcano::BottomVolcanoEffect::new(
            anchor.point(),
            effects::bottom_volcano::VOLCANO_GREEN,
        )),
        EffectId::BottomSuiton => Box::new(effects::bottom_volcano::BottomVolcanoEffect::new(
            anchor.point(),
            effects::bottom_volcano::SUITON,
        )),

        // Basilica — two stacked layers producing 8 cells of layered
        // square pillars. Distinct from the BottomMagnus square pillar,
        // which is a single 4-sided pillar.
        EffectId::BottomBasilica => {
            Box::new(effects::basilica::BasilicaEffect::new(anchor.point()))
        }

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

        // Batch CYL — cylinder effects.
        EffectId::Potionpillar => Box::new(effects::potion_pillar::PotionPillarEffect::new(
            anchor.point(),
            effects::potion_pillar::DEFAULT,
        )),
        EffectId::Revive => Box::new(effects::revive::ReviveEffect::new(anchor.point())),
        // Pierce reads caster→target direction to aim the horizontal cone.
        // Trail anchor carries both endpoints; a Point anchor collapses
        // to from == to and the effect falls back to a fixed compass.
        // `hit_count` carries the skill level (1..=10): N bursts spaced
        // 20 frames apart, each with its own particle storm after the
        // first.
        EffectId::Pierce => {
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::pierce::PierceEffect::new_with_level(
                from,
                to,
                hit_count.unwrap_or(1),
            ))
        }
        EffectId::PotionBerserk => Box::new(
            effects::potion_berserk::PotionBerserkEffect::new(anchor.point()),
        ),

        // Batch GD — GroundDisc decals.
        EffectId::Bowlingbash => {
            // Trail anchor's `to` aims the two swept cylinder slashes
            // along the caster→target direction. Single-point anchors
            // collapse to `from == to` and the slashes fall back to a
            // default facing.
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::bowling_bash::BowlingBashEffect::new_with_direction(from, to))
        }
        EffectId::Dragonsmoke => {
            // Trail anchor: `from` is the chimney source, `to - from`
            // sets the wind direction so the smoke column curves.
            // Single-point anchors collapse to a vertical rise.
            let (from, to) = match anchor {
                EffectAnchor::Trail { from, to } => (from, to),
                EffectAnchor::Point(p) => (p, p),
            };
            Box::new(effects::dragonsmoke::DragonsmokeEffect::new(from, to))
        }
        EffectId::Overthrust | EffectId::Sonicblow => {
            Box::new(effects::overthrust::OverthrustEffect::new(anchor.point()))
        }
        EffectId::Callzone => {
            Box::new(effects::callzone::CallzoneEffect::new(anchor.point()))
        }
        EffectId::Groundsample => {
            Box::new(effects::ground_sample::GroundSampleEffect::new(anchor.point()))
        }

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
            | EffectId::Bash
            | EffectId::Hasteup
            | EffectId::Flasher
            | EffectId::Blessing
            | EffectId::Endure
            | EffectId::Enhance
            | EffectId::Entry
            | EffectId::Exit
            | EffectId::Glasswall
            | EffectId::Healsp
            | EffectId::Portal
            | EffectId::Portal2
            | EffectId::Portal3
            | EffectId::Portal4
            | EffectId::Portal5
            | EffectId::Spraypond
            | EffectId::Firearrow
            | EffectId::Fireball
            | EffectId::Napalmbeat
            | EffectId::Sandwind
            | EffectId::Frostdiver
            | EffectId::Frostdiver2
            | EffectId::Soulstrike
            | EffectId::Yufitel
            | EffectId::Blitzbeat
            | EffectId::Waterball
            | EffectId::Fireivy
            | EffectId::Detecting
            | EffectId::Toprank
            | EffectId::Party
            | EffectId::Curseattack
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
            | EffectId::Sonicblowhit
            | EffectId::Cartrevolution
            | EffectId::Glasswall2
            | EffectId::Providence
            | EffectId::Kouenka
            | EffectId::Napalmvalcan
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
            | EffectId::Gumgang2
            | EffectId::Defender
            | EffectId::Wind
            | EffectId::Bash3d
            | EffectId::Bash3d2
            | EffectId::Bash3d3
            | EffectId::Bash3d4
            | EffectId::Bash3d5
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
            | EffectId::BottomVo
            | EffectId::BottomDe
            | EffectId::BottomVi
            | EffectId::BottomSuiton
            | EffectId::BottomBasilica
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
            | EffectId::Potionpillar
            | EffectId::Revive
            | EffectId::Pierce
            | EffectId::PotionBerserk
            | EffectId::ItemLight
            | EffectId::Forestlight
            | EffectId::Forestlight2
            | EffectId::Forestlight3
            | EffectId::Forestlight4
            | EffectId::Wink
            | EffectId::Fvoice
            | EffectId::TempOk
            | EffectId::TempFail
            | EffectId::Tarotcard1
            | EffectId::Tarotcard2
            | EffectId::Tarotcard3
            | EffectId::Tarotcard4
            | EffectId::Tarotcard5
            | EffectId::Tarotcard6
            | EffectId::Tarotcard7
            | EffectId::Tarotcard8
            | EffectId::Tarotcard9
            | EffectId::Tarotcard10
            | EffectId::Tarotcard11
            | EffectId::Tarotcard12
            | EffectId::Tarotcard13
            | EffectId::Tarotcard14
            | EffectId::NpcSlowcast
            | EffectId::Hyousensou
            | EffectId::Earthspike
            | EffectId::Bowlingbash
            | EffectId::Overthrust
            | EffectId::Callzone
            | EffectId::Groundsample
            | EffectId::Flowercast
            | EffectId::Yufitel2
            | EffectId::TextureFalling
            | EffectId::Aciddemon
            | EffectId::Rainbow
            | EffectId::Agiup
            | EffectId::Lightsphere
            | EffectId::Lightsphere2
            | EffectId::MapMagiczone
            | EffectId::MapMagiczone2
            | EffectId::Glow4
            | EffectId::Quakebody
            | EffectId::Quakebody2
            | EffectId::Quakebody3
            | EffectId::Quakebody4
            | EffectId::Twohandquicken
            | EffectId::Spearquicken
            | EffectId::Lkconcentration
            | EffectId::Mappillar
            | EffectId::Mappillar2
            | EffectId::Mappillar3
            | EffectId::Mappillar4
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_dispatches() {
        let e = make_effect(EffectId::Warp, EffectAnchor::Point([0.0; 3]), None);
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
            None,
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
        assert!(make_effect(EffectId::Stormkick, EffectAnchor::Point([0.0; 3]), None).is_some());
        assert!(!is_real_impl(EffectId::Stormkick));
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
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
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
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
            assert_eq!(e.str_overlay(), None);
        }
    }

    #[test]
    fn bottom_hermode_dispatches_to_world_quad_cube() {
        assert!(is_real_impl(EffectId::BottomHermode));
        let e = make_effect(EffectId::BottomHermode, EffectAnchor::Point([0.0; 3]), None).unwrap();
        assert_eq!(e.str_overlay(), None);
    }

    #[test]
    fn bottom_rokisweil_dispatches_to_billboard_pulse() {
        assert!(is_real_impl(EffectId::BottomRokisweil));
        let e = make_effect(EffectId::BottomRokisweil, EffectAnchor::Point([0.0; 3]), None).unwrap();
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
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
            assert_eq!(e.str_overlay(), None);
        }
    }

    #[test]
    fn bottom_light_variants_dispatch_to_world_quad_curtain() {
        // 2 BottomLight ids must land on the BottomLight custom effect
        // (WorldQuad ribbon segments), not the placeholder.
        for id in [EffectId::BottomEternalchaos, EffectId::BottomSiegfried] {
            assert!(is_real_impl(id), "{:?} must have a real impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
            assert_eq!(e.str_overlay(), None);
        }
    }

    #[test]
    fn bottom_magnus_variants_dispatch_to_frustum_pillar() {
        // BottomMag + BottomFogwall must land on the BottomMagnus
        // custom effect (Frustum sides=4), not the placeholder.
        for id in [EffectId::BottomMag, EffectId::BottomFogwall] {
            assert!(is_real_impl(id), "{:?} must have a real impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
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
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
            assert_eq!(
                e.str_overlay(),
                None,
                "{:?} is pure custom, no STR overlay",
                id
            );
        }
    }

    #[test]
    fn bottom_volcano_and_basilica_dispatch_to_real_impl() {
        // Sociable: the 5 ids historically misclassified as BottomSong
        // (their on-screen look is the rising volcano / basilica pillars)
        // must route to BottomVolcano / Basilica, not the pink placeholder, with no
        // STR overlay attached. The spec must also resolve to Custom —
        // a stray `str_aliases` entry would shadow the factory dispatch
        // and the holder would try to load a non-existent .str file
        // instead of running our effect.
        use super::super::spec::{EffectAnchor, EffectSpec};
        use super::super::table::effect_spec;
        for id in [
            EffectId::BottomVo,
            EffectId::BottomDe,
            EffectId::BottomVi,
            EffectId::BottomSuiton,
            EffectId::BottomBasilica,
        ] {
            assert!(is_real_impl(id), "{:?} must have a real factory impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
            assert_eq!(
                e.str_overlay(),
                None,
                "{:?} is pure custom, no STR overlay",
                id
            );
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom { .. })),
                "{:?} spec must be Custom, got {:?}",
                id,
                effect_spec(id),
            );
        }
    }

    #[test]
    fn tarot_and_slowcast_dispatch_to_custom_billboards() {
        // EffectTextureSet billboards: each must route to a real impl with no
        // STR overlay, and the spec must resolve to Custom. A leftover
        // `str_aliases` guess (`"tarotcard1"`, `"npc_slowcast"`) would shadow
        // the factory dispatch and the holder would chase a missing .str.
        use super::super::spec::{EffectAnchor, EffectSpec};
        use super::super::table::effect_spec;
        for id in [
            EffectId::Tarotcard1,
            EffectId::Tarotcard14,
            EffectId::NpcSlowcast,
        ] {
            assert!(is_real_impl(id), "{:?} must have a real factory impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
            assert_eq!(e.str_overlay(), None, "{:?} has no STR overlay", id);
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom { .. })),
                "{:?} spec must be Custom, got {:?}",
                id,
                effect_spec(id),
            );
        }
    }

    #[test]
    fn mappillar_family_dispatches_to_custom_without_str() {
        // MAPPILLAR rotating ring columns: pure procedural, no STR file in
        // the classic GRF. Their `mappillar*` str_alias entries would shadow
        // the factory dispatch and the holder would chase a missing
        // `mappillar.str`, so the spec must resolve to Custom.
        use super::super::spec::{EffectAnchor, EffectSpec};
        use super::super::table::effect_spec;
        for id in [
            EffectId::Mappillar,
            EffectId::Mappillar2,
            EffectId::Mappillar3,
            EffectId::Mappillar4,
        ] {
            assert!(is_real_impl(id), "{:?} must have a real factory impl", id);
            let e = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
            assert_eq!(e.str_overlay(), None, "{:?} has no STR overlay", id);
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom { .. })),
                "{:?} spec must be Custom, got {:?}",
                id,
                effect_spec(id),
            );
        }
    }

    #[test]
    fn batch_fh_dispatches_to_real_impls() {
        // Sociable: the 5 effects landed in Batch FH must each route to
        // a non-placeholder and report is_real_impl. Hamicastle's spec
        // is SPR (resolved via spr_def in table.rs); the other 4 are
        // Custom-dispatched. Cartrevolution is in is_hybrid() so its
        // str_overlay must be present.
        use super::super::spec::EffectAnchor;
        for id in [
            EffectId::Sonicblowhit,
            EffectId::Cartrevolution,
            EffectId::Gumgang2,
            EffectId::Napalmvalcan,
        ] {
            assert!(is_real_impl(id), "{:?} must have a real factory impl", id);
            let _ = make_effect(id, EffectAnchor::Point([0.0; 3]), None).unwrap();
        }
        // Cartrevolution still emits its STR overlay so the holder plays
        // CartRevolution.str alongside the primitive bursts.
        let cart =
            make_effect(EffectId::Cartrevolution, EffectAnchor::Point([0.0; 3]), None).unwrap();
        assert_eq!(cart.str_overlay(), Some("CartRevolution"));
        // Hamicastle is SPR-driven; spec must resolve via the bucket
        // default, not the Custom factory path.
        use super::super::spec::EffectSpec;
        use super::super::table::effect_spec;
        assert!(matches!(effect_spec(EffectId::Hamicastle), Some(EffectSpec::Spr { .. })));
    }

    #[test]
    fn hybrid_placeholder_carries_str_overlay() {
        // Coin is a StrHybrid id with no real impl — factory routes it
        // through `HybridPlaceholderEffect` so its STR file still plays.
        let e = make_effect(EffectId::Coin, EffectAnchor::Point([0.0; 3]), None).unwrap();
        assert_eq!(e.str_overlay(), Some(str_aliases(EffectId::Coin)[0]));
    }

    #[test]
    fn soulstrike_dispatches_with_trail_and_hit_count() {
        assert!(is_real_impl(EffectId::Soulstrike));
        let e = make_effect(
            EffectId::Soulstrike,
            EffectAnchor::Trail {
                from: [0.0, 0.0, 0.0],
                to: [0.0, 0.0, 60.0],
            },
            Some(3),
        )
        .unwrap();
        assert_eq!(e.str_overlay(), None);
    }
}
